//! RocksDB cache backend implementation.
//!
//! This backend stores cache entries in a local RocksDB database, providing high-performance
//! persistent caching with excellent read/write throughput and low latency. RocksDB is
//! optimized for fast storage hardware (SSDs) and offers superior performance compared to
//! traditional file-based storage for high-throughput caching needs.
//!
//! # Features
//!
//! * High-performance persistent caching
//! * Optimized for SSDs and fast storage
//! * Excellent read/write throughput
//! * TTL (time-to-live) support for expiring entries
//! * Automatic cleanup of expired entries on access
//! * Built-in metrics for hits, misses, and insertions
//! * Atomic operations for data integrity
//!
//! # Usage
//!
//! The RocksDB backend requires a directory path where it will store its database files.
//! The directory will be created if it doesn't exist.
//!
//! ```rust,no_run
//! use fncache::{backends::rocksdb::RocksDBBackend, init_global_cache, fncache};
//! use std::time::Duration;
//!
//! // Initialize the RocksDB backend with a directory path
//! let db_path = "/path/to/cache/db";
//! let backend = RocksDBBackend::new(db_path).unwrap();
//! init_global_cache(backend).unwrap();
//!
//! // Define a cached function with TTL of 1 hour
//! #[fncache(ttl = 3600)]
//! fn compute_expensive_value(input: u32) -> Vec<u8> {
//!     println!("Computing expensive value for {}", input);
//!     // Simulate expensive computation
//!     vec![input as u8; 1024] // 1KB of data
//! }
//!
//! // Call the function - first time will execute and store result
//! let result1 = compute_expensive_value(42);
//! // Second call with same input will use the cached value from RocksDB
//! let result2 = compute_expensive_value(42);
//! ```
//!
//! # Implementation Details
//!
//! * Cache entries are serialized using bincode for efficient binary storage
//! * TTL is implemented by storing expiration timestamps with each entry
//! * Expired entries are cleaned up when accessed
//! * Key-value pairs are stored directly in RocksDB's native format
//! * The clear operation iterates through all keys for deletion

use crate::{backends::CacheBackend, error::Error, metrics::Metrics, Result};
use async_trait::async_trait;
use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime},
};

/// Entry stored in the RocksDB cache
///
/// This structure represents a single cache entry that's serialized using bincode
/// and stored in RocksDB. It contains both the value bytes and optional expiration time.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// The cached value as bytes
    value: Vec<u8>,
    /// When the entry expires (if ever)
    /// If None, the entry never expires
    expires_at: Option<SystemTime>,
}

/// RocksDB-based cache backend for high-performance persistent caching
///
/// This backend stores cache entries in a RocksDB database, providing high-performance
/// persistent caching optimized for SSDs and fast storage devices. It offers superior
/// read/write performance compared to file-based backends, especially for large datasets.
///
/// # Features
///
/// * High-performance persistent storage
/// * Optimized for SSDs and fast storage
/// * TTL support with automatic expiration
/// * Excellent read/write throughput
/// * Atomic operations for data integrity
/// * Metrics collection
///
/// # Example
///
/// ```rust,no_run
/// use fncache::backends::rocksdb::RocksDBBackend;
/// use std::time::Duration;
///
/// # fn run() -> fncache::Result<()> {
/// // Create a RocksDB backend with specific storage directory
/// let backend = RocksDBBackend::new("/path/to/rocksdb")?;
///
/// // Store a value with 1-hour TTL
/// let key = "user:profile:123".to_string();
/// let value = b"{\"name\": \"John Doe\"}".to_vec();
/// tokio::runtime::Runtime::new()?                   
///     .block_on(async {
///         backend.set(key.clone(), value, Some(Duration::from_secs(3600))).await?
///         
///         // Retrieve the value later
///         if let Some(data) = backend.get(&key).await? {
///             println!("Retrieved {} bytes from cache", data.len());
///         }
///         
///         Ok::<(), fncache::error::Error>(())
///     })?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RocksDBBackend {
    /// RocksDB database handle
    db: Arc<DB>,
    /// Cache metrics
    metrics: Arc<Metrics>,
}

impl RocksDBBackend {
    /// Creates a new RocksDBBackend with the specified database path.
    ///
    /// # Arguments
    /// * `db_path` - Path where the RocksDB database will be stored
    ///
    /// # Returns
    /// A new RocksDBBackend instance
    ///
    /// # Errors
    /// Returns an error if the RocksDB database could not be opened
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        // Create RocksDB options
        let mut options = Options::default();
        options.create_if_missing(true);

        // Open the database
        let db = DB::open(&options, db_path)
            .map_err(|e| Error::Backend(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
            metrics: Arc::new(Metrics::new()),
        })
    }

    /// Check if a cache entry is expired
    fn is_expired(entry: &CacheEntry) -> bool {
        if let Some(expires_at) = entry.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }
}

