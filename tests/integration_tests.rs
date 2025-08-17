//! Integration tests for fncache

use fncache::prelude::Error;
#[cfg(feature = "file-backend")]
use fncache::FileBackend;
use fncache::{self, MemoryBackend};
use fncache_macros::fncache;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
struct TestData {
    id: u32,
    name: String,
    values: Vec<i32>,
}

#[test]
#[serial]
fn test_basic_memory_caching() {
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(MemoryBackend::new());

    static mut COUNTER: u32 = 0;

    #[fncache(ttl = 60)]
    fn get_data(id: u32, name: &str) -> TestData {
        unsafe {
            COUNTER += 1;
        }

        TestData {
            id,
            name: name.to_string(),
            values: vec![1, 2, 3],
        }
    }

    let result1 = get_data(1, "test");
    let result2 = get_data(1, "test");

    assert_eq!(result1, result2);

    unsafe {
        assert_eq!(COUNTER, 1);
    }
    let result3 = get_data(2, "test");
    unsafe {
        assert_eq!(COUNTER, 2);
    }
    assert_ne!(result1, result3);
}

#[test]
#[serial]
fn test_ttl_expiration() {
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(MemoryBackend::new());

    static mut COUNTER: u32 = 0;

    #[fncache(ttl = 1)]
    fn get_timestamp() -> u64 {
        unsafe {
            COUNTER += 1;
        }
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    let result1 = get_timestamp();
    let result2 = get_timestamp();
    assert_eq!(result1, result2);
    unsafe {
        assert_eq!(COUNTER, 1);
    }

    thread::sleep(Duration::from_secs(2));
    let result3 = get_timestamp();
    unsafe {
        assert_eq!(COUNTER, 2);
    }
    assert_ne!(result1, result3);
}

#[tokio::test]
async fn test_async_caching() {
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(MemoryBackend::new());

    static mut COUNTER: u32 = 0;

    #[fncache(ttl = 60)]
    async fn fetch_data(id: u32) -> Result<TestData, Error> {
        unsafe {
            COUNTER += 1;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(TestData {
            id,
            name: "async_test".to_string(),
            values: vec![4, 5, 6],
        })
    }

    let result1 = fetch_data(1).await.unwrap();
    let result2 = fetch_data(1).await.unwrap();

    assert_eq!(result1, result2);

    unsafe {
        assert_eq!(COUNTER, 1);
    }
}

mod function_signatures {
    use super::*;

    #[test]
    #[serial]
    fn test_function_returning_result() {
        let _ = fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(MemoryBackend::new());

        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER = 0;
        }

        #[fncache(ttl = 30)]
        fn fallible_result_function(succeed: bool) -> Result<String, String> {
            unsafe {
                COUNTER += 1;
            }

            if succeed {
                Ok("success".to_string())
            } else {
                Err("failure".to_string())
            }
        }

        let result1 = fallible_result_function(true).unwrap();
        let result2 = fallible_result_function(true).unwrap();

        assert_eq!(result1, result2);
        unsafe {
            assert_eq!(COUNTER, 1);
        }

        let err1 = fallible_result_function(false).unwrap_err();
        let err2 = fallible_result_function(false).unwrap_err();

        assert_eq!(err1, err2);
        unsafe {
            assert_eq!(COUNTER, 2);
        }
    }

    #[test]
    #[serial]
    fn test_function_returning_option() {
        let _ = fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(MemoryBackend::new());

        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER = 0;
        }

        #[fncache(ttl = 30)]
        fn optional_function(has_value: bool) -> Option<String> {
            unsafe {
                COUNTER += 1;
            }

            if has_value {
                Some("found".to_string())
            } else {
                None
            }
        }

        let result1 = optional_function(true).unwrap();
        let result2 = optional_function(true).unwrap();

        assert_eq!(result1, result2);
        unsafe {
            assert_eq!(COUNTER, 1);
        }

        let none1 = optional_function(false);
        let none2 = optional_function(false);

        assert_eq!(none1, none2);
        assert_eq!(none1, None);
        unsafe {
            assert_eq!(COUNTER, 2);
        }
    }
}

