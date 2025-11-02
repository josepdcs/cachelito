//! # Result Type Caching Example
//!
//! This example demonstrates caching with `Result` types. Only successful
//! (`Ok`) results are cached - errors are not cached and will be recomputed
//! on each call.

use cachelito::cache;

/// Performs a risky operation that may fail for odd numbers.
///
/// This function demonstrates caching with `Result` types. Only successful
/// (`Ok`) results are cached - errors are not cached and will be recomputed
/// on each call.
///
/// # Arguments
///
/// * `x` - The input number
///
/// # Returns
///
/// * `Ok(x * 2)` if x is even (this result is cached)
/// * `Err(message)` if x is odd (this error is NOT cached)
#[cache]
fn risky_operation(x: u32) -> Result<u32, String> {
    println!("Running risky operation for {}", x);
    if x.is_multiple_of(2) {
        Ok(x * 2)
    } else {
        Err(format!("Odd number: {}", x))
    }
}

fn main() {
    println!("=== Result Type Caching Example ===\n");

    println!("--- Testing Ok Values (should be cached) ---");

    // First call with even number: computes, caches, returns Ok
    println!("First call with 2:");
    let result1 = risky_operation(2);
    println!("Result: {:?}\n", result1);

    // Second call with same even number: returns cached Ok (no "Running" message)
    println!("Second call with 2 (should be cached):");
    let result2 = risky_operation(2);
    println!("Result: {:?}\n", result2);

    assert_eq!(result1, Ok(4));
    assert_eq!(result2, Ok(4));

    println!("--- Testing Err Values (should NOT be cached) ---");

    // First call with odd number: computes and returns Err (NOT cached)
    println!("First call with 3:");
    let result3 = risky_operation(3);
    println!("Result: {:?}\n", result3);

    // Second call with same odd number: computes again (notice "Running" message)
    println!("Second call with 3 (should NOT be cached):");
    let result4 = risky_operation(3);
    println!("Result: {:?}\n", result4);

    assert!(result3.is_err());
    assert!(result4.is_err());

    println!("--- Testing Multiple Even Numbers ---");

    println!("First call with 4:");
    let result5 = risky_operation(4);
    println!("Result: {:?}\n", result5);

    println!("First call with 6:");
    let result6 = risky_operation(6);
    println!("Result: {:?}\n", result6);

    println!("Second call with 4 (should be cached):");
    let result7 = risky_operation(4);
    println!("Result: {:?}\n", result7);

    assert_eq!(result5, Ok(8));
    assert_eq!(result6, Ok(12));
    assert_eq!(result7, Ok(8));

    println!("✅ Result Type Caching Test PASSED");
    println!("   Only Ok values are cached, Err values are recomputed each time.");
}
