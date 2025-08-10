//! Backend implementations for different storage systems.

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
pub type Key = String;

/// A value in the cache.
pub type Value = Vec<u8>;

/// Trait defining the interface for all cache backends.
#[async_trait]
pub trait CacheBackend: Send + Sync + Debug {
    /// Gets a value from the cache by key.
    async fn get(&self, key: &Key) -> crate::Result<Option<Value>>;

    /// Sets a value in the cache with an optional TTL.
    async fn set(&self, key: Key, value: Value, ttl: Option<Duration>) -> crate::Result<()>;

    /// Removes a value from the cache by key.
    async fn remove(&self, key: &Key) -> crate::Result<()>;

    /// Checks if a key exists in the cache.
    async fn contains_key(&self, key: &Key) -> crate::Result<bool>;

    /// Clears all values from the cache.
    async fn clear(&self) -> crate::Result<()>;
}

/// A boxed cache backend that can be used as a trait object.
pub type Backend = Box<dyn CacheBackend>;
