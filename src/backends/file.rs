//! File-based cache backend implementation.
//!
//! This backend stores cache entries as individual files in the local filesystem,
//! providing persistent storage between application restarts. It uses a simple directory
//! structure with content-addressed storage based on key hashing to ensure proper
//! filesystem compatibility.
//!
//! # Features
//!
//! * Persistent storage that survives application restarts
//! * TTL (time-to-live) support for expiring entries
//! * Automatic cleanup of expired entries
//! * Thread-safe access using async locks
//! * Built-in metrics for hits, misses, and insertions
//! * Efficient storage with binary serialization
//!
//! # Usage
//!
//! The file backend requires a base directory path where it will store all cached data.
//! It will automatically create the directory structure as needed.
//!
//! ```rust,no_run
//! use fncache::{backends::file::FileBackend, init_global_cache, fncache};
//! use std::time::Duration;
//!
//! // Initialize the file backend with a directory path
//! let cache_dir = "/tmp/fncache";
//! let backend = FileBackend::new(cache_dir).unwrap();
//! init_global_cache(backend).unwrap();
//!
//! // Define a cached function with TTL of 60 seconds
//! #[fncache(ttl = 60)]
//! fn compute_value(input: u32) -> String {
//!     println!("Computing value for {}", input);
//!     format!("value_{}", input)
//! }
//!
//! // Call the function - first time will execute and store result
//! let result1 = compute_value(42);
//! // Second call with same input will use the cached value
//! let result2 = compute_value(42);
//! ```
//!
//! # Storage Format
//!
//! The file backend stores each cache entry in its own file using a path derived from the key:
//!
//! - Keys are hashed for safe filenames
//! - Files are organized in a two-level directory structure (first two characters of hash as directory)
//! - Each entry is serialized using bincode format
//! - Entries include both the value and optional expiration timestamp

use crate::{backends::CacheBackend, error::Error, metrics::Metrics, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, create_dir_all, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;

/// Entry stored in the file cache
///
/// This structure represents a single cache entry that's serialized to disk.
/// It contains both the value bytes and optional expiration time.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// The cached value as bytes
    value: Vec<u8>,
    /// When the entry expires (if ever)
    /// If None, the entry never expires
    expires_at: Option<SystemTime>,
}

/// File-based cache backend for persistent storage
///
/// This backend stores cache entries as individual files in a directory structure,
/// providing persistent cache storage that survives application restarts.
/// Each key is hashed to create a safe file path, ensuring filesystem compatibility
/// regardless of the characters in the original cache keys.
///
/// # Features
///
/// * Disk-based persistent storage
/// * TTL (time-to-live) support
/// * Automatic cleanup of expired entries
/// * Thread-safe file access using async locks
/// * Metrics collection
///
/// # Example
///
/// ```rust,no_run
/// use fncache::backends::file::FileBackend;
/// use std::time::Duration;
///
/// # async fn run() -> fncache::Result<()> {
/// // Create a file backend with a specific storage directory
/// let backend = FileBackend::new("/path/to/cache")?
///
/// // Store a value with 5-minute TTL
/// let key = "user:profile:123".to_string();
/// let value = b"{\"name\": \"John Doe\"}".to_vec();
/// backend.set(key.clone(), value, Some(Duration::from_secs(300))).await?;
///
/// // Retrieve the value later
/// if let Some(data) = backend.get(&key).await? {
///     println!("Retrieved {} bytes from cache", data.len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct FileBackend {
    /// Base directory for cache files
    base_dir: PathBuf,
    /// Cache metrics
    metrics: Arc<Metrics>,
    /// Lock to ensure thread-safety for file operations
    file_lock: RwLock<()>,
}

impl FileBackend {
    /// Creates a new FileBackend with the specified base directory.
    ///
    /// # Arguments
    /// * `base_dir` - The base directory where cache files will be stored
    ///
    /// # Returns
    /// A new FileBackend instance
    ///
    /// # Errors
    /// Returns an error if the base directory could not be created
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let path = base_dir.as_ref().to_path_buf();
        create_dir_all(&path)?;

