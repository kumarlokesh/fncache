//! Advanced metrics tests

use crate::backends::memory::{MemoryBackend, MemoryBackendConfig};
use crate::backends::CacheBackend;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_advanced_metrics() {
        let backend = MemoryBackend::with_config(MemoryBackendConfig {
            max_capacity: 10,
            eviction_policy: "lru".to_string(),
        });

        let metrics = backend.metrics();
        assert_eq!(metrics.entry_count(), 0);
        assert_eq!(metrics.total_bytes(), 0);

        backend
            .set("key1".to_string(), vec![1, 2, 3], None)
            .await
            .unwrap();
        backend
            .set("key2".to_string(), vec![4, 5, 6], None)
            .await
            .unwrap();

        let metrics = backend.metrics();
        assert_eq!(metrics.entry_count(), 2);
        assert!(metrics.total_bytes() > 0, "Total bytes should be non-zero");
        assert!(
            metrics.average_entry_size() > 0,
            "Average entry size should be non-zero"
        );

        assert_eq!(metrics.get_latency().count, 0);
        assert!(metrics.set_latency().count >= 2);
        assert!(
            metrics.average_set_latency_ns() > 0.0,
            "Set latency should be non-zero"
        );

        let _value = backend.get(&"key1".to_string()).await.unwrap();

        let metrics = backend.metrics();
        assert_eq!(metrics.get_latency().count, 1);
        assert!(
            metrics.average_get_latency_ns() > 0.0,
            "Get latency should be non-zero"
        );

        backend
            .set("key3".to_string(), vec![7, 8, 9], None)
            .await
            .unwrap();

        let metrics = backend.metrics();
        assert_eq!(metrics.entry_count(), 3);

        for i in 4..12 {
            backend
                .set(format!("key{}", i), vec![i as u8], None)
                .await
                .unwrap();
        }

        let metrics = backend.metrics();
        assert!(metrics.evictions() > 0, "Should have evictions");
        assert_eq!(metrics.entry_count(), 10); // Should be at max capacity
    }

    #[tokio::test]
    async fn test_metrics_size_tracking() {
        let backend = MemoryBackend::default();

        backend
            .set("size_key".to_string(), vec![1, 2, 3, 4, 5], None)
            .await
            .unwrap();

        let metrics = backend.metrics();
        let initial_size = metrics.total_bytes();
        assert!(initial_size > 0, "Initial size should be non-zero");

        backend
            .set(
                "size_key".to_string(),
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                None,
            )
            .await
            .unwrap();

        let metrics = backend.metrics();
        let new_size = metrics.total_bytes();
        assert!(
            new_size > initial_size,
            "New size should be larger than initial size"
        );
        assert_eq!(metrics.entry_count(), 1, "Entry count should remain 1");

        backend.remove(&"size_key".to_string()).await.unwrap();

        let metrics = backend.metrics();
        assert_eq!(
            metrics.total_bytes(),
            0,
            "Total bytes should be zero after removal"
        );
        assert_eq!(
            metrics.entry_count(),
            0,
            "Entry count should be zero after removal"
        );
    }
}
