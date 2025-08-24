# fncache

[![Crates.io](https://img.shields.io/crates/v/fncache.svg)](https://crates.io/crates/fncache)
[![Documentation](https://docs.rs/fncache/badge.svg)](https://docs.rs/fncache)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A zero-boilerplate Rust library for function-level caching with pluggable backends, inspired by `functools.lru_cache` and `request-cache`.

## Features

- **Zero Boilerplate**: Simple `#[fncache]` attribute for instant caching
- **Pluggable Backends**: Memory, File, Redis, RocksDB support
- **Async/Sync**: Seamless support for both synchronous and asynchronous functions
- **Type Safety**: Strong typing throughout the caching layer with compile-time guarantees
- **Advanced Metrics**: Built-in instrumentation with latency, hit rates, and size tracking
- **Cache Invalidation**: Tag-based and prefix-based cache invalidation
- **Background Warming**: Proactive cache population for improved performance

## Quick Start

Add `fncache` to your `Cargo.toml` with the desired features:

```toml
[dependencies]
fncache = { version = "0.1.1", features = ["memory"] }
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

### Basic Usage

```rust
use fncache::{
    backends::memory::MemoryBackend,
    init_global_cache,
    fncache,
    Result,
};
use std::time::Instant;

#[fncache(ttl = 60)]
fn expensive_operation(x: u64) -> u64 {
    println!("Performing expensive operation for {}", x);
    std::thread::sleep(std::time::Duration::from_secs(1));
    x * x
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize with memory backend
    init_global_cache(MemoryBackend::new())?;
    
    // First call executes the function
    let t1 = Instant::now();
    let result1 = expensive_operation(5);
    let d1 = t1.elapsed();
    println!("Result 1: {} (took {:?})", result1, d1); // ~1s
    
    // Second call returns cached result
    let t2 = Instant::now();
    let result2 = expensive_operation(5);
    let d2 = t2.elapsed();
    println!("Result 2: {} (took {:?})", result2, d2); // microseconds
    
    Ok(())
}
```

Note: The first call performs the real work (~1s here). Subsequent calls with the same arguments return the cached result in microseconds.

### Async Function Example

```rust
use std::time::Duration;
use tokio::time::sleep;

#[fncache(ttl = 300)]
async fn fetch_data(id: &str) -> String {
    println!("Fetching data for {}", id);
    sleep(Duration::from_secs(1)).await;
    format!("Data for {}", id)
}
```

## Backend Examples

- **Memory backend**: see `examples/backend_memory.rs`
  - Run: `cargo run --example backend_memory`

- **File backend**: see `examples/backend_file.rs`
  - Run: `cargo run --example backend_file --features file-backend`

- **Redis backend**: see `examples/backend_redis.rs`
  - Requires a running Redis server at `redis://127.0.0.1:6379`
  - Run: `cargo run --example backend_redis --features redis-backend`

## Available Features

| Feature | Description | Default |
|---------|-------------|----------|
| `memory` | In-memory cache backend | ✅ |
| `redis-backend` | Redis backend support | ❌ |
| `file-backend` | File-based persistent cache | ❌ |
| `rocksdb-backend` | RocksDB high-performance backend | ❌ |
| `metrics` | Performance metrics collection | ✅ |
| `invalidation` | Tag-based cache invalidation | ✅ |

## Requirements

- **Rust**: 1.70+
- **Runtime**: `tokio` only if you use async cached functions or run async examples
- **Features/Backends**: enable via Cargo features (see table above)

## Performance

- **Memory Backend**: ~1-2μs set latency, ~850-970μs get hit latency
- **Redis Backend**: ~1ms cache hit latency (network dependent)
- **File Backend**: ~100μs cache hit latency
- **Throughput**: Thousands of operations/second (memory backend)

See `benches/` for detailed benchmarks.

## Documentation

- [Architecture](ARCHITECTURE.md) - Internal design and architecture

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and breaking changes.

## License

MIT License - see [LICENSE](LICENSE) for details.
