//! Key derivation strategies for cache keys
//!
//! This module provides different strategies for generating cache keys,
//! including runtime and compile-time approaches.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Strategy to use for deriving cache keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivation {
    /// Runtime key derivation: uses actual parameter values
    Runtime,

    /// Compile-time key derivation: uses function signature information only
    CompileTime,
}

impl Default for KeyDerivation {
    fn default() -> Self {
        KeyDerivation::Runtime
    }
}

/// Generate a compile-time key for a function
///
/// This creates a deterministic hash based on the function name,
/// module path, parameter types, and return type.
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

/// Extracts the type name from a type as a string
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