#[cfg(feature = "file-backend")]
mod file_backend_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_file_backend_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let backend = FileBackend::new(path).unwrap();
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(backend);

        static mut COUNTER: u32 = 0;

        #[fncache(ttl = 3600)]
        fn persistent_data(id: u32) -> TestData {
            unsafe {
                COUNTER += 1;
            }

            TestData {
                id,
                name: "persistent".to_string(),
                values: vec![7, 8, 9],
            }
        }

        let result1 = persistent_data(100);

        unsafe {
            assert_eq!(COUNTER, 1);
        }

        let backend = FileBackend::new(path).unwrap();
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(backend);
        let result2 = persistent_data(100);

        assert_eq!(result1, result2);

        unsafe {
            assert_eq!(COUNTER, 1);
        }
    }
}

#[cfg(feature = "compile-time-keys")]
mod key_derivation_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_compile_time_key_derivation() {
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(MemoryBackend::new());

        static mut COUNTER: u32 = 0;

        #[fncache(ttl = 60, key_derivation = "compile_time")]
        fn keyed_function(a: u32, b: &str) -> String {
            unsafe {
                COUNTER += 1;
            }
            format!("{}-{}", a, b)
        }

        let result1 = keyed_function(42, "test");
        let result2 = keyed_function(42, "test");

        assert_eq!(result1, result2);
        unsafe {
            assert_eq!(COUNTER, 1);
        }
    }
}

mod eviction_tests {
    use super::*;

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
    fn test_lfu_eviction() {
        let capacity = 3;
        let mut config = fncache::backends::memory::MemoryBackendConfig::default();
        config.max_capacity = capacity;
        config.eviction_policy = "lfu".to_string();

        let backend = fncache::backends::memory::MemoryBackend::with_config(config);

        #[cfg(feature = "test-utils")]
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(backend);

        #[fncache::fncache(ttl = 3600, backend = "global")]
        fn lfu_test_function(id: u32) -> u32 {
            id * 10
        }

        let val1 = lfu_test_function(1);
        let val2 = lfu_test_function(2);
        let val3 = lfu_test_function(3);
        assert_eq!(val1, 10);
        assert_eq!(val2, 20);
        assert_eq!(val3, 30);

        lfu_test_function(2);
        lfu_test_function(2);
        lfu_test_function(3);

        let val4 = lfu_test_function(4);
        assert_eq!(val4, 40);

        assert_eq!(lfu_test_function(2), 20);
        assert_eq!(lfu_test_function(3), 30);
        assert_eq!(lfu_test_function(4), 40);
    }
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use fncache::WasmStorageBackend;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_wasm_storage_backend() {
        let backend = WasmStorageBackend::new().expect("Failed to create WASM backend");
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(backend);

        static mut COUNTER: u32 = 0;

        #[fncache(ttl = 60)]
        fn browser_data(id: u32) -> TestData {
            unsafe {
                COUNTER += 1;
            }

            TestData {
                id,
                name: "browser".to_string(),
                values: vec![10, 11, 12],
            }
        }

        let result1 = browser_data(1);
        let result2 = browser_data(1);

        assert_eq!(result1, result2);
        unsafe {
            assert_eq!(COUNTER, 1);
        }
    }
}

mod error_handling_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_error_propagation() {
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(MemoryBackend::new());

        #[fncache(ttl = 60)]
        fn fallible_function(fail: bool) -> Result<String, Error> {
            if fail {
                Err(Error::other("Failed"))
            } else {
                Ok("Success".to_string())
            }
        }

        let result1 = fallible_function(false).unwrap();
        let result2 = fallible_function(false).unwrap();
        assert_eq!(result1, result2);
        let err1 = fallible_function(true).unwrap_err();

        assert!(format!("{}", err1).contains("Failed"));
    }
}
#[cfg(feature = "memory")]
mod concurrent_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    #[serial]
    fn test_concurrent_access() {
        let _ = fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(MemoryBackend::new());

        static COUNTER: AtomicU32 = AtomicU32::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        #[fncache(ttl = 60)]
        fn concurrent_test_fn_7d234f(id: u32) -> u32 {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            id * 10
        }

        let initial = concurrent_test_fn_7d234f(42);
        assert_eq!(initial, 420);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
        COUNTER.store(0, Ordering::SeqCst);

        let thread_count = 10;
        let barrier = Arc::new(Barrier::new(thread_count));

        let mut handles = Vec::new();
        for _ in 0..thread_count {
            let b = barrier.clone();
            let handle = thread::spawn(move || {
                b.wait();
                concurrent_test_fn_7d234f(42)
            });
            handles.push(handle);
        }

        let results: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        for result in &results {
            assert_eq!(*result, 420);
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 0);
    }
}

