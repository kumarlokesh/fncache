# fncache Architecture

## Overview

fncache is a zero-boilerplate Rust library for function-level caching with pluggable backends. It uses a procedural macro to automatically cache function results based on arguments.

## System Components

```ascii
┌─────────────────┐     ┌───────────────────────────┐
│ #[fncache(...)] │────▶│ Function Argument Parser  │
└─────────────────┘     └───────────────┬───────────┘
                                        │
                                        ▼
┌─────────────────┐     ┌───────────────────────────┐
│  Serialization  │◀───▶│     Cache Key Builder     │
└─────────────────┘     └───────────────┬───────────┘
                                        │
                                        ▼
┌─────────────────┐     ┌───────────────────────────┐
│    Metrics      │◀───▶│      GlobalCache          │
└─────────────────┘     └───────────────┬───────────┘
                                        │
                                        ▼
┌─────────────────┐     ┌───────────────────────────┐
│  Invalidation   │◀───▶│    CacheBackend Trait     │
└─────────────────┘     └───────────────┬───────────┘
                                        │
                        ┌───────────────┼───────────┐
                        │               │           │
                        ▼               ▼           ▼
              ┌─────────────────┐ ┌────────────┐ ┌─────────┐
              │MemoryBackend    │ │FileBackend │ │ Redis   │ ...
              └─────────────────┘ └────────────┘ └─────────┘
```

## Component Details

### Procedural Macro (`#[fncache(...)]`)

- Analyzes function signatures and arguments
- Generates cache key derivation code
- Wraps function execution with caching logic

### Cache Key Builder

- Converts function arguments to a unique cache key
- Uses serialization to handle complex types

### GlobalCache

- Singleton wrapper around the configured backend
- Thread-safe access to the cache via sync primitives

### CacheBackend Trait

- Common interface for all storage backends
- Async methods for get, set, remove, etc.

### Invalidation System

- Tag-based cache invalidation
- Prefix-based cache invalidation
- Both sync and async APIs

### Eviction Policies

- LRU (Least Recently Used) strategy
- LFU (Least Frequently Used) strategy
- Configurable capacity limits

### Metrics

- Hit/miss tracking
- Cache efficiency statistics

## Cache Invalidation Architecture

The invalidation system enables selectively clearing cached values through tags and prefixes.

```ascii
┌─────────────────┐     ┌───────────────────────────┐
│ TaggedCacheEntry│────▶│    InvalidationCache      │
└─────────────────┘     └───────────────┬───────────┘
                                        │
                                        ▼
┌─────────────────┐     ┌───────────────────────────┐
│  Tag Registry   │◀───▶│    CacheInvalidation      │
└─────────────────┘     │    AsyncCacheInvalidation │
                        └───────────────┬───────────┘
                                        │
                                        ▼
┌─────────────────┐     ┌───────────────────────────┐
│  Prefix Registry│◀───▶│      CacheBackend         │
└─────────────────┘     └───────────────────────────┘
```

### Tag and Prefix Invalidation

1. **Tags**: Cache entries can be associated with one or more tags
   - Invalidating a tag clears all cache entries associated with that tag
   - Multiple tags can be invalidated at once

2. **Prefixes**: Cache keys can be invalidated by their prefix
   - Prefix invalidation clears all cache entries whose keys start with the given prefix
   - Multiple prefixes can be invalidated at once

3. **Implementation**:
   - `InvalidationCache` wraps a backend and maintains tag and prefix mappings
   - Thread-safe registries track the relationships between tags/prefixes and cache keys
   - Both sync (`CacheInvalidation`) and async (`AsyncCacheInvalidation`) APIs are provided

## Data Flow

1. Function call with `#[fncache]` attribute is intercepted by the macro
2. Arguments are serialized to create a cache key
3. Cache is checked for an existing value
4. If found, value is deserialized and returned
5. If not found, function is executed, result cached, then returned
6. Invalidation tags/prefixes are maintained as needed

## Future Improvements

- Eviction policies (LRU, LFU)
- Background cache warming
- More advanced metrics
- Compile-time key derivation optimization
- WASM support
