//! Dedicated eviction policy tests
//!
//! These tests verify the LRU and LFU eviction policies work correctly.

use fncache;
use serial_test::serial;
use std::cell::Cell;

#[test]
#[serial]
fn test_lru_eviction() {
    println!("\n=== Starting LRU Eviction Test ===");

    let capacity = 2;
    let mut config = fncache::backends::memory::MemoryBackendConfig::default();
    config.max_capacity = capacity;
    config.eviction_policy = "lru".to_string();

    let backend = fncache::backends::memory::MemoryBackend::with_config(config);
    println!("Created LRU backend with max_capacity={}", capacity);

    #[cfg(feature = "test-utils")]
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(backend);

    thread_local! {
        static COUNTER: Cell<u32> = Cell::new(0);
    }
    COUNTER.with(|c| c.set(0));
    println!("Initial COUNTER: {}", COUNTER.with(|c| c.get()));

    #[fncache::fncache(ttl = 3600, backend = "global")]
    fn lru_test_function(id: u32) -> u32 {
        let result = id * 10;
        COUNTER.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
            println!("Function executed with id={}, counter={}", id, new_val);
        });
        result
    }

    println!("\n=== Phase 1: Filling cache to capacity (2) ===\n");
    let val1 = lru_test_function(1);
    let val2 = lru_test_function(2);
    println!("Added items: val1={}, val2={}", val1, val2);
    println!("COUNTER after initial fill: {}", COUNTER.with(|c| c.get()));

    println!("\n=== Phase 2: Verifying cache hits ===\n");
    let counter_before_cache_test = COUNTER.with(|c| c.get());
    let val1_cached = lru_test_function(1);
    let val2_cached = lru_test_function(2);
    println!(
        "Retrieved cached values: val1={}, val2={}",
        val1_cached, val2_cached
    );
    let counter_after_cache_test = COUNTER.with(|c| c.get());
    println!(
        "COUNTER before cache test: {}, after: {}",
        counter_before_cache_test, counter_after_cache_test
    );
    assert_eq!(
        counter_before_cache_test, counter_after_cache_test,
        "Cache hits should not increment counter"
    );

    println!("\n=== Phase 3: Preparing eviction order ===\n");
    println!("Accessing item 2 to make it most recently used...");
    let _val2_again = lru_test_function(2);
    println!("\n=== Phase 4: Adding item beyond capacity ===\n");
    println!("Adding item 3 (should evict item 1)...");
    let val3 = lru_test_function(3);
    println!("Added new item: val3={}", val3);

    println!("\n=== Phase 5: Testing eviction ===\n");
    println!("Accessing item 2 (should be cached)...");
    let val2_final = lru_test_function(2);
    println!("Accessing item 3 (should be cached)...");
    let val3_final = lru_test_function(3);
    println!("Accessing item 1 (should be evicted and re-executed)...");
    let val1_final = lru_test_function(1);
    println!(
        "Retrieved values: val2={}, val3={}, val1={}",
        val2_final, val3_final, val1_final
    );

    let counter_before_eviction_test = counter_after_cache_test + 1;
    let counter_after_eviction_test = COUNTER.with(|c| c.get());
    let eviction_executions = counter_after_eviction_test - counter_before_eviction_test;
    println!("\nEviction test executions: {}", eviction_executions);
    assert_eq!(
        eviction_executions, 1,
        "Expected 1 execution (for evicted item 1), found {}",
        eviction_executions
    );
}

#[test]
#[serial]
fn test_lfu_eviction() {
    println!("\n=== Starting LFU Eviction Test ===");

    let capacity = 3;
    let mut config = fncache::backends::memory::MemoryBackendConfig::default();
    config.max_capacity = capacity;
    config.eviction_policy = "lfu".to_string();

    let backend = fncache::backends::memory::MemoryBackend::with_config(config);
    println!("Created LFU backend with max_capacity={}", capacity);

    #[cfg(feature = "test-utils")]
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(backend);

    thread_local! {
        static COUNTER: Cell<u32> = Cell::new(0);
    }
    COUNTER.with(|c| c.set(0));
    println!("Initial COUNTER: {}", COUNTER.with(|c| c.get()));

    #[fncache::fncache(ttl = 3600, backend = "global")]
    fn lfu_test_function(id: u32) -> u32 {
        let result = id * 10;
        COUNTER.with(|c| {
            let new_val = c.get() + 1;
            c.set(new_val);
            println!("Function executed with id={}, counter={}", id, new_val);
        });
        result
    }

    println!("\n=== Phase 1: Filling cache to capacity (3) ===\n");
    println!("Adding items 1, 2, and 3 to cache...");
    let val1 = lfu_test_function(1);
    let val2 = lfu_test_function(2);
    let val3 = lfu_test_function(3);
    println!(
        "Initial cache values: val1={}, val2={}, val3={}",
        val1, val2, val3
    );
    println!("COUNTER after initial fill: {}", COUNTER.with(|c| c.get()));

    println!("\n=== Phase 2: Verifying cache hits ===\n");
    let counter_before_cache_test = COUNTER.with(|c| c.get());
    let val1_cached = lfu_test_function(1);
    let val2_cached = lfu_test_function(2);
    let val3_cached = lfu_test_function(3);
    println!(
        "Retrieved cached values: val1={}, val2={}, val3={}",
        val1_cached, val2_cached, val3_cached
    );
    let counter_after_cache_test = COUNTER.with(|c| c.get());
    println!(
        "COUNTER before cache test: {}, after: {}",
        counter_before_cache_test, counter_after_cache_test
    );
    assert_eq!(
        counter_before_cache_test, counter_after_cache_test,
        "Cache hits should not increment counter"
    );

    println!("\n=== Phase 3: Establishing access frequencies ===\n");
    println!("Accessing item 2 three times...");
    let _val2_again1 = lfu_test_function(2);
    let _val2_again2 = lfu_test_function(2);
    let _val2_again3 = lfu_test_function(2);

    println!("Accessing item 3 two times...");
    let _val3_again1 = lfu_test_function(3);
    let _val3_again2 = lfu_test_function(3);

    println!("Item 1 accessed only once");

    println!("\n=== Phase 4: Adding item beyond capacity ===\n");
    println!("Adding item 4 (should evict item 1)...");
    let val4 = lfu_test_function(4);
    println!("Added new item: val4={}", val4);

    println!("\n=== Phase 5: Testing eviction ===\n");
    println!("Accessing item 2 (should be cached, used 4 times)...");
    let val2_final = lfu_test_function(2);
    println!("Accessing item 3 (should be cached, used 3 times)...");
    let val3_final = lfu_test_function(3);
    println!("Accessing item 4 (should be cached, newest)...");
    let val4_final = lfu_test_function(4);
    println!("Accessing item 1 (should be evicted and re-executed)...");
    let val1_final = lfu_test_function(1);
    println!(
        "Retrieved values: val2={}, val3={}, val4={}, val1={}",
        val2_final, val3_final, val4_final, val1_final
    );

    let expected_final_counter = counter_after_cache_test + 2;
    let actual_final_counter = COUNTER.with(|c| c.get());
    println!(
        "\nExpected final counter: {}, actual: {}",
        expected_final_counter, actual_final_counter
    );
    assert_eq!(
        actual_final_counter, expected_final_counter,
        "Expected {} total executions, found {}",
        expected_final_counter, actual_final_counter
    );
}
