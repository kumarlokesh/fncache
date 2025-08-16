//! Example demonstrating error handling in fncache
//!
//! This example shows how fncache handles errors from different sources:
//! - Backend errors
//! - Function errors
//! - Serialization errors

use fncache::{backends::memory::MemoryBackend, fncache, init_global_cache, Result};
use std::fmt;

// Function that returns a Result
#[fncache(ttl = 60)]
fn might_fail(input: i32) -> std::result::Result<i32, String> {
    println!("Executing might_fail({})...", input);
    if input < 0 {
        Err("Input cannot be negative".to_string())
    } else {
        Ok(input * 2)
    }
}

// Function with custom error type
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CustomError(String);

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Custom error: {}", self.0)
    }
}

impl std::error::Error for CustomError {}

// This function uses a custom error type
#[fncache(ttl = 60)]
fn with_custom_error(value: u32) -> std::result::Result<u32, CustomError> {
    println!("Executing with_custom_error({})...", value);
    if value == 0 {
        Err(CustomError("Cannot process zero".to_string()))
    } else {
        Ok(value * 10)
    }
}

// Example of handling non-serializable type with a manual wrapper
#[derive(serde::Serialize, serde::Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

// This makes Point non-serializable for demonstration purposes
struct NonSerializable {
    point: Point,
    _phantom: std::marker::PhantomData<*const ()>, // Makes it non-serializable
}

impl NonSerializable {
    fn new(x: i32, y: i32) -> Self {
        Self {
            point: Point { x, y },
            _phantom: std::marker::PhantomData,
        }
    }
}

// Function that handles non-serializable input by transforming input/output
#[fncache(ttl = 60)]
fn process_point(x: i32, y: i32) -> Point {
    println!("Processing point ({}, {})...", x, y);
    // In real code, we would do something with NonSerializable here
    let non_serializable = NonSerializable::new(x, y);
    std::thread::sleep(std::time::Duration::from_millis(500));
    non_serializable.point
}

fn main() -> Result<()> {
    // Initialize cache
    init_global_cache(MemoryBackend::new())?;

    println!("Error Handling Examples");
    println!("======================");

    // Example 1: Function returning a Result
    println!("\n--- Functions returning Result ---");

    // Successful case
    match might_fail(10) {
        Ok(value) => println!("Success: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    // Second call (should be cached)
    match might_fail(10) {
        Ok(value) => println!("Cached success: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    // Error case
    match might_fail(-5) {
        Ok(value) => println!("Success: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    // Second error call (errors are not cached)
    match might_fail(-5) {
        Ok(value) => println!("Success: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    // Example 2: Custom error types
    println!("\n--- Custom error types ---");

    match with_custom_error(5) {
        Ok(value) => println!("Success: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    match with_custom_error(0) {
        Ok(value) => println!("Success: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    // Example 3: Dealing with non-serializable types
    println!("\n--- Non-serializable types ---");

    // First call
    let point1 = process_point(3, 4);
    println!("Result: Point({}, {})", point1.x, point1.y);

    // Second call (should be cached)
    let point2 = process_point(3, 4);
    println!("Cached result: Point({}, {})", point2.x, point2.y);

    Ok(())
}
