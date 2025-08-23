// Run with:
//   cargo run --example backend_redis --features redis-backend
// Requires a running Redis server at redis://127.0.0.1:6379

use tokio::time::sleep;

use fncache::{fncache, init_global_cache, Result};

use fncache::backends::redis::RedisBackend;

#[fncache(ttl = 60)]
async fn cached_fetch(key: &str) -> String {
    println!("Fetching from source for '{key}' ...");
    sleep(Duration::from_secs(1)).await;
    format!("value_for_{key}")
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Initializing Redis backend at redis://127.0.0.1:6379 (prefix 'example:')");
    let backend = RedisBackend::new("redis://127.0.0.1:6379", Some("example:")).await?;
    init_global_cache(backend)?;

    // First call: computes and stores in Redis
    let v1 = cached_fetch("alpha").await;
    println!("Result 1: {v1}");

    // Second call: served from Redis cache
    let v2 = cached_fetch("alpha").await;
    println!("Result 2: {v2}");

    Ok(())
}
