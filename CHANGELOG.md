# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2025-08-24

### Improved

- Redis backend now reuses a single async connection via `redis::aio::ConnectionManager`, reducing latency and connection churn.
- Benchmarks: removed artificial sleeps from `benches/redis_bench.rs` for realistic timings; observed ~2x speedup vs 0.1.1 on basic ops.

### Internal

- Enabled `redis` crate `connection-manager` feature in `Cargo.toml`.
- Implemented custom `Debug` for `RedisBackend` to satisfy trait bounds while omitting non-Debug fields.

## [0.1.1] - 2025-08-17

### Fixed

- Minor fixes and documentation updates.

## [0.1.0] - 2025-08-17

### Added

#### Core Features

- **Zero Boilerplate**: Simple attribute-based API
- **Type Safety**: Strong compile-time guarantees
- **Async Support**: Seamless async/sync function caching
- **Flexibility**: Support for Memory, File, Redis, and RocksDB backends
- **Feature Flags**: Optional dependencies for lean builds
- **Custom Serialization**: Support for different serialization formats

#### Advanced Features

- **Cache Invalidation**: Tag-based and prefix-based cache invalidation
- **Background Warming**: Proactive cache population for improved performance
- **Eviction Policies**: LRU and LFU strategies
- **Metrics**: Built-in instrumentation with latency, hit rates, and size tracking
