//! Example of using fncache with asynchronous functions

use fncache::{backends::memory::MemoryBackend, fncache, init_global_cache, Result};
use tokio::time::sleep;

#[fncache(ttl = 30)]
async fn expensive_async_operation(x: u64) -> u64 {
    println!("Performing expensive async operation for {}", x);
    sleep(Duration::from_secs(1)).await;
    x * x * x
}

#[tokio::main]
async fn main() -> Result<()> {
    init_global_cache(MemoryBackend::new())?;

    let result1 = expensive_async_operation(3).await;
    println!("Result 1: {}", result1);

    let result2 = expensive_async_operation(3).await;
    println!("Result 2: {}", result2);

    let result3 = expensive_async_operation(4).await;
    println!("Result 3: {}", result3);

    Ok(())
}
