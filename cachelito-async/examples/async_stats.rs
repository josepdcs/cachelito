//! # Async Cache Statistics Example
//!
//! This example demonstrates how to use cache statistics with async functions.

use cachelito_async::{cache_async, stats_registry};
use std::time::Duration;

#[cache_async]
async fn compute(x: u32) -> u32 {
    println!("Computing {}...", x);
    tokio::time::sleep(Duration::from_millis(100)).await;
    x * x
}

#[cache_async(name = "my_async_cache")]
async fn custom_compute(x: u32) -> u32 {
    println!("Custom computing {}...", x);
    tokio::time::sleep(Duration::from_millis(50)).await;
    x + 10
}

#[tokio::main]
async fn main() {
    println!("=== Async Cache Statistics Example ===\n");

    // Test default function name
    println!("--- Testing default cache (function name) ---");
    compute(1).await;
    compute(2).await;
    compute(1).await; // cache hit
    compute(3).await;
    compute(2).await; // cache hit
    compute(1).await; // cache hit

    // Get statistics using function name
    if let Some(stats) = stats_registry::get("compute") {
        println!("\nStatistics for 'compute':");
        println!("  Hits:            {}", stats.hits());
        println!("  Misses:          {}", stats.misses());
        println!("  Total accesses:  {}", stats.total_accesses());
        println!("  Hit rate:        {:.2}%", stats.hit_rate() * 100.0);
        println!("  Miss rate:       {:.2}%", stats.miss_rate() * 100.0);
    }

    println!("\n--- Testing custom cache name ---");
    custom_compute(5).await;
    custom_compute(6).await;
    custom_compute(5).await; // cache hit
    custom_compute(5).await; // cache hit

    // Get statistics using custom name
    if let Some(stats) = stats_registry::get("my_async_cache") {
        println!("\nStatistics for 'my_async_cache':");
        println!("  Hits:            {}", stats.hits());
        println!("  Misses:          {}", stats.misses());
        println!("  Total accesses:  {}", stats.total_accesses());
        println!("  Hit rate:        {:.2}%", stats.hit_rate() * 100.0);
        println!("  Miss rate:       {:.2}%", stats.miss_rate() * 100.0);
    }

    // List all registered caches
    println!("\n--- All registered async caches ---");
    let all_caches = stats_registry::list();
    for cache_name in all_caches {
        if let Some(stats) = stats_registry::get(&cache_name) {
            println!(
                "  {}: {} hits, {} misses",
                cache_name,
                stats.hits(),
                stats.misses()
            );
        }
    }

    // Verify expectations
    let compute_stats = stats_registry::get("compute").unwrap();
    assert_eq!(
        compute_stats.hits(),
        3,
        "Expected 3 cache hits for 'compute'"
    );
    assert_eq!(
        compute_stats.misses(),
        3,
        "Expected 3 cache misses for 'compute'"
    );

    let custom_stats = stats_registry::get("my_async_cache").unwrap();
    assert_eq!(
        custom_stats.hits(),
        2,
        "Expected 2 cache hits for 'my_async_cache'"
    );
    assert_eq!(
        custom_stats.misses(),
        2,
        "Expected 2 cache misses for 'my_async_cache'"
    );

    println!("\n✅ Async Cache Statistics Test PASSED");
}
