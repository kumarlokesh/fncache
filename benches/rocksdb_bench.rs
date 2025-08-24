//! RocksDB backend benchmarks
//!
//! Run with:
//!   cargo bench --bench rocksdb_bench --features rocksdb-backend
//!
//! Requires feature: `rocksdb-backend`

use criterion::{criterion_group, criterion_main, Criterion};
use futures::executor::block_on;
use std::sync::Arc;
use std::{
    env, fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "rocksdb-backend")]
use fncache::backends::rocksdb::RocksDBBackend;
#[cfg(feature = "rocksdb-backend")]
use fncache::backends::CacheBackend;

const SMALL_DATA_SIZE: usize = 100;

fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

#[cfg(feature = "rocksdb-backend")]
fn rocksdb_backend_benchmarks(c: &mut Criterion) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let dir = env::temp_dir().join(format!("fncache_rocksdb_bench_{}_{}", pid, ts));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.to_str().unwrap();

    let backend = Arc::new(RocksDBBackend::new(path).unwrap());

    let mut group = c.benchmark_group("rocksdb_backend");

    // set
    {
        let backend = Arc::clone(&backend);
        let data = generate_data(SMALL_DATA_SIZE);
        group.bench_function("set", |b| {
            let data = data.clone();
            b.iter(|| {
                let key = "bench:set".to_string();
                block_on(backend.set(key, data.clone(), None)).unwrap()
            });
        });
    }

    // get_miss
    {
        let backend = Arc::clone(&backend);
        group.bench_function("get_miss", |b| {
            b.iter(|| block_on(backend.get(&"bench:missing".to_string())).unwrap());
        });
    }

    // get_hit
    {
        let backend = Arc::clone(&backend);
        let key = "bench:hit".to_string();
        let data = generate_data(SMALL_DATA_SIZE);
        block_on(backend.set(key.clone(), data, None)).unwrap();
        group.bench_function("get_hit", |b| {
            let key = key.clone();
            b.iter(|| block_on(backend.get(&key)).unwrap());
        });
    }

    group.finish();
}

#[cfg(not(feature = "rocksdb-backend"))]
fn rocksdb_backend_benchmarks(_: &mut Criterion) {
    // Feature not enabled
}

criterion_group!(benches, rocksdb_backend_benchmarks);
criterion_main!(benches);
