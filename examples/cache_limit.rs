//! # Cache Limit Example
//!
//! This example demonstrates using cache limits to control memory usage.
//! When the cache reaches its limit, older entries are evicted according
//! to the specified policy (FIFO or LRU).
use cachelito::cache;
use std::cell::RefCell;
// Counter to verify how many times the function executes
thread_local! {
    static EXEC_COUNT: RefCell<usize> = RefCell::new(0);
}
/// Simulates a slow addition operation with a cache limit of 2 entries.
///
/// Uses LRU (Least Recently Used) eviction policy.
//#[cfg(feature = "stats")]
#[cache(limit = 2, policy = "lru")]
fn slow_add(a: u32, b: u32) -> u32 {
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() += 1;
    });
    println!("Computing {} + {}", a, b);
    a + b
}

//#[cfg(feature = "stats")]
fn main() {
    println!("=== Cache Limit Example (LRU) ===\n");
    // Reset counter
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() = 0;
    });
    println!("Cache limit: 2 entries");
    println!("Policy: LRU (Least Recently Used)\n");
    println!("--- Adding entries to cache ---");
    // Call 1: miss -> cache: [(1,1)]
    println!("\nCall 1: slow_add(1, 1)");
    let result = slow_add(1, 1);
    println!("Result: {}", result);
    // Call 2: hit -> cache: [(1,1)]
    println!("\nCall 2: slow_add(1, 1) - should be cached");
    let result = slow_add(1, 1);
    println!("Result: {}", result);
    // Call 3: miss -> cache: [(1,1), (1,2)]
    println!("\nCall 3: slow_add(1, 2)");
    let result = slow_add(1, 2);
    println!("Result: {}", result);
    // Call 4: hit -> cache: [(1,1), (1,2)]
    println!("\nCall 4: slow_add(1, 2) - should be cached");
    let result = slow_add(1, 2);
    println!("Result: {}", result);
    // Call 5: hit -> cache: [(1,2), (1,1)] (LRU moves (1,1) to end)
    println!("\nCall 5: slow_add(1, 1) - should be cached");
    let result = slow_add(1, 1);
    println!("Result: {}", result);
    // Call 6: miss -> cache: [(1,1), (2,2)] (evicts (1,2) as it's least recent)
    println!("\nCall 6: slow_add(2, 2) - cache full, will evict least recent");
    let result = slow_add(2, 2);
    println!("Result: {}", result);
    // Call 7: miss -> (1,2) was evicted
    println!("\nCall 7: slow_add(1, 2) - was evicted, will recompute");
    let result = slow_add(1, 2);
    println!("Result: {}", result);
    // Verify execution count
    let exec_count = EXEC_COUNT.with(|count| *count.borrow());
    println!("\n--- Results ---");
    println!("Total executions: {}", exec_count);
    println!("Expected: 4 executions");
    println!("  - Call 1: slow_add(1,1) - miss");
    println!("  - Call 3: slow_add(1,2) - miss");
    println!("  - Call 6: slow_add(2,2) - miss");
    println!("  - Call 7: slow_add(1,2) - miss (was evicted)");
    assert_eq!(
        exec_count, 4,
        "Expected 4 function executions but got {}",
        exec_count
    );
    println!("\n✅ Cache Limit Test PASSED");
    println!("   Cache limit successfully controls memory usage.");
    println!("   LRU policy evicts least recently used entries.");
}
