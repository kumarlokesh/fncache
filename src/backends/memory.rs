//! An in-memory cache backend using `dashmap`.
//!
//! This module provides a high-performance, thread-safe in-memory cache implementation
//! that uses `dashmap` for concurrent map access. The `MemoryBackend` supports:
//!
//! * Configurable maximum capacity
//! * TTL-based entry expiration
//! * Pluggable eviction policies (LRU, LFU)
//! * Performance metrics collection
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```
//! use fncache::backends::memory::MemoryBackend;
//! use fncache::backends::CacheBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> fncache::Result<()> {
//! // Create a new memory backend with default settings
//! let backend = MemoryBackend::new();
//!
//! // Set a value with a 10-second TTL
//! backend.set(
//!     "user:profile:123".to_string(),
//!     serde_json::to_vec(&"User data").unwrap(),
//!     Some(Duration::from_secs(10))
//! ).await?;
//!
//! // Get the value back
//! let value = backend.get("user:profile:123").await?;
//!
//! // Clear all entries
//! backend.clear().await?;
//! # Ok(())
//! # }
//! ```
//!
//! With configuration options:
//!
//! ```
//! use fncache::backends::memory::{MemoryBackend, MemoryBackendConfig};
//!
//! // Create a backend with 1000 item capacity and LFU eviction
//! let config = MemoryBackendConfig {
//!     max_capacity: 1000,
//!     eviction_policy: "lfu".to_string(),
//! };
//!
//! let backend = MemoryBackend::with_config(config);
//! ```
//!
//! Using builder methods:
//!
//! ```
//! use fncache::backends::memory::MemoryBackend;
//!
//! // Create a backend with 500 item capacity and LRU eviction
//! let backend = MemoryBackend::new()
//!     .with_capacity(500)
//!     .with_eviction_policy("lru");
//! ```

use super::*;
use crate::eviction::EvictionPolicy;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// An entry in the in-memory cache.
///
/// Each cache entry stores the serialized value and an optional expiration time.
/// When the expiration time is reached, the entry is considered invalid and will
/// be removed on the next access or during cleanup operations.
#[derive(Debug)]
struct CacheEntry {
    /// The actual cached value (serialized as bytes)
    value: Value,
    /// Optional expiration timestamp, after which the entry is considered invalid
    expires_at: Option<Instant>,
}

/// Configuration options for the memory backend.
///
/// This struct allows customizing the behavior of the `MemoryBackend`,
/// including setting capacity limits and choosing an eviction policy.
///
/// # Examples
///
/// ```
/// use fncache::backends::memory::{MemoryBackend, MemoryBackendConfig};
///
/// // Create a configuration with 10,000 item limit and LFU eviction
/// let config = MemoryBackendConfig {
///     max_capacity: 10_000,
///     eviction_policy: "lfu".to_string(),
/// };
///
/// // Use the config to create a memory backend
/// let backend = MemoryBackend::with_config(config);
/// ```
#[derive(Debug, Clone)]
pub struct MemoryBackendConfig {
    /// Maximum number of items in the cache (0 = unlimited).
    ///
    /// When this limit is reached, the configured eviction policy will
    /// be used to determine which items to remove. Setting this to 0
    /// disables the capacity limit.
    pub max_capacity: usize,

    /// Eviction policy name ("lru", "lfu").
    ///
    /// Supported values:
    /// - "lru": Least Recently Used - removes least recently accessed items first
    /// - "lfu": Least Frequently Used - removes least frequently accessed items first
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
///
/// This is the primary in-memory implementation of the `CacheBackend` trait.
/// It provides a thread-safe, high-performance caching solution with support
/// for TTL, eviction policies, and performance metrics.
///
/// The backend uses `dashmap` for concurrent map access, making it suitable
/// for multi-threaded applications without requiring additional synchronization.
///
/// # Features
///
/// * Thread-safe concurrent access
/// * Optional TTL for entries
/// * Configurable maximum capacity
/// * Pluggable eviction policies
/// * Performance metrics collection
///
/// # Examples
///
/// ```
/// use fncache::backends::memory::MemoryBackend;
/// use fncache::backends::CacheBackend;
/// use std::time::Duration;
///
/// # async fn example() -> fncache::Result<()> {
/// // Create a new memory backend with LRU eviction and 1000 item capacity
/// let backend = MemoryBackend::new()
///     .with_capacity(1000)
///     .with_eviction_policy("lru");
///
/// // Store an item with 30-second TTL
/// let key = "session:user123".to_string();
/// let value = vec![1, 2, 3, 4];
/// backend.set(key.clone(), value.clone(), Some(Duration::from_secs(30))).await?;
///
/// // Retrieve the item
/// let result = backend.get(&key).await?;
/// assert_eq!(result, Some(value));
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MemoryBackend {
    /// The actual storage for cache entries
    store: DashMap<Key, CacheEntry>,
    /// Collection of performance metrics for this backend instance
    metrics: crate::metrics::Metrics,
    /// Configuration settings for this backend
    config: MemoryBackendConfig,
    /// The active eviction policy implementation
    eviction_policy: Arc<dyn EvictionPolicy<Key, Value>>,
}

impl Default for MemoryBackend {
    /// Returns a default `MemoryBackend` instance with unlimited capacity and LRU eviction policy.
    ///
    /// This is equivalent to calling `MemoryBackend::new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBackend {
    /// Creates a new `MemoryBackend` with default configuration.
    ///
    /// The default configuration uses an unlimited capacity and the LRU eviction policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::backends::memory::MemoryBackend;
    ///
    /// let backend = MemoryBackend::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(MemoryBackendConfig::default())
    }

