//! Key derivation strategies for cache keys
//!
//! This module provides different strategies for generating unique and consistent cache keys
//! for function caching. The choice of key derivation strategy affects how cache hits and
//! misses are determined.
//!
//! # Key Derivation Approaches
//!
//! The library supports two main approaches for key derivation:
//!
//! * **Runtime Key Derivation**: Creates cache keys based on the runtime values of function
//!   arguments. This is the default approach and provides high-precision caching, where functions
//!   are only considered cache hits when called with identical argument values.
//!
//! * **Compile-Time Key Derivation**: Creates cache keys based on the function's signature
//!   (name, module path, parameter types, and return type) without considering actual parameter
//!   values. This is useful for functions where you want to cache based on the function identity
//!   rather than specific arguments, such as functions that fetch fresh data regardless of input.
//!
//! # Examples
//!
//! ```
//! use fncache::key_derivation::KeyDerivation;
//! use fncache::{fncache, MemoryBackend, init_global_cache};
//!
//! // Initialize global cache
//! init_global_cache(MemoryBackend::new()).unwrap();
//!
//! // Default runtime key derivation - cache based on parameter values
//! #[fncache]
//! fn calculate_with_runtime_key(x: i32, y: i32) -> i32 {
//!     println!("Computing result");
//!     x + y
//! }
//!
//! // Compile-time key derivation - cache based on function signature only
//! #[fncache(key_derivation = "CompileTime")]
//! fn fetch_with_compile_time_key(resource_id: i32) -> String {
//!     println!("Fetching resource");
//!     format!("Resource {}", resource_id)
//! }
//!
//! // First call computes, second call uses cache
//! calculate_with_runtime_key(1, 2); // Prints "Computing result"
//! calculate_with_runtime_key(1, 2); // Uses cached result
//! calculate_with_runtime_key(3, 4); // Prints "Computing result" (different parameters)
//!
//! // With compile-time keys, all calls with any parameters use the same cache entry
//! fetch_with_compile_time_key(1); // Prints "Fetching resource"
//! fetch_with_compile_time_key(2); // Uses cached result despite different parameter
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Strategy to use for deriving cache keys.
///
/// This enum represents the available key derivation strategies that determine
/// how cache keys are generated for cached functions.
///
/// # Examples
///
/// ```
/// use fncache::key_derivation::KeyDerivation;
///
/// // Using runtime key derivation (default)
/// let strategy1 = KeyDerivation::default();  // Returns Runtime
///
/// // Explicitly selecting compile-time key derivation
/// let strategy2 = KeyDerivation::CompileTime;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivation {
    /// Runtime key derivation: uses actual parameter values to compute cache keys.
    ///
    /// With this strategy, functions are only considered cache hits when called with
    /// identical parameter values. This is the default strategy and provides the most
    /// precise caching behavior.
    Runtime,

    /// Compile-time key derivation: uses function signature information only.
    ///
    /// With this strategy, cache keys are derived from the function's name,
    /// module path, parameter types, and return type, without considering the
    /// actual parameter values. This means all calls to the same function will
    /// use the same cache entry regardless of parameter values.
    CompileTime,
}

impl Default for KeyDerivation {
    fn default() -> Self {
        KeyDerivation::Runtime
    }
}

/// Generate a compile-time key for a function.
///
/// This creates a deterministic hash based on the function name,
/// module path, parameter types, and return type. The hash is consistent
/// across multiple runs of the program, as long as the function signature
/// doesn't change.
///
/// This function is primarily used by the `fncache` procedural macro when
/// `key_derivation = "CompileTime"` is specified.
///
/// # Arguments
///
/// * `fn_name` - The function name (e.g., "add", "fetch_data")
/// * `mod_path` - The module path of the function (e.g., "my_crate::utils")
/// * `param_type_names` - A slice of parameter type names as strings
/// * `return_type_name` - The return type name as a string
///
/// # Returns
///
/// A 64-bit hash value that uniquely identifies the function signature
///
/// # Examples
///
/// ```
/// use fncache::key_derivation::generate_compile_time_key;
///
/// let key = generate_compile_time_key(
///     "add",                      // Function name
///     "my_app::math",           // Module path
///     &["i32", "i32"],          // Parameter types
///     "i32",                    // Return type
/// );
///
/// // This will always produce the same hash for the same inputs
/// ```
pub fn generate_compile_time_key(
    fn_name: &str,
    mod_path: &str,
    param_type_names: &[&str],
    return_type_name: &str,
) -> u64 {
    let mut hasher = DefaultHasher::new();

    fn_name.hash(&mut hasher);
    mod_path.hash(&mut hasher);

    for param_type in param_type_names {
        param_type.hash(&mut hasher);
    }
    return_type_name.hash(&mut hasher);

    hasher.finish()
}

/// Extracts the type name from a type as a string.
///
/// This helper function uses Rust's `std::any::type_name` to extract the
/// fully qualified type name of a type. It is used by the `fncache` procedural
/// macro to get the type names for compile-time key generation.
///
/// This function is only available when the `compile-time-keys` feature is enabled.
///
/// # Type Parameters
///
/// * `T` - The type to extract the name from
///
/// # Arguments
///
/// * `_` - An unused reference to a value of type `T` (used only for type inference)
///
/// # Returns
///
/// A static string containing the fully qualified type name
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "compile-time-keys")]
/// # {
/// use fncache::key_derivation::type_name_of;
///
/// let name1 = type_name_of(&42_i32);               // "i32"
/// let name2 = type_name_of(&"hello".to_string());  // "alloc::string::String"
/// # }
/// ```
#[cfg(feature = "compile-time-keys")]
pub fn type_name_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_time_key_generation() {
        let key1 = generate_compile_time_key(
            "test_fn",
            "fncache::test",
            &["i32", "String", "bool"],
            "Result<String, Error>",
        );

        let key2 = generate_compile_time_key(
            "test_fn",
            "fncache::test",
            &["i32", "String", "bool"],
            "Result<String, Error>",
        );

        assert_eq!(key1, key2);

        let key3 = generate_compile_time_key(
            "other_fn",
            "fncache::test",
            &["i32", "String", "bool"],
            "Result<String, Error>",
        );

        assert_ne!(key1, key3);

        let key4 = generate_compile_time_key(
            "test_fn",
            "fncache::test",
            &["i32", "u64", "bool"],
            "Result<String, Error>",
        );

        assert_ne!(key1, key4);
    }
}
