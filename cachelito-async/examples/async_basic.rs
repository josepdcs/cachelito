//! # Basic Async Cache Example
//!
//! This example demonstrates basic async function caching.
//! The first call executes the function, subsequent calls return cached results.

use cachelito_async::cache_async;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Simple async addition with caching
#[cache_async]
async fn slow_add(a: u32, b: u32) -> u32 {
    EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
    println!("Computing {} + {} (async)", a, b);
    tokio::time::sleep(Duration::from_millis(100)).await;
    a + b
}

#[tokio::main]
async fn main() {
    println!("=== Basic Async Cache Example ===\n");

    // Call 1: cache miss
    let start1 = Instant::now();
    let result1 = slow_add(1, 1).await;
    let elapsed1 = start1.elapsed();
    println!(
        "Call 1: slow_add(1, 1) -> {} (took {:?})",
        result1, elapsed1
    );

    // Call 2: cache hit (same args)
    let start2 = Instant::now();
    let result2 = slow_add(1, 1).await;
    let elapsed2 = start2.elapsed();
    println!(
        "Call 2: slow_add(1, 1) -> {} (took {:?}) [should be instant]",
        result2, elapsed2
    );

    // Call 3: cache miss (different args)
    let start3 = Instant::now();
    let result3 = slow_add(2, 3).await;
    let elapsed3 = start3.elapsed();
    println!(
        "Call 3: slow_add(2, 3) -> {} (took {:?})",
        result3, elapsed3
    );

    // Call 4: cache hit (same args as call 3)
    let start4 = Instant::now();
    let result4 = slow_add(2, 3).await;
    let elapsed4 = start4.elapsed();
    println!(
        "Call 4: slow_add(2, 3) -> {} (took {:?}) [should be instant]",
        result4, elapsed4
    );

    let exec_count = EXEC_COUNT.load(Ordering::SeqCst);
    println!("\n--- Results ---");
    println!("Total executions: {}", exec_count);
    println!("Expected: 2 executions (cached calls returned instantly)");

    assert_eq!(
        exec_count, 2,
        "Expected 2 function executions but got {}",
        exec_count
    );

    println!("\n✅ Basic Async Cache Test PASSED");
    println!("   Function executed {} times instead of 4", exec_count);
    println!(
        "   Cache successfully prevented {} redundant executions",
        4 - exec_count
    );
}
