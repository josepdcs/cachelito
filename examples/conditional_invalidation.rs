//! Example demonstrating conditional cache invalidation using custom check functions.
//!
//! This example shows how to use conditional invalidation to selectively
//! invalidate cache entries based on runtime conditions rather than just tags or events.
//!
//! # Key Features
//!
//! - Invalidate entries by key pattern
//! - Invalidate entries by key value range
//! - Global conditional invalidation across all caches
//! - Named invalidation check functions as macro attributes

use cachelito::cache;
use cachelito_core::{invalidate_all_with, invalidate_with};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
    updated_at: u64, // Unix timestamp
}

/// Fetch user from database (simulated)
#[cache(scope = "global", name = "get_user", limit = 100, policy = "lru")]
fn get_user(user_id: u64) -> User {
    println!("Fetching user {} from database...", user_id);
    thread::sleep(Duration::from_millis(100)); // Simulate DB latency

    User {
        id: user_id,
        name: format!("User {}", user_id),
        updated_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

/// Fetch product from database (simulated)
#[cache(scope = "global", name = "get_product", limit = 50, policy = "lru")]
fn get_product(product_id: u64) -> String {
    println!("Fetching product {} from database...", product_id);
    thread::sleep(Duration::from_millis(50));
    format!("Product {}", product_id)
}

fn main() {
    println!("=== Conditional Cache Invalidation Example ===\n");

    // ===================================================================
    // Example 1: Invalidate entries by key value range
    // ===================================================================
    println!("Example 1: Invalidate users with ID > 1000\n");

    // Populate cache
    for id in [100, 500, 1001, 1500, 2000].iter() {
        let user = get_user(*id);
        println!("Cached: {:?}", user);
    }

    println!("\nAccessing cached users (should hit cache):");
    let _ = get_user(100);
    let _ = get_user(500);
    let _ = get_user(1001);

    println!("\nInvalidating all users with ID > 1000...");
    invalidate_with("get_user", |key: &str| {
        key.parse::<u64>().unwrap_or(0) > 1000
    });

    println!("\nAccessing users again:");
    println!("User 100 (should hit cache):");
    let _ = get_user(100);

    println!("User 1001 (should re-fetch from DB):");
    let _ = get_user(1001);

    println!("User 2000 (should re-fetch from DB):");
    let _ = get_user(2000);

    // ===================================================================
    // Example 2: Invalidate entries by key pattern
    // ===================================================================
    println!("\n\n=== Example 2: Invalidate by key pattern ===\n");

    // Populate product cache
    for id in [1, 10, 100, 200, 300].iter() {
        let product = get_product(*id);
        println!("Cached: {}", product);
    }

    println!("\nInvalidating all products with 3-digit IDs (100-999)...");
    invalidate_with("get_product", |key: &str| {
        if let Ok(id) = key.parse::<u64>() {
            id >= 100 && id < 1000
        } else {
            false
        }
    });

    println!("\nAccessing products again:");
    println!("Product 1 (should hit cache):");
    let _ = get_product(1);

    println!("Product 100 (should re-fetch from DB):");
    let _ = get_product(100);

    // ===================================================================
    // Example 3: Global invalidation across all caches
    // ===================================================================
    println!("\n\n=== Example 3: Global conditional invalidation ===\n");

    // Re-populate both caches
    for id in [1, 2, 3, 4, 5].iter() {
        let _ = get_user(*id);
        let _ = get_product(*id);
    }

    println!("\nInvalidating all entries with key '3' or greater across ALL caches...");
    let count =
        invalidate_all_with(|_cache_name: &str, key: &str| key.parse::<u64>().unwrap_or(0) >= 3);
    println!("Applied check function to {} caches", count);

    println!("\nAccessing entries again:");
    println!("User 2 (should hit cache):");
    let _ = get_user(2);

    println!("User 3 (should re-fetch from DB):");
    let _ = get_user(3);

    println!("Product 2 (should hit cache):");
    let _ = get_product(2);

    println!("Product 4 (should re-fetch from DB):");
    let _ = get_product(4);

    // ===================================================================
    // Example 4: Complex conditional check conditions
    // ===================================================================
    println!("\n\n=== Example 4: Complex conditional checks ===\n");

    // Populate cache with specific IDs
    let ids = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for id in &ids {
        let _ = get_user(*id);
    }

    println!("Invalidating users with ID divisible by 30...");
    invalidate_with("get_user", |key: &str| {
        if let Ok(id) = key.parse::<u64>() {
            id % 30 == 0
        } else {
            false
        }
    });

    println!("\nAccessing users:");
    for id in &[30, 60, 90, 20, 40] {
        println!("User {} (divisible by 30: {}):", id, id % 30 == 0);
        let _ = get_user(*id);
    }

    println!("\n=== Example Complete ===");
}