    /// Returns a reference to the metrics instance.
    ///
    /// This provides access to performance metrics collected by this backend,
    /// such as hit/miss counts, latencies, and eviction statistics.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::backends::memory::MemoryBackend;
    ///
    /// let backend = MemoryBackend::new();
    /// let metrics = backend.metrics();
    ///
    /// println!("Cache hits: {}", metrics.hits());
    /// println!("Cache misses: {}", metrics.misses());
    /// println!("Hit ratio: {:.2}%", metrics.hit_ratio() * 100.0);
    /// ```
    pub fn metrics(&self) -> &crate::metrics::Metrics {
        &self.metrics
    }

    /// Creates a new `MemoryBackend` with the given configuration.
    ///
    /// This constructor allows full customization of the backend through
    /// the `MemoryBackendConfig` struct.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration options for the memory backend
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::backends::memory::{MemoryBackend, MemoryBackendConfig};
    ///
    /// let config = MemoryBackendConfig {
    ///     max_capacity: 5000,
    ///     eviction_policy: "lfu".to_string(),
    /// };
    ///
    /// let backend = MemoryBackend::with_config(config);
    /// ```
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
    ///
    /// This is a builder method that returns `self` for method chaining.
    /// When the number of items in the cache exceeds this capacity,
    /// the configured eviction policy will be used to remove items.
    /// Setting the capacity to 0 means unlimited capacity.
    ///
    /// # Arguments
    ///
    /// * `max_capacity` - The maximum number of items the cache can hold (0 = unlimited)
    ///
    /// # Returns
    ///
    /// The modified `MemoryBackend` instance with the new capacity setting
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::backends::memory::MemoryBackend;
    ///
    /// let backend = MemoryBackend::new()
    ///     .with_capacity(10_000); // Set a 10K item limit
    /// ```
    pub fn with_capacity(mut self, max_capacity: usize) -> Self {
        self.config.max_capacity = max_capacity;
        self
    }

    /// Sets the eviction policy for the cache.
    ///
    /// This is a builder method that returns `self` for method chaining.
    /// The policy determines which items are removed when the cache reaches
    /// its maximum capacity.
    ///
    /// # Arguments
    ///
    /// * `policy_name` - The name of the eviction policy to use ("lru", "lfu")
    ///
    /// # Returns
    ///
    /// The modified `MemoryBackend` instance with the new eviction policy
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::backends::memory::MemoryBackend;
    ///
    /// // Create a backend with Least Frequently Used eviction policy
    /// let backend = MemoryBackend::new()
    ///     .with_eviction_policy("lfu");
    /// ```
    pub fn with_eviction_policy(mut self, policy_name: &str) -> Self {
        self.config.eviction_policy = policy_name.to_string();
        self.eviction_policy = crate::eviction::create_policy(policy_name);
        self
    }

