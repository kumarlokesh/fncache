//! Example demonstrating memory backend configuration options
//!
//! This example shows how to configure the memory backend with different options,
//! including capacity limits and eviction policies.

use fncache::{backends::memory::MemoryBackend, fncache, init_global_cache, Result};

#[fncache(ttl = 60)]
fn compute_value(input: u32) -> u32 {
    println!("Computing value for {}", input);
    input * 42
}

/// Example showing different memory backend configurations
fn main() -> Result<()> {
    // Example 1: Default configuration
    {
        println!("\n--- Default Memory Backend ---");
        let backend = MemoryBackend::new();
        init_global_cache(backend)?;

        // First call will execute function
        let result1 = compute_value(10);
        println!("Result 1: {}", result1);

        // Second call uses cache
        let result2 = compute_value(10);
        println!("Result 2: {}", result2);

        // Different input executes function
        let result3 = compute_value(20);
        println!("Result 3: {}", result3);
    }

    // Example 2: With capacity limit
    {
        println!("\n--- Memory Backend with Capacity ---");
        let backend = MemoryBackend::new().with_capacity(100);
        init_global_cache(backend)?;

        for i in 0..5 {
            let result = compute_value(i);
            println!("Result for {}: {}", i, result);
        }
    }

    // Example 3: With LRU eviction policy
    {
        println!("\n--- Memory Backend with LRU Policy ---");
        let backend = MemoryBackend::new()
            .with_capacity(10) // Small capacity to demonstrate eviction
            .with_eviction_policy("lru"); // Use the built-in eviction policy factory
        init_global_cache(backend)?;

        // Fill cache
        for i in 0..15 {
            compute_value(i);
        }

        // Access some items to update LRU order
        compute_value(5);
        compute_value(7);
        compute_value(9);

        // Check which ones stay in cache (would see in console output)
        for i in 0..15 {
            compute_value(i);
        }
    }

    // Example 4: With LFU eviction policy
    {
        println!("\n--- Memory Backend with LFU Policy ---");
        let backend = MemoryBackend::new()
            .with_capacity(10) // Small capacity to demonstrate eviction
            .with_eviction_policy("lfu"); // Use the built-in eviction policy factory
        init_global_cache(backend)?;

        // Fill cache and access some items more frequently
        for _ in 0..3 {
            compute_value(1);
            compute_value(3);
            compute_value(5);
        }

        for i in 0..15 {
            compute_value(i);
        }

        // Check which ones stay in cache (would see in console output)
        for i in 0..15 {
            compute_value(i);
        }
    }

    Ok(())
}
