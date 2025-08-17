//! # Cache Invalidation
//!
//! This module provides comprehensive mechanisms for selectively invalidating cached items
//! using tags and prefix-based matching.
//!
//! ## Key Features
//!
//! - **Tag-based Invalidation**: Associate tags with cached items for group invalidation
//! - **Prefix-based Invalidation**: Invalidate groups of items with key prefixes
//! - **Bulk Operations**: Invalidate multiple tags or prefixes in a single call
//! - **Both Sync and Async APIs**: Support for different programming models
//!
//! ## Usage Examples
//!
//! See `examples/cache_invalidation.rs` for comprehensive usage examples.
//!
//! ## Tag-Based Invalidation
//!
//! Tag-based invalidation allows associating metadata with cached entries
//! for selective invalidation. This is useful for complex invalidation scenarios
//! where keys alone don't provide enough context.
//!
//! ## Prefix-Based Invalidation
//!
//! Prefix-based invalidation relies on structured key naming to group
//! related cache entries. For instance, all user profile data might use keys
//! that start with `user:{id}:`

use crate::serialization::Serializer;
use crate::{backends::CacheBackend, error::Error, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;

/// Represents a tag attached to a cached item for invalidation purposes.
///
/// Tags allow you to associate metadata with cached items for group-based invalidation.
/// Multiple cached items can share the same tag, making it possible to invalidate
/// them all at once regardless of their key structure.
///
/// # Examples
///
/// ```
/// use fncache::invalidation::Tag;
///
/// // Create tags in different ways
/// let tag1 = Tag::new("user:123");
/// let tag2: Tag = "role:admin".into();  // From &str
/// let tag3 = Tag::from(String::from("product:456")); // From String
///
/// // Get the underlying string
/// assert_eq!(tag1.as_str(), "user:123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag(String);

impl Tag {
    /// Create a new tag from any type that can be converted into a String.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::invalidation::Tag;
    ///
    /// let tag1 = Tag::new("user:profile");
    /// let tag2 = Tag::new(String::from("post:123"));
    /// ```
    pub fn new<S: Into<String>>(tag: S) -> Self {
        Self(tag.into())
    }

    /// Get the tag value as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::invalidation::Tag;
    ///
    /// let tag = Tag::new("category:books");
    /// assert_eq!(tag.as_str(), "category:books");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Cache invalidation functionality for synchronous context.
///
/// This trait provides methods to selectively invalidate cache entries
/// using tags or key prefixes. It's designed for use in synchronous code
/// where you don't need async/await.
///
/// Implementations will typically use a runtime internally to execute
/// the async operations needed to interact with the underlying cache.
///
/// # Examples
///
/// ```no_run
/// use fncache::{invalidation::{CacheInvalidation, InvalidationCache, Tag}, MemoryBackend};
///
/// // Create a cache with invalidation support
/// let cache = InvalidationCache::new(MemoryBackend::new());
///
/// // Invalidate by tag
/// cache.invalidate_tag(&Tag::new("user:123")).unwrap();
///
/// // Invalidate by prefix
/// cache.invalidate_prefix("products:").unwrap();
///
/// // Invalidate multiple tags
/// cache.invalidate_tags(vec![Tag::new("category:books"), Tag::new("category:movies")]).unwrap();
/// ```
pub trait CacheInvalidation {
    /// Invalidate all cache entries with the given tag.
    ///
    /// This method finds all cache keys associated with the specified tag and removes them
    /// from the cache. It also updates internal tracking structures to maintain consistency.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    fn invalidate_tag(&self, tag: &Tag) -> Result<()>;

    /// Invalidate all cache entries with keys that start with the given prefix.
    ///
    /// This method finds all cache keys that start with the specified prefix and removes them
    /// from the cache. It's useful for invalidating groups of related items.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The key prefix to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    fn invalidate_prefix(&self, prefix: &str) -> Result<()>;

    /// Invalidate all cache entries with any of the given tags.
    ///
    /// This is a convenience method that invalidates multiple tags at once.
    ///
    /// # Arguments
    ///
    /// * `tags` - An iterator of tags to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    fn invalidate_tags<I>(&self, tags: I) -> Result<()>
    where
        I: IntoIterator<Item = Tag>;

    /// Invalidate all cache entries with keys that start with any of the given prefixes.
    ///
    /// This is a convenience method that invalidates multiple prefixes at once.
    ///
    /// # Arguments
    ///
    /// * `prefixes` - An iterator of prefixes to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    fn invalidate_prefixes<I>(&self, prefixes: I) -> Result<()>
    where
        I: IntoIterator<Item = String>;
}

/// Async version of cache invalidation functionality.
///
/// This trait extends the basic `CacheBackend` with tag and prefix-based invalidation
/// capabilities for asynchronous contexts. It provides async methods for invalidating
/// cache entries by tags or key prefixes.
///
/// # Examples
///
/// ```no_run
/// use fncache::{invalidation::{AsyncCacheInvalidation, InvalidationCache, Tag}, MemoryBackend};
///
/// async fn example() {
///     // Create a cache with invalidation support
///     let cache = InvalidationCache::new(MemoryBackend::new());
///     
///     // Invalidate by tag
///     cache.invalidate_tag(&Tag::new("user:123")).await.unwrap();
///     
///     // Invalidate by prefix
///     cache.invalidate_prefix("products:").await.unwrap();
///     
///     // Invalidate multiple tags
///     let tags = vec![Tag::new("category:books"), Tag::new("category:movies")];
///     cache.invalidate_tags(tags).await.unwrap();
/// }
/// ```
#[async_trait]
pub trait AsyncCacheInvalidation: Send + Sync + crate::backends::CacheBackend {
    /// Get all keys associated with a specific tag.
    ///
    /// This internal method is used to retrieve the set of cache keys
    /// that have been associated with a particular tag.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to query
    ///
    /// # Returns
    ///
    /// A `HashSet` containing all keys associated with the tag
    fn get_keys_by_tag(&self, tag: &Tag) -> HashSet<String>;

    /// Get all keys with a specific prefix.
    ///
    /// This internal method is used to retrieve the set of cache keys
    /// that begin with the specified prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The key prefix to query
    ///
    /// # Returns
    ///
    /// A `HashSet` containing all keys that start with the prefix
    fn get_keys_by_prefix(&self, prefix: &str) -> HashSet<String>;

    /// Invalidate all keys associated with the specified tag.
    ///
    /// Removes all cache entries that have been tagged with the given tag.
    /// This is useful for group invalidation of related items.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag identifying the group of items to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    async fn invalidate_tag(&self, tag: &Tag) -> Result<()> {
        let keys_to_remove = self.get_keys_by_tag(tag);

        for key in keys_to_remove {
            self.remove(&key).await?
        }
        Ok(())
    }

    /// Invalidate all keys with the specified prefix.
    ///
    /// Removes all cache entries whose keys start with the given prefix.
    /// This is useful for invalidating a collection of related items.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix identifying the group of items to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    async fn invalidate_prefix(&self, prefix: &str) -> Result<()> {
        let keys_to_remove = self.get_keys_by_prefix(prefix);

        for key in keys_to_remove {
            self.remove(&key).await?
        }
        Ok(())
    }

    /// Invalidate multiple tags at once.
    ///
    /// This is a convenience method that invalidates all cache entries
    /// associated with any of the specified tags.
    ///
    /// # Arguments
    ///
    /// * `tags` - An iterator of tags to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    async fn invalidate_tags<I>(&self, tags: I) -> Result<()>
    where
        I: IntoIterator<Item = Tag> + Send,
        I::IntoIter: Send,
    {
        let tags_vec: Vec<Tag> = tags.into_iter().collect();
        for tag in tags_vec {
            AsyncCacheInvalidation::invalidate_tag(self, &tag).await?;
        }
        Ok(())
    }

    /// Invalidate multiple prefixes at once.
    ///
    /// This is a convenience method that invalidates all cache entries
    /// whose keys start with any of the specified prefixes.
    ///
    /// # Arguments
    ///
    /// * `prefixes` - An iterator of prefixes to invalidate
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or an error if invalidation failed
    async fn invalidate_prefixes<I>(&self, prefixes: I) -> Result<()>
    where
        I: IntoIterator<Item = String> + Send,
        I::IntoIter: Send,
    {
        let prefixes_vec: Vec<String> = prefixes.into_iter().collect();
        for prefix in prefixes_vec {
            AsyncCacheInvalidation::invalidate_prefix(self, &prefix).await?;
        }
        Ok(())
    }
}

