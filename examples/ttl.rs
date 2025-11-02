//! # TTL (Time To Live) Caching Example
//!
//! This example demonstrates the TTL (Time To Live) feature of the cache.
//! Entries expire after a specified number of seconds and are automatically
//! removed from the cache.

use cachelito::cache;
use std::cell::RefCell;
use std::thread;
use std::time::Duration;

// Counter to verify how many times the function executes
thread_local! {
    static EXEC_COUNT: RefCell<usize> = RefCell::new(0);
}

/// Simulates fetching user data with a TTL of 2 seconds.
/// After 2 seconds, the cached value expires and the function executes again.
#[cache(ttl = 2)]
fn get_user_data(user_id: u32) -> String {
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() += 1;
    });
    println!("Fetching user data for user_id: {}", user_id);
    format!("User data for ID: {}", user_id)
}

/// Simulates an expensive computation with TTL and LRU policy.
#[cache(limit = 3, policy = "lru", ttl = 3)]
fn expensive_computation(x: i32, y: i32) -> i32 {
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() += 1;
    });
    println!("Computing {} + {}", x, y);
    thread::sleep(Duration::from_millis(100)); // Simulate expensive work
    x + y
}

fn main() {
    println!("=== TTL (Time To Live) Caching Example ===\n");

    // Reset counter
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() = 0;
    });

    println!("--- Test 1: Basic TTL Functionality ---");
    println!("TTL set to 2 seconds\n");

    // Call 1: Cache miss, function executes
    println!("Call 1: get_user_data(1)");
    let result1 = get_user_data(1);
    println!("Result: {}\n", result1);
    assert_eq!(result1, "User data for ID: 1");

    // Call 2: Cache hit (within TTL), function does NOT execute
    println!("Call 2: get_user_data(1) - immediately after (should be cached)");
    let result2 = get_user_data(1);
    println!("Result: {}\n", result2);
    assert_eq!(result2, "User data for ID: 1");

    // Verify only 1 execution so far
    let count1 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count1, 1, "Expected 1 execution but got {}", count1);
    println!("✓ Verified: Only 1 execution so far (cache hit)\n");

    // Wait 1 second (still within TTL)
    println!("Waiting 1 second (still within 2s TTL)...");
    thread::sleep(Duration::from_secs(1));

    // Call 3: Cache hit (still within TTL)
    println!("Call 3: get_user_data(1) - after 1 second (should still be cached)");
    let result3 = get_user_data(1);
    println!("Result: {}\n", result3);
    assert_eq!(result3, "User data for ID: 1");

    let count2 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count2, 1, "Expected 1 execution but got {}", count2);
    println!("✓ Verified: Still only 1 execution (cache hit)\n");

    // Wait another 1.5 seconds (total 2.5 seconds, exceeds TTL)
    println!("Waiting another 1.5 seconds (total 2.5s, exceeds 2s TTL)...");
    thread::sleep(Duration::from_millis(1500));

    // Call 4: Cache miss (expired), function executes again
    println!("Call 4: get_user_data(1) - after 2.5 seconds (should be expired)");
    let result4 = get_user_data(1);
    println!("Result: {}\n", result4);
    assert_eq!(result4, "User data for ID: 1");

    let count3 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count3, 2, "Expected 2 executions but got {}", count3);
    println!("✓ Verified: 2 executions (cache expired and refreshed)\n");

    // Reset counter for next test
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() = 0;
    });

    println!("\n--- Test 2: TTL with LRU Policy and Limits ---");
    println!("TTL: 3 seconds, Limit: 3 entries, Policy: LRU\n");

    // Add 3 entries
    println!("Adding 3 entries to cache:");
    expensive_computation(1, 1); // Entry 1
    expensive_computation(2, 2); // Entry 2
    expensive_computation(3, 3); // Entry 3

    let count4 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count4, 3);
    println!("\n✓ 3 entries added\n");

    // Access first entry (cache hit)
    println!("Accessing (1,1) again - should be cached:");
    expensive_computation(1, 1);

    let count5 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count5, 3, "Should still be 3 executions (cache hit)");
    println!("✓ Cache hit confirmed\n");

    // Wait 2 seconds (still within TTL)
    println!("Waiting 2 seconds...");
    thread::sleep(Duration::from_secs(2));

    // Access entries (should all still be valid)
    println!("Accessing all entries - should all be cached:");
    expensive_computation(1, 1);
    expensive_computation(2, 2);
    expensive_computation(3, 3);

    let count6 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count6, 3, "Should still be 3 executions (all cache hits)");
    println!("✓ All entries still valid within TTL\n");

    // Wait another 2 seconds (total 4 seconds, exceeds 3s TTL)
    println!("Waiting another 2 seconds (total 4s, exceeds 3s TTL)...");
    thread::sleep(Duration::from_secs(2));

    // Access entries (should all be expired)
    println!("Accessing entries after TTL expiration:");
    expensive_computation(1, 1);
    expensive_computation(2, 2);
    expensive_computation(3, 3);

    let count7 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count7, 6, "Should be 6 executions (all entries expired)");
    println!("✓ All entries expired and recomputed\n");

    println!("=== Test 3: Different keys with TTL ===\n");

    // Reset counter
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() = 0;
    });

    println!("Testing different user IDs:");
    get_user_data(10);
    get_user_data(20);
    get_user_data(30);

    let count8 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count8, 3, "Should execute 3 times for different keys");
    println!("✓ 3 different keys, 3 executions\n");

    println!("Accessing same keys again (within TTL):");
    get_user_data(10);
    get_user_data(20);
    get_user_data(30);

    let count9 = EXEC_COUNT.with(|count| *count.borrow());
    assert_eq!(count9, 3, "Should still be 3 executions (all cache hits)");
    println!("✓ All cache hits\n");

    println!("\n✅ ALL TTL TESTS PASSED!");
    println!("\nSummary:");
    println!("  • TTL expiration works correctly");
    println!("  • Expired entries are removed from cache");
    println!("  • TTL works with LRU policy and cache limits");
    println!("  • Multiple keys can have independent TTL timers");
}
