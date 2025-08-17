use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fncache::backends::memory::MemoryBackend;
use fncache::backends::CacheBackend;
use futures::executor::block_on;
use std::sync::Arc;

fn simple_set_benchmark(c: &mut Criterion) {
    let backend = Arc::new(MemoryBackend::new());

    let mut group = c.benchmark_group("simple_memory_benchmark");
    group.bench_function("set", |b| {
        let backend_clone = backend.clone();
        b.iter(|| {
            block_on(backend_clone.set(
                "test_key".to_string(),
                black_box("test_value".as_bytes().to_vec()),
                None,
            ))
        });
    });
    group.finish();
}

criterion_group!(benches, simple_set_benchmark);
criterion_main!(benches);
