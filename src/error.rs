//! Error types for the fncache library.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The main error type for the fncache library.
#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
pub enum Error {
    /// An error that occurred during serialization or deserialization.
    #[error("Codec error: {0}")]
    Codec(String),

    /// The requested key was not found in the cache.
    #[error("Cache miss for key")]
    CacheMiss,

    /// The backend returned an error.
    #[error("Backend error: {0}")]
    Backend(String),

    /// An error that occurred while initializing the global cache more than once.
    #[error("global cache has already been initialized")]
    AlreadyInitialized,

    /// An error that doesn't fit into other categories.
    #[error("Cache error: {0}")]
    Other(String),
}

impl Error {
    /// Creates a new backend error.
    pub fn backend<E: fmt::Display>(error: E) -> Self {
        Self::Backend(error.to_string())
    }

    /// Creates a new other error.
    pub fn other<E: fmt::Display>(error: E) -> Self {
        Self::Other(error.to_string())
    }
}

/// A specialized `Result` type for cache operations.
pub type Result<T> = std::result::Result<T, Error>;