/// Extended cache entry data structure that supports associating tags with cached values.
///
/// This wrapper allows you to store a value along with tags for selective invalidation.
/// It's particularly useful when you need to manage complex invalidation scenarios where
/// simple key-based invalidation is insufficient.
///
/// # Examples
///
/// ```
/// use fncache::invalidation::{Tag, TaggedCacheEntry};
/// use std::collections::HashSet;
///
/// // Create a tagged entry with a single tag
/// let entry1 = TaggedCacheEntry::new("cached value")
///     .with_tag("user:123");
///
/// // Create a tagged entry with multiple tags
/// let entry2 = TaggedCacheEntry::new(42)
///     .with_tags(vec!["product:456", "category:electronics"]);
///
/// assert_eq!(entry1.value, "cached value");
/// assert!(entry1.tags.contains(&Tag::new("user:123")));
/// ```
#[derive(Debug, Clone)]
pub struct TaggedCacheEntry<T> {
    /// The cached value
    pub value: T,
    /// Associated tags for invalidation
    pub tags: HashSet<Tag>,
}

impl<T> TaggedCacheEntry<T> {
    /// Create a new tagged cache entry with the given value and no tags.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::invalidation::TaggedCacheEntry;
    ///
    /// let entry = TaggedCacheEntry::new("some value");
    /// assert_eq!(entry.value, "some value");
    /// assert!(entry.tags.is_empty());
    /// ```
    pub fn new(value: T) -> Self {
        Self {
            value,
            tags: HashSet::new(),
        }
    }

