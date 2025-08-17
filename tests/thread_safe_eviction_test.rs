//! Thread-safe eviction test

use fncache::backends::memory::{MemoryBackend, MemoryBackendConfig};
use fncache::backends::CacheBackend;
use futures::executor::block_on;
use serial_test::serial;
use std::sync::Arc;

#[test]
#[serial]
fn test_thread_safe_lru_eviction() {
    let config = MemoryBackendConfig {
        max_capacity: 2,
        eviction_policy: "lru".to_string(),
        ..Default::default()
    };
    let backend = Arc::new(MemoryBackend::with_config(config));

    block_on(backend.clear()).unwrap();

    let key1 = "key1".to_string();
    let key2 = "key2".to_string();
    let key3 = "key3".to_string();
    let val1 = vec![1, 2, 3];
    let val2 = vec![4, 5, 6];
    let val3 = vec![7, 8, 9];

    block_on(backend.set(key1.clone(), val1.clone(), None)).unwrap();
    block_on(backend.set(key2.clone(), val2.clone(), None)).unwrap();

    let result1 = block_on(backend.get(&key1)).unwrap().unwrap();
    let result2 = block_on(backend.get(&key2)).unwrap().unwrap();
    assert_eq!(result1, val1);
    assert_eq!(result2, val2);

    let _ = block_on(backend.get(&key1)).unwrap();

    block_on(backend.set(key3.clone(), val3.clone(), None)).unwrap();

    let result1_again = block_on(backend.get(&key1)).unwrap();
    assert!(result1_again.is_some(), "Key1 should still be in cache");
    assert_eq!(result1_again.unwrap(), val1);
    let result3 = block_on(backend.get(&key3)).unwrap();
    assert!(result3.is_some(), "Key3 should be in cache");
    assert_eq!(result3.unwrap(), val3);

    let result2_again = block_on(backend.get(&key2)).unwrap();
    assert!(result2_again.is_none(), "Key2 should have been evicted");
}

#[test]
#[serial]
fn test_thread_safe_lfu_eviction() {
    let config = MemoryBackendConfig {
        max_capacity: 2,
        eviction_policy: "lfu".to_string(),
        ..Default::default()
    };
    let backend = Arc::new(MemoryBackend::with_config(config));

    block_on(backend.clear()).unwrap();

    let key1 = "key1".to_string();
    let key2 = "key2".to_string();
    let key3 = "key3".to_string();
    let val1 = vec![1, 2, 3];
    let val2 = vec![4, 5, 6];
    let val3 = vec![7, 8, 9];

    block_on(backend.set(key1.clone(), val1.clone(), None)).unwrap();
    block_on(backend.set(key2.clone(), val2.clone(), None)).unwrap();

    let result1 = block_on(backend.get(&key1)).unwrap().unwrap();
    let result2 = block_on(backend.get(&key2)).unwrap().unwrap();
    assert_eq!(result1, val1);
    assert_eq!(result2, val2);

    let _ = block_on(backend.get(&key2)).unwrap();
    let _ = block_on(backend.get(&key2)).unwrap();

    block_on(backend.set(key3.clone(), val3.clone(), None)).unwrap();

    let result2_again = block_on(backend.get(&key2)).unwrap();
    assert!(result2_again.is_some(), "Key2 should still be in cache");
    assert_eq!(result2_again.unwrap(), val2);
    let result3 = block_on(backend.get(&key3)).unwrap();
    assert!(result3.is_some(), "Key3 should be in cache");
    assert_eq!(result3.unwrap(), val3);

    let result1_again = block_on(backend.get(&key1)).unwrap();
    assert!(result1_again.is_none(), "Key1 should have been evicted");
}
