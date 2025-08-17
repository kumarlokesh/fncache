//! # FnCache Benchmarks
//!
//! This benchmark suite measures the performance of various cache operations
//! across different backends and scenarios. The benchmarks are designed to help
//! evaluate:
//!
//! * Relative performance of different backends (memory, file, Redis, RocksDB)
//! * Impact of data size on cache operations (small, medium, large)
//! * Eviction policy performance characteristics (LRU, LFU)
//! * Key serialization overhead
//! * TTL operations performance
//!
//! ## Interpreting Results
//!
//! * **Backend comparison**: Memory backend should be fastest, followed by
//!   RocksDB, file backend, and Redis (network-dependent).
//! * **Data size impact**: Larger data sizes will significantly impact
//!   serialization, deserialization, and network transfer times.
//! * **Operation cost**: `set` operations typically cost more than `get` operations
//!   due to serialization overhead; `get_miss` is usually faster than `get_hit`
//!   as no deserialization is needed.
//! * **Eviction policies**: LRU typically has better throughput than LFU but may
//!   have worse cache hit rates for certain access patterns.
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench --features "file-backend"
//!
//! # Run only memory backend benchmarks
//! cargo bench --features "file-backend" -- memory_backend
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use futures::executor::block_on;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use fncache::backends::memory::{MemoryBackend, MemoryBackendConfig};
use fncache::backends::CacheBackend;

#[cfg(feature = "file-backend")]
use fncache::backends::file::FileBackend;
#[cfg(feature = "file-backend")]
use tempfile::TempDir;

#[cfg(feature = "redis-backend")]
use fncache::backends::redis::RedisBackend;

#[cfg(feature = "rocksdb-backend")]
use fncache::backends::rocksdb::RocksDBBackend;

const SMALL_DATA_SIZE: usize = 100;
const MEDIUM_DATA_SIZE: usize = 1000;
const LARGE_DATA_SIZE: usize = 10000;
const EVICTION_CACHE_CAPACITY: usize = 1000;
const EVICTION_ITEMS_TO_INSERT: usize = 1100;

const MEASUREMENT_TIME_MS: u64 = 2000;
const WARMUP_TIME_MS: u64 = 500;
const MIN_SAMPLE_SIZE: usize = 10;

#[cfg(feature = "redis-backend")]
const REDIS_URL: &str = "redis://127.0.0.1:6379";

const DEFAULT_TTL_SECONDS: u64 = 60;

const RNG_SEED_SET: u64 = 42;
const RNG_SEED_GET_MISS: u64 = 43;
const RNG_SEED_REMOVE: u64 = 44;
const RNG_SEED_TTL: u64 = 45;

/// Generate test data of specified size
fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Configure a benchmark group with standard measurement parameters
fn configure_benchmark_group(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    large_dataset: bool,
) {
    let time_multiplier = if large_dataset { 2 } else { 1 };
    group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS * time_multiplier));
    group.warm_up_time(Duration::from_millis(WARMUP_TIME_MS));
    group.sample_size(MIN_SAMPLE_SIZE);
}

/// Helper function to run a cache set benchmark
fn bench_cache_set<B: CacheBackend + 'static>(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    backend: Arc<B>,
    data: Vec<u8>,
) {
    group.bench_function("set", |b| {
        let backend = backend.clone();
        let mut rng = StdRng::seed_from_u64(RNG_SEED_SET);
        b.iter(|| {
            let key = format!("bench_key_{}", rng.gen::<u64>());
            block_on(backend.set(key, data.clone(), None))
        });
    });
}

/// Helper function to run a cache get hit benchmark
fn bench_cache_get_hit<B: CacheBackend + 'static>(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    backend: Arc<B>,
    get_key: String,
) {
    group.bench_function("get_hit", |b| {
        let backend = backend.clone();
        let key = get_key.clone();
        b.iter(|| block_on(backend.get(&key)));
    });
}

/// Helper function to run a cache get miss benchmark
fn bench_cache_get_miss<B: CacheBackend + 'static>(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    backend: Arc<B>,
) {
    group.bench_function("get_miss", |b| {
        let backend = backend.clone();
        let mut rng = StdRng::seed_from_u64(RNG_SEED_GET_MISS);
        b.iter(|| block_on(backend.get(&format!("nonexistent_key_{}", rng.gen::<u64>()))));
    });
}

/// Helper function to run a cache remove benchmark
fn bench_cache_remove<B: CacheBackend + 'static>(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    backend: Arc<B>,
    data: Vec<u8>,
) {
    group.bench_function("remove", |b| {
        let backend = backend.clone();
        let mut rng = StdRng::seed_from_u64(RNG_SEED_REMOVE);
        b.iter(|| {
            let key = format!("bench_remove_key_{}", rng.gen::<u64>());
            block_on(async {
                backend.set(key.clone(), data.clone(), None).await?;
                backend.remove(&key).await
            })
        });
    });
}