    /// Removes expired entries from the cache.
    ///
    /// This method scans the cache for entries whose TTL has expired and removes them.
    /// It's called automatically during operations like `get` to ensure expired
    /// entries are not returned to clients.
    ///
    /// When an entry is removed due to expiration, the eviction policy is notified
    /// and metrics are updated.
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
    ///
    /// When the cache exceeds its configured capacity, this method is called
    /// to remove items according to the active eviction policy.
    ///
    /// If the eviction policy fails to identify items for eviction despite
    /// being over capacity, a warning is printed but the method continues
    /// without error.
    fn enforce_capacity_limit(&self) {
        if self.config.max_capacity == 0 || self.store.len() <= self.config.max_capacity {
            return;
        }

        let to_evict = self.store.len() - self.config.max_capacity;

        let eviction_result = self.eviction_policy.evict(to_evict);

        if eviction_result.keys_to_evict.is_empty() && to_evict > 0 {
            eprintln!("Warning: Eviction policy returned no keys to evict when {} items needed to be evicted", to_evict);
        }

        for key in eviction_result.keys_to_evict {
            self.store.remove(&key);
            self.metrics.record_eviction();
        }
    }

    /// Returns the current number of items in the cache.
    ///
    /// This method can be useful for monitoring and debugging cache usage.
    /// Note that this count may include expired items that haven't been cleaned up yet.
    ///
    /// # Returns
    ///
    /// The number of key-value pairs currently stored in the cache
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::backends::memory::MemoryBackend;
    /// use fncache::backends::CacheBackend;
    ///
    /// # async fn example() -> fncache::Result<()> {
    /// let backend = MemoryBackend::new();
    ///
    /// backend.set("key1".to_string(), vec![1, 2, 3], None).await?;
    /// backend.set("key2".to_string(), vec![4, 5, 6], None).await?;
    ///
    /// let count = backend.get_store_len().await;
    /// assert_eq!(count, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_store_len(&self) -> usize {
        self.store.len()
    }
}

/// Implementation of the `CacheBackend` trait for the in-memory backend.
///
/// This implementation provides thread-safe, high-performance cache operations
/// with support for TTL, eviction policies, and metrics collection.
#[async_trait]
impl CacheBackend for MemoryBackend {
    async fn get(&self, key: &Key) -> crate::Result<Option<Value>> {
        // Begin timing
        let timing = self.metrics.begin_get_timing();

        self.cleanup_expired();

        let result = if let Some(entry) = self.store.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    self.metrics.record_miss();
                    self.store.remove(key);
                    Ok(None)
                } else {
                    self.eviction_policy.on_access(key);

                    self.metrics.record_hit();
                    Ok(Some(entry.value.clone()))
                }
            } else {
                self.eviction_policy.on_access(key);

                self.metrics.record_hit();
                Ok(Some(entry.value.clone()))
            }
        } else {
            self.metrics.record_miss();
            Ok(None)
        };

        self.metrics.record_get_latency(timing);

        result
    }

    async fn set(&self, key: Key, value: Value, ttl: Option<Duration>) -> crate::Result<()> {
        let timing = self.metrics.begin_set_timing();

        let new_size = bincode::serialized_size(&value).unwrap_or(0) as usize;

        let old_size = if let Some(old_entry) = self.store.get(&key) {
            bincode::serialized_size(&old_entry.value).unwrap_or(0) as usize
        } else {
            0
        };

        let is_existing_key = self.store.contains_key(&key);
        if !is_existing_key
            && self.config.max_capacity > 0
            && self.store.len() >= self.config.max_capacity
        {
            let to_evict = 1;
            let eviction_result = self.eviction_policy.evict(to_evict);

            for key_to_evict in eviction_result.keys_to_evict {
                if let Some(evicted_entry) = self.store.get(&key_to_evict) {
                    let evicted_size =
                        bincode::serialized_size(&evicted_entry.value).unwrap_or(0) as usize;
                    self.metrics.record_entry_removal(evicted_size);
                }
                self.store.remove(&key_to_evict);
                self.metrics.record_eviction();
            }
        }

        let entry = CacheEntry {
            value: value.clone(),
            expires_at: ttl.map(|ttl| Instant::now() + ttl),
        };

        self.metrics.record_entry_size(old_size, new_size);

        self.eviction_policy.on_insert(&key, &value);
        self.store.insert(key, entry);
        self.metrics.record_insertion();

        if self.config.max_capacity > 0 && self.store.len() > self.config.max_capacity {
            self.enforce_capacity_limit();
        }

        self.metrics.record_set_latency(timing);

        Ok(())
    }

    async fn remove(&self, key: &Key) -> crate::Result<()> {
        let size = if let Some(entry) = self.store.get(key) {
            bincode::serialized_size(&entry.value).unwrap_or(0) as usize
        } else {
            0
        };

        self.eviction_policy.on_remove(key);

        let removed = self.store.remove(key).is_some();
        if removed && size > 0 {
            self.metrics.record_entry_removal(size);
        }

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
