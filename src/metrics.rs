//! Metrics collection for cache operations.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Represents a latency measurement for a cache operation.
#[derive(Debug, Clone, Copy)]
pub struct LatencyMetric {
    /// Total time spent on operations (in nanoseconds)
    pub total_ns: u64,
    /// Number of operations measured
    pub count: u64,
    /// Maximum observed latency (in nanoseconds)
    pub max_ns: u64,
}

impl LatencyMetric {
    /// Creates a new empty latency metric.
    pub fn new() -> Self {
        Self {
            total_ns: 0,
            count: 0,
            max_ns: 0,
        }
    }

    /// Returns the average latency in nanoseconds.
    pub fn average_ns(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_ns as f64 / self.count as f64
        }
    }

    /// Returns the average latency as a Duration.
    pub fn average_duration(&self) -> Duration {
        if self.count == 0 {
            Duration::from_nanos(0)
        } else {
            Duration::from_nanos((self.total_ns / self.count) as u64)
        }
    }
}

impl Default for LatencyMetric {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks cache metrics like hits, misses, evictions, latency and size.
#[derive(Debug, Default)]
pub struct Metrics {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    insertions: AtomicU64,

    // Size metrics
    total_bytes: AtomicUsize,
    entry_count: AtomicUsize,

    // Latency metrics are stored in a thread-local to avoid contention
    // They are combined when requested
    get_latency: std::sync::Mutex<LatencyMetric>,
    set_latency: std::sync::Mutex<LatencyMetric>,
}

impl Metrics {
    /// Creates a new `Metrics` instance with all counters set to zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a cache hit.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache eviction.
    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache insertion.
    pub fn record_insertion(&self) {
        self.insertions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records the size of a cache entry.
    pub fn record_entry_size(&self, old_size: usize, new_size: usize) {
        if old_size > 0 {
            let _ = self.total_bytes.fetch_sub(old_size, Ordering::Relaxed);
        } else {
            let _ = self.entry_count.fetch_add(1, Ordering::Relaxed);
        }

        if new_size > 0 {
            let _ = self.total_bytes.fetch_add(new_size, Ordering::Relaxed);
        }
    }

    /// Records removal of a cache entry and its size.
    pub fn record_entry_removal(&self, size: usize) {
        if size > 0 {
            let _ = self.total_bytes.fetch_sub(size, Ordering::Relaxed);
        }
        let _ = self.entry_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Begins timing a get operation.
    pub fn begin_get_timing(&self) -> Instant {
        Instant::now()
    }

    /// Records the latency of a get operation.
    pub fn record_get_latency(&self, start: Instant) {
        let duration = start.elapsed();
        let nanos = duration.as_nanos() as u64;

        let mut get_latency = self.get_latency.lock().unwrap();
        get_latency.total_ns += nanos;
        get_latency.count += 1;
        if nanos > get_latency.max_ns {
            get_latency.max_ns = nanos;
        }
    }

    /// Begins timing a set operation.
    pub fn begin_set_timing(&self) -> Instant {
        Instant::now()
    }

    /// Records the latency of a set operation.
    pub fn record_set_latency(&self, start: Instant) {
        let duration = start.elapsed();
        let nanos = duration.as_nanos() as u64;

        let mut set_latency = self.set_latency.lock().unwrap();
        set_latency.total_ns += nanos;
        set_latency.count += 1;
        if nanos > set_latency.max_ns {
            set_latency.max_ns = nanos;
        }
    }

    /// Returns the current hit count.
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Returns the current miss count.
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Returns the current eviction count.
    pub fn evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Returns the current insertion count.
    pub fn insertions(&self) -> u64 {
        self.insertions.load(Ordering::Relaxed)
    }

    /// Returns the total cache size in bytes.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes.load(Ordering::Relaxed)
    }

    /// Returns the total number of entries in the cache.
    pub fn entry_count(&self) -> usize {
        self.entry_count.load(Ordering::Relaxed)
    }

    /// Returns the average entry size in bytes.
    pub fn average_entry_size(&self) -> usize {
        let count = self.entry_count();
        let bytes = self.total_bytes();

        if count == 0 {
            0
        } else {
            bytes / count
        }
    }

    /// Returns latency metrics for get operations.
    pub fn get_latency(&self) -> LatencyMetric {
        self.get_latency.lock().unwrap().clone()
    }

    /// Returns latency metrics for set operations.
    pub fn set_latency(&self) -> LatencyMetric {
        self.set_latency.lock().unwrap().clone()
    }

    /// Returns the average latency for get operations in nanoseconds.
    pub fn average_get_latency_ns(&self) -> f64 {
        self.get_latency.lock().unwrap().average_ns()
    }

    /// Returns the average latency for set operations in nanoseconds.
    pub fn average_set_latency_ns(&self) -> f64 {
        self.set_latency.lock().unwrap().average_ns()
    }

    /// Returns the hit rate as a float between 0.0 and 1.0.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits();
        let misses = self.misses();

        if hits == 0 && misses == 0 {
            0.0
        } else {
            hits as f64 / (hits + misses) as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counting() {
        let metrics = Metrics::new();

        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 0);
        assert_eq!(metrics.evictions(), 0);
        assert_eq!(metrics.insertions(), 0);
        assert_eq!(metrics.hit_rate(), 0.0);

        metrics.record_hit();
        metrics.record_miss();
        metrics.record_eviction();
        metrics.record_insertion();

        assert_eq!(metrics.hits(), 1);
        assert_eq!(metrics.misses(), 1);
        assert_eq!(metrics.evictions(), 1);
        assert_eq!(metrics.insertions(), 1);
        assert_eq!(metrics.hit_rate(), 0.5);
    }

    #[test]
    fn test_hit_rate_edge_cases() {
        let metrics = Metrics::new();

        assert_eq!(metrics.hit_rate(), 0.0);

        metrics.record_miss();
        assert_eq!(metrics.hit_rate(), 0.0);

        metrics.record_hit();
        let metrics = Metrics::new();
        metrics.record_hit();
        assert_eq!(metrics.hit_rate(), 1.0);
    }
}
