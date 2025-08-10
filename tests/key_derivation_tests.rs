use fncache::{init_global_cache, FncacheError, MemoryBackend};
use std::sync::Once;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let backend = MemoryBackend::new();
        init_global_cache(backend).unwrap();
    });
}

#[fncache::fncache(ttl = 60)]
fn cached_runtime_function(a: i32, b: &str) -> Result<String, FncacheError> {
    Ok(format!("Runtime result: {} - {}", a, b))
}

#[fncache::fncache(ttl = 60, key_derivation = compile_time)]
fn cached_compile_time_function(a: i32, b: &str) -> Result<String, FncacheError> {
    Ok(format!("Compile-time result: {} - {}", a, b))
}

#[fncache::fncache(ttl = 60)]
async fn cached_async_runtime_function(a: i32, b: &str) -> Result<String, FncacheError> {
    Ok(format!("Async runtime result: {} - {}", a, b))
}
#[fncache::fncache(ttl = 60, key_derivation = compile_time)]
async fn cached_async_compile_time_function(a: i32, b: &str) -> Result<String, FncacheError> {
    Ok(format!("Async compile-time result: {} - {}", a, b))
}

#[tokio::test]
async fn test_runtime_key_derivation() {
    setup();

    let result1 = cached_runtime_function(1, "test").unwrap();
    assert_eq!(result1, "Runtime result: 1 - test");

    let result2 = cached_runtime_function(2, "test").unwrap();
    assert_eq!(result2, "Runtime result: 2 - test");

    let result3 = cached_runtime_function(1, "test").unwrap();
    assert_eq!(result3, "Runtime result: 1 - test");
}

#[tokio::test]
async fn test_compile_time_key_derivation() {
    setup();

    let result1 = cached_compile_time_function(1, "test").unwrap();
    assert_eq!(result1, "Compile-time result: 1 - test");

    let result2 = cached_compile_time_function(2, "different").unwrap();
    assert_eq!(result2, "Compile-time result: 2 - different");
}

#[tokio::test]
async fn test_async_runtime_key_derivation() {
    setup();

    let result1 = cached_async_runtime_function(1, "test").await.unwrap();
    assert_eq!(result1, "Async runtime result: 1 - test");

    let result2 = cached_async_runtime_function(2, "test").await.unwrap();
    assert_eq!(result2, "Async runtime result: 2 - test");

    let result3 = cached_async_runtime_function(1, "test").await.unwrap();
    assert_eq!(result3, "Async runtime result: 1 - test");
}

#[tokio::test]
async fn test_async_compile_time_key_derivation() {
    setup();

    let result1 = cached_async_compile_time_function(1, "test").await.unwrap();
    assert_eq!(result1, "Async compile-time result: 1 - test");

    let result2 = cached_async_compile_time_function(2, "different")
        .await
        .unwrap();
    assert_eq!(result2, "Async compile-time result: 2 - different");
}
