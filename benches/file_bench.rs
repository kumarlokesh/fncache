//! File backend benchmarks
//!
//! Run with:
//!   cargo bench --bench file_bench --features file-backend
//!
//! Requires feature: `file-backend`

#![allow(clippy::needless_return)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use futures::executor::block_on;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "file-backend")]
use fncache::backends::file::FileBackend;

#[cfg(feature = "file-backend")]
use fncache::backends::CacheBackend;

#[cfg(feature = "file-backend")]
use tempfile::TempDir;

const SMALL_DATA_SIZE: usize = 100;
const MEASUREMENT_TIME_MS: u64 = 2000;
const WARMUP_TIME_MS: u64 = 500;
const MIN_SAMPLE_SIZE: usize = 10;
const DEFAULT_TTL_SECONDS: u64 = 60;

fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

fn configure_benchmark_group(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
) {
    group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.warm_up_time(Duration::from_millis(WARMUP_TIME_MS));
    group.sample_size(MIN_SAMPLE_SIZE);
    group.sampling_mode(SamplingMode::Flat);
}

#[cfg(feature = "file-backend")]
fn file_backend_benchmarks(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    // Basic ops
    {
        let mut group = c.benchmark_group("file_backend_basic");
        configure_benchmark_group(&mut group);

        let backend = Arc::new(FileBackend::new(path).unwrap());
        let data = generate_data(SMALL_DATA_SIZE);

        group.bench_function("set", |b| {
            let backend = Arc::clone(&backend);
            let data = data.clone();
            b.iter(|| {
                let key = format!("bench_set_{}", black_box(42));
                block_on(backend.set(key, data.clone(), None)).unwrap()
            });
        });

        let get_key = "bench_get_hit".to_string();
        block_on(backend.set(get_key.clone(), data.clone(), None)).unwrap();

        group.bench_function("get_hit", |b| {
            let backend = Arc::clone(&backend);
            let key = get_key.clone();
            b.iter(|| block_on(backend.get(&key)).unwrap());
        });

        group.bench_function("get_miss", |b| {
            let backend = Arc::clone(&backend);
            b.iter(|| block_on(backend.get(&"nonexistent".to_string())).unwrap());
        });

        group.bench_function("remove", |b| {
            let backend = Arc::clone(&backend);
            let data = data.clone();
            b.iter(|| {
                let key = "bench_remove".to_string();
                block_on(async {
                    backend.set(key.clone(), data.clone(), None).await.unwrap();
                    backend.remove(&key).await.unwrap();
                });
            });
        });

        group.finish();
    }

    // TTL
    {
        let mut group = c.benchmark_group("file_backend_ttl");
        configure_benchmark_group(&mut group);

        let backend = Arc::new(FileBackend::new(path).unwrap());
        let data = generate_data(SMALL_DATA_SIZE);

        group.bench_function("set_with_ttl", |b| {
            let backend = Arc::clone(&backend);
            let data = data.clone();
            b.iter(|| {
                let key = "bench_ttl".to_string();
                block_on(backend.set(
                    key,
                    data.clone(),
                    Some(Duration::from_secs(DEFAULT_TTL_SECONDS)),
                ))
                .unwrap()
            });
        });

        group.finish();
    }
}

#[cfg(not(feature = "file-backend"))]
fn file_backend_benchmarks(_: &mut Criterion) {
    // Feature not enabled; nothing to run
}

criterion_group!(benches, file_backend_benchmarks);
criterion_main!(benches);
