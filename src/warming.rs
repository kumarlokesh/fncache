//! Background cache warming implementation.
//!
//! This module provides functionality for proactively loading or refreshing cache entries
//! before they're needed, reducing latency for frequently accessed items.

use crate::backends::CacheBackend;
use crate::error::Error;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::BoxFuture;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Represents a cache warmer that can preload or refresh cache entries.
pub struct CacheWarmer<B: CacheBackend + Send + Sync + 'static> {
    /// The cache backend to warm
    backend: Arc<B>,
    /// Registered warming functions mapped to their keys
    warmers: Arc<Mutex<HashMap<String, WarmingEntry>>>,
    /// Task handles for active warming tasks
    tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

/// A function that can generate a value for a cache key
pub type WarmingFn = Arc<dyn Fn() -> Result<Vec<u8>, Error> + Send + Sync>;

/// An async function that can generate a value for a cache key
pub type AsyncWarmingFn = Arc<dyn Fn() -> BoxFuture<'static, Result<Vec<u8>, Error>> + Send + Sync>;

/// Entry for a warming function and its configuration
struct WarmingEntry {
    /// The warming function
    warmer_fn: WarmingType,
    /// Time-to-live for the cached entry
    ttl: Option<Duration>,
    /// Interval at which to refresh the entry
    refresh_interval: Duration,
    /// Last time the entry was refreshed
    last_refreshed: Option<Instant>,
}

/// Type of warming function - sync or async
#[derive(Clone)]
enum WarmingType {
    /// Synchronous warming function
    Sync(WarmingFn),
    /// Asynchronous warming function
    Async(AsyncWarmingFn),
}

impl<B> Clone for CacheWarmer<B>
where
    B: CacheBackend + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            warmers: self.warmers.clone(),
            tasks: self.tasks.clone(),
        }
    }
}

