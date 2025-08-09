//! Metrics collection for cache operations.

use std::sync::atomic::{AtomicU64, Ordering};

/// Tracks cache metrics like hits, misses, and evictions.
#[derive(Debug, Default)]
pub struct Metrics {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    insertions: AtomicU64,
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

    /// Returns the hit rate as a float between 0.0 and 1.0.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits();
        let misses = self.misses();

        if hits + misses == 0 {
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
