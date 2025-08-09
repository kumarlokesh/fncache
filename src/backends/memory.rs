//! An in-memory cache backend using `dashmap`.

use super::*;
use dashmap::DashMap;
use std::time::{Duration, Instant};

/// An entry in the in-memory cache.
#[derive(Debug)]
struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
}

/// An in-memory cache backend using `dashmap`.
#[derive(Debug, Default)]
pub struct MemoryBackend {
    store: DashMap<Key, CacheEntry>,
    metrics: crate::metrics::Metrics,
}

impl MemoryBackend {
    /// Creates a new `MemoryBackend`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes expired entries from the cache.
    fn cleanup_expired(&self) {
        let now = Instant::now();
        self.store.retain(|_, entry| {
            if let Some(expires_at) = entry.expires_at {
                if now >= expires_at {
                    self.metrics.record_eviction();
                    return false;
                }
            }
            true
        });
    }
}

#[async_trait]
impl CacheBackend for MemoryBackend {
    async fn get(&self, key: &Key) -> crate::Result<Option<Value>> {
        self.cleanup_expired();

        match self.store.get(key) {
            Some(entry) => {
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() >= expires_at {
                        self.metrics.record_miss();
                        return Ok(None);
                    }
                }
                self.metrics.record_hit();
                Ok(Some(entry.value.clone()))
            }
            None => {
                self.metrics.record_miss();
                Ok(None)
            }
        }
    }

    async fn set(&self, key: Key, value: Value, ttl: Option<Duration>) -> crate::Result<()> {
        let expires_at = ttl.map(|duration| Instant::now() + duration);
        let entry = CacheEntry { value, expires_at };

        self.store.insert(key, entry);
        self.metrics.record_insertion();

        Ok(())
    }

    async fn remove(&self, key: &Key) -> crate::Result<()> {
        self.store.remove(key);
        Ok(())
    }

    async fn contains_key(&self, key: &Key) -> crate::Result<bool> {
        self.cleanup_expired();
        Ok(self.store.contains_key(key))
    }

    async fn clear(&self) -> crate::Result<()> {
        self.store.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_get_set() {
        let backend = MemoryBackend::new();
        let key = "test_key".to_string();
        let value = b"test_value".to_vec();

        backend.set(key.clone(), value.clone(), None).await.unwrap();
        let result = backend.get(&key).await.unwrap();
        assert_eq!(result, Some(value));
    }

    #[tokio::test]
    #[serial]
    async fn test_ttl() {
        let backend = MemoryBackend::new();
        let key = "test_ttl".to_string();
        let value = b"test_value".to_vec();

        backend
            .set(key.clone(), value, Some(Duration::from_millis(100)))
            .await
            .unwrap();

        assert!(backend.get(&key).await.unwrap().is_some());

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(backend.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_metrics() {
        let backend = MemoryBackend::new();
        let key = "test_metrics".to_string();
        let value = b"test_value".to_vec();

        assert_eq!(backend.metrics.hits(), 0);
        assert_eq!(backend.metrics.misses(), 0);

        assert!(backend.get(&key).await.unwrap().is_none());
        assert_eq!(backend.metrics.misses(), 1);

        backend.set(key.clone(), value, None).await.unwrap();
        assert!(backend.get(&key).await.unwrap().is_some());
        assert_eq!(backend.metrics.hits(), 1);
    }
}
