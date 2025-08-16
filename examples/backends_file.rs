//! Example demonstrating file backend for persistent caching
//!
//! This example shows how to use the file backend for persistent caching
//! across application restarts.
//!
//! NOTE: This example requires the "file-backend" feature to be enabled.
//! Run with: cargo run --example backends_file --features file-backend

// Compile-time check to ensure the file-backend feature is enabled
#[cfg(not(feature = "file-backend"))]
compile_error!(
    "This example requires the 'file-backend' feature to be enabled. \
                Run with: cargo run --example backends_file --features file-backend"
);

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
    // This function will never be called due to the compile_error above
    // but is needed for the example to have a main function
    // The compile_error macro will prevent compilation anyway
}

#[cfg(feature = "file-backend")]
fn main() -> Result<()> {
    // Create cache directory if it doesn't exist
    let cache_dir = "/tmp/fncache_example";
    std::fs::create_dir_all(cache_dir).expect("Failed to create cache directory");

    // Initialize with file backend
    println!("Initializing file backend at {}", cache_dir);
    let backend = FileBackend::new(cache_dir)?;
    init_global_cache(backend)?;

    // Compute some values
    println!("\n--- Computing values ---");
    for i in 1..6 {
        let result = compute_expensive_value(i);
        println!("Value for {}: {}", i, result);
    }

    // Check if values are cached
    println!("\n--- Reading from cache ---");
    for i in 1..6 {
        let result = compute_expensive_value(i);
        println!("Value for {}: {}", i, result);
    }

    // Manually inspect cache directory
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
