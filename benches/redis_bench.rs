//! Redis backend benchmarks
//!
//! Run with:
//!   cargo bench --bench redis_bench --features redis-backend
//!
//! Requires a running Redis (default: redis://127.0.0.1:6379).
//! Override with env var `FN_REDIS_URL` or `REDIS_URL`.
//!
//! Requires feature: `redis-backend`

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

#[cfg(feature = "redis-backend")]
use fncache::backends::redis::RedisBackend;
#[cfg(feature = "redis-backend")]
use fncache::backends::CacheBackend;

const SMALL_DATA_SIZE: usize = 100;
const DEFAULT_TTL_SECONDS: u64 = 60;
const MEASUREMENT_TIME_MS: u64 = 1500;
const WARMUP_TIME_MS: u64 = 400;
const MIN_SAMPLE_SIZE: usize = 10;
fn redis_url() -> String {
    env::var("FN_REDIS_URL")
        .or_else(|_| env::var("REDIS_URL"))
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

#[cfg(feature = "redis-backend")]
fn redis_backend_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    // Try to connect; skip gracefully if unavailable
    let url = redis_url();
    let backend = match rt.block_on(RedisBackend::new(&url, Some("bench:"))) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "Skipping Redis benchmarks: server not available at {} (error: {:?})",
                url, e
            );
            return;
        }
    };

    let mut group = c.benchmark_group("redis_backend");
    group.measurement_time(std::time::Duration::from_millis(MEASUREMENT_TIME_MS));
    group.warm_up_time(std::time::Duration::from_millis(WARMUP_TIME_MS));
    group.sample_size(MIN_SAMPLE_SIZE);
    group.sampling_mode(SamplingMode::Flat);
    let backend = Arc::new(backend);

    // set
    {
        let backend = Arc::clone(&backend);
        let data = generate_data(SMALL_DATA_SIZE);
        group.bench_function("set", |b| {
            let data = data.clone();
            b.iter(|| {
                let key = "bench:set".to_string();
                rt.block_on(backend.set(key, data.clone(), None)).unwrap()
            });
        });
    }

    // get_miss
    {
        let backend = Arc::clone(&backend);
        group.bench_function("get_miss", |b| {
            b.iter(|| {
                rt.block_on(backend.get(&"bench:missing".to_string()))
                    .unwrap()
            });
        });
    }

    // get_hit
    {
        let backend = Arc::clone(&backend);
        let key = "bench:hit".to_string();
        let data = generate_data(SMALL_DATA_SIZE);
        rt.block_on(backend.set(key.clone(), data, None)).unwrap();
        group.bench_function("get_hit", |b| {
            let key = key.clone();
            b.iter(|| rt.block_on(backend.get(&key)).unwrap());
        });
    }

    // set_with_ttl
    {
        let backend = Arc::clone(&backend);
        let data = generate_data(SMALL_DATA_SIZE);
        group.bench_function("set_with_ttl", |b| {
            let data = data.clone();
            b.iter(|| {
                let key = "bench:ttl".to_string();
                rt.block_on(backend.set(
                    key,
                    data.clone(),
                    Some(Duration::from_secs(DEFAULT_TTL_SECONDS)),
                ))
                .unwrap()
            });
        });
    }

    group.finish();
}

#[cfg(not(feature = "redis-backend"))]
fn redis_backend_benchmarks(_: &mut Criterion) {
    // Feature not enabled
}

criterion_group!(benches, redis_backend_benchmarks);
criterion_main!(benches);
