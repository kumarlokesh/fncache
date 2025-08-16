//! Benchmarks for fncache operations
//!
//! This benchmark suite measures the performance of various cache operations
//! across different backends and scenarios.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fncache::backends::CacheBackend;
use fncache::{self, MemoryBackend};
// Eviction policies are now configured by string names
use fncache::backends::memory::MemoryBackendConfig;

#[cfg(feature = "file-backend")]
use fncache::FileBackend;

#[cfg(feature = "redis-backend")]
use fncache::RedisBackend;

#[cfg(feature = "rocksdb-backend")]
use fncache::RocksDbBackend;

use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "file-backend")]
use tempfile::TempDir;

const SMALL_DATA_SIZE: usize = 100;
const MEDIUM_DATA_SIZE: usize = 1000;
const LARGE_DATA_SIZE: usize = 10000;

/// Generate test data of specified size
fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
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
        let data = generate_data(SMALL_DATA_SIZE);

        group.bench_function("set", |b| {
            let backend = backend.clone();
            b.iter(|| {
                let key = format!("bench_key_{}", rand::random::<u64>());
                block_on(backend.set(key, data.clone(), None))
            });
        });

        let get_key = "bench_get_key_small".to_string();
        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        group.bench_function("get_hit", |b| {
            let backend = backend.clone();
            b.iter(|| block_on(backend.get(&get_key)));
        });

        group.bench_function("get_miss", |b| {
            let backend = backend.clone();
            b.iter(|| block_on(backend.get(&format!("nonexistent_key_{}", rand::random::<u64>()))));
        });

        group.bench_function("remove", |b| {
            let backend = backend.clone();
            b.iter(|| {
                let key = format!("bench_remove_key_{}", rand::random::<u64>());
                block_on(async {
                    backend.set(key.clone(), data.clone(), None).await?;
                    backend.remove(&key).await
                })
            });
        });

        group.finish();
    }

    // Medium data benchmark
    {
        let mut group = c.benchmark_group(format!("{}_medium_data", backend_name));
        let data = generate_data(MEDIUM_DATA_SIZE);

        group.bench_function("set", |b| {
            let backend = backend.clone();
            b.iter(|| {
                let key = format!("bench_key_{}", rand::random::<u64>());
                block_on(backend.set(key, data.clone(), None))
            });
        });

        let get_key = "bench_get_key_medium".to_string();
        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        group.bench_function("get_hit", |b| {
            let backend = backend.clone();
            b.iter(|| block_on(backend.get(&get_key)));
        });

        group.finish();
    }

    // Large data benchmark
    {
        let mut group = c.benchmark_group(format!("{}_large_data", backend_name));
        let data = generate_data(LARGE_DATA_SIZE);

        group.bench_function("set", |b| {
            let backend = backend.clone();
            b.iter(|| {
                let key = format!("bench_key_{}", rand::random::<u64>());
                block_on(backend.set(key, data.clone(), None))
            });
        });

        let get_key = "bench_get_key_large".to_string();
        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        group.bench_function("get_hit", |b| {
            let backend = backend.clone();
            b.iter(|| block_on(backend.get(&get_key)));
        });

        group.finish();
    }
}

/// Benchmark TTL operations
fn bench_ttl_operations<B: fncache::backends::CacheBackend + 'static>(
    c: &mut Criterion,
    backend: B,
    backend_name: &str,
) {
    let backend = Arc::new(backend);
    let mut group = c.benchmark_group(format!("{}_ttl", backend_name));
    let data = generate_data(SMALL_DATA_SIZE);

    group.bench_function("set_with_ttl", |b| {
        let backend = backend.clone();
        b.iter(|| {
            let key = format!("bench_ttl_key_{}", rand::random::<u64>());
            block_on(backend.set(key, data.clone(), Some(Duration::from_secs(60))))
        });
    });

    group.finish();
}

/// Benchmark key serialization performance
fn bench_key_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_serialization");

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
fn bench_eviction_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("eviction_policies");

    group.bench_function("lru_eviction", |b| {
        let mut backend = MemoryBackend::new();
        backend = backend.with_capacity(1000);
        backend = backend.with_eviction_policy("lru");
        let data = generate_data(SMALL_DATA_SIZE);

        b.iter(|| {
            for i in 0..1100 {
                let key = format!("lru_key_{}", i);
                block_on(backend.set(key, data.clone(), None)).unwrap();
            }
        });
    });

    group.bench_function("lfu_eviction", |b| {
        let mut backend = MemoryBackend::new();
        backend = backend.with_capacity(1000);
        backend = backend.with_eviction_policy("lfu");
        let data = generate_data(SMALL_DATA_SIZE);

        b.iter(|| {
            for i in 0..1100 {
                let key = format!("lfu_key_{}", i);
                block_on(backend.set(key, data.clone(), None)).unwrap();
            }
        });
    });

    group.finish();
}

fn memory_backend_benchmarks(c: &mut Criterion) {
    let backend = MemoryBackend::new();
    bench_basic_operations(c, backend, "memory_backend");

    let mut config = MemoryBackendConfig::default();
    config.max_capacity = 1000;
    config.eviction_policy = "lru".to_string();
    let backend = MemoryBackend::with_config(config);
    bench_ttl_operations(c, backend, "memory_lru");

    let mut config = MemoryBackendConfig::default();
    config.max_capacity = 1000;
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
    match RedisBackend::new("redis://127.0.0.1:6379") {
        Ok(backend) => {
            block_on(backend.clear()).unwrap();

            bench_basic_operations(c, backend, "redis_backend");

            let backend = RedisBackend::new("redis://127.0.0.1:6379").unwrap();
            bench_ttl_operations(c, backend, "redis_backend");
        }
        Err(_) => {
            println!("Skipping Redis benchmarks: server not available");
        }
    }
}

#[cfg(feature = "rocksdb-backend")]
fn rocksdb_backend_benchmarks(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    match RocksDbBackend::new(temp_dir.path().to_str().unwrap()) {
        Ok(backend) => {
            bench_basic_operations(c, backend, "rocksdb_backend");

            let backend = RocksDbBackend::new(temp_dir.path().to_str().unwrap()).unwrap();
            bench_ttl_operations(c, backend, "rocksdb_backend");
        }
        Err(e) => {
            println!("Skipping RocksDB benchmarks: {}", e);
        }
    }
}

#[cfg(not(feature = "file-backend"))]
criterion_group!(
    benches,
    memory_backend_benchmarks,
    bench_key_serialization,
    bench_eviction_policies
);

#[cfg(feature = "file-backend")]
criterion_group!(
    benches,
    memory_backend_benchmarks,
    file_backend_benchmarks,
    bench_key_serialization,
    bench_eviction_policies
);

#[cfg(feature = "redis-backend")]
criterion_group!(redis_benches, redis_backend_benchmarks);

#[cfg(feature = "rocksdb-backend")]
criterion_group!(rocksdb_benches, rocksdb_backend_benchmarks);

#[cfg(all(feature = "redis-backend", feature = "rocksdb-backend"))]
criterion_main!(benches, redis_benches, rocksdb_benches);

#[cfg(all(feature = "redis-backend", not(feature = "rocksdb-backend")))]
criterion_main!(benches, redis_benches);

#[cfg(all(not(feature = "redis-backend"), feature = "rocksdb-backend"))]
criterion_main!(benches, rocksdb_benches);

#[cfg(all(not(feature = "redis-backend"), not(feature = "rocksdb-backend")))]
criterion_main!(benches);
