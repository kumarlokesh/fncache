//! WebAssembly-specific backend implementation
//!
//! This backend uses browser's localStorage for cache storage when running in WASM environments.
//! It's enabled with the `wasm` feature flag.

#![cfg(feature = "wasm")]

use crate::backends::CacheBackend;
use crate::error::Result;

use async_trait::async_trait;
use js_sys::{Function, Reflect, JSON};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Storage};

use std::time::Duration;

/// A cache backend that uses browser's localStorage when running in WASM environments.
pub struct WasmStorageBackend {
    storage: Storage,
}

impl WasmStorageBackend {
    /// Creates a new WASM storage backend
    ///
    /// # Returns
    ///
    /// A new `WasmStorageBackend` instance
    ///
    /// # Errors
    ///
    /// Returns an error if the browser's window or localStorage is not available
    pub fn new() -> Result<Self> {
        let window =
            window().ok_or_else(|| crate::error::Error::Backend("Window not available".into()))?;

        let storage = window
            .local_storage()
            .map_err(|_| crate::error::Error::Backend("Failed to get localStorage".into()))?
            .ok_or_else(|| crate::error::Error::Backend("localStorage not available".into()))?;

        Ok(Self { storage })
    }

    /// Helper function to get key with TTL prefix
    fn get_ttl_key(&self, key: &str) -> String {
        format!("fncache_ttl_{}", key)
    }

    /// Helper function to log to browser console (useful for debugging)
    fn log(&self, msg: &str) {
        if let Some(window) = window() {
            if let Some(console) = window.document().and_then(|doc| doc.default_view()) {
                if let Ok(console) = Reflect::get(&console, &JsValue::from_str("console")) {
                    if let Ok(log_fn) = Reflect::get(&console, &JsValue::from_str("log")) {
                        let log_fn = log_fn.dyn_into::<Function>().unwrap();
                        let _ = log_fn.call1(&JsValue::null(), &JsValue::from_str(msg));
                    }
                }
            }
        }
    }
}

#[async_trait]
impl CacheBackend for WasmStorageBackend {
    async fn get(&self, key: &String) -> Result<Option<Vec<u8>>> {
        let ttl_key = self.get_ttl_key(key);
        if let Ok(Some(ttl_str)) = self.storage.get_item(&ttl_key) {
            let ttl: u64 = ttl_str.parse().map_err(|_| {
                crate::error::Error::Backend("Failed to parse TTL timestamp".into())
            })?;

            let now = js_sys::Date::now() as u64;
            if now > ttl {
                let _ = self.storage.remove_item(key);
                let _ = self.storage.remove_item(&ttl_key);
                return Ok(None);
            }
        }

        match self.storage.get_item(key) {
            Ok(Some(val)) => {
                let parsed = JSON::parse(&val).map_err(|_| {
                    crate::error::Error::Codec("Failed to parse JSON from localStorage".into())
                })?;

                let array = js_sys::Uint8Array::new(&parsed);
                let mut bytes = vec![0; array.length() as usize];
                array.copy_to(&mut bytes);

                Ok(Some(bytes))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::error::Error::Backend(format!(
                "localStorage error: {:?}",
                e
            ))),
        }
    }

    async fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let array = js_sys::Uint8Array::new_with_length(value.len() as u32);
        array.copy_from(&value);

        let json = JSON::stringify(&array).map_err(|_| {
            crate::error::Error::Codec("Failed to stringify Uint8Array to JSON".into())
        })?;

        self.storage
            .set_item(&key, &json.as_string().unwrap())
            .map_err(|_| {
                crate::error::Error::Backend("Failed to set item in localStorage".into())
            })?;

        if let Some(ttl_duration) = ttl {
            let now = js_sys::Date::now() as u64;
            let expiry = now + ttl_duration.as_millis() as u64;
            let ttl_key = self.get_ttl_key(&key);

            self.storage
                .set_item(&ttl_key, &expiry.to_string())
                .map_err(|_| {
                    crate::error::Error::Backend("Failed to set TTL in localStorage".into())
                })?;
        }

        Ok(())
    }

    async fn remove(&self, key: &String) -> Result<()> {
        self.storage.remove_item(key).map_err(|_| {
            crate::error::Error::Backend("Failed to remove item from localStorage".into())
        })?;

        let ttl_key = self.get_ttl_key(key);
        let _ = self.storage.remove_item(&ttl_key);

        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        self.storage
            .clear()
            .map_err(|_| crate::error::Error::Backend("Failed to clear localStorage".into()))?;

        Ok(())
    }
}