/// Implementation of the CacheBackend trait for RocksDBBackend
///
/// This implementation provides:
/// * High-performance persistent storage with RocksDB
/// * TTL support with automatic cleanup of expired entries
/// * Atomic read/write operations
/// * Metrics for hits, misses and insertions
/// * Bincode serialization for efficient binary storage
#[async_trait]
impl CacheBackend for RocksDBBackend {
    async fn get(&self, key: &String) -> Result<Option<Vec<u8>>> {
        // Try to get the value from RocksDB
        match self.db.get(key.as_bytes()) {
            Ok(Some(bytes)) => {
                // Deserialize the entry
                match bincode::deserialize::<CacheEntry>(&bytes) {
                    Ok(entry) => {
                        // Check if entry is expired
                        if Self::is_expired(&entry) {
                            // Entry is expired, remove it
                            if let Err(e) = self.db.delete(key.as_bytes()) {
                                return Err(Error::Backend(format!(
                                    "Failed to delete expired key: {}",
                                    e
                                )));
                            }
                            self.metrics.record_miss();
                            Ok(None)
                        } else {
                            // Entry is valid
                            self.metrics.record_hit();
                            Ok(Some(entry.value))
                        }
                    }
                    Err(e) => {
                        // Deserialization error
                        self.metrics.record_miss();
                        Err(Error::Codec(format!(
                            "Failed to deserialize cache entry: {}",
                            e
                        )))
                    }
                }
            }
            Ok(None) => {
                // Key doesn't exist
                self.metrics.record_miss();
                Ok(None)
            }
            Err(e) => {
                // Database error
                Err(Error::Backend(format!("RocksDB error: {}", e)))
            }
        }
    }

    async fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        // Create the cache entry
        let expires_at = ttl.map(|duration| {
            SystemTime::now()
                .checked_add(duration)
                .unwrap_or_else(|| SystemTime::now() + duration)
        });

        let entry = CacheEntry { value, expires_at };

        // Serialize the entry
        let bytes = bincode::serialize(&entry)
            .map_err(|e| Error::Codec(format!("Failed to serialize cache entry: {}", e)))?;

        // Store in RocksDB
        self.db
            .put(key.as_bytes(), bytes)
            .map_err(|e| Error::Backend(format!("Failed to store in RocksDB: {}", e)))?;

        self.metrics.record_insertion();
        Ok(())
    }

    async fn remove(&self, key: &String) -> Result<()> {
        self.db
            .delete(key.as_bytes())
            .map_err(|e| Error::Backend(format!("Failed to remove from RocksDB: {}", e)))?;

        Ok(())
    }

    async fn contains_key(&self, key: &String) -> Result<bool> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(bytes)) => {
                // Deserialize to check if expired
                match bincode::deserialize::<CacheEntry>(&bytes) {
                    Ok(entry) => {
                        if Self::is_expired(&entry) {
                            // Entry is expired, consider it doesn't exist
                            Ok(false)
                        } else {
                            Ok(true)
                        }
                    }
                    Err(_) => {
                        // Corrupted entry, consider it doesn't exist
                        Ok(false)
                    }
                }
            }
            Ok(None) => Ok(false),
            Err(e) => Err(Error::Backend(format!("RocksDB error: {}", e))),
        }
    }

    async fn clear(&self) -> Result<()> {
        // Iterate through all keys and delete them
        // This is not atomic but RocksDB doesn't provide a "clear all" operation

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        // Collect keys first to avoid mutating while iterating
        let keys: Vec<Vec<u8>> = iter.map(|item| item.unwrap().0.to_vec()).collect();

        // Delete all keys
        for key in keys {
            if let Err(e) = self.db.delete(&key) {
                return Err(Error::Backend(format!(
                    "Failed to delete key during clear: {}",
                    e
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::tempdir;
    use tokio::time::sleep;

    #[tokio::test]
    #[serial]
    async fn test_get_set() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path();

        let backend = RocksDBBackend::new(db_path).unwrap();

        let key = "test_key".to_string();
        let value = b"test_value".to_vec();

        // Set value
        backend.set(key.clone(), value.clone(), None).await.unwrap();

        // Get value
        let result = backend.get(&key).await.unwrap();
        assert_eq!(result, Some(value));

        // Contains key
        assert!(backend.contains_key(&key).await.unwrap());

        // Remove key
        backend.remove(&key).await.unwrap();
        assert_eq!(backend.get(&key).await.unwrap(), None);
        assert!(!backend.contains_key(&key).await.unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn test_ttl() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path();

        let backend = RocksDBBackend::new(db_path).unwrap();

        let key = "test_ttl".to_string();
        let value = b"test_value".to_vec();

        // Set with short TTL
        backend
            .set(key.clone(), value, Some(Duration::from_millis(100)))
            .await
            .unwrap();

        // Get immediately - should exist
        assert!(backend.get(&key).await.unwrap().is_some());

        // Wait for expiration
        sleep(Duration::from_millis(150)).await;

        // Get after expiration - should be gone
        assert!(backend.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_clear() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path();

        let backend = RocksDBBackend::new(db_path).unwrap();

        let key1 = "test_key1".to_string();
        let key2 = "test_key2".to_string();
        let value = b"test_value".to_vec();

        // Set multiple values
        backend
            .set(key1.clone(), value.clone(), None)
            .await
            .unwrap();
        backend
            .set(key2.clone(), value.clone(), None)
            .await
            .unwrap();

        // Verify both exist
        assert!(backend.contains_key(&key1).await.unwrap());
        assert!(backend.contains_key(&key2).await.unwrap());

        // Clear all values
        backend.clear().await.unwrap();

        // Verify both gone
        assert!(!backend.contains_key(&key1).await.unwrap());
        assert!(!backend.contains_key(&key2).await.unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn test_metrics() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path();

        let backend = RocksDBBackend::new(db_path).unwrap();

        let key = "test_metrics".to_string();
        let value = b"test_value".to_vec();

        // Initial metrics
        assert_eq!(backend.metrics.hits(), 0);
        assert_eq!(backend.metrics.misses(), 0);

        // Miss (key doesn't exist)
        assert!(backend.get(&key).await.unwrap().is_none());
        assert_eq!(backend.metrics.misses(), 1);

        // Set key
        backend.set(key.clone(), value, None).await.unwrap();

        // Hit (key exists)
        assert!(backend.get(&key).await.unwrap().is_some());
        assert_eq!(backend.metrics.hits(), 1);
    }
}
