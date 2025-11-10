//! # Async LRU Cache Example
//!
//! This example demonstrates LRU (Least Recently Used) eviction policy
//! with async functions. When the cache limit is reached, the least
//! recently used entry is evicted.

use cachelito_async::cache_async;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Async computation with LRU cache limit
#[cache_async(limit = 2, policy = "lru")]
async fn compute(x: u32) -> u32 {
    EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
    println!("Computing {}^2 (async)", x);
    tokio::time::sleep(Duration::from_millis(50)).await;
    x * x
}

#[tokio::main]
async fn main() {
    println!("=== Async LRU Cache Example ===\n");
    println!("Cache limit: 2 entries");
    println!("Policy: LRU (Least Recently Used)\n");

    // Call 1: miss -> cache: [1]
    println!("Call 1: compute(1)");
    let result = compute(1).await;
    println!("Result: {} (cache: [1])\n", result);

    // Call 2: miss -> cache: [1, 2]
    println!("Call 2: compute(2)");
    let result = compute(2).await;
    println!("Result: {} (cache: [1, 2])\n", result);

    // Call 3: hit -> cache: [2, 1] (1 becomes most recent)
    println!("Call 3: compute(1) - should be cached");
    let result = compute(1).await;
    println!("Result: {} (cache: [2, 1])\n", result);

    // Call 4: miss -> cache: [1, 3] (evicts 2, least recent)
    println!("Call 4: compute(3) - cache full, will evict 2 (least recent)");
    let result = compute(3).await;
    println!("Result: {} (cache: [1, 3])\n", result);

    // Call 5: miss -> 2 was evicted
    println!("Call 5: compute(2) - was evicted, will recompute");
    let result = compute(2).await;
    println!("Result: {} (cache: [3, 2])\n", result);

    let exec_count = EXEC_COUNT.load(Ordering::SeqCst);
    println!("--- Results ---");
    println!("Total executions: {}", exec_count);
    println!("Expected: 4 executions");
    println!("  - Call 1: compute(1) - miss");
    println!("  - Call 2: compute(2) - miss");
    println!("  - Call 4: compute(3) - miss");
    println!("  - Call 5: compute(2) - miss (was evicted)");

    assert_eq!(
        exec_count, 4,
        "Expected 4 function executions but got {}",
        exec_count
    );

    println!("\n✅ Async LRU Cache Test PASSED");
}