    /// Add a single tag to this cache entry.
    ///
    /// This method consumes and returns self to enable method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::invalidation::{Tag, TaggedCacheEntry};
    ///
    /// let entry = TaggedCacheEntry::new(123)
    ///     .with_tag("user:456");
    ///
    /// assert!(entry.tags.contains(&Tag::new("user:456")));
    /// ```
    pub fn with_tag(mut self, tag: impl Into<Tag>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Add multiple tags to this cache entry.
    ///
    /// This method consumes and returns self to enable method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use fncache::invalidation::{Tag, TaggedCacheEntry};
    ///
    /// let entry = TaggedCacheEntry::new("product info")
    ///     .with_tags(vec!["product:789", "category:books"]);
    ///
    /// assert!(entry.tags.contains(&Tag::new("product:789")));
    /// assert!(entry.tags.contains(&Tag::new("category:books")));
    /// ```
    pub fn with_tags<I>(mut self, tags: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Tag>,
    {
        for tag in tags {
            self.tags.insert(tag.into());
        }
        self
    }
}

/// Cache backend wrapper that adds tag-based invalidation functionality
#[derive(Debug)]
pub struct InvalidationCache<B> {
    backend: Arc<B>,
    tag_to_keys: std::sync::Mutex<std::collections::HashMap<Tag, HashSet<String>>>,
    prefixes: std::sync::Mutex<std::collections::HashMap<String, HashSet<String>>>,
}

impl<B> InvalidationCache<B>
where
    B: CacheBackend,
{
    /// Create a new invalidation cache wrapper around a backend
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            tag_to_keys: std::sync::Mutex::new(std::collections::HashMap::new()),
            prefixes: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Set a value in the cache with associated tags
    pub async fn set_with_tags<T>(
        &self,
        key: String,
        value: T,
        ttl: Option<std::time::Duration>,
        tags: impl IntoIterator<Item = Tag>,
    ) -> Result<()>
    where
        T: serde::Serialize,
    {
        let serializer = crate::serialization::BincodeSerializer::new();
        let serialized = serializer.serialize(&value)?;

        self.backend.set(key.clone(), serialized, ttl).await?;
        self.register_key_with_tags(&key, tags);

        Ok(())
    }

    /// Get a value from the cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let key_string = key.to_string();
        match self.backend.get(&key_string).await? {
            Some(bytes) => {
                let serializer = crate::serialization::BincodeSerializer::new();
                let value = serializer.deserialize(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &str) -> Result<()> {
        let key_string = key.to_string();
        let result = self.backend.remove(&key_string).await;

        if result.is_ok() {
            self.unregister_key(key);
        }

        result
    }

    /// Register a key with tags for invalidation
    pub fn register_key_with_tags(&self, key: &str, tags: impl IntoIterator<Item = Tag>) {
        let mut tag_map = self.tag_to_keys.lock().unwrap();

        for tag in tags {
            tag_map
                .entry(tag)
                .or_insert_with(HashSet::new)
                .insert(key.to_string());
        }

        self.register_key_with_prefixes(key);
    }

    /// Register a key with its prefixes for prefix invalidation
    fn register_key_with_prefixes(&self, key: &str) {
        let mut prefix_map = self.prefixes.lock().unwrap();

        // Register the key with all its possible prefixes
        // For example, for key "users:123:profile", we'd register:
        // - "users:"
        // - "users:123:"
        let parts: Vec<&str> = key.split(':').collect();
        let mut current_prefix = String::new();

        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                current_prefix.push(':');
            }
            current_prefix.push_str(part);

            if i < parts.len() - 1 {
                prefix_map
                    .entry(current_prefix.clone())
                    .or_insert_with(HashSet::new)
                    .insert(key.to_string());
            }
        }
    }

    /// Remove a key from the tag and prefix mappings
    pub fn unregister_key(&self, key: &str) {
        let mut tag_map = self.tag_to_keys.lock().unwrap();

        let mut empty_tags = Vec::new();

        for (tag, keys) in tag_map.iter_mut() {
            keys.remove(key);

            if keys.is_empty() {
                empty_tags.push(tag.clone());
            }
        }

        for tag in empty_tags {
            tag_map.remove(&tag);
        }

        let mut prefix_map = self.prefixes.lock().unwrap();
        let mut empty_prefixes = Vec::new();

        for (prefix, keys) in prefix_map.iter_mut() {
            keys.remove(key);

            if keys.is_empty() {
                empty_prefixes.push(prefix.clone());
            }
        }

        for prefix in empty_prefixes {
            prefix_map.remove(&prefix);
        }
    }

    fn get_keys_by_tag(&self, tag: &Tag) -> HashSet<String> {
        let tag_map = self.tag_to_keys.lock().unwrap();
        tag_map.get(tag).cloned().unwrap_or_default()
    }

    fn get_keys_by_prefix(&self, prefix: &str) -> HashSet<String> {
        let prefix_map = self.prefixes.lock().unwrap();
        prefix_map.get(prefix).cloned().unwrap_or_default()
    }

    /// Get tag map for testing
    #[cfg(test)]
    pub fn get_tag_map(
        &self,
    ) -> std::sync::MutexGuard<std::collections::HashMap<Tag, HashSet<String>>> {
        self.tag_to_keys.lock().unwrap()
    }

    /// Get prefix map for testing
    #[cfg(test)]
    pub fn get_prefix_map(
        &self,
    ) -> std::sync::MutexGuard<std::collections::HashMap<String, HashSet<String>>> {
        self.prefixes.lock().unwrap()
    }
}

#[async_trait]
impl<B> crate::backends::CacheBackend for InvalidationCache<B>
where
    B: crate::backends::CacheBackend,
{
    async fn get(
        &self,
        key: &crate::backends::Key,
    ) -> crate::Result<Option<crate::backends::Value>> {
        self.backend.get(key).await
    }

    async fn set(
        &self,
        key: crate::backends::Key,
        value: crate::backends::Value,
        ttl: Option<std::time::Duration>,
    ) -> crate::Result<()> {
        self.backend.set(key, value, ttl).await
    }

    async fn remove(&self, key: &crate::backends::Key) -> crate::Result<()> {
        self.backend.remove(key).await
    }

    async fn contains_key(&self, key: &crate::backends::Key) -> crate::Result<bool> {
        self.backend.contains_key(key).await
    }

    async fn clear(&self) -> crate::Result<()> {
        self.backend.clear().await
    }
}

impl<B> CacheInvalidation for InvalidationCache<B>
where
    B: CacheBackend + 'static,
{
    fn invalidate_tag(&self, tag: &Tag) -> Result<()> {
        let keys = self.get_keys_by_tag(tag);

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Other(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            for key in keys {
                self.backend.remove(&key).await?;
                self.unregister_key(&key);
            }
            Ok::<_, Error>(())
        })
    }

    fn invalidate_prefix(&self, prefix: &str) -> Result<()> {
        let keys = self.get_keys_by_prefix(prefix);

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Other(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            for key in keys {
                self.backend.remove(&key).await?;
                self.unregister_key(&key);
            }
            Ok::<_, Error>(())
        })
    }

