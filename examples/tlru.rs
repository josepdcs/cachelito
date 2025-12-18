//! Example demonstrating TLRU (Time-aware Least Recently Used) eviction policy.
//!
//! TLRU combines three factors to make eviction decisions:
//! - **Recency**: How recently the entry was accessed
//! - **Frequency**: How often the entry has been accessed
//! - **Age**: How close the entry is to its TTL expiration
//!
//! This makes TLRU ideal for caches with time-sensitive data and varying access patterns.
//!
//! ## Async Version
//! For async/await examples with TLRU, see `cachelito-async/examples/async_tlru.rs`

use cachelito::cache;
use std::thread;
use std::time::{Duration, Instant};

/// Cache with TLRU policy and 2-second TTL
#[cache(policy = "tlru", limit = 5, ttl = 2)]
fn fetch_weather_data(city: String) -> String {
    println!("🌤️  Fetching weather data for {}", city);
    format!("Weather data for {}", city)
}

/// Cache with TLRU policy without TTL (behaves like ARC)
#[cache(policy = "tlru", limit = 3)]
fn compute_expensive(n: u64) -> u64 {
    println!("💻 Computing expensive operation for {}", n);
    thread::sleep(Duration::from_millis(100)); // Simulate expensive operation
    n * n
}

/// Global cache with TLRU and short TTL
#[cache(policy = "tlru", limit = 4, ttl = 1, scope = "global")]
fn get_stock_price(symbol: String) -> f64 {
    println!("📈 Fetching stock price for {}", symbol);
    thread::sleep(Duration::from_millis(50));
    100.0 + (symbol.len() as f64 * 5.0) // Mock calculation
}

/// Time-sensitive data cache
#[cache(policy = "tlru", limit = 2, ttl = 2)]
fn time_sensitive_data(id: u32) -> String {
    println!("⏱️  Fetching time-sensitive data for {}", id);
    format!("Data-{}", id)
}

fn main() {
    println!("=== TLRU Policy Demo ===\n");

    // Example 1: Basic TLRU with TTL
    println!("--- Example 1: Weather Cache with TTL ---");
    let cities = vec!["Madrid", "Barcelona", "Valencia", "Seville", "Bilbao"];

    // Fill the cache
    for city in &cities {
        fetch_weather_data(city.to_string());
    }

    println!("\n✅ Cache filled with {} cities", cities.len());

    // Access some cities to increase their frequency
    println!("\n📊 Increasing frequency for Madrid and Barcelona...");
    for _ in 0..3 {
        fetch_weather_data("Madrid".to_string());
        fetch_weather_data("Barcelona".to_string());
    }

    // Wait a bit to make some entries older
    thread::sleep(Duration::from_millis(500));

    // Add new city - should evict based on TLRU score (low frequency + older)
    println!("\n➕ Adding new city (Malaga)...");
    fetch_weather_data("Malaga".to_string());

    // High frequency cities should still be cached
    println!("\n✓ Checking if high-frequency cities are still cached:");
    fetch_weather_data("Madrid".to_string()); // Should be cached (no fetch)
    fetch_weather_data("Barcelona".to_string()); // Should be cached (no fetch)

    // Example 2: TTL expiration
    println!("\n\n--- Example 2: TTL Expiration ---");
    fetch_weather_data("Zaragoza".to_string());
    println!("✓ Cached Zaragoza weather");

    println!("\n⏳ Waiting 3 seconds for TTL to expire...");
    thread::sleep(Duration::from_secs(3));

    println!("🔄 Accessing expired entry:");
    fetch_weather_data("Zaragoza".to_string()); // Should refetch (expired)

    // Example 3: TLRU without TTL (behaves like ARC)
    println!("\n\n--- Example 3: TLRU without TTL (ARC-like) ---");

    // Fill cache
    compute_expensive(1);
    compute_expensive(2);
    compute_expensive(3);

    println!("\n📈 Increasing frequency for operation 1...");
    for _ in 0..5 {
        compute_expensive(1); // High frequency
    }

    println!("\n➕ Adding new operations...");
    compute_expensive(4); // Should evict based on frequency (3 has lowest)

    println!("\n✓ High-frequency operation should be cached:");
    compute_expensive(1); // Should be cached (no computation)

    // Example 4: Global cache with concurrent access
    println!("\n\n--- Example 4: Global TLRU Cache ---");

    let stocks = vec!["AAPL", "GOOGL", "MSFT", "AMZN"];

    println!("📊 Fetching stock prices...");
    for stock in &stocks {
        get_stock_price(stock.to_string());
    }

    // Simulate multiple threads accessing the cache
    let handles: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(i * 100));
                get_stock_price("AAPL".to_string()); // Increase frequency
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n✓ AAPL accessed from multiple threads (high frequency)");

    // Wait for some entries to age
    thread::sleep(Duration::from_millis(600));

    // Add new stock
    println!("\n➕ Adding new stock...");
    get_stock_price("TSLA".to_string());

    println!("\n✓ High-frequency stock should still be cached:");
    get_stock_price("AAPL".to_string()); // Should be cached

    // Example 5: Demonstrating age factor
    println!("\n\n--- Example 5: Age Factor in Action ---");

    // Add first entry
    time_sensitive_data(1);

    // Wait to make it older
    thread::sleep(Duration::from_secs(1));

    // Add second entry (fresher)
    time_sensitive_data(2);

    // Increase frequency of both
    time_sensitive_data(1);
    time_sensitive_data(2);
    time_sensitive_data(2); // 2 has higher frequency now

    // Add third entry
    println!("\n➕ Adding third entry with cache full...");
    time_sensitive_data(3);

    // Entry 1 is older (closer to TTL expiration), so it should be evicted
    // even if it has some frequency
    println!("\n✓ Newer entry should be cached:");
    time_sensitive_data(2); // Should be cached
    time_sensitive_data(3); // Should be cached

    // Performance comparison
    println!("\n\n--- Performance Metrics ---");
    let start = Instant::now();
    for _ in 0..100 {
        compute_expensive(1); // All cached
    }
    let cached_time = start.elapsed();

    println!("✓ 100 cached lookups: {:?}", cached_time);
    println!("\n🎯 TLRU provides optimal balance between:");
    println!("   - Temporal locality (LRU behavior)");
    println!("   - Access frequency (LFU behavior)");
    println!("   - Time-based relevance (age factor)");
}
