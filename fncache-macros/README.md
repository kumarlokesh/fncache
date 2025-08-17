# fncache-macros

[![Crates.io](https://img.shields.io/crates/v/fncache-macros.svg)](https://crates.io/crates/fncache-macros)
[![Documentation](https://docs.rs/fncache-macros/badge.svg)](https://docs.rs/fncache-macros)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Procedural macros for the [fncache](https://crates.io/crates/fncache) library, providing attribute macros for zero-boilerplate function caching.

## Overview

This crate implements the procedural macros that power the `fncache` caching library. It's typically not used directly but through the main `fncache` crate.

## Features

- **#[fncache]** - The main attribute macro for caching function results
- Supports both synchronous and asynchronous functions
- Runtime and compile-time key derivation strategies
- TTL (Time-To-Live) configuration

## Usage

This crate is meant to be used through the main `fncache` crate:

```toml
# Cargo.toml
[dependencies]
fncache = "0.1.0"
```

The macros are re-exported by the main crate:

```rust
use fncache::fncache;

#[fncache(ttl = 60)]
fn expensive_calculation(input: u64) -> u64 {
    // This result will be cached for 60 seconds
    input * input
}

#[fncache(ttl = 300, key_derivation = "compile_time")]
async fn fetch_data(id: &str) -> String {
    // This uses compile-time key derivation and 300 second TTL
    format!("Data for {}", id)
}
```

## Options

- **ttl** (optional, default: 60) - Cache time-to-live in seconds
- **key_derivation** (optional, default: "runtime")
  - "runtime" - Keys are derived from function arguments
  - "compile_time" - Keys are derived from the function name and module path

## License

MIT License - see [LICENSE](../LICENSE) for details.