    fn invalidate_tags<I>(&self, tags: I) -> Result<()>
    where
        I: IntoIterator<Item = Tag>,
    {
        for tag in tags {
            CacheInvalidation::invalidate_tag(self, &tag)?;
        }
        Ok(())
    }

    fn invalidate_prefixes<I>(&self, prefixes: I) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        for prefix in prefixes {
            CacheInvalidation::invalidate_prefix(self, &prefix)?;
        }
        Ok(())
    }
}

#[async_trait]
impl<B> AsyncCacheInvalidation for InvalidationCache<B>
where
    B: CacheBackend + 'static,
{
    fn get_keys_by_tag(&self, tag: &Tag) -> HashSet<String> {
        let tag_map = self.tag_to_keys.lock().unwrap();
        tag_map.get(tag).cloned().unwrap_or_default()
    }

    fn get_keys_by_prefix(&self, prefix: &str) -> HashSet<String> {
        let prefix_map = self.prefixes.lock().unwrap();
        prefix_map.get(prefix).cloned().unwrap_or_default()
    }
    async fn invalidate_tag(&self, tag: &Tag) -> Result<()> {
        let keys = {
            let tag_map = self.tag_to_keys.lock().unwrap();
            match tag_map.get(tag) {
                Some(keys) => keys.clone(),
                None => return Ok(()),
            }
        };

        for key in keys {
            self.backend.remove(&key).await?;

            self.unregister_key(&key);
        }

        Ok(())
    }

    async fn invalidate_prefix(&self, prefix: &str) -> Result<()> {
        let keys = {
            let prefix_map = self.prefixes.lock().unwrap();
            match prefix_map.get(prefix) {
                Some(keys) => keys.clone(),
                None => return Ok(()),
            }
        };

        for key in keys {
            self.backend.remove(&key).await?;

            self.unregister_key(&key);
        }

        Ok(())
    }

    async fn invalidate_tags<I>(&self, tags: I) -> Result<()>
    where
        I: IntoIterator<Item = Tag> + Send,
        I::IntoIter: Send,
    {
        let tags_vec: Vec<Tag> = tags.into_iter().collect();
        for tag in tags_vec {
            AsyncCacheInvalidation::invalidate_tag(self, &tag).await?;
        }
        Ok(())
    }

    async fn invalidate_prefixes<I>(&self, prefixes: I) -> Result<()>
    where
        I: IntoIterator<Item = String> + Send,
        I::IntoIter: Send,
    {
        let prefixes_vec: Vec<String> = prefixes.into_iter().collect();
        for prefix in prefixes_vec {
            AsyncCacheInvalidation::invalidate_prefix(self, &prefix).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tag_creation() {
        let tag1 = Tag::new("user:123");
        let tag2: Tag = "user:123".into();
        let tag3 = Tag::from(String::from("user:123"));

        assert_eq!(tag1, tag2);
        assert_eq!(tag2, tag3);
        assert_eq!(tag1.as_str(), "user:123");
    }
}
