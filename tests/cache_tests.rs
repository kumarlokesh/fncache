// Only run these integration tests when the test-utils feature is enabled
// This is necessary because we need to reset the global cache between tests
#![cfg(feature = "test-utils")]

use fncache::{backends::memory::MemoryBackend, init_global_cache, FncacheError, Result};

// Only import reset_global_cache_for_testing when the test-utils feature is enabled
#[cfg(feature = "test-utils")]
use fncache::reset_global_cache_for_testing;
use serial_test::serial;
use std::time::Duration;
use tokio::time::sleep;

use bincode;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};

fn system_time_error_to_fncache_error(err: std::time::SystemTimeError) -> FncacheError {
    FncacheError::Backend(err.to_string())
}

fn setup_test_cache() -> Result<()> {
    #[cfg(feature = "test-utils")]
    reset_global_cache_for_testing();
    init_global_cache(MemoryBackend::new())
}

#[test]
#[serial]
fn test_sync_caching() -> Result<()> {
    setup_test_cache()?;

    #[fncache::fncache(ttl = 5)]
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    let result1 = add(2, 3);
    assert_eq!(result1, 5);

    let result2 = add(2, 3);
    assert_eq!(result2, 5);

    let result3 = add(3, 4);
    assert_eq!(result3, 7);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_async_caching() -> Result<()> {
    setup_test_cache()?;

    #[fncache::fncache(ttl = 5)]
    async fn multiply(a: i32, b: i32) -> i32 {
        sleep(Duration::from_millis(100)).await;
        a * b
    }

    let result1 = multiply(2, 3).await;
    assert_eq!(result1, 6);

    let result2 = multiply(2, 3).await;
    assert_eq!(result2, 6);

    let result3 = multiply(3, 4).await;
    assert_eq!(result3, 12);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_ttl_expiration() -> Result<()> {
    setup_test_cache()?;

    #[fncache::fncache(ttl = 1)]
    async fn get_timestamp() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    let timestamp1 = get_timestamp().await;
    let timestamp2 = get_timestamp().await;
    assert_eq!(timestamp1, timestamp2);

    sleep(Duration::from_secs(2)).await;

    let timestamp3 = get_timestamp().await;
    assert_ne!(timestamp1, timestamp3);

    Ok(())
}

#[test]
#[serial]
fn test_error_handling() -> Result<()> {
    setup_test_cache()?;

    #[fncache::fncache(ttl = 5)]
    fn might_fail(succeed: bool) -> Option<String> {
        if succeed {
            Some("success".to_string())
        } else {
            None
        }
    }

    let result = might_fail(true);
    assert_eq!(result, Some("success".to_string()));

    assert_eq!(might_fail(false), None);

    Ok(())
}

#[test]
#[serial]
fn test_different_argument_types() -> Result<()> {
    setup_test_cache()?;

    #[fncache::fncache(ttl = 5)]
    fn process_data(s: &str, nums: Vec<i32>) -> String {
        let sum: i32 = nums.iter().sum();
        format!("{}: {}", s, sum)
    }

    let result1 = process_data("test", vec![1, 2, 3]);
    assert_eq!(result1, "test: 6");

    let result2 = process_data("test", vec![1, 2, 3]);
    assert_eq!(result2, "test: 6");

    let result3 = process_data("sum", vec![4, 5, 6]);
    assert_eq!(result3, "sum: 15");

    Ok(())
}