impl<B> CacheWarmer<B>
where
    B: CacheBackend + Send + Sync + 'static,
{
    /// Create a new cache warmer for the given backend
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            warmers: Arc::new(Mutex::new(HashMap::new())),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a synchronous warming function for a cache key
    pub async fn register_warmer(
        &self,
        key: &str,
        warmer_fn: WarmingFn,
        ttl: Option<Duration>,
        refresh_interval: Duration,
    ) -> Result<(), Error> {
        let mut warmers = self.warmers.lock().await;

        warmers.insert(
            key.to_string(),
            WarmingEntry {
                warmer_fn: WarmingType::Sync(warmer_fn),
                ttl,
                refresh_interval,
                last_refreshed: None,
            },
        );

        Ok(())
    }

    /// Register an asynchronous warming function for a cache key
    pub async fn register_async_warmer(
        &self,
        key: &str,
        warmer_fn: AsyncWarmingFn,
        ttl: Option<Duration>,
        refresh_interval: Duration,
    ) -> Result<(), Error> {
        let mut warmers = self.warmers.lock().await;

        warmers.insert(
            key.to_string(),
            WarmingEntry {
                warmer_fn: WarmingType::Async(warmer_fn),
                ttl,
                refresh_interval,
                last_refreshed: None,
            },
        );

        Ok(())
    }

    /// Manually warm a specific cache key
    pub async fn warm(&self, key: &str) -> Result<(), Error> {
        let (has_key, ttl) = {
            let warmers = self.warmers.lock().await;
            match warmers.get(key) {
                Some(entry) => (true, entry.ttl),
                None => (false, None),
            }
        };

        if !has_key {
            return Err(Error::KeyNotFound);
        }

        let value = self.execute_warmer(key).await?;
        self.backend.set(key.to_string(), value, ttl).await?;

        {
            let mut warmers = self.warmers.lock().await;
            if let Some(entry) = warmers.get_mut(key) {
                entry.last_refreshed = Some(Instant::now());
            }
        }

        Ok(())
    }

    /// Execute a warming function for a key
    async fn execute_warmer(&self, key: &str) -> Result<Vec<u8>, Error> {
        let warmer_type = {
            let warmers = self.warmers.lock().await;
            match warmers.get(key) {
                Some(entry) => match &entry.warmer_fn {
                    WarmingType::Sync(f) => Some(WarmingType::Sync(f.clone())),
                    WarmingType::Async(f) => Some(WarmingType::Async(f.clone())),
                },
                None => None,
            }
        };

        match warmer_type {
            Some(WarmingType::Sync(f)) => f(),
            Some(WarmingType::Async(f)) => f().await,
            None => Err(Error::KeyNotFound),
        }
    }

    /// Start background warming for all registered keys
    pub async fn start_warming(&self) -> Result<(), Error> {
        let keys_to_warm = {
            let warmers = self.warmers.lock().await;
            warmers.keys().cloned().collect::<Vec<_>>()
        };

        let mut tasks = self.tasks.lock().await;

        for key in keys_to_warm {
            if !tasks.contains_key(&key) {
                let key_clone = key.to_owned();
                let warmer_self_clone = self.clone();
                let warmers_clone = self.warmers.clone();
                let handle = tokio::spawn(async move {
                    loop {
                        let refresh_interval =
                            match Self::get_refresh_interval(&warmers_clone, &key_clone).await {
                                Some(interval) => interval,
                                None => break,
                            };

                        sleep(refresh_interval).await;

                        match warmer_self_clone.warm(&key_clone).await {
                            Ok(_) => {}
                            Err(e) => {
                                if matches!(e, Error::KeyNotFound) {
                                    break;
                                }
                                // Other error, log but continue
                                // TODO: Add metrics for failed warming
                            }
                        }
                    }
                });

                tasks.insert(key, handle);
            }
        }

        Ok(())
    }

    /// Helper method to safely get refresh interval for a key
    async fn get_refresh_interval(
        warmers: &Arc<Mutex<HashMap<String, WarmingEntry>>>,
        key: &str,
    ) -> Option<Duration> {
        let warmers_guard = warmers.lock().await;
        warmers_guard.get(key).map(|entry| entry.refresh_interval)
    }

    /// Stop background warming for a specific key
    pub async fn stop_warming(&self, key: &str) -> Result<(), Error> {
        let mut tasks = self.tasks.lock().await;

        if let Some(handle) = tasks.remove(key) {
            handle.abort();
        }

        Ok(())
    }

    /// Stop all background warming
    pub async fn stop_all_warming(&self) -> Result<(), Error> {
        let mut tasks = self.tasks.lock().await;

        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        Ok(())
    }

    /// Get the last time a key was refreshed
    pub async fn last_refreshed(&self, key: &str) -> Result<Option<Instant>, Error> {
        let warmers = self.warmers.lock().await;

        if let Some(entry) = warmers.get(key) {
            Ok(entry.last_refreshed)
        } else {
            Err(Error::KeyNotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::memory::MemoryBackend;

    #[tokio::test]
    async fn test_sync_warming() {
        let backend = MemoryBackend::new();
        let warmer = CacheWarmer::new(backend);

        warmer
            .register_warmer(
                "test_key",
                Arc::new(|| Ok(vec![1, 2, 3])),
                None,
                Duration::from_secs(1),
            )
            .await
            .unwrap();

        warmer.warm("test_key").await.unwrap();

        let backend_ref = warmer.backend.clone();
        let value = backend_ref.get(&"test_key".to_string()).await.unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_async_warming() {
        let backend = MemoryBackend::new();
        let warmer = CacheWarmer::new(backend);

        warmer
            .register_async_warmer(
                "test_key",
                Arc::new(|| Box::pin(async { Ok(vec![4, 5, 6]) })),
                None,
                Duration::from_secs(1),
            )
            .await
            .unwrap();

        warmer.warm("test_key").await.unwrap();

        let backend_ref = warmer.backend.clone();
        let value = backend_ref.get(&"test_key".to_string()).await.unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap(), vec![4, 5, 6]);
    }

    #[tokio::test]
    async fn test_background_warming() {
        let backend = MemoryBackend::new();
        let warmer = CacheWarmer::new(backend);

        warmer
            .register_warmer(
                "test_key",
                Arc::new(|| Ok(vec![7, 8, 9])),
                None,
                Duration::from_millis(100),
            )
            .await
            .unwrap();

        warmer.start_warming().await.unwrap();

        tokio::time::sleep(Duration::from_millis(150)).await;

        let backend_ref = warmer.backend.clone();
        let value = backend_ref.get(&"test_key".to_string()).await.unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap(), vec![7, 8, 9]);

        warmer.stop_all_warming().await.unwrap();
    }

    #[tokio::test]
    async fn test_last_refreshed() {
        let backend = MemoryBackend::new();
        let warmer = CacheWarmer::new(backend);

        warmer
            .register_warmer(
                "test_key",
                Arc::new(|| Ok(vec![10, 11, 12])),
                None,
                Duration::from_secs(1),
            )
            .await
            .unwrap();

        let last_refreshed = warmer.last_refreshed("test_key").await.unwrap();
        assert!(last_refreshed.is_none());

        warmer.warm("test_key").await.unwrap();

        let last_refreshed = warmer.last_refreshed("test_key").await.unwrap();
        assert!(last_refreshed.is_some());
    }
}
