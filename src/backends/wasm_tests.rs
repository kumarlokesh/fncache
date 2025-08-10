//! Tests for the WASM backend
//!
//! Note: These tests are meant to run in a browser environment with wasm-pack.
//! They won't run in the standard Rust test environment.

#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use wasm_bindgen_test::*;
use wasm_bindgen_futures::JsFuture;
use crate::backends::wasm::WasmStorageBackend;
use crate::backends::CacheBackend;
use std::time::Duration;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_wasm_backend_set_get() {
    let backend = WasmStorageBackend::new().expect("Failed to create WASM backend");
    
    let test_key = "test_key".to_string();
    let test_value = vec![1, 2, 3, 4];
    backend.set(test_key.clone(), test_value.clone(), None)
        .await
        .expect("Failed to set value");
    
    let result = backend.get(&test_key).await.expect("Failed to get value");
    
    assert_eq!(result, Some(test_value), "Retrieved value should match stored value");
}

#[wasm_bindgen_test]
async fn test_wasm_backend_remove() {
    let backend = WasmStorageBackend::new().expect("Failed to create WASM backend");
    
    let test_key = "test_remove_key".to_string();
    backend.set(test_key.clone(), vec![5, 6, 7], None)
        .await
        .expect("Failed to set value");
    
    let exists = backend.contains_key(&test_key).await.expect("Failed to check key");
    assert!(exists, "Key should exist before removal");
    
    backend.remove(&test_key).await.expect("Failed to remove key");

    let exists = backend.contains_key(&test_key).await.expect("Failed to check key");
    assert!(!exists, "Key should not exist after removal");
}

#[wasm_bindgen_test]
async fn test_wasm_backend_ttl() {
    use wasm_bindgen::JsCast;
    
    let backend = WasmStorageBackend::new().expect("Failed to create WASM backend");
    
    let test_key = "test_ttl_key".to_string();
    backend.set(test_key.clone(), vec![8, 9, 10], Some(Duration::from_millis(100)))
        .await
        .expect("Failed to set value");
    
    let result = backend.get(&test_key).await.expect("Failed to get value");
    assert!(result.is_some(), "Value should exist before TTL expiration");
    
    let window = web_sys::window().unwrap();
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let closure = wasm_bindgen::closure::Closure::once(move || {
            resolve.call0(&JsValue::NULL).unwrap();
        });
        
        window.set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            150,
        ).unwrap();
        
        closure.forget();
    });
    
    JsFuture::from(promise).await.unwrap();
    
    let result = backend.get(&test_key).await.expect("Failed to get value");
    assert!(result.is_none(), "Value should be gone after TTL expiration");
}

#[wasm_bindgen_test]
async fn test_wasm_backend_clear() {
    let backend = WasmStorageBackend::new().expect("Failed to create WASM backend");
    
    backend.set("clear_test_1".to_string(), vec![1], None).await.expect("Failed to set value 1");
    backend.set("clear_test_2".to_string(), vec![2], None).await.expect("Failed to set value 2");
    backend.set("clear_test_3".to_string(), vec![3], None).await.expect("Failed to set value 3");
    
    backend.clear().await.expect("Failed to clear cache");
    
    let result1 = backend.get(&"clear_test_1".to_string()).await.expect("Failed to get value 1");
    let result2 = backend.get(&"clear_test_2".to_string()).await.expect("Failed to get value 2");
    let result3 = backend.get(&"clear_test_3".to_string()).await.expect("Failed to get value 3");
    
    assert!(result1.is_none(), "Value 1 should be gone after clear");
    assert!(result2.is_none(), "Value 2 should be gone after clear");
    assert!(result3.is_none(), "Value 3 should be gone after clear");
}