        Ok(Self {
            base_dir: path,
            metrics: Arc::new(Metrics::new()),
            file_lock: RwLock::new(()),
        })
    }

    /// Convert a cache key to a file path
    fn key_to_path(&self, key: &str) -> PathBuf {
        let hash = Self::hash_key(key);
        let dir_name = &hash[0..2];
        let file_name = &hash[2..];

        let mut path = self.base_dir.clone();
        path.push(dir_name);
        path.push(file_name);

        path
    }

    /// Simple hash function to convert keys to valid filenames
    fn hash_key(key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Ensure the parent directory exists for a given file path
    fn ensure_dir_exists(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Clean up expired entries
    async fn cleanup_expired(&self) -> Result<()> {
        let _guard = self.file_lock.read().await;

        let base_dir = &self.base_dir;
        if !base_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(base_dir)?;

        for entry_result in entries {
            let entry = entry_result?;
            let path = entry.path();

            if path.is_dir() {
                if let Ok(subentries) = fs::read_dir(&path) {
                    for subentry_result in subentries {
                        let subentry = subentry_result?;
                        let subpath = subentry.path();

                        if subpath.is_file() {
                            self.check_and_remove_if_expired(&subpath).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a cache file is expired and remove it if necessary
    async fn check_and_remove_if_expired(&self, path: &Path) -> Result<bool> {
        if let Ok(file) = File::open(path) {
            let mut reader = io::BufReader::new(file);

            match bincode::deserialize_from::<_, CacheEntry>(&mut reader) {
                Ok(entry) => {
                    if let Some(expires_at) = entry.expires_at {
                        if SystemTime::now() > expires_at {
                            if let Err(e) = fs::remove_file(path) {
                                eprintln!("Failed to remove expired cache file: {}", e);
                            }
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                Err(_) => {
                    if let Err(e) = fs::remove_file(path) {
                        eprintln!("Failed to remove invalid cache file: {}", e);
                    }
                    Ok(true)
                }
            }
        } else {
            Ok(false)
        }
    }
}

/// Implementation of the CacheBackend trait for FileBackend
///
/// This implementation provides:
/// * Thread-safe file operations using async locks
/// * TTL support with automatic cleanup of expired entries
/// * Metrics collection for hits, misses, and insertions
/// * Bincode-based serialization for efficient storage
#[async_trait::async_trait]
impl CacheBackend for FileBackend {
    async fn get(&self, key: &String) -> Result<Option<Vec<u8>>> {
        self.cleanup_expired().await?;

        let path = self.key_to_path(key);
        let _guard = self.file_lock.read().await;

        if !path.exists() {
            self.metrics.record_miss();
            return Ok(None);
        }

        match File::open(&path) {
            Ok(file) => {
                let mut reader = io::BufReader::new(file);

                match bincode::deserialize_from::<_, CacheEntry>(&mut reader) {
                    Ok(entry) => {
                        if let Some(expires_at) = entry.expires_at {
                            if SystemTime::now() > expires_at {
                                let _ = fs::remove_file(&path);
                                self.metrics.record_miss();
                                return Ok(None);
                            }
                        }

                        self.metrics.record_hit();
                        Ok(Some(entry.value))
                    }
                    Err(e) => {
                        self.metrics.record_miss();
                        Err(Error::Codec(format!(
                            "Failed to deserialize cache entry: {}",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    self.metrics.record_miss();
                    Ok(None)
                } else {
                    Err(Error::Backend(format!("File error: {}", e)))
                }
            }
        }
    }

    async fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let path = self.key_to_path(&key);
        let _guard = self.file_lock.write().await;
        self.ensure_dir_exists(&path)?;

        let expires_at = ttl.map(|duration| {
            SystemTime::now()
                .checked_add(duration)
                .unwrap_or_else(|| SystemTime::now() + duration)
        });

        let entry = CacheEntry { value, expires_at };

        let file = File::create(&path)?;
        let mut writer = io::BufWriter::new(file);

        bincode::serialize_into(&mut writer, &entry)
            .map_err(|e| Error::Codec(format!("Failed to serialize cache entry: {}", e)))?;

        writer.flush()?;

        self.metrics.record_insertion();
        Ok(())
    }

    async fn remove(&self, key: &String) -> Result<()> {
        let path = self.key_to_path(key);
        let _guard = self.file_lock.write().await;

        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    async fn contains_key(&self, key: &String) -> Result<bool> {
        self.cleanup_expired().await?;

        let path = self.key_to_path(key);
        let _guard = self.file_lock.read().await;

        Ok(path.exists())
    }

    async fn clear(&self) -> Result<()> {
        let _guard = self.file_lock.write().await;
        if self.base_dir.exists() {
            fs::remove_dir_all(&self.base_dir)?;
        }
        create_dir_all(&self.base_dir)?;

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
        let backend = FileBackend::new(temp_dir.path()).unwrap();

        let key = "test_key".to_string();
        let value = b"test_value".to_vec();

        backend.set(key.clone(), value.clone(), None).await.unwrap();

        let result = backend.get(&key).await.unwrap();
        assert_eq!(result, Some(value));

        assert!(backend.contains_key(&key).await.unwrap());

        backend.remove(&key).await.unwrap();
        assert_eq!(backend.get(&key).await.unwrap(), None);
        assert!(!backend.contains_key(&key).await.unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn test_ttl() {
        let temp_dir = tempdir().unwrap();
        let backend = FileBackend::new(temp_dir.path()).unwrap();

        let key = "test_ttl".to_string();
        let value = b"test_value".to_vec();

        backend
            .set(key.clone(), value, Some(Duration::from_millis(100)))
            .await
            .unwrap();

        assert!(backend.get(&key).await.unwrap().is_some());

        sleep(Duration::from_millis(150)).await;

        assert!(backend.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_clear() {
        let temp_dir = tempdir().unwrap();
        let backend = FileBackend::new(temp_dir.path()).unwrap();

        let key1 = "test_key1".to_string();
        let key2 = "test_key2".to_string();
        let value = b"test_value".to_vec();

        backend
            .set(key1.clone(), value.clone(), None)
            .await
            .unwrap();
        backend
            .set(key2.clone(), value.clone(), None)
            .await
            .unwrap();

        assert!(backend.contains_key(&key1).await.unwrap());
        assert!(backend.contains_key(&key2).await.unwrap());

        backend.clear().await.unwrap();

        assert!(!backend.contains_key(&key1).await.unwrap());
        assert!(!backend.contains_key(&key2).await.unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn test_metrics() {
        let temp_dir = tempdir().unwrap();
        let backend = FileBackend::new(temp_dir.path()).unwrap();

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
