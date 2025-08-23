//! Example demonstrating file backend for persistent caching
//!
//! This example shows how to use the file backend for persistent caching
//! across application restarts.
//!
//! NOTE: This example requires the "file-backend" feature to be enabled.
//! Run with: cargo run --example backend_file --features file-backend

#[cfg(feature = "file-backend")]
use fncache::{backends::file::FileBackend, fncache, init_global_cache, Result};

#[cfg(feature = "file-backend")]
#[fncache(ttl = 3600)] // Cache for 1 hour
fn compute_expensive_value(input: u32) -> u32 {
    println!("Computing expensive value for {}", input);
    std::thread::sleep(std::time::Duration::from_millis(500));
    input * 100
}

#[cfg(not(feature = "file-backend"))]
fn main() {
    eprintln!(
        "This example requires the 'file-backend' feature.\nRun with: cargo run --example backend_file --features file-backend"
    );
}

#[cfg(feature = "file-backend")]
fn main() -> Result<()> {
    let cache_dir = "/tmp/fncache_example";
    std::fs::create_dir_all(cache_dir).expect("Failed to create cache directory");

    println!("Initializing file backend at {}", cache_dir);
    let backend = FileBackend::new(cache_dir)?;
    init_global_cache(backend)?;

    println!("\n--- Computing values ---");
    for i in 1..6 {
        let result = compute_expensive_value(i);
        println!("Value for {}: {}", i, result);
    }

    println!("\n--- Reading from cache ---");
    for i in 1..6 {
        let result = compute_expensive_value(i);
        println!("Value for {}: {}", i, result);
    }

    println!("\n--- Cache files in {} ---", cache_dir);
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                println!("Cache file: {}", entry.file_name().to_string_lossy());
            }
        }
    }

    println!("\nNote: The cache files will persist after this program exits.");
    println!("Run this example again to see that values are loaded from cache.");

    Ok(())
}
