//! Example demonstrating cache invalidation techniques
//!
//! This example shows different ways to invalidate cached values:
//! - Using TTL (time-to-live)
//! - Manual invalidation
//! - Tag-based invalidation
//! - Prefix-based invalidation

use fncache::{
    backends::{memory::MemoryBackend, CacheBackend},
    fncache, init_global_cache,
    invalidation::{CacheInvalidation, InvalidationCache, Tag},
    Result,
};
use std::time::Duration;

#[fncache(ttl = 5)] // Short TTL for demonstration
fn data_with_ttl(id: u32) -> String {
    println!("Computing data with TTL for id {}", id);
    format!("Data-{}", id)
}

#[fncache(tags = ["user_data"])]
fn get_user_data(user_id: u32) -> String {
    println!("Fetching user data for id {}", user_id);
    format!("User data for {}", user_id)
}

#[fncache(tags = ["product", "inventory"])]
fn get_product_info(product_id: u32) -> String {
    println!("Fetching product info for id {}", product_id);
    format!("Product-{} info", product_id)
}

#[fncache(prefix = "config")]
fn get_config(name: &str) -> String {
    println!("Fetching config {}", name);
    format!("Config value for {}", name)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the global cache with a memory backend
    // This will be used by the fncache macro for caching function results
    init_global_cache(MemoryBackend::new())?;

    // Create a separate local cache for demonstrating invalidation
    // We use InvalidationCache wrapper for explicit invalidation operations
    let inv_cache = InvalidationCache::new(MemoryBackend::new());

    // Example 1: TTL-based expiration
    println!("\n--- TTL-based expiration ---");
    let data1 = data_with_ttl(1);
    println!("First call: {}", data1);

    let data2 = data_with_ttl(1);
    println!("Second call (cached): {}", data2);

    println!("Waiting for TTL to expire (5 seconds)...");
    tokio::time::sleep(Duration::from_secs(6)).await;

    let data3 = data_with_ttl(1);
    println!("After TTL expired: {}", data3);

    // Example 2: Manual key invalidation
    println!("\n--- Manual key invalidation ---");
    let config1 = get_config("api_url");
    println!("Config: {}", config1);

    let config2 = get_config("api_url");
    println!("Config (cached): {}", config2);

    // Manually invalidate the key by removing it directly
    println!("Invalidating config:api_url...");
    // Use the global cache's remove method
    let cache = fncache::global_cache();
    let cache_guard = cache.lock().unwrap();
    cache_guard.remove(&"config:api_url".to_string()).await?;

    let config3 = get_config("api_url");
    println!("Config after invalidation: {}", config3);

    // Example 3: Tag-based invalidation
    println!("\n--- Tag-based invalidation ---");
    // Cache some user data
    let user1 = get_user_data(101);
    let user2 = get_user_data(102);
    println!("Users: {}, {}", user1, user2);

    // Cache some product data
    let product1 = get_product_info(201);
    let product2 = get_product_info(202);
    println!("Products: {}, {}", product1, product2);

    // Invalidate all items with the "user_data" tag
    println!("Invalidating 'user_data' tag...");
    // For tag invalidation, we use our local InvalidationCache
    // In a real application, you would use the same cache for both storage and invalidation
    inv_cache.invalidate_tag(&Tag::new("user_data"))?;

    // To simulate the effect on the global cache, we'll also invalidate the key directly
    let cache = fncache::global_cache();
    let cache_guard = cache.lock().unwrap();
    cache_guard.remove(&"user_data:101".to_string()).await?;
    cache_guard.remove(&"user_data:102".to_string()).await?;

    // User data should be recomputed, but product data should still be cached
    println!("After tag invalidation:");
    println!("User data: {}", get_user_data(101)); // Should recompute
    println!("Product info: {}", get_product_info(201)); // Should use cache

    // Example 4: Prefix-based invalidation
    println!("\n--- Prefix-based invalidation ---");
    // Cache multiple config items
    let db_config = get_config("db_url");
    let api_config = get_config("api_key");
    println!("Configs: {}, {}", db_config, api_config);

    // Invalidate all items with the "config" prefix
    println!("Invalidating 'config' prefix...");
    // For prefix invalidation, use our local InvalidationCache
    inv_cache.invalidate_prefix("config")?;

    // Simulate the effect on the global cache
    let cache = fncache::global_cache();
    let cache_guard = cache.lock().unwrap();
    cache_guard.remove(&"config:db_url".to_string()).await?;
    cache_guard.remove(&"config:api_key".to_string()).await?;

    // All config items should be recomputed
    println!("After prefix invalidation:");
    println!("DB Config: {}", get_config("db_url")); // Should recompute
    println!("API Config: {}", get_config("api_key")); // Should recompute

    Ok(())
}
