//! Backend implementations for different storage systems.
//!
//! This module provides various cache backend implementations that can be used
//! with the fncache library. Each backend implements the `CacheBackend` trait,
//! which defines a common interface for cache operations such as get, set, remove,
//! and clear.
//!
//! # Available Backends
//!
//! * **Memory Backend** (always available): In-memory cache using `dashmap` with support
//!   for configurable eviction policies (LRU, LFU).
//!
//! * **File Backend** (with `file-backend` feature): Persistent cache stored on disk with
//!   optional compression.
//!
//! * **Redis Backend** (with `redis-backend` feature): Distributed cache using Redis.
//!
//! * **RocksDB Backend** (with `rocksdb-backend` feature): Persistent embedded key-value
//!   store with high performance.
//!
//! * **WASM Backend** (with `wasm` feature): Backend optimized for WebAssembly environments.
//!
//! # Example: Using the Memory Backend
//!
//! ```
//! use fncache::backends::{CacheBackend, memory::MemoryBackend};
//!
//! # async fn example() -> fncache::Result<()> {
//! let backend = MemoryBackend::new();
//!
//! // Store a value
//! let key = "user:123".to_string();
//! let value = vec![1, 2, 3, 4];
//! backend.set(key.clone(), value.clone(), None).await?;
//!
//! // Retrieve the value
//! let result = backend.get(&key).await?;
//! assert_eq!(result, Some(value));
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use std::{fmt::Debug, time::Duration};

#[cfg(feature = "file-backend")]
pub mod file;
pub mod memory;
#[cfg(feature = "redis-backend")]
pub mod redis;
#[cfg(feature = "rocksdb-backend")]
pub mod rocksdb;
#[cfg(feature = "wasm")]
pub mod wasm;

/// A key in the cache.
///
/// Keys are represented as strings for maximum flexibility and compatibility
/// across different backend implementations. For best performance, keep keys
/// relatively short and avoid extremely long strings.
pub type Key = String;

/// A value in the cache.
///
/// Values are stored as byte vectors (`Vec<u8>`), allowing any serializable data
/// to be cached. You'll typically need to serialize your data structures to bytes
/// before storing them and deserialize them after retrieval.
///
/// The `fncache` proc macro handles serialization/deserialization automatically
/// for cached functions.
pub type Value = Vec<u8>;

/// Trait defining the interface for all cache backends.
///
/// This trait provides a uniform interface for interacting with different cache
/// storage systems. All cache backends must implement this trait to be used with
/// the fncache library.
///
/// The trait requires implementing five async methods for basic cache operations:
/// get, set, remove, contains_key, and clear. It also requires that implementors
/// are Send, Sync, and implement Debug.
///
/// # Examples
///
/// Implementing a custom cache backend:
///
/// ```
/// use async_trait::async_trait;
/// use fncache::backends::{CacheBackend, Key, Value};
/// use std::collections::HashMap;
/// use std::sync::Mutex;
/// use std::time::Duration;
///
/// #[derive(Debug)]
/// struct MyCustomBackend {
///     store: Mutex<HashMap<Key, Value>>,
/// }
///
/// impl MyCustomBackend {
///     fn new() -> Self {
///         Self {
///             store: Mutex::new(HashMap::new()),
///         }
///     }
/// }
///
/// #[async_trait]
/// impl CacheBackend for MyCustomBackend {
///     async fn get(&self, key: &Key) -> fncache::Result<Option<Value>> {
///         let store = self.store.lock().unwrap();
///         Ok(store.get(key).cloned())
///     }
///
///     async fn set(&self, key: Key, value: Value, _ttl: Option<Duration>) -> fncache::Result<()> {
///         let mut store = self.store.lock().unwrap();
///         store.insert(key, value);
///         Ok(())
///     }
///
///     async fn remove(&self, key: &Key) -> fncache::Result<()> {
///         let mut store = self.store.lock().unwrap();
///         store.remove(key);
///         Ok(())
///     }
///
///     async fn contains_key(&self, key: &Key) -> fncache::Result<bool> {
///         let store = self.store.lock().unwrap();
///         Ok(store.contains_key(key))
///     }
///
///     async fn clear(&self) -> fncache::Result<()> {
///         let mut store = self.store.lock().unwrap();
///         store.clear();
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait CacheBackend: Send + Sync + Debug {
    /// Gets a value from the cache by key.
    ///
    /// This method retrieves the value associated with the given key from the cache.
    /// If the key doesn't exist or has expired, it returns `Ok(None)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the cache
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Value))` - The value was found in the cache
    /// * `Ok(None)` - The value was not found or has expired
    /// * `Err(...)` - An error occurred while accessing the cache
    async fn get(&self, key: &Key) -> crate::Result<Option<Value>>;

    /// Sets a value in the cache with an optional TTL.
    ///
    /// This method stores a value in the cache with the specified key.
    /// If the key already exists, its value will be overwritten.
    /// The optional TTL parameter allows setting a time-to-live after which the
    /// entry will automatically expire.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to store the value under
    /// * `value` - The value to store in the cache
    /// * `ttl` - Optional time-to-live duration after which the entry will expire
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The value was successfully stored
    /// * `Err(...)` - An error occurred while storing the value
    async fn set(&self, key: Key, value: Value, ttl: Option<Duration>) -> crate::Result<()>;

    /// Removes a value from the cache by key.
    ///
    /// This method removes the entry with the specified key from the cache.
    /// If the key doesn't exist, the operation is still considered successful.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove from the cache
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The operation was successful (even if the key didn't exist)
    /// * `Err(...)` - An error occurred while removing the key
    async fn remove(&self, key: &Key) -> crate::Result<()>;

    /// Checks if a key exists in the cache.
    ///
    /// This method checks whether an entry with the specified key exists in the cache.
    /// It returns `true` if the key exists and has not expired, and `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check in the cache
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - The key exists in the cache
    /// * `Ok(false)` - The key does not exist in the cache or has expired
    /// * `Err(...)` - An error occurred while checking the key
    async fn contains_key(&self, key: &Key) -> crate::Result<bool>;

    /// Clears all values from the cache.
    ///
    /// This method removes all entries from the cache, effectively resetting it
    /// to an empty state.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The cache was successfully cleared
    /// * `Err(...)` - An error occurred while clearing the cache
    async fn clear(&self) -> crate::Result<()>;
}

/// A boxed cache backend that can be used as a trait object.
///
/// This type alias represents a heap-allocated `CacheBackend` trait object.
/// It's useful when you need to store or pass around different backend
/// implementations through a common interface.
///
/// # Examples
///
/// ```
/// use fncache::backends::{Backend, memory::MemoryBackend};
///
/// fn create_backend() -> Backend {
///     Box::new(MemoryBackend::new())
/// }
/// ```
pub type Backend = Box<dyn CacheBackend>;
