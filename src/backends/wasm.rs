//! WebAssembly-specific backend implementation for browser environments.
//!
//! This backend provides a cache implementation that uses the browser's localStorage
//! for persistent storage when running in WebAssembly environments. It enables caching
//! functionality in web applications compiled to WASM.
//!
//! # Features
//!
//! * Browser-based persistent storage via localStorage API
//! * TTL (time-to-live) support using separate expiry timestamps
//! * Automatic cleanup of expired entries on access
//! * Efficient storage using JSON-serialized Uint8Array
//! * Compatible with standard web browsers
//!
//! # Usage
//!
//! This backend is available when the `wasm` feature flag is enabled.
//! It provides a way to cache function results in browser-based WASM applications.
//!
//! ```rust,no_run
//! use fncache::{backends::wasm::WasmStorageBackend, init_global_cache, fncache};
//! use std::time::Duration;
//!
//! #[wasm_bindgen(start)]
//! pub fn start() -> Result<(), JsValue> {
//!     // Initialize the WASM backend using browser localStorage
//!     let backend = WasmStorageBackend::new().expect("Failed to initialize browser storage");
//!     init_global_cache(backend).expect("Failed to initialize global cache");
//!
//!     // Define a cached function with TTL of 5 minutes
//!     #[fncache(ttl = 300)]
//!     fn compute_value(input: u32) -> Vec<u8> {
//!         // Expensive computation that should be cached
//!         vec![input as u8; 1024]
//!     }
//!
//!     // Use the function - first call stores in localStorage
//!     let _ = compute_value(42);
//!     // Subsequent calls retrieve from localStorage until TTL expires
//!     let _ = compute_value(42);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Implementation Details
//!
//! * Uses browser's localStorage API for persistent storage
//! * Stores binary data as JSON-serialized Uint8Array
//! * TTL is implemented using separate keys with timestamp prefixes
//! * Keys are prefixed with "fncache_ttl_" for expiration tracking
//! * Automatically cleans up expired entries when accessed

#![cfg(feature = "wasm")]

use crate::backends::CacheBackend;
use crate::error::Result;

use async_trait::async_trait;
use js_sys::{Function, Reflect, JSON};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Storage};

use std::time::Duration;

/// A cache backend for WebAssembly that uses browser's localStorage API.
///
/// This backend provides persistent caching capability for WASM applications
/// running in browsers by leveraging the localStorage API. It supports TTL-based
/// expiration, binary data storage, and conforms to the `CacheBackend` trait.
///
/// # Features
///
/// * Browser-persistent storage across page reloads
/// * TTL implementation with automatic expiration
/// * Binary data storage using JSON serialization
/// * Standard cache operations: get, set, remove, clear
///
/// # Example
///
/// ```rust,no_run
/// use fncache::backends::wasm::WasmStorageBackend;
/// use std::time::Duration;
///
/// # async fn example() -> fncache::Result<()> {
/// // Create a new WASM backend using browser's localStorage
/// let backend = WasmStorageBackend::new()?;
///
/// // Store a value with 1-hour TTL
/// let key = "user:profile:123".to_string();
/// let value = b"{\"name\": \"John Doe\"}".to_vec();
/// backend.set(key.clone(), value, Some(Duration::from_secs(3600))).await?;
///
/// // Retrieve the value later (even after page reload)
/// if let Some(data) = backend.get(&key).await? {
///     // Process retrieved data
///     console_log!("Retrieved {} bytes of data", data.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Storage Format
///
/// * Values are stored as JSON-serialized Uint8Array objects
/// * TTL information is stored in separate keys with "fncache_ttl_" prefix
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

/// Implementation of the CacheBackend trait for WasmStorageBackend
///
/// This implementation provides:
/// * Browser-persistent storage via localStorage
/// * TTL support with automatic expiration
/// * Binary data storage through Uint8Array serialization
/// * Automatic cleanup of expired entries
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
