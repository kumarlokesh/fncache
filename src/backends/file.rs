//! File-based cache backend implementation.
//!
//! This module provides a file-based cache backend that stores cache entries
//! in the local filesystem.

use crate::{backends::CacheBackend, error::Error, metrics::Metrics, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, create_dir_all, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;

/// Entry stored in the file cache
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// The cached value as bytes
    value: Vec<u8>,
    /// When the entry expires (if ever)
    expires_at: Option<SystemTime>,
}

/// File-based cache backend
///
/// This backend stores cache entries as individual files in a directory structure.
/// Each key is hashed to create a file path, ensuring safe filenames.
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
