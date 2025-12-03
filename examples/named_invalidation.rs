//! Example demonstrating named invalidation check functions for cache validation.
//!
//! This example shows how to use invalidation check functions to validate cached entries
//! on every access. The check function receives the key and value, and returns true
//! if the entry should be considered stale and removed.

use cachelito::cache;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
    updated_at: Instant,
}

// Invalidation check function that checks if a user is stale (older than 5 seconds)
fn is_user_stale(_key: &String, value: &User) -> bool {
    value.updated_at.elapsed() > Duration::from_secs(5)
}

/// Fetch user with stale check - entries older than 5 seconds are auto-invalidated
#[cache(
    scope = "global",
    name = "get_user_with_check",
    limit = 100,
    policy = "lru",
    invalidate_on = is_user_stale
)]
fn get_user(user_id: u64) -> User {
    println!("Fetching user {} from database...", user_id);
    thread::sleep(Duration::from_millis(50)); // Simulate DB latency

    User {
        id: user_id,
        name: format!("User {}", user_id),
        updated_at: Instant::now(),
    }
}

// Another check function that always considers entries fresh
fn never_stale(_key: &String, _value: &String) -> bool {
    false
}

#[cache(
    scope = "global",
    name = "get_product",
    limit = 50,
    invalidate_on = never_stale
)]
fn get_product(product_id: u64) -> String {
    println!("Fetching product {} from database...", product_id);
    thread::sleep(Duration::from_millis(30));
    format!("Product {}", product_id)
}

fn is_admin_key(key: &String, _value: &String) -> bool {
    // Invalidate if key contains admin (for security)
    // Note: keys are stored with quotes, so we use contains
    key.contains("admin")
}

#[cache(
    scope = "global",
    name = "get_data",
    limit = 100,
    invalidate_on = is_admin_key
)]
fn get_data(key: String) -> String {
    println!("Fetching data for key '{}'...", key);
    format!("Data for {}", key)
}

fn main() {
    println!("=== Named Invalidation Check Functions Example ===\n");

    // ===================================================================
    // Example 1: Time-based staleness check
    // ===================================================================
    println!("Example 1: Auto-invalidation based on age\n");

    println!("First access - cache miss:");
    let user1 = get_user(1);
    println!("Got user: {:?}\n", user1);

    println!("Immediate second access - cache hit:");
    let user1_again = get_user(1);
    println!("Got user: {:?}\n", user1_again);

    println!("Waiting 6 seconds for entry to become stale...");
    thread::sleep(Duration::from_secs(6));

    println!("Access after 6 seconds - check function detects staleness, re-fetches:");
    let user1_stale = get_user(1);
    println!("Got user: {:?}\n", user1_stale);

    // ===================================================================
    // Example 2: Always fresh check function
    // ===================================================================
    println!("\n=== Example 2: Check function that never invalidates ===\n");

    println!("First access:");
    let product = get_product(100);
    println!("Got: {}\n", product);

    println!("Second access (will always hit cache):");
    let product_again = get_product(100);
    println!("Got: {}\n", product_again);

    // ===================================================================
    // Example 3: Key-based invalidation
    // ===================================================================
    println!("\n=== Example 3: Invalidation based on key pattern ===\n");

    println!("Accessing normal key:");
    let data1 = get_data("user_123".to_string());
    println!("Got: {}\n", data1);

    println!("Second access to normal key (cache hit):");
    let data1_again = get_data("user_123".to_string());
    println!("Got: {}\n", data1_again);

    println!("Accessing admin key (check function invalidates immediately):");
    let data2 = get_data("admin_456".to_string());
    println!("Got: {}\n", data2);

    println!("Second access to admin key (check function invalidates again):");
    let data2_again = get_data("admin_456".to_string());
    println!("Got: {}\n", data2_again);

    println!("\n=== Example Complete ===");
    println!("\nKey takeaways:");
    println!("1. Check functions run on EVERY cache access");
    println!("2. Return true to invalidate (remove) the entry");
    println!("3. Return false to keep the entry (cache hit)");
    println!("4. Check functions receive both key and value");
    println!("5. Useful for time-based, pattern-based, or custom validation");
}
