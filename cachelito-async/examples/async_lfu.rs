use cachelito_async::cache_async;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Example demonstrating the LFU (Least Frequently Used) eviction policy for async functions.
///
/// Run with: cargo run --example async_lfu

// Simulate API call counter
static API_CALLS: AtomicU32 = AtomicU32::new(0);

async fn simulate_api_call(product_id: u32) -> String {
    API_CALLS.fetch_add(1, Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(100)).await;
    format!("Product #{} details", product_id)
}

/// Cached async function with LFU eviction policy
///
/// With limit=3, only 3 products can be cached.
/// When adding a 4th product, the least frequently accessed one is evicted.
#[cache_async(limit = 3, policy = "lfu")]
async fn get_product_details(product_id: u32) -> String {
    simulate_api_call(product_id).await
}

#[tokio::main]
async fn main() {
    println!("=== Async LFU (Least Frequently Used) Cache Demo ===\n");

    API_CALLS.store(0, Ordering::SeqCst);

    println!("Phase 1: Load 3 products into cache");
    println!("  Product 1: {}", get_product_details(1).await);
    println!("  Product 2: {}", get_product_details(2).await);
    println!("  Product 3: {}", get_product_details(3).await);
    println!("  API calls: {}\n", API_CALLS.load(Ordering::SeqCst));

    println!("Phase 2: Access products with different frequencies");
    println!("  Accessing product 1 five times:");
    for i in 0..5 {
        println!("    Access {}: {}", i + 1, get_product_details(1).await);
    }
    println!("  Accessing product 2 twice:");
    for i in 0..2 {
        println!("    Access {}: {}", i + 1, get_product_details(2).await);
    }
    println!(
        "  API calls: {} (should still be 3)\n",
        API_CALLS.load(Ordering::SeqCst)
    );

    println!("Access frequency:");
    println!("  Product 1: 6 accesses");
    println!("  Product 2: 3 accesses");
    println!("  Product 3: 1 access");
    println!();

    println!("Phase 3: Add product 4 (triggers eviction)");
    println!("  Product 4: {}", get_product_details(4).await);
    println!("  API calls: {}\n", API_CALLS.load(Ordering::SeqCst));

    println!("Phase 4: Verify eviction (product 3 should be evicted)");
    println!("  Product 3: {}", get_product_details(3).await);
    let calls = API_CALLS.load(Ordering::SeqCst);
    println!("  API calls: {} (product 3 was re-fetched)\n", calls);

    println!("  Product 1: {}", get_product_details(1).await);
    println!(
        "  API calls: {} (product 1 still cached)\n",
        API_CALLS.load(Ordering::SeqCst)
    );

    println!("=== Summary ===");
    println!("LFU policy keeps frequently accessed items in cache,");
    println!("making it ideal for scenarios with 'hot' data.");
    println!("Total API calls: {}", API_CALLS.load(Ordering::SeqCst));
}