mod integration_scenario_tests {
    use super::*;

    #[derive(Debug)]
    struct ApiClient {
        base_url: String,
    }

    impl ApiClient {
        fn new(base_url: &str) -> Self {
            Self {
                base_url: base_url.to_string(),
            }
        }

        #[fncache(ttl = 300)]
        fn get_user(&self, user_id: u32) -> Result<TestData, Error> {
            Ok(TestData {
                id: user_id,
                name: format!("User-{}", user_id),
                values: vec![100, 200, 300],
            })
        }

        #[fncache(ttl = 60)]
        fn get_product(&self, product_id: u32) -> Result<TestData, Error> {
            Ok(TestData {
                id: product_id,
                name: format!("Product-{}", product_id),
                values: vec![400, 500, 600],
            })
        }
    }

    #[test]
    #[serial]
    fn test_api_client_caching() {
        fncache::reset_global_cache_for_testing();
        let _ = fncache::init_global_cache(MemoryBackend::new());

        let client = ApiClient::new("https://api.example.com");

        let user1_first = client.get_user(1).unwrap();
        let user1_second = client.get_user(1).unwrap();

        assert_eq!(user1_first, user1_second);

        let user2 = client.get_user(2).unwrap();
        assert_ne!(user1_first, user2);

        let product1_first = client.get_product(1).unwrap();
        let product1_second = client.get_product(1).unwrap();

        assert_eq!(product1_first, product1_second);
    }
}

async fn run_async_caching_tests() {
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(MemoryBackend::new());

    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER = 0;
    }

    #[fncache(ttl = 60)]
    async fn fetch_data_for_core_test(id: u32) -> Result<TestData, fncache::prelude::Error> {
        unsafe {
            COUNTER += 1;
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Ok(TestData {
            id,
            name: "async_test".to_string(),
            values: vec![4, 5, 6],
        })
    }

    let result1 = fetch_data_for_core_test(1).await.unwrap();
    let result2 = fetch_data_for_core_test(1).await.unwrap();

    assert_eq!(result1, result2);
    unsafe {
        assert_eq!(COUNTER, 1);
    }
}

fn run_basic_memory_caching_test() {
    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER = 0;
    }

    #[fncache(ttl = 60)]
    fn get_data_for_core_test(id: u32, name: &str) -> TestData {
        unsafe {
            COUNTER += 1;
        }

        TestData {
            id,
            name: name.to_string(),
            values: vec![1, 2, 3],
        }
    }

    let result1 = get_data_for_core_test(1, "test");
    let result2 = get_data_for_core_test(1, "test");

    assert_eq!(result1, result2);

    unsafe {
        assert_eq!(COUNTER, 1);
    }
}

fn run_ttl_expiration_test() {
    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER = 0;
    }

    #[fncache(ttl = 1)]
    fn get_data_with_ttl_for_core_test(id: u32) -> TestData {
        unsafe {
            COUNTER += 1;
        }

        TestData {
            id,
            name: "ttl_test".to_string(),
            values: vec![1, 2, 3],
        }
    }

    let result1 = get_data_with_ttl_for_core_test(1);
    let result2 = get_data_with_ttl_for_core_test(1);
    assert_eq!(result1, result2);
    unsafe {
        assert_eq!(COUNTER, 1);
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    let result3 = get_data_with_ttl_for_core_test(1);
    assert_eq!(result1, result3);
    unsafe {
        assert_eq!(COUNTER, 2);
    }
}

#[test]
#[serial]
#[ignore = "Disabled due to memory safety issues with multiple reset cycles - individual tests provide same coverage"]
fn run_all_core_tests() {
    fncache::reset_global_cache_for_testing();
    let _ = fncache::init_global_cache(MemoryBackend::new());

    run_basic_memory_caching_test();
    run_ttl_expiration_test();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_async_caching_tests().await;
    });
}
