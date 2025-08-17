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

#[fncache(ttl = 5)]
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
    init_global_cache(MemoryBackend::new())?;
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

    println!("Invalidating config:api_url...");
    let cache = fncache::global_cache();
    let cache_guard = cache.lock().unwrap();
    cache_guard.remove(&"config:api_url".to_string()).await?;

    let config3 = get_config("api_url");
    println!("Config after invalidation: {}", config3);

    // Example 3: Tag-based invalidation
    println!("\n--- Tag-based invalidation ---");
    let user1 = get_user_data(101);
    let user2 = get_user_data(102);
    println!("Users: {}, {}", user1, user2);

    let product1 = get_product_info(201);
    let product2 = get_product_info(202);
    println!("Products: {}, {}", product1, product2);

    println!("Invalidating 'user_data' tag...");
    inv_cache.invalidate_tag(&Tag::new("user_data"))?;

    let cache = fncache::global_cache();
    let cache_guard = cache.lock().unwrap();
    cache_guard.remove(&"user_data:101".to_string()).await?;
    cache_guard.remove(&"user_data:102".to_string()).await?;

    // User data should be recomputed, but product data should still be cached
    println!("After tag invalidation:");
    println!("User data: {}", get_user_data(101));
    println!("Product info: {}", get_product_info(201));

    // Example 4: Prefix-based invalidation
    println!("\n--- Prefix-based invalidation ---");
    let db_config = get_config("db_url");
    let api_config = get_config("api_key");
    println!("Configs: {}, {}", db_config, api_config);

    println!("Invalidating 'config' prefix...");
    inv_cache.invalidate_prefix("config")?;
    let cache = fncache::global_cache();
    let cache_guard = cache.lock().unwrap();
    cache_guard.remove(&"config:db_url".to_string()).await?;
    cache_guard.remove(&"config:api_key".to_string()).await?;

    // All config items should be recomputed
    println!("After prefix invalidation:");
    println!("DB Config: {}", get_config("db_url"));
    println!("API Config: {}", get_config("api_key"));

    Ok(())
}
