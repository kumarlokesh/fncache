use crate::backends::memory::{MemoryBackend, MemoryBackendConfig};
use crate::backends::CacheBackend;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lru_eviction_with_capacity_limit() {
        let backend = MemoryBackend::with_config(MemoryBackendConfig {
            max_capacity: 2,
            eviction_policy: "lru".to_string(),
        });

        backend
            .set("key1".to_string(), vec![1, 2, 3], None)
            .await
            .unwrap();
        backend
            .set("key2".to_string(), vec![4, 5, 6], None)
            .await
            .unwrap();

        backend.get(&"key1".to_string()).await.unwrap();

        backend
            .set("key3".to_string(), vec![7, 8, 9], None)
            .await
            .unwrap();

        assert!(backend.contains_key(&"key1".to_string()).await.unwrap());
        assert!(!backend.contains_key(&"key2".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key3".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_lfu_eviction_with_capacity_limit() {
        let backend = MemoryBackend::with_config(MemoryBackendConfig {
            max_capacity: 2,
            eviction_policy: "lfu".to_string(),
        });

        backend
            .set("key1".to_string(), vec![1, 2, 3], None)
            .await
            .unwrap();

        backend.get(&"key1".to_string()).await.unwrap();
        backend.get(&"key1".to_string()).await.unwrap();

        backend
            .set("key2".to_string(), vec![4, 5, 6], None)
            .await
            .unwrap();
        backend
            .set("key3".to_string(), vec![7, 8, 9], None)
            .await
            .unwrap();

        let store_len = backend.get_store_len().await;

        assert_eq!(
            store_len, 2,
            "Cache should have exactly 2 items, found {}",
            store_len
        );

        assert!(backend.contains_key(&"key1".to_string()).await.unwrap());
        assert!(!backend.contains_key(&"key2".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key3".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_eviction_policy_change() {
        let backend = MemoryBackend::with_config(MemoryBackendConfig {
            max_capacity: 2,
            eviction_policy: "lfu".to_string(),
        });

        backend
            .set("key1".to_string(), vec![1, 2, 3], None)
            .await
            .unwrap();
        backend
            .set("key2".to_string(), vec![4, 5, 6], None)
            .await
            .unwrap();

        backend.get(&"key2".to_string()).await.unwrap();
        backend.get(&"key2".to_string()).await.unwrap();

        backend
            .set("key3".to_string(), vec![7, 8, 9], None)
            .await
            .unwrap();

        let store_len = backend.get_store_len().await;
        assert_eq!(
            store_len, 2,
            "Cache should have exactly 2 items, found {}",
            store_len
        );

        assert!(!backend.contains_key(&"key1".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key2".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key3".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_ttl_with_eviction_policy() {
        let backend = MemoryBackend::with_config(MemoryBackendConfig {
            max_capacity: 3,
            eviction_policy: "lru".to_string(),
        });

        backend
            .set(
                "key1".to_string(),
                vec![1, 2, 3],
                Some(Duration::from_millis(50)),
            )
            .await
            .unwrap();
        backend
            .set(
                "key2".to_string(),
                vec![4, 5, 6],
                Some(Duration::from_millis(150)),
            )
            .await
            .unwrap();
        backend
            .set("key3".to_string(), vec![7, 8, 9], None)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(75)).await;

        assert!(!backend.contains_key(&"key1".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key2".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key3".to_string()).await.unwrap());

        backend
            .set("key4".to_string(), vec![10, 11, 12], None)
            .await
            .unwrap();

        assert!(!backend.contains_key(&"key1".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key2".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key3".to_string()).await.unwrap());
        assert!(backend.contains_key(&"key4".to_string()).await.unwrap());
    }
}
