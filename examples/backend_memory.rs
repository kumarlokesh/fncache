//! Example: Default memory backend configuration

use fncache::{backends::memory::MemoryBackend, fncache, init_global_cache, Result};

#[fncache(ttl = 60)]
fn compute_value(input: u32) -> u32 {
    println!("Computing value for {}", input);
    input * 42
}

fn main() -> Result<()> {
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

    Ok(())
}
