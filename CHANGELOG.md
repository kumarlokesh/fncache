# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Production-ready README with security considerations
- Comprehensive security audit documentation
- Performance benchmarks and metrics
- CONTRIBUTING.md and development guidelines
- ROADMAP.md for future development plans

### Changed

- Updated README structure for production quality
- Moved development roadmap to separate document
- Enhanced documentation with security best practices

### Fixed

- Global cache type mismatch compilation errors
- Memory safety issues in test infrastructure
- API examples updated to current implementation

## [0.3.0] - 2024-XX-XX

### Added

- Cache invalidation via tags and prefixes
- Background cache warming capabilities
- Advanced metrics (latency, size tracking)
- Compile-time key derivation strategies
- LRU and LFU eviction policies
- Comprehensive security audit
- Thread-safe eviction policy implementations

### Changed

- Improved error handling and type safety
- Enhanced macro hygiene and expansion
- Better serialization/deserialization support

### Fixed

- Race conditions in concurrent access scenarios
- Memory leaks in eviction policy implementations
- Test isolation and interference issues

## [0.2.0] - 2024-XX-XX

### Added

- `CacheBackend` trait for pluggable storage
- File-based backend with persistent storage
- Redis backend using `redis-rs`
- RocksDB backend for high-performance caching
- Custom serialization support
- Feature flags for optional backends

### Changed

- Modular architecture with trait-based backends
- Improved async/sync function support
- Enhanced error types and handling

### Fixed

- Serialization compatibility issues
- Backend initialization edge cases

## [0.1.0] - 2024-XX-XX

### Added

- Initial release with core functionality
- `#[fncache]` attribute macro
- Basic key derivation from function arguments
- Thread-safe in-memory storage backend
- Time-based expiry (TTL) support
- Basic metrics (hit/miss counts)
- Async and sync function support
- Type-safe caching with strong guarantees

### Security

- Initial security review completed
- Safe defaults for all configuration options
- Input validation and sanitization

---

## Release Notes

### Version 0.3.0 - Production Ready

This release marks fncache as production-ready with comprehensive security auditing, advanced features, and robust testing infrastructure.

**Key Highlights:**

- **Security First**: Complete security audit with mitigation strategies
- **Advanced Features**: Cache invalidation, background warming, eviction policies
- **Performance**: Optimized for high-throughput scenarios
- **Documentation**: Production-quality documentation and examples

### Version 0.2.0 - Pluggable Backends

Introduced the pluggable backend architecture, enabling support for multiple storage systems.

**Key Highlights:**

- **Flexibility**: Support for Memory, File, Redis, and RocksDB backends
- **Feature Flags**: Optional dependencies for lean builds
- **Custom Serialization**: Support for different serialization formats

### Version 0.1.0 - MVP Release

Initial release providing core function-level caching capabilities.

**Key Highlights:**

- **Zero Boilerplate**: Simple attribute-based API
- **Type Safety**: Strong compile-time guarantees
- **Async Support**: Seamless async/sync function caching
