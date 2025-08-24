//! Dedicated eviction policy tests
//!
//! These tests verify the LRU and LFU eviction policies work correctly.

use fncache;
use serial_test::serial;
use std::cell::Cell;
use std::thread;
use std::time::Duration;

#[test]
#[serial]
fn test_lru_eviction() {
    let capacity = 2;
    let mut config = fncache::backends::memory::MemoryBackendConfig::default();
    config.max_capacity = capacity;
    config.eviction_policy = "lru".to_string();

    let backend = fncache::backends::memory::MemoryBackend::with_config(config);

    #[cfg(feature = "test-utils")]
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(backend);

    #[fncache::fncache(ttl = 3600, backend = "global")]
    fn lru_test_function(id: u32) -> u32 {
        id * 10
    }

    assert_eq!(
        lru_test_function(1),
        10,
        "Function should return 10 for id=1"
    );
    assert_eq!(
        lru_test_function(2),
        20,
        "Function should return 20 for id=2"
    );

    let val1 = lru_test_function(1);
    let val2 = lru_test_function(2);
    assert_eq!(val1, 10, "Expected value for id=1");
    assert_eq!(val2, 20, "Expected value for id=2");

    let _ = lru_test_function(2);

    let val3 = lru_test_function(3);
    assert_eq!(val3, 30, "Expected value for id=3");

    assert_eq!(lru_test_function(2), 20, "val2 should still be cached");
    assert_eq!(lru_test_function(3), 30, "val3 should still be cached");
}

#[test]
#[serial]
#[ignore]
fn test_lfu_eviction() {
    let capacity = 3;
    let mut config = fncache::backends::memory::MemoryBackendConfig::default();
    config.max_capacity = capacity;
    config.eviction_policy = "lfu".to_string();

    let backend = fncache::backends::memory::MemoryBackend::with_config(config);

    #[cfg(feature = "test-utils")]
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(backend);

    thread::sleep(Duration::from_millis(100));

    #[cfg(feature = "test-utils")]
    fncache::invalidate_all_cache_entries();

    thread_local! {
        static COUNTER: Cell<u32> = Cell::new(0);
    }
    COUNTER.with(|c| c.set(0));

    #[fncache::fncache(ttl = 3600, backend = "global")]
    fn lfu_test_function(id: u32) -> u32 {
        let result = id * 10;
        COUNTER.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
        });
        result
    }

    let _val1 = lfu_test_function(1);
    let _val2 = lfu_test_function(2);
    let _val3 = lfu_test_function(3);

    thread::sleep(Duration::from_millis(50));

    COUNTER.with(|c| c.set(0));

    let _val1_again = lfu_test_function(1);
    let _val2_again = lfu_test_function(2);
    let _val3_again = lfu_test_function(3);

    assert_eq!(
        COUNTER.with(|c| c.get()),
        0,
        "Expected cached values not to execute function"
    );

    let _val2_again1 = lfu_test_function(2);
    let _val2_again2 = lfu_test_function(2);
    let _val2_again3 = lfu_test_function(2);
    let _val3_again1 = lfu_test_function(3);
    let _val3_again2 = lfu_test_function(3);

    assert_eq!(
        COUNTER.with(|c| c.get()),
        0,
        "Frequency increases should not execute function"
    );

    let _val4 = lfu_test_function(4);
    assert_eq!(
        COUNTER.with(|c| c.get()),
        1,
        "Only val4 should cause function execution"
    );

    COUNTER.with(|c| c.set(0));
    let _val2_final = lfu_test_function(2);
    let _val3_final = lfu_test_function(3);
    let _val4_final = lfu_test_function(4);
    assert_eq!(
        COUNTER.with(|c| c.get()),
        0,
        "val2, val3, and val4 should be cached"
    );

    let _val1_final = lfu_test_function(1);
    assert_eq!(
        COUNTER.with(|c| c.get()),
        1,
        "val1 should have been evicted"
    );
}
