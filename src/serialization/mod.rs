//! Serialization support for fncache.
//!
//! This module provides trait definitions and implementations for
//! serializing and deserializing cache values with different formats.

use crate::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// Trait for serializing and deserializing cache values.
pub trait Serializer: Send + Sync + Debug {
    /// Serialize a value into bytes.
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>>;

    /// Deserialize bytes into a value.
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T>;
}

/// Bincode serializer implementation.
#[cfg(feature = "bincode")]
#[derive(Debug, Clone, Copy)]
pub struct BincodeSerializer;

#[cfg(feature = "bincode")]
impl BincodeSerializer {
    /// Create a new BincodeSerializer.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "bincode")]
impl Default for BincodeSerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "bincode")]
impl Serializer for BincodeSerializer {
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        bincode::serialize(value).map_err(|e| crate::error::Error::Codec(format!("{}", e)))
    }

    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        bincode::deserialize(bytes).map_err(|e| crate::error::Error::Codec(format!("{}", e)))
    }
}

/// JSON serializer implementation.
#[cfg(feature = "serde_json")]
#[derive(Debug, Clone, Copy)]
pub struct JsonSerializer;

#[cfg(feature = "serde_json")]
impl JsonSerializer {
    /// Create a new JsonSerializer.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "serde_json")]
impl Default for JsonSerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "serde_json")]
impl Serializer for JsonSerializer {
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        serde_json::to_vec(value)
            .map_err(|e| crate::error::Error::Codec(format!("JSON serialization error: {}", e)))
    }

    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        serde_json::from_slice(bytes)
            .map_err(|e| crate::error::Error::Codec(format!("JSON deserialization error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        value: i32,
    }

    #[test]
    #[cfg(any(feature = "file-backend", feature = "rocksdb-backend"))]
    fn test_bincode_serializer() {
        let serializer = BincodeSerializer::new();

        let value = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let bytes = serializer.serialize(&value).unwrap();

        let deserialized: TestStruct = serializer.deserialize(&bytes).unwrap();

        assert_eq!(value, deserialized);
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_json_serializer() {
        let serializer = JsonSerializer::new();

        let value = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let bytes = serializer.serialize(&value).unwrap();

        let deserialized: TestStruct = serializer.deserialize(&bytes).unwrap();

        assert_eq!(value, deserialized);
    }
}