/// Helper function to run a cache set with TTL benchmark
fn bench_cache_set_ttl<B: CacheBackend + 'static>(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    backend: Arc<B>,
    data: Vec<u8>,
) {
    group.bench_function("set_with_ttl", |b| {
        let backend = backend.clone();
        let mut rng = StdRng::seed_from_u64(RNG_SEED_TTL);
        b.iter(|| {
            let key = format!("bench_ttl_key_{}", rng.gen::<u64>());
            block_on(backend.set(
                key,
                data.clone(),
                Some(Duration::from_secs(DEFAULT_TTL_SECONDS)),
            ))
        });
    });
}

/// Test data structure for benchmarks
#[derive(Serialize, Deserialize, Clone)]
struct TestData {
    id: u64,
    name: String,
    values: Vec<u32>,
}

/// Generate test data structure of specified complexity
fn generate_test_data(complexity: usize) -> TestData {
    TestData {
        id: 12345,
        name: "test_data".to_string(),
        values: (0..complexity).map(|i| i as u32).collect(),
    }
}

/// Benchmark basic cache operations (get/set/remove)
fn bench_basic_operations<B: fncache::backends::CacheBackend + 'static>(
    c: &mut Criterion,
    backend: B,
    backend_name: &str,
) {
    let backend = Arc::new(backend);

    // Small data benchmark
    {
        let mut group = c.benchmark_group(format!("{}_small_data", backend_name));
        configure_benchmark_group(&mut group, false);

        let get_key = "memory_get_key".to_string();
        let data = generate_data(SMALL_DATA_SIZE);

        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        bench_cache_set(&mut group, backend.clone(), data.clone());
        bench_cache_get_hit(&mut group, backend.clone(), get_key.clone());
        bench_cache_get_miss(&mut group, backend.clone());
        bench_cache_remove(&mut group, backend.clone(), data.clone());

        group.finish();
    }

    // Medium data benchmark
    {
        let mut group = c.benchmark_group(format!("{}_medium_data", backend_name));
        configure_benchmark_group(&mut group, false);
        let data = generate_data(MEDIUM_DATA_SIZE);

        let get_key = "bench_get_key_medium".to_string();
        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        bench_cache_set(&mut group, backend.clone(), data.clone());
        bench_cache_get_hit(&mut group, backend.clone(), get_key.clone());

        group.finish();
    }

    // Large data benchmark
    {
        let mut group = c.benchmark_group(format!("{}_large_data", backend_name));
        configure_benchmark_group(&mut group, true);

        let get_key = "memory_large_key".to_string();
        let data = generate_data(LARGE_DATA_SIZE);

        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        bench_cache_set(&mut group, backend.clone(), data.clone());
        bench_cache_get_hit(&mut group, backend.clone(), get_key.clone());

        group.finish();
    }
}

/// Benchmark TTL operations
///
/// This benchmark measures the performance of cache operations with TTL (time-to-live)
/// settings, which are important for caches that need automatic expiration.
///
/// TTL operations are typically slightly more expensive than regular operations
/// because they require additional tracking of expiration times.
fn bench_ttl_operations<B: fncache::backends::CacheBackend + 'static>(
    c: &mut Criterion,
    backend: B,
    backend_name: &str,
) {
    let backend = Arc::new(backend);
    let mut group = c.benchmark_group(format!("{}_ttl", backend_name));
    configure_benchmark_group(&mut group, false);
    let data = generate_data(SMALL_DATA_SIZE);

    bench_cache_set_ttl(&mut group, backend, data);

    group.finish();
}

/// Benchmark key serialization performance
///
/// This benchmark measures the overhead of serializing different types of cache keys.
/// Efficient key serialization is important for cache performance, especially
/// for complex data structures used as keys.
///
/// Results can help decide between simple string keys vs complex object keys,
/// and evaluate the cost of serialization for different data types.
fn bench_key_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_serialization");
    configure_benchmark_group(&mut group, false);

    group.bench_function("simple_key", |b| {
        b.iter(|| {
            let key = format!("simple_key_{}", black_box(12345));
            black_box(key)
        });
    });

    group.bench_function("complex_key_bincode", |b| {
        b.iter(|| {
            let data = generate_test_data(50);
            black_box(bincode::serialize(&data).unwrap())
        });
    });

    group.finish();
}

