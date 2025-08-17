//! Example demonstrating key derivation strategies
//!
//! This example shows the two key derivation strategies:
//! 1. Runtime key derivation - More flexible but with serialization overhead
//! 2. Compile-time key derivation - More efficient but with some limitations

use fncache::{backends::memory::MemoryBackend, fncache, init_global_cache, Result};
use std::time::{Duration, Instant};

// Runtime key derivation (default)
#[fncache(key_derivation = "runtime")]
fn with_runtime_derivation(a: u32, b: String, c: bool) -> u32 {
    println!("Computing with runtime derivation: {}, {}, {}", a, b, c);
    std::thread::sleep(Duration::from_millis(10));
    if c {
        a + b.len() as u32
    } else {
        a - b.len() as u32
    }
}

// Compile-time key derivation
#[fncache(key_derivation = "compile_time")]
fn with_compile_time_derivation(a: u32, b: String, c: bool) -> u32 {
    println!(
        "Computing with compile-time derivation: {}, {}, {}",
        a, b, c
    );
    std::thread::sleep(Duration::from_millis(10));
    if c {
        a + b.len() as u32
    } else {
        a - b.len() as u32
    }
}

fn main() -> Result<()> {
    init_global_cache(MemoryBackend::new())?;

    println!("Key Derivation Strategies Example");
    println!("=================================");

    // Example 1: Runtime key derivation
    println!("\n--- Runtime Key Derivation ---");

    // First call (cache miss)
    let start = Instant::now();
    let result1 = with_runtime_derivation(42, "hello".to_string(), true);
    let duration1 = start.elapsed();
    println!("First call result: {} (took {:?})", result1, duration1);

    // Second call (cache hit)
    let start = Instant::now();
    let result2 = with_runtime_derivation(42, "hello".to_string(), true);
    let duration2 = start.elapsed();
    println!("Second call result: {} (took {:?})", result2, duration2);

    // Different arguments (cache miss)
    let result3 = with_runtime_derivation(42, "world".to_string(), true);
    println!("Different args result: {}", result3);

    // Example 2: Compile-time key derivation
    println!("\n--- Compile-time Key Derivation ---");

    // First call (cache miss)
    let start = Instant::now();
    let result1 = with_compile_time_derivation(42, "hello".to_string(), true);
    let duration1 = start.elapsed();
    println!("First call result: {} (took {:?})", result1, duration1);

    // Second call (cache hit)
    let start = Instant::now();
    let result2 = with_compile_time_derivation(42, "hello".to_string(), true);
    let duration2 = start.elapsed();
    println!("Second call result: {} (took {:?})", result2, duration2);

    // Different arguments (cache miss)
    let result3 = with_compile_time_derivation(42, "world".to_string(), true);
    println!("Different args result: {}", result3);

    println!("\n--- Benchmark: 1000 cache hits ---");

    // Runtime key derivation benchmark
    let start = Instant::now();
    for _ in 0..1000 {
        with_runtime_derivation(42, "hello".to_string(), true);
    }
    let runtime_duration = start.elapsed();
    println!("Runtime derivation: {:?}", runtime_duration);

    // Compile-time key derivation benchmark
    let start = Instant::now();
    for _ in 0..1000 {
        with_compile_time_derivation(42, "hello".to_string(), true);
    }
    let compile_time_duration = start.elapsed();
    println!("Compile-time derivation: {:?}", compile_time_duration);

    println!(
        "\nCompile-time is approximately {:.2}x faster for cache hits",
        runtime_duration.as_nanos() as f64 / compile_time_duration.as_nanos() as f64
    );

    Ok(())
}
