//! Performance optimization module for fncache
//!
//! This module provides utilities and strategies for optimizing cache performance.
//! It includes adaptive TTL, preloading, and intelligent prefetching strategies.

use std::time::Duration;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::backends::CacheBackend;
use crate::{Error, Result};

/// Performance statistics tracking for a cached function
#[derive(Debug)]
pub struct CacheStats {
    /// Number of cache hits
    hits: AtomicU64,
    /// Number of cache misses
    misses: AtomicU64,
    /// Total time saved by cache hits (nanoseconds)
    time_saved_ns: AtomicU64,
    /// Total execution time for cache misses (nanoseconds)
    execution_time_ns: AtomicU64,
}

impl CacheStats {
    /// Create a new CacheStats instance
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            time_saved_ns: AtomicU64::new(0),
            execution_time_ns: AtomicU64::new(0),
        }
    }

    /// Record a cache hit that saved the specified execution time
    pub fn record_hit(&self, saved_time_ns: u64) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.time_saved_ns.fetch_add(saved_time_ns, Ordering::Relaxed);
    }

    /// Record a cache miss with the specified execution time
    pub fn record_miss(&self, execution_time_ns: u64) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.execution_time_ns.fetch_add(execution_time_ns, Ordering::Relaxed);
    }

    /// Get the hit count
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get the miss count
    pub fn miss_count(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Get the hit ratio (hits / total)
    pub fn hit_ratio(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        
        if hits + misses == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }

    /// Get the total time saved by cache hits
    pub fn time_saved(&self) -> Duration {
        Duration::from_nanos(self.time_saved_ns.load(Ordering::Relaxed))
    }

    /// Get the average execution time for cache misses
    pub fn average_execution_time(&self) -> Duration {
        let total_time = self.execution_time_ns.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        
        if misses == 0 {
            Duration::from_nanos(0)
        } else {
            Duration::from_nanos(total_time / misses)
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.time_saved_ns.store(0, Ordering::Relaxed);
        self.execution_time_ns.store(0, Ordering::Relaxed);
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive TTL strategy that adjusts TTL based on access patterns
pub struct AdaptiveTtl {
    /// Base TTL value in seconds
    base_ttl: u64,
    /// Minimum TTL value in seconds
    min_ttl: u64,
    /// Maximum TTL value in seconds
    max_ttl: u64,
    /// Access count threshold for TTL adjustment
    access_threshold: u64,
    /// Multiplier for TTL adjustment (>1.0 increases TTL, <1.0 decreases)
    multiplier: f64,
}

impl AdaptiveTtl {
    /// Create a new AdaptiveTtl instance
    pub fn new(base_ttl: u64, min_ttl: u64, max_ttl: u64) -> Self {
        Self {
            base_ttl,
            min_ttl,
            max_ttl,
            access_threshold: 5,
            multiplier: 1.5,
        }
    }

    /// Set the access threshold for TTL adjustment
    pub fn with_access_threshold(mut self, threshold: u64) -> Self {
        self.access_threshold = threshold;
        self
    }

    /// Set the TTL multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Calculate the adjusted TTL based on access count
    pub fn calculate_ttl(&self, access_count: u64) -> Duration {
        if access_count < self.access_threshold {
            return Duration::from_secs(self.base_ttl);
        }
        
        // Adjust TTL based on access count and multiplier
        let factor = (access_count as f64 / self.access_threshold as f64).min(10.0);
        let adjusted_ttl = (self.base_ttl as f64 * self.multiplier * factor) as u64;
        
        // Clamp to min/max range
        Duration::from_secs(adjusted_ttl.clamp(self.min_ttl, self.max_ttl))
    }
}

/// Prefetching strategy for proactively caching related items
pub struct Prefetcher<B: CacheBackend> {
    /// The cache backend to use for prefetching
    backend: B,
    /// Maximum number of items to prefetch at once
    max_items: usize,
}

impl<B: CacheBackend> Prefetcher<B> {
    /// Create a new Prefetcher instance
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            max_items: 10,
        }
    }

    /// Set the maximum number of items to prefetch
    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = max_items;
        self
    }

    /// Prefetch related items based on a pattern
    pub async fn prefetch<F, T>(&self, pattern_fn: F, ttl: Option<Duration>) -> Result<()>
    where
        F: Fn() -> Vec<(String, Vec<u8>)>,
        T: 'static,
    {
        let items = pattern_fn();

        let items = items.into_iter().take(self.max_items);

        futures::future::try_join_all(
            items.map(|(key, value)| {
                let backend = &self.backend;
                let ttl = ttl.clone();
                
                async move {
                    backend.set(key, value, ttl).await
                }
            })
        ).await?;
        
        Ok(())
    }
}

/// Batch operation helper for efficiently performing multiple cache operations
pub struct BatchOperations<B: CacheBackend> {
    /// The cache backend
    backend: B,
    /// Operations queue
    operations: Vec<BatchOperation>,
}

