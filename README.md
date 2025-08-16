# fncache

[![Crates.io](https://img.shields.io/crates/v/fncache.svg)](https://crates.io/crates/fncache)
[![Documentation](https://docs.rs/fncache/badge.svg)](https://docs.rs/fncache)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A zero-boilerplate Rust library for function-level caching with pluggable backends, inspired by `functools.lru_cache` and `request-cache`.

## Features

- **🚀 Zero Boilerplate**: Simple `#[fncache]` attribute for instant caching
- **🔌 Pluggable Backends**: Memory, File, Redis, RocksDB support
- **⚡ Async/Sync**: Seamless support for both synchronous and asynchronous functions
- **🛡️ Type Safety**: Strong typing throughout the caching layer with compile-time guarantees
- **📊 Advanced Metrics**: Built-in instrumentation with latency, hit rates, and size tracking
- **🏷️ Cache Invalidation**: Tag-based and prefix-based cache invalidation
- **🔥 Background Warming**: Proactive cache population for improved performance

## Quick Start

Add `fncache` to your `Cargo.toml` with the desired features:

```toml
[dependencies]
fncache = { version = "0.1", features = ["memory"] }
```

### Basic Usage

```rust
use fncache::{
    backends::memory::MemoryBackend,
    init_global_cache,
    fncache,
    Result,
};

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
    let result1 = expensive_operation(5);
    println!("Result 1: {}", result1); // Takes ~1 second
    
    // Second call returns cached result
    let result2 = expensive_operation(5);
    println!("Result 2: {}", result2); // Returns immediately
    
    Ok(())
}
```

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

### Redis Backend

```rust
use fncache::backends::redis::RedisBackend;

#[tokio::main]
async fn main() -> Result<()> {
    let backend = RedisBackend::new("redis://localhost:6379")?;
    init_global_cache(backend)?;
    
    // Your cached functions work the same way
    Ok(())
}
```

### File Backend

```rust
use fncache::backends::file::FileBackend;

fn main() -> Result<()> {
    let backend = FileBackend::new("/tmp/cache")?;
    init_global_cache(backend)?;
    Ok(())
}
```

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

- **Rust**: 1.70 or later
- **Runtime**: `tokio` for async support
- **Dependencies**: Automatically managed via features

## Performance

- **Memory Backend**: ~10ns cache hit latency
- **Redis Backend**: ~1ms cache hit latency (network dependent)
- **File Backend**: ~100μs cache hit latency
- **Throughput**: >1M operations/second (memory backend)

See `benches/` for detailed benchmarks.

## Documentation

- **[Architecture](ARCHITECTURE.md)** - Internal design and architecture

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and breaking changes.

## License

MIT License - see [LICENSE](LICENSE) for details.
