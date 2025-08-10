#![warn(missing_docs)]
//! # fncache
//!
//! A zero-boilerplate Rust library for function-level caching with pluggable backends.
//!
//! ```ignore
//! // Example usage (not actually run in tests due to proc-macro limitations)
//! use fncache::{fncache, init_global_cache, MemoryBackend};
//!
//! // Initialize the global cache with an in-memory backend
//! init_global_cache(MemoryBackend::new()).unwrap();
//!
//! #[fncache(ttl = 60)] // Cache for 60 seconds
//! fn expensive_operation(x: u64) -> u64 {
//!     println!("Performing expensive operation for {}", x);
//!     x * x
//! }
//! ```

use backends::CacheBackend;
use std::sync::{Mutex, OnceLock};

pub mod backends;
pub mod error;
pub mod eviction;
pub mod invalidation;
pub mod key_derivation;
pub mod metrics;
pub mod serialization;
mod utils;
pub mod warming;

#[cfg(test)]
mod invalidation_tests;

#[cfg(test)]
mod eviction_tests;

#[cfg(test)]
mod key_derivation_tests;

#[cfg(test)]
mod metrics_tests;

// Re-export error type for macro usage
pub use error::Error as FncacheError;

/// Internal structure to hold the cache backend
#[derive(Debug)]
pub struct GlobalCache(Box<dyn CacheBackend + Send + Sync>);

// Use a regular OnceLock for production code
#[cfg(not(any(test, feature = "test-utils")))]
static GLOBAL_CACHE: OnceLock<Mutex<GlobalCache>> = OnceLock::new();

// Use a regular Mutex for tests, allowing reset
#[cfg(any(test, feature = "test-utils"))]
static mut GLOBAL_CACHE: Option<Mutex<GlobalCache>> = None;

// Re-export commonly used items
pub use backends::memory::MemoryBackend;

/// Re-export of the proc macro for convenience.
///
/// This allows users to write `use fncache::fncache;`
/// instead of `use fncache_macros::fncache;`.
///
/// # Examples
///
/// ```ignore
/// // This example illustrates how to use fncache, but is not actually run in tests
/// // Import the necessary items
/// use fncache::{fncache, init_global_cache, MemoryBackend};
/// use fncache::FncacheError;
///
/// // Initialize the cache backend
/// init_global_cache(MemoryBackend::new()).unwrap();
///
/// // Cache the function result for 5 seconds
/// #[fncache(ttl = 5)]
/// fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
///
/// // For async functions
/// #[fncache(ttl = 10)]
/// async fn fetch_data(id: &str) -> std::result::Result<String, FncacheError> {
///     // Fetch data from some source
///     Ok(format!("data for {}", id))
/// }
/// ```
#[doc(inline)]
pub use fncache_macros::fncache;

/// The main cache result type.
pub type Result<T> = std::result::Result<T, error::Error>;

/// Initialize the global cache with the specified backend.
///
/// # Examples
///
/// ```no_run
/// use fncache::{init_global_cache, MemoryBackend};
///
/// // Initialize with the in-memory backend
/// init_global_cache(MemoryBackend::new()).unwrap();
/// ```
#[cfg(not(any(test, feature = "test-utils")))]
pub fn init_global_cache<B>(backend: B) -> Result<()>
where
    B: CacheBackend + Send + Sync + 'static,
{
    let global_cache = GlobalCache(Box::new(backend));
    GLOBAL_CACHE
        .set(Mutex::new(global_cache))
        .map_err(|_| error::Error::AlreadyInitialized)?;
    Ok(())
}

/// Initialize the global cache with the specified backend (test version).
#[cfg(any(test, feature = "test-utils"))]
pub fn init_global_cache<B>(backend: B) -> Result<()>
where
    B: CacheBackend + Send + Sync + 'static,
{
    // SAFETY: We're in test code and this is synchronized by tests using #[serial]
    unsafe {
        GLOBAL_CACHE = Some(Mutex::new(GlobalCache(Box::new(backend))));
    }
    Ok(())
}

/// Get a reference to the global cache.
///
/// # Panics
///
/// Panics if the global cache has not been initialized.
#[cfg(not(any(test, feature = "test-utils")))]
pub fn global_cache() -> &'static Mutex<GlobalCache> {
    GLOBAL_CACHE
        .get()
        .expect("Global cache not initialized. Call init_global_cache first.")
}

/// Get a reference to the global cache (test version).
///
/// # Panics
///
/// Panics if the global cache has not been initialized.
#[cfg(any(test, feature = "test-utils"))]
#[allow(static_mut_refs)]
pub fn global_cache() -> &'static Mutex<GlobalCache> {
    unsafe {
        GLOBAL_CACHE
            .as_ref()
            .expect("Global cache not initialized. Call init_global_cache first.")
    }
}

/// Reset the global cache for testing purposes.
///
/// This should only be used in tests and never in production code.
///
/// # Safety
///
/// This function should only be used in test code, using the test-utils feature.
/// It allows resetting the global cache between tests to avoid AlreadyInitialized errors.
#[cfg(any(test, feature = "test-utils"))]
pub fn reset_global_cache_for_testing() {
    unsafe {
        GLOBAL_CACHE = None;
    }
}

#[async_trait::async_trait]
impl CacheBackend for GlobalCache {
    async fn get(&self, key: &String) -> Result<Option<Vec<u8>>> {
        self.0.get(key).await
    }

    async fn set(
        &self,
        key: String,
        value: Vec<u8>,
        ttl: Option<std::time::Duration>,
    ) -> Result<()> {
        self.0.set(key, value, ttl).await
    }

    async fn remove(&self, key: &String) -> Result<()> {
        self.0.remove(key).await
    }

    async fn contains_key(&self, key: &String) -> Result<bool> {
        self.0.contains_key(key).await
    }

    async fn clear(&self) -> Result<()> {
        self.0.clear().await
    }
}

/// Common prelude for using the library.
pub mod prelude {
    pub use crate::{
        backends::{Backend, CacheBackend},
        error::Error,
        fncache, global_cache, init_global_cache,
        metrics::Metrics,
        Result,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_global_cache_initialization() {
        init_global_cache(MemoryBackend::new()).unwrap();
        let _cache = global_cache();
    }

    #[test]
    #[should_panic(expected = "Global cache not initialized")]
    #[serial]
    fn test_global_cache_uninitialized() {
        static TEST_CACHE: OnceLock<Mutex<GlobalCache>> = OnceLock::new();
        let _ = TEST_CACHE.get().expect("Global cache not initialized");
    }
}
