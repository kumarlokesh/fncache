//! An in-memory cache backend using `dashmap`.

use super::*;
use crate::eviction::EvictionPolicy;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// An entry in the in-memory cache.
#[derive(Debug)]
struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
}

/// Configuration options for the memory backend.
#[derive(Debug, Clone)]
pub struct MemoryBackendConfig {
    /// Maximum number of items in the cache (0 = unlimited).
    pub max_capacity: usize,
    /// Eviction policy name ("lru", "lfu").
    pub eviction_policy: String,
}

impl Default for MemoryBackendConfig {
    fn default() -> Self {
        Self {
            max_capacity: 0, // Unlimited by default
            eviction_policy: "lru".to_string(),
        }
    }
}

/// An in-memory cache backend using `dashmap`.
#[derive(Debug)]
pub struct MemoryBackend {
    store: DashMap<Key, CacheEntry>,
    metrics: crate::metrics::Metrics,
    config: MemoryBackendConfig,
    eviction_policy: Arc<dyn EvictionPolicy<Key, Value>>,
}

impl MemoryBackend {
    /// Creates a new `MemoryBackend` with default configuration.
    pub fn new() -> Self {
        Self::with_config(MemoryBackendConfig::default())
    }

    /// Creates a new `MemoryBackend` with the given configuration.
    pub fn with_config(config: MemoryBackendConfig) -> Self {
        let eviction_policy = crate::eviction::create_policy(&config.eviction_policy);

        Self {
            store: DashMap::new(),
            metrics: crate::metrics::Metrics::default(),
            config,
            eviction_policy,
        }
    }

    /// Sets the maximum capacity of the cache.
    pub fn with_capacity(mut self, max_capacity: usize) -> Self {
        self.config.max_capacity = max_capacity;
        self
    }

    /// Sets the eviction policy for the cache.
    pub fn with_eviction_policy(mut self, policy_name: &str) -> Self {
        self.config.eviction_policy = policy_name.to_string();
        self.eviction_policy = crate::eviction::create_policy(policy_name);
        self
    }

    /// Removes expired entries from the cache.
    fn cleanup_expired(&self) {
        let now = Instant::now();
        self.store.retain(|key, entry| {
            if let Some(expires_at) = entry.expires_at {
                if now >= expires_at {
                    self.metrics.record_eviction();
                    self.eviction_policy.on_remove(key);
                    return false;
                }
            }
            true
        });
    }

    /// Enforces the capacity limit by evicting items if necessary.
    fn enforce_capacity_limit(&self) {
        if self.config.max_capacity == 0 || self.store.len() <= self.config.max_capacity {
            return;
        }

        let to_evict = self.store.len() - self.config.max_capacity;

        let eviction_result = self.eviction_policy.evict(to_evict);

        // Make sure we actually evict items
        if eviction_result.keys_to_evict.is_empty() && to_evict > 0 {
            // Log the issue - this indicates the eviction policy isn't working properly
            eprintln!("Warning: Eviction policy returned no keys to evict when {} items needed to be evicted", to_evict);
        }

        for key in eviction_result.keys_to_evict {
            self.store.remove(&key);
            self.metrics.record_eviction();
        }
    }

    /// Returns the current size of the cache store
    pub async fn get_store_len(&self) -> usize {
        self.store.len()
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
                self.eviction_policy.on_access(key);
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
        let entry = CacheEntry {
            value: value.clone(),
            expires_at: ttl.map(|ttl| Instant::now() + ttl),
        };

        self.eviction_policy.on_insert(&key, &value);

        self.store.insert(key, entry);
        self.metrics.record_insertion();

        self.enforce_capacity_limit();

        Ok(())
    }

    async fn remove(&self, key: &Key) -> crate::Result<()> {
        self.eviction_policy.on_remove(key);

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
