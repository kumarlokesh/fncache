//! Eviction policy implementations.
//!
//! This module provides various cache eviction policies that determine which items
//! to remove when the cache reaches capacity.

use std::time::Instant;

use std::hash::Hash;
use std::sync::Arc;

/// Result of an eviction policy decision.
pub struct EvictionResult<K> {
    /// The keys that should be evicted from the cache.
    pub keys_to_evict: Vec<K>,
}

/// A trait for cache eviction policies.
///
/// Implementations of this trait decide which items to remove from the cache
/// when it reaches its capacity limit.
pub trait EvictionPolicy<K, V>: Send + Sync + std::fmt::Debug {
    /// Called when an item is inserted into the cache.
    fn on_insert(&self, key: &K, value: &V);

    /// Called when an item is accessed from the cache.
    fn on_access(&self, key: &K);

    /// Called when an item is removed from the cache.
    fn on_remove(&self, key: &K);

    /// Called when the cache needs to evict items to make space.
    ///
    /// Returns a list of keys that should be evicted from the cache.
    fn evict(&self, count: usize) -> EvictionResult<K>;

    /// Reset the policy's internal state.
    fn reset(&self);
}

/// LRU (Least Recently Used) eviction policy.
///
/// Discards the least recently used items first.
#[derive(Debug)]
pub struct LruPolicy<K: std::hash::Hash + std::cmp::Eq + Clone + Send + Sync + std::fmt::Debug> {
    access_order: dashmap::DashMap<K, Instant>,
}

impl<K> LruPolicy<K>
where
    K: Eq + Hash + Clone + Send + Sync + std::fmt::Debug,
{
    /// Creates a new LRU eviction policy.
    pub fn new() -> Self {
        Self {
            access_order: dashmap::DashMap::new(),
        }
    }
}

impl<K, V> EvictionPolicy<K, V> for LruPolicy<K>
where
    K: Eq + Hash + Clone + Send + Sync + std::fmt::Debug,
{
    fn on_insert(&self, key: &K, _value: &V) {
        self.access_order.insert(key.clone(), Instant::now());
    }

    fn on_access(&self, key: &K) {
        if self.access_order.contains_key(key) {
            self.access_order.insert(key.clone(), Instant::now());
        }
    }

    fn on_remove(&self, key: &K) {
        self.access_order.remove(key);
    }

    fn evict(&self, count: usize) -> EvictionResult<K> {
        let mut entries: Vec<(K, Instant)> = self
            .access_order
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict = entries
            .into_iter()
            .take(count)
            .map(|(key, _)| {
                self.access_order.remove(&key);
                key
            })
            .collect();

        EvictionResult { keys_to_evict }
    }

    fn reset(&self) {
        self.access_order.clear();
    }
}

/// LFU (Least Frequently Used) eviction policy.
///
/// Discards the least frequently used items first.
#[derive(Debug)]
pub struct LfuPolicy<K: std::hash::Hash + std::cmp::Eq + Clone + Send + Sync + std::fmt::Debug> {
    access_count: dashmap::DashMap<K, usize>,
}

impl<K> LfuPolicy<K>
where
    K: Eq + Hash + Clone + Send + Sync + std::fmt::Debug,
{
    /// Creates a new LFU eviction policy.
    pub fn new() -> Self {
        Self {
            access_count: dashmap::DashMap::new(),
        }
    }
}

impl<K, V> EvictionPolicy<K, V> for LfuPolicy<K>
where
    K: Eq + Hash + Clone + Send + Sync + std::fmt::Debug,
{
    fn on_insert(&self, key: &K, _value: &V) {
        self.access_count.insert(key.clone(), 1);
    }

    fn on_access(&self, key: &K) {
        self.access_count
            .entry(key.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    fn on_remove(&self, key: &K) {
        self.access_count.remove(key);
    }

    fn evict(&self, count: usize) -> EvictionResult<K> {
        if count == 0 {
            return EvictionResult {
                keys_to_evict: Vec::new(),
            };
        }

        let mut entries: Vec<(K, usize)> = self
            .access_count
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();

        if entries.is_empty() {
            return EvictionResult {
                keys_to_evict: Vec::new(),
            };
        }

        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let to_take = std::cmp::min(count, entries.len());
        let keys_to_evict = entries
            .into_iter()
            .take(to_take)
            .map(|(key, _count)| {
                self.access_count.remove(&key);
                key
            })
            .collect();

        EvictionResult { keys_to_evict }
    }

    fn reset(&self) {
        self.access_count.clear();
    }
}

/// Factory for creating eviction policies.
pub fn create_policy<K, V>(policy_type: &str) -> Arc<dyn EvictionPolicy<K, V>>
where
    K: Eq + Hash + Clone + Send + Sync + std::fmt::Debug + 'static,
    V: Send + Sync + 'static,
{
    match policy_type.to_lowercase().as_str() {
        "lru" => Arc::new(LruPolicy::new()),
        "lfu" => Arc::new(LfuPolicy::new()),
        _ => Arc::new(LruPolicy::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_eviction() {
        let policy: LruPolicy<String> = LruPolicy::new();

        policy.on_insert(&"key1".to_string(), &42);
        policy.on_insert(&"key2".to_string(), &43);
        policy.on_insert(&"key3".to_string(), &44);

        <LruPolicy<String> as EvictionPolicy<String, i32>>::on_access(&policy, &"key1".to_string());

        let result: EvictionResult<String> =
            <LruPolicy<String> as EvictionPolicy<String, i32>>::evict(&policy, 1);
        assert_eq!(result.keys_to_evict.len(), 1);
        assert!(
            result.keys_to_evict.contains(&"key2".to_string())
                || result.keys_to_evict.contains(&"key3".to_string())
        );
    }

    #[test]
    fn test_lfu_eviction() {
        let policy: LfuPolicy<String> = LfuPolicy::new();

        policy.on_insert(&"key1".to_string(), &42);
        policy.on_insert(&"key2".to_string(), &43);
        policy.on_insert(&"key3".to_string(), &44);

        <LfuPolicy<String> as EvictionPolicy<String, i32>>::on_access(&policy, &"key1".to_string());
        <LfuPolicy<String> as EvictionPolicy<String, i32>>::on_access(&policy, &"key3".to_string());

        let result: EvictionResult<String> =
            <LfuPolicy<String> as EvictionPolicy<String, i32>>::evict(&policy, 1);
        assert_eq!(result.keys_to_evict.len(), 1);
        assert_eq!(result.keys_to_evict[0], "key2".to_string());
    }
}
