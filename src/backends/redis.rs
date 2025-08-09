//! Redis cache backend implementation.
//!
//! This module provides a Redis-based cache backend that stores cache entries
//! in a Redis server.

use crate::{backends::CacheBackend, error::Error, metrics::Metrics, Result};
use async_trait::async_trait;
use redis::aio::Connection;
use redis::{AsyncCommands, Client, RedisError};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

/// Entry stored in the Redis cache
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    /// The cached value as bytes
    value: Vec<u8>,
    /// When the entry was created
    created_at: u64,
}

/// Redis-based cache backend
///
/// This backend stores cache entries in a Redis server.
#[derive(Debug)]
pub struct RedisBackend {
    /// Redis client
    client: Client,
    /// Key prefix for all cache entries
    prefix: String,
    /// Cache metrics
    metrics: Arc<Metrics>,
}

impl RedisBackend {
    /// Creates a new RedisBackend with the given Redis URL.
    ///
    /// # Arguments
    /// * `redis_url` - The URL to the Redis server (e.g., "redis://127.0.0.1:6379")
    /// * `prefix` - Optional prefix for all cache keys to avoid collisions
    ///
    /// # Returns
    /// A new RedisBackend instance wrapped in a Result
    ///
    /// # Errors
    /// Returns an error if connection to Redis fails
    pub async fn new(redis_url: &str, prefix: Option<&str>) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::Backend(format!("Failed to create Redis client: {}", e)))?;

        let mut conn = client
            .get_async_connection()
            .await
            .map_err(|e| Error::Backend(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            prefix: prefix.unwrap_or("fncache:").to_string(),
            metrics: Arc::new(Metrics::new()),
        })
    }

    /// Generate a prefixed key for Redis storage
    fn prefixed_key(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key)
    }

    /// Convert Redis errors to fncache errors
    fn convert_redis_error(err: RedisError) -> Error {
        Error::Backend(format!("Redis error: {}", err))
    }

    /// Convert a system time to Unix timestamp in seconds
    fn system_time_to_timestamp(time: SystemTime) -> u64 {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs()
    }

    /// Calculate the TTL in seconds from a duration
    fn duration_to_ttl_secs(duration: Duration) -> i64 {
        duration.as_secs() as i64
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &String) -> Result<Option<Vec<u8>>> {
        let redis_key = self.prefixed_key(key);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| Error::Backend(format!("Failed to connect to Redis: {}", e)))?;

        let result: redis::RedisResult<Option<String>> = conn.get(&redis_key).await;

        match result {
            Ok(Some(json_str)) => match serde_json::from_str::<CacheEntry>(&json_str) {
                Ok(entry) => {
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
            },
            Ok(None) => {
                self.metrics.record_miss();
                Ok(None)
            }
            Err(e) => {
                self.metrics.record_miss();
                Err(Self::convert_redis_error(e))
            }
        }
    }

    async fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let redis_key = self.prefixed_key(&key);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| Error::Backend(format!("Failed to connect to Redis: {}", e)))?;

        let entry = CacheEntry {
            value,
            created_at: Self::system_time_to_timestamp(SystemTime::now()),
        };

        let json_str = serde_json::to_string(&entry)
            .map_err(|e| Error::Codec(format!("Failed to serialize cache entry: {}", e)))?;

        let result = match ttl {
            Some(duration) => {
                let ttl_secs = Self::duration_to_ttl_secs(duration);
                conn.set_ex(redis_key, json_str, ttl_secs as usize).await
            }
            None => conn.set(redis_key, json_str).await,
        };

        match result {
            Ok(_) => {
                self.metrics.record_insertion();
                Ok(())
            }
            Err(e) => Err(Self::convert_redis_error(e)),
        }
    }

    async fn remove(&self, key: &String) -> Result<()> {
        let redis_key = self.prefixed_key(key);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| Error::Backend(format!("Failed to connect to Redis: {}", e)))?;

        let result: redis::RedisResult<i64> = conn.del(redis_key).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Self::convert_redis_error(e)),
        }
    }

    async fn contains_key(&self, key: &String) -> Result<bool> {
        let redis_key = self.prefixed_key(key);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| Error::Backend(format!("Failed to connect to Redis: {}", e)))?;

        let result: redis::RedisResult<bool> = conn.exists(redis_key).await;

        match result {
            Ok(exists) => Ok(exists),
            Err(e) => Err(Self::convert_redis_error(e)),
        }
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| Error::Backend(format!("Failed to connect to Redis: {}", e)))?;

        let pattern = format!("{}*", self.prefix);
        let keys: redis::RedisResult<Vec<String>> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await;

        match keys {
            Ok(keys) => {
                if !keys.is_empty() {
                    let result: redis::RedisResult<()> =
                        redis::cmd("DEL").arg(keys).query_async(&mut conn).await;

                    match result {
                        Ok(_) => Ok(()),
                        Err(e) => Err(Self::convert_redis_error(e)),
                    }
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(Self::convert_redis_error(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // These tests require a running Redis server at localhost:6379
    // They're marked with #[ignore] by default to avoid breaking CI

    async fn create_test_backend() -> Result<RedisBackend> {
        RedisBackend::new("redis://127.0.0.1:6379", Some("test:")).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn test_get_set() -> Result<()> {
        let backend = create_test_backend().await?;

        backend.clear().await?;

        let key = "test_key".to_string();
        let value = b"test_value".to_vec();

        backend.set(key.clone(), value.clone(), None).await?;

        let result = backend.get(&key).await?;
        assert_eq!(result, Some(value));

        assert!(backend.contains_key(&key).await?);

        backend.remove(&key).await?;
        assert_eq!(backend.get(&key).await?, None);
        assert!(!backend.contains_key(&key).await?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn test_ttl() -> Result<()> {
        let backend = create_test_backend().await?;

        backend.clear().await?;

        let key = "test_ttl".to_string();
        let value = b"test_value".to_vec();

        backend
            .set(key.clone(), value, Some(Duration::from_secs(1)))
            .await?;

        assert!(backend.get(&key).await?.is_some());

        tokio::time::sleep(Duration::from_secs(2)).await;

        assert!(backend.get(&key).await?.is_none());

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn test_clear() -> Result<()> {
        let backend = create_test_backend().await?;

        backend.clear().await?;

        let key1 = "test_key1".to_string();
        let key2 = "test_key2".to_string();
        let value = b"test_value".to_vec();

        backend.set(key1.clone(), value.clone(), None).await?;
        backend.set(key2.clone(), value.clone(), None).await?;

        assert!(backend.contains_key(&key1).await?);
        assert!(backend.contains_key(&key2).await?);

        backend.clear().await?;

        assert!(!backend.contains_key(&key1).await?);
        assert!(!backend.contains_key(&key2).await?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn test_metrics() -> Result<()> {
        let backend = create_test_backend().await?;

        backend.clear().await?;

        let key = "test_metrics".to_string();
        let value = b"test_value".to_vec();

        assert_eq!(backend.metrics.hits(), 0);
        assert_eq!(backend.metrics.misses(), 0);

        assert!(backend.get(&key).await?.is_none());
        assert_eq!(backend.metrics.misses(), 1);

        backend.set(key.clone(), value, None).await?;

        assert!(backend.get(&key).await?.is_some());
        assert_eq!(backend.metrics.hits(), 1);

        Ok(())
    }
}
