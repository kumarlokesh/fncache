//! Minimal eviction policy test
//!
//! Tests the simplest possible eviction scenario to identify core issues

use fncache::{self, backends::memory::MemoryBackend};
use std::cell::Cell;

// Enable debug logging for more visibility
const ENABLE_DEBUG: bool = true;

/// Simple macro for debug logging
macro_rules! debug {
    ($($arg:tt)*) => {
        if ENABLE_DEBUG {
            println!($($arg)*)
        }
    };
}

#[test]
fn test_minimal_lru_eviction() {
    #[cfg(feature = "test-utils")]
    fncache::reset_global_cache_for_testing();

    let mut config = fncache::backends::memory::MemoryBackendConfig::default();
    config.max_capacity = 2;
    config.eviction_policy = "lru".to_string();
    let backend = MemoryBackend::with_config(config);

    let _ = fncache::init_global_cache(backend);

    thread_local! {
        static COUNTER: Cell<u32> = Cell::new(0);
    }
    COUNTER.with(|c| c.set(0));

    debug!("Starting minimal LRU test with counter = 0");

    #[fncache::fncache(ttl = 60, backend = "global")]
    fn minimal_lru_test(id: u32) -> u32 {
        let val = id * 10;
        COUNTER.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
            println!("Function executed with id={}, counter={}", id, new_val);
        });
        val
    }

    let val1 = minimal_lru_test(1);
    let val2 = minimal_lru_test(2);
    println!("Added initial items: val1={}, val2={}", val1, val2);
    let count_after_fill = COUNTER.with(|c| c.get());
    println!("Counter after initial fill: {}", count_after_fill);
    assert_eq!(
        count_after_fill, 2,
        "Initial fill should execute function twice"
    );

    let val1_cached = minimal_lru_test(1);
    let val2_cached = minimal_lru_test(2);
    println!(
        "Retrieved cached values: val1={}, val2={}",
        val1_cached, val2_cached
    );
    let count_after_cache_hit = COUNTER.with(|c| c.get());
    println!(
        "Counter after accessing cached items: {}",
        count_after_cache_hit
    );
    assert_eq!(
        count_after_cache_hit, 2,
        "Cache hits should not increment counter"
    );

    let _val2_again = minimal_lru_test(2);
    println!("Accessed item 2 to make it most recently used");
    let val3 = minimal_lru_test(3);
    debug!(
        "CALL 4: Result = {}, Counter = {}",
        val3,
        COUNTER.with(|c| c.get())
    );
    println!("Added new item val3={}", val3);
    let count_after_new_item = COUNTER.with(|c| c.get());
    println!("Counter after adding new item: {}", count_after_new_item);
    assert_eq!(
        count_after_new_item, 3,
        "Adding new item should execute function once"
    );

    println!("Testing eviction behavior:");
    let val2_final = minimal_lru_test(2);
    debug!(
        "CALL 5: Result = {}, Counter = {}",
        val2_final,
        COUNTER.with(|c| c.get())
    );
    println!("Accessed item 2 (should be cached): {}", val2_final);
    debug!("CALL 6: minimal_lru_test(3) - expecting cached result");
    let val3_final = minimal_lru_test(3); // Should be cached
    debug!(
        "CALL 6: Result = {}, Counter = {}",
        val3_final,
        COUNTER.with(|c| c.get())
    );
    println!("Accessed item 3 (should be cached): {}", val3_final);
    debug!("CALL 7: minimal_lru_test(1) - expecting function re-execution (entry was evicted)");
    let val1_final = minimal_lru_test(1); // Should be re-executed (was evicted)
    debug!(
        "CALL 7: Result = {}, Counter = {}",
        val1_final,
        COUNTER.with(|c| c.get())
    );
    println!("Accessed item 1 (should be re-executed): {}", val1_final);

    let final_count = COUNTER.with(|c| c.get());
    debug!("Test summary: Total function calls = {}", final_count);
    println!("Final counter: {}", final_count);
    assert_eq!(
        final_count, 4,
        "Expected 4 executions total (2 initial + 1 new + 1 evicted)"
    );
}

#[test]
fn test_minimal_lfu_eviction() {
    #[cfg(feature = "test-utils")]
    fncache::reset_global_cache_for_testing();

    let mut config = fncache::backends::memory::MemoryBackendConfig::default();
    config.max_capacity = 2;
    config.eviction_policy = "lfu".to_string();
    let backend = MemoryBackend::with_config(config);

    let _ = fncache::init_global_cache(backend);

    thread_local! {
        static COUNTER: Cell<u32> = Cell::new(0);
    }
    COUNTER.with(|c| c.set(0));

    println!("Starting minimal LFU test with counter = 0");

    #[fncache::fncache(ttl = 60, backend = "global")]
    fn minimal_lfu_test(id: u32) -> u32 {
        let val = id * 10;
        COUNTER.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
            println!("Function executed with id={}, counter={}", id, new_val);
        });
        val
    }

    let val1 = minimal_lfu_test(1);
    let val2 = minimal_lfu_test(2);
    println!("Added initial items: val1={}, val2={}", val1, val2);
    let count_after_fill = COUNTER.with(|c| c.get());
    println!("Counter after initial fill: {}", count_after_fill);
    assert_eq!(
        count_after_fill, 2,
        "Initial fill should execute function twice"
    );

    let val2_hit1 = minimal_lfu_test(2);
    let val2_hit2 = minimal_lfu_test(2);
    println!("Accessed item 2 twice: {}, {}", val2_hit1, val2_hit2);
    let count_after_hits = COUNTER.with(|c| c.get());
    println!("Counter after accessing cached item: {}", count_after_hits);
    assert_eq!(
        count_after_hits, 2,
        "Cache hits should not increment counter"
    );

    let val3 = minimal_lfu_test(3);
    println!("Added new item val3={}", val3);
    let count_after_new_item = COUNTER.with(|c| c.get());
    println!("Counter after adding new item: {}", count_after_new_item);
    assert_eq!(
        count_after_new_item, 3,
        "Adding new item should execute function once"
    );

    println!("Testing eviction behavior:");
    let val2_final = minimal_lfu_test(2);
    println!("Accessed item 2 (should be cached): {}", val2_final);
    let val3_final = minimal_lfu_test(3);
    let val1_final = minimal_lfu_test(1);
    println!("Accessed item 1 (should be re-executed): {}", val1_final);

    let final_count = COUNTER.with(|c| c.get());
    println!("Final counter: {}", final_count);
    assert_eq!(
        final_count, 4,
        "Expected 4 executions total (2 initial + 1 new + 1 evicted)"
    );
}