/// Types of batch operations
enum BatchOperation {
    /// Set a key-value pair
    Set {
        key: String,
        value: Vec<u8>,
        ttl: Option<Duration>,
    },
    /// Remove a key
    Remove(String),
}

impl<B: CacheBackend> BatchOperations<B> {
    /// Create a new BatchOperations instance
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            operations: Vec::new(),
        }
    }

    /// Queue a set operation
    pub fn set(&mut self, key: String, value: Vec<u8>, ttl: Option<Duration>) -> &mut Self {
        self.operations.push(BatchOperation::Set { key, value, ttl });
        self
    }

    /// Queue a remove operation
    pub fn remove(&mut self, key: String) -> &mut Self {
        self.operations.push(BatchOperation::Remove(key));
        self
    }

    /// Execute all queued operations
    pub async fn execute(self) -> Result<()> {
        for op in self.operations {
            match op {
                BatchOperation::Set { key, value, ttl } => {
                    self.backend.set(key, value, ttl).await?;
                },
                BatchOperation::Remove(key) => {
                    self.backend.remove(&key).await?;
                }
            }
        }
        
        Ok(())
    }
}

/// Memory optimization strategy that monitors memory usage and evicts items when needed
#[cfg(feature = "memory")]
pub struct MemoryOptimizer {
    /// Maximum memory usage in bytes
    max_memory: usize,
    /// Current memory usage in bytes
    current_memory: AtomicU64,
}

#[cfg(feature = "memory")]
impl MemoryOptimizer {
    /// Create a new MemoryOptimizer instance
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            max_memory: max_memory_mb * 1024 * 1024, // Convert MB to bytes
            current_memory: AtomicU64::new(0),
        }
    }

    /// Record memory usage for a new cache entry
    pub fn record_allocation(&self, size_bytes: usize) {
        self.current_memory.fetch_add(size_bytes as u64, Ordering::Relaxed);
    }

    /// Record memory freed when an entry is removed
    pub fn record_deallocation(&self, size_bytes: usize) {
        self.current_memory.fetch_sub(size_bytes as u64, Ordering::Relaxed);
    }

    /// Check if memory usage exceeds the maximum
    pub fn should_evict(&self) -> bool {
        self.current_memory.load(Ordering::Relaxed) as usize > self.max_memory
    }

    /// Get the current memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.current_memory.load(Ordering::Relaxed) as usize
    }

    /// Get the memory usage as a percentage of maximum
    pub fn memory_usage_percent(&self) -> f64 {
        let current = self.current_memory.load(Ordering::Relaxed) as f64;
        let max = self.max_memory as f64;
        
        (current / max) * 100.0
    }
}

/// Compression utility for reducing cache entry size
#[cfg(feature = "bincode")]
pub struct Compression {
    /// Compression level (0-9, where 9 is highest compression)
    level: u32,
}

#[cfg(feature = "bincode")]
impl Compression {
    /// Create a new Compression instance
    pub fn new(level: u32) -> Self {
        Self {
            level: level.clamp(0, 9),
        }
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::{write::ZlibEncoder, Compression as FlateCompression};
        use std::io::Write;
        
        let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::new(self.level));
        encoder.write_all(data).map_err(|e| Error::Codec(e.to_string()))?;
        encoder.finish().map_err(|e| Error::Codec(e.to_string()))
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| Error::Codec(e.to_string()))?;
        
        Ok(decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_stats() {
        let stats = CacheStats::new();

        stats.record_hit(1_000_000); // 1ms
        stats.record_hit(2_000_000); // 2ms
        stats.record_miss(5_000_000); // 5ms

        assert_eq!(stats.hit_count(), 2);
        assert_eq!(stats.miss_count(), 1);
        assert_eq!(stats.hit_ratio(), 2.0 / 3.0);
        assert_eq!(stats.time_saved().as_nanos(), 3_000_000);
        assert_eq!(stats.average_execution_time().as_nanos(), 5_000_000);

        stats.reset();
        assert_eq!(stats.hit_count(), 0);
        assert_eq!(stats.miss_count(), 0);
    }
    
    #[test]
    fn test_adaptive_ttl() {
        let adaptive_ttl = AdaptiveTtl::new(60, 10, 3600);

        assert_eq!(adaptive_ttl.calculate_ttl(0).as_secs(), 60);
        assert_eq!(adaptive_ttl.calculate_ttl(4).as_secs(), 60);

        let ttl_6 = adaptive_ttl.calculate_ttl(6).as_secs();
        let ttl_10 = adaptive_ttl.calculate_ttl(10).as_secs();
        
        assert!(ttl_6 > 60);
        assert!(ttl_10 > ttl_6);

        let ttl_1000 = adaptive_ttl.calculate_ttl(1000).as_secs();
        assert_eq!(ttl_1000, 3600);
    }
}
