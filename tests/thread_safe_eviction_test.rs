//! Thread-safe eviction test
//!
//! This test isolates the eviction policy testing to avoid segmentation faults

use fncache::{self, backends::memory::MemoryBackend};
use std::cell::Cell;
use std::sync::Once;

static INIT: Once = Once::new();

#[test]
fn test_thread_safe_lru_eviction() {
    INIT.call_once(|| {
        #[cfg(feature = "test-utils")]
        fncache::reset_global_cache_for_testing();

        let mut config = fncache::backends::memory::MemoryBackendConfig::default();
        config.max_capacity = 2;
        config.eviction_policy = "lru".to_string();
        let backend = MemoryBackend::with_config(config);
        let _ = fncache::init_global_cache(backend);
    });

    thread_local! {
        static COUNTER: Cell<u32> = Cell::new(0);
    }
    COUNTER.with(|c| c.set(0));

    #[fncache::fncache(ttl = 3600, backend = "global")]
    fn safe_lru_test_fn(id: u32) -> u32 {
        let result = id * 10;
        COUNTER.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
        });
        result
    }

    let val1 = safe_lru_test_fn(1);
    let val2 = safe_lru_test_fn(2);

    assert_eq!(val1, 10);
    assert_eq!(val2, 20);
    assert_eq!(COUNTER.with(|c| c.get()), 2);

    let val1_cached = safe_lru_test_fn(1);
    let val2_cached = safe_lru_test_fn(2);

    assert_eq!(val1_cached, 10);
    assert_eq!(val2_cached, 20);
    assert_eq!(COUNTER.with(|c| c.get()), 2);

    let _val2_again = safe_lru_test_fn(2);

    let val3 = safe_lru_test_fn(3);
    assert_eq!(val3, 30);
    assert_eq!(COUNTER.with(|c| c.get()), 3);

    let val2_final = safe_lru_test_fn(2);
    let val3_final = safe_lru_test_fn(3);
    let val1_final = safe_lru_test_fn(1);
    assert_eq!(val2_final, 20);
    assert_eq!(val3_final, 30);
    assert_eq!(val1_final, 10);
    assert_eq!(
        COUNTER.with(|c| c.get()),
        4,
        "Expected 1 new execution for evicted item 1"
    );
}

#[test]
fn test_thread_safe_lfu_eviction() {
    static LFU_INIT: Once = Once::new();
    LFU_INIT.call_once(|| {
        #[cfg(feature = "test-utils")]
        fncache::reset_global_cache_for_testing();

        let mut config = fncache::backends::memory::MemoryBackendConfig::default();
        config.max_capacity = 3;
        config.eviction_policy = "lfu".to_string();
        let backend = MemoryBackend::with_config(config);

        let _ = fncache::init_global_cache(backend);
    });

    thread_local! {
        static COUNTER_LFU: Cell<u32> = Cell::new(0);
    }
    COUNTER_LFU.with(|c| c.set(0));

    #[fncache::fncache(ttl = 3600, backend = "global")]
    fn safe_lfu_test_fn(id: u32) -> u32 {
        let result = id * 10;
        COUNTER_LFU.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
        });
        result
    }

    let val1 = safe_lfu_test_fn(1);
    let val2 = safe_lfu_test_fn(2);
    let val3 = safe_lfu_test_fn(3);
    assert_eq!(val1, 10);
    assert_eq!(val2, 20);
    assert_eq!(val3, 30);
    assert_eq!(
        COUNTER_LFU.with(|c| c.get()),
        3,
        "Initial fill should execute function 3 times"
    );

    let val1_cached = safe_lfu_test_fn(1);
    let val2_cached = safe_lfu_test_fn(2);
    let val3_cached = safe_lfu_test_fn(3);
    assert_eq!(val1_cached, 10);
    assert_eq!(val2_cached, 20);
    assert_eq!(val3_cached, 30);
    assert_eq!(
        COUNTER_LFU.with(|c| c.get()),
        3,
        "Cached access should not execute function"
    );

    let _val2_again1 = safe_lfu_test_fn(2);
    let _val2_again2 = safe_lfu_test_fn(2);
    let _val3_again1 = safe_lfu_test_fn(3);
    assert_eq!(
        COUNTER_LFU.with(|c| c.get()),
        3,
        "Repeated cache hits should not execute function"
    );

    let val4 = safe_lfu_test_fn(4);
    assert_eq!(val4, 40);
    assert_eq!(
        COUNTER_LFU.with(|c| c.get()),
        4,
        "Adding new item should execute function once"
    );

    let val2_final = safe_lfu_test_fn(2);
    let val3_final = safe_lfu_test_fn(3);
    let val4_final = safe_lfu_test_fn(4);
    let val1_final = safe_lfu_test_fn(1);

    assert_eq!(val2_final, 20);
    assert_eq!(val3_final, 30);
    assert_eq!(val4_final, 40);
    assert_eq!(val1_final, 10);

    let final_count = COUNTER_LFU.with(|c| c.get());
    assert_eq!(
        final_count, 5,
        "Expected 5 total executions (3 initial + 1 new item + 1 evicted), got {}",
        final_count
    );
}
