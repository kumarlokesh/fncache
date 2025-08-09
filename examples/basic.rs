//! Basic example of using fncache with synchronous functions

use fncache::{backends::memory::MemoryBackend, init_global_cache, Result};

// Required for the macro to work
use bincode;
use futures;

// Ensure the memory backend is available
#[cfg(not(feature = "memory"))]
compile_error!("This example requires the 'memory' feature to be enabled");

#[fncache::fncache(ttl = 5)]
fn expensive_operation(a: u64, b: u64) -> Result<u64> {
    println!("Performing expensive computation...");
    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(a + b)
}

fn main() -> Result<()> {
    init_global_cache(MemoryBackend::new())?;

    let result1 = expensive_operation(2, 3)?;
    println!("Result 1: {}", result1);

    let result2 = expensive_operation(2, 3)?;
    println!("Result 2: {}", result2);

    let result3 = expensive_operation(5, 5)?;
    println!("Result 3: {}", result3);

    Ok(())
}
