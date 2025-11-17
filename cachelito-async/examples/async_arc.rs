//! Example demonstrating the ARC (Adaptive Replacement Cache) eviction policy
//! with async/await functions.
//!
//! ARC is a self-tuning cache algorithm that dynamically balances between:
//! - Recency (LRU-like behavior): Recently accessed items
//! - Frequency (LFU-like behavior): Frequently accessed items
//!
//! This example shows how ARC adapts to different async access patterns.

use cachelito_async::cache_async;
use tokio::time::{sleep, Duration};

/// Simulate an expensive async database query
#[cache_async(policy = "arc", limit = 10)]
async fn fetch_user(user_id: u32) -> String {
    println!("  [DB QUERY] Fetching user {}", user_id);
    sleep(Duration::from_millis(100)).await;
    format!("User#{}", user_id)
}

/// Simulate an expensive API call
#[cache_async(policy = "arc", limit = 5)]
async fn fetch_weather(city: String) -> String {
    println!("  [API CALL] Fetching weather for {}", city);
    sleep(Duration::from_millis(150)).await;
    format!("Weather in {}: Sunny, 25°C", city)
}

/// Complex async computation
#[cache_async(policy = "arc", limit = 8)]
async fn compute_prime_factors(n: u64) -> Vec<u64> {
    sleep(Duration::from_millis(20)).await;
    let mut factors = vec![];
    let mut n = n;
    let mut divisor = 2;

    while divisor * divisor <= n {
        while n % divisor == 0 {
            factors.push(divisor);
            n /= divisor;
        }
        divisor += 1;
    }

    if n > 1 {
        factors.push(n);
    }

    factors
}

#[tokio::main]
async fn main() {
    println!("=== ARC (Adaptive Replacement Cache) - Async Demo ===\n");

    // Example 1: Database queries with mixed access patterns
    println!("📊 Example 1: Database Queries with Mixed Patterns");
    println!("Cache limit: 10 entries\n");

    // Initial sequential access (tests recency)
    println!("Sequential access pattern:");
    for id in 1..=5 {
        let user = fetch_user(id).await;
        println!("  → {}", user);
    }

    println!("\n🔥 Frequent access to some users:");
    for _ in 0..3 {
        fetch_user(2).await;
        fetch_user(4).await;
    }
    println!("  Users 2 and 4 accessed 3 times each (now HOT items)");

    println!("\n📈 Adding more users (will trigger eviction):");
    for id in 6..=12 {
        let user = fetch_user(id).await;
        println!("  → {}", user);
    }

    println!("\n🔍 Checking which users survived:");
    println!("User 2 (frequently accessed):");
    fetch_user(2).await;

    println!("User 4 (frequently accessed):");
    fetch_user(4).await;

    println!("User 1 (accessed once, long ago):");
    fetch_user(1).await;

    // Example 2: Concurrent access with ARC
    println!("\n\n⚡ Example 2: Concurrent Async Operations");
    println!("Cache limit: 5 entries\n");

    println!("Spawning 10 concurrent tasks...");
    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            // Each task accesses overlapping cities
            let city = match i % 3 {
                0 => "Madrid".to_string(),
                1 => "Barcelona".to_string(),
                _ => "Valencia".to_string(),
            };
            fetch_weather(city).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        println!("  Task {} → {}", i, result);
    }

    println!("\n📊 Frequent cities should be cached:");
    println!("Madrid (frequently accessed):");
    fetch_weather("Madrid".to_string()).await;

    println!("Barcelona (frequently accessed):");
    fetch_weather("Barcelona".to_string()).await;

    // Example 3: Scan-resistant behavior
    println!("\n\n🛡️  Example 3: Scan-Resistant Behavior");
    println!("Cache limit: 8 entries\n");

    println!("Establishing hot items (prime factors of 100 and 144):");
    for _ in 0..3 {
        compute_prime_factors(100).await;
        compute_prime_factors(144).await;
    }
    println!("  prime_factors(100) and prime_factors(144) are now HOT");

    println!("\nPerforming scan (many sequential computations):");
    for n in 10..=30 {
        compute_prime_factors(n).await;
    }
    println!("  Scanned prime_factors(10) through prime_factors(30)");

    println!("\n✅ Checking if hot items survived the scan:");
    println!("prime_factors(100) (hot item):");
    let start = tokio::time::Instant::now();
    let result = compute_prime_factors(100).await;
    let elapsed = start.elapsed();
    println!(
        "  → {:?} (took {}ms - should be cached!)",
        result,
        elapsed.as_millis()
    );

    println!("prime_factors(144) (hot item):");
    let start = tokio::time::Instant::now();
    let result = compute_prime_factors(144).await;
    let elapsed = start.elapsed();
    println!(
        "  → {:?} (took {}ms - should be cached!)",
        result,
        elapsed.as_millis()
    );

    println!("prime_factors(25) (from scan, likely evicted):");
    let start = tokio::time::Instant::now();
    let result = compute_prime_factors(25).await;
    let elapsed = start.elapsed();
    println!("  → {:?} (took {}ms)", result, elapsed.as_millis());

    // Example 4: Adaptive behavior demonstration
    println!("\n\n🧠 Example 4: Adaptive Behavior");
    println!("ARC automatically balances recency and frequency\n");

    #[cache_async(policy = "arc", limit = 5)]
    async fn process_request(req_id: u32) -> String {
        sleep(Duration::from_millis(50)).await;
        format!("Request-{} processed", req_id)
    }

    println!("Creating frequency pattern:");
    println!("  Request 100: accessed 5 times (HIGH frequency)");
    for _ in 0..5 {
        process_request(100).await;
    }

    println!("  Requests 1-5: accessed once each (LOW frequency)");
    for i in 1..=5 {
        process_request(i).await;
    }

    println!("\nAdding more requests to trigger eviction:");
    for i in 6..=10 {
        process_request(i).await;
    }

    println!("\n📊 ARC Score Calculation:");
    println!("  score = frequency × recency_weight");
    println!("  Lower score → gets evicted first");
    println!();
    println!("Request 100 (high frequency, old):");
    let start = tokio::time::Instant::now();
    process_request(100).await;
    println!(
        "  Took {}ms - ARC kept it cached due to HIGH frequency!",
        start.elapsed().as_millis()
    );

    println!("\nRequest 1 (low frequency, very old):");
    let start = tokio::time::Instant::now();
    process_request(1).await;
    println!(
        "  Took {}ms - likely evicted (low score)",
        start.elapsed().as_millis()
    );

    // Summary
    println!("\n\n=== 🎯 ARC Policy Benefits ===");
    println!("✅ Automatically adapts to workload patterns");
    println!("✅ Protects frequently accessed items from eviction");
    println!("✅ Scan-resistant: sequential access won't pollute cache");
    println!("✅ O(1) underlying map access in async context (using DashMap), but ARC eviction policy adds O(n) overhead for eviction and reordering");
    println!("✅ Self-tuning: no manual configuration needed");
    println!("✅ Perfect for mixed workloads (frequency + recency)");
    println!("\n🚀 Use ARC when:");
    println!("  • Your workload has both hot items and temporal patterns");
    println!("  • You want automatic adaptation without tuning");
    println!("  • You need protection against cache pollution");
    println!("  • You have mixed async operations (DB, API, compute)");
}
