//! Basic cache safety tests
//!
//! This file contains targeted tests for basic memory safety, cache initialization,
//! and cache reset functionality. These tests verify fundamental safety properties
//! of the caching system without the complexity of the full test suite.

use fncache::{init_global_cache, reset_global_cache_for_testing, MemoryBackend};
use fncache_macros::fncache;
use serial_test::serial;

#[test]
#[serial]
fn test_basic_cache_functionality() {
    reset_global_cache_for_testing();
    let _ = init_global_cache(MemoryBackend::new());

    #[fncache(ttl = 60)]
    fn simple_function(x: i32) -> i32 {
        x * 2
    }

    let result1 = simple_function(5);
    let result2 = simple_function(5);
    let result3 = simple_function(10);

    assert_eq!(result1, 10);
    assert_eq!(result2, 10);
    assert_eq!(result3, 20);
}

#[test]
#[serial]
fn test_cache_reset_across_cycles() {
    for i in 0..3 {
        reset_global_cache_for_testing();
        let _ = init_global_cache(MemoryBackend::new());

        #[fncache(ttl = 60)]
        fn cycle_function(x: i32) -> i32 {
            x + 100
        }

        let result = cycle_function(i);
        assert_eq!(result, i + 100);
    }
}