/// Benchmark eviction policies
///
/// This benchmark measures the performance characteristics of different cache eviction
/// policies (LRU and LFU). It intentionally inserts more items than the cache capacity
/// to trigger evictions.
///
/// Key insights from these benchmarks:
/// - LRU (Least Recently Used) is typically faster but may have worse hit rates for some workloads
/// - LFU (Least Frequently Used) may have better hit rates for frequency-based access patterns
///   but has more overhead for tracking access counts
///
/// These benchmarks help identify the throughput cost of different eviction strategies
/// when the cache is under pressure (at or above capacity).
fn bench_eviction_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("eviction_policies");
    group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS * 2));
    group.warm_up_time(Duration::from_millis(WARMUP_TIME_MS));
    group.sample_size(MIN_SAMPLE_SIZE);
    group.sampling_mode(SamplingMode::Flat);

    let data = generate_data(SMALL_DATA_SIZE);
    let mut keys = Vec::with_capacity(EVICTION_ITEMS_TO_INSERT);
    for i in 0..EVICTION_ITEMS_TO_INSERT {
        keys.push(format!("cache_key_{}", i));
    }

    group.bench_function("lru_eviction", |b| {
        let mut backend = MemoryBackend::new();
        backend = backend.with_capacity(EVICTION_CACHE_CAPACITY);
        backend = backend.with_eviction_policy("lru");

        b.iter(|| {
            for key in &keys {
                block_on(backend.set(key.clone(), data.clone(), None)).unwrap();
            }
        });
    });

    group.bench_function("lfu_eviction", |b| {
        let mut backend = MemoryBackend::new();
        backend = backend.with_capacity(EVICTION_CACHE_CAPACITY);
        backend = backend.with_eviction_policy("lfu");

        b.iter(|| {
            for key in &keys {
                block_on(backend.set(key.clone(), data.clone(), None)).unwrap();
            }
        });
    });

    group.finish();
}

fn memory_backend_benchmarks(c: &mut Criterion) {
    let backend = MemoryBackend::new();
    bench_basic_operations(c, backend, "memory_backend");

    let mut config = MemoryBackendConfig::default();
    config.max_capacity = EVICTION_CACHE_CAPACITY;
    config.eviction_policy = "lru".to_string();
    let backend = MemoryBackend::with_config(config);
    bench_ttl_operations(c, backend, "memory_lru");

    let mut config = MemoryBackendConfig::default();
    config.max_capacity = EVICTION_CACHE_CAPACITY;
    config.eviction_policy = "lfu".to_string();
    let backend = MemoryBackend::with_config(config);
    bench_ttl_operations(c, backend, "memory_lfu");
}

#[cfg(feature = "file-backend")]
fn file_backend_benchmarks(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let backend = FileBackend::new(temp_dir.path().to_str().unwrap()).unwrap();

    bench_basic_operations(c, backend, "file_backend");

    let backend = FileBackend::new(temp_dir.path().to_str().unwrap()).unwrap();
    bench_ttl_operations(c, backend, "file_backend");
}

#[cfg(feature = "redis-backend")]
fn redis_backend_benchmarks(c: &mut Criterion) {
    match RedisBackend::new(REDIS_URL) {
        Ok(backend) => match block_on(backend.clear()) {
            Ok(_) => {
                bench_basic_operations(c, backend, "redis_backend");

                match RedisBackend::new(REDIS_URL) {
                    Ok(ttl_backend) => {
                        bench_ttl_operations(c, ttl_backend, "redis_backend");
                    }
                    Err(e) => {
                        eprintln!("Error creating Redis backend for TTL benchmarks: {:?}", e);
                        eprintln!("Make sure Redis server is running at {}", REDIS_URL);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error clearing Redis database: {:?}", e);
                eprintln!("Skipping Redis benchmarks due to database clear failure");
            }
        },
        Err(e) => {
            eprintln!(
                "Skipping Redis benchmarks: server not available at {}",
                REDIS_URL
            );
            eprintln!("Error details: {:?}", e);
        }
    }
}

#[cfg(feature = "rocksdb-backend")]
fn rocksdb_backend_benchmarks(c: &mut Criterion) {
    match TempDir::new() {
        Ok(temp_dir) => {
            let db_path = temp_dir.path().to_str().unwrap_or_else(|| {
                eprintln!("Error: Unable to convert temp directory path to string");
                "./rocksdb_temp"
            });

            match RocksDBBackend::new(db_path) {
                Ok(backend) => {
                    bench_basic_operations(c, backend, "rocksdb_backend");

                    let ttl_path = format!("{}_ttl", db_path);
                    match RocksDBBackend::new(&ttl_path) {
                        Ok(ttl_backend) => {
                            bench_ttl_operations(c, ttl_backend, "rocksdb_backend");
                        }
                        Err(e) => {
                            eprintln!("Error creating RocksDB backend for TTL benchmarks: {:?}", e);
                            eprintln!("Path attempted: {}", ttl_path);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error creating RocksDB backend: {:?}", e);
                    eprintln!("Make sure RocksDB dependencies are properly installed");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to create temporary directory for RocksDB: {:?}", e);
            eprintln!("Skipping RocksDB benchmarks due to filesystem error");
        }
    }
}

criterion_group!(
    benches,
    memory_backend_benchmarks,
    bench_key_serialization,
    bench_eviction_policies
);

#[cfg(feature = "file-backend")]
criterion_group!(file_benches, file_backend_benchmarks);

#[cfg(feature = "redis-backend")]
criterion_group!(redis_benches, redis_backend_benchmarks);

#[cfg(feature = "rocksdb-backend")]
criterion_group!(rocksdb_benches, rocksdb_backend_benchmarks);

criterion_main!(benches);
