use cachelito_async::cache_async;
use tokio::time::{sleep, Duration};

/// Demonstrates the Random eviction policy in async contexts
///
/// The Random policy evicts a randomly selected entry when the cache is full.
/// This provides:
/// - O(1) eviction performance
/// - Minimal overhead in async contexts
/// - No access pattern tracking
/// - Good for random or unpredictable access patterns

#[cache_async(policy = "random", limit = 5)]
async fn fetch_data(id: u32) -> String {
    println!("  [ASYNC] Fetching data for ID: {}", id);
    // Simulate async I/O operation
    sleep(Duration::from_millis(50)).await;
    format!("Data-{}", id)
}

#[cache_async(policy = "random", limit = 3, ttl = 200)]
async fn fetch_user(user_id: u32) -> String {
    println!("  [ASYNC] Fetching user: {}", user_id);
    sleep(Duration::from_millis(30)).await;
    format!("User-{}", user_id)
}

#[tokio::main]
async fn main() {
    println!("=== Async Random Policy Cache Demo ===\n");

    println!("Phase 1: Fill cache to limit (5 entries)");
    println!("=====================================");
    for i in 1..=5 {
        let result = fetch_data(i).await;
        println!("fetch_data({}) = {} [MISS - fetched]", i, result);
    }

    println!("\nPhase 2: Access cached values (all should be hits)");
    println!("=====================================");
    for i in 1..=5 {
        let result = fetch_data(i).await;
        println!("fetch_data({}) = {} [HIT - from cache]", i, result);
    }

    println!("\nPhase 3: Add new values (triggers random evictions)");
    println!("=====================================");
    println!("Adding IDs 6-10 will cause random evictions...\n");

    for i in 6..=10 {
        let result = fetch_data(i).await;
        println!(
            "fetch_data({}) = {} [MISS - fetched, random eviction occurred]",
            i, result
        );
    }

    println!("\nPhase 4: Concurrent async requests");
    println!("=====================================");
    println!("Spawning 10 concurrent tasks...\n");

    let mut tasks = vec![];
    for i in 0..10 {
        let task = tokio::spawn(async move {
            let result = fetch_data(i * 2).await;
            println!("  Task {} completed: {}", i, result);
            result
        });
        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        task.await.unwrap();
    }

    println!("\nPhase 5: Test with TTL (Time-To-Live)");
    println!("=====================================");

    // Fill cache
    for i in 1..=3 {
        fetch_user(i).await;
    }

    println!("Cached 3 users. Waiting for TTL expiration (200ms)...");
    sleep(Duration::from_millis(250)).await;

    println!("After TTL expiration - entries should be refetched:");
    for i in 1..=3 {
        let result = fetch_user(i).await;
        println!("fetch_user({}) = {} [MISS - expired]", i, result);
    }

    println!("\nPhase 6: Random policy in async workloads");
    println!("=====================================");
    println!("Random eviction is particularly useful for:");
    println!("  ✓ Async workloads with unpredictable access patterns");
    println!("  ✓ Baseline performance measurements");
    println!("  ✓ Scenarios where simplicity is preferred over optimization");
    println!("  ✓ Reducing lock contention (no order updates on hits)");
    println!("  ✓ Truly random access patterns where no policy provides advantage");

    #[cfg(feature = "stats")]
    {
        use cachelito_core::stats_registry;

        if let Some(stats) = stats_registry::get("fetch_data") {
            println!("\n=== Cache Statistics (fetch_data) ===");
            println!("Total accesses: {}", stats.total_accesses());
            println!("Hits: {}", stats.hits());
            println!("Misses: {}", stats.misses());
            println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }

        if let Some(stats) = stats_registry::get("fetch_user") {
            println!("\n=== Cache Statistics (fetch_user) ===");
            println!("Total accesses: {}", stats.total_accesses());
            println!("Hits: {}", stats.hits());
            println!("Misses: {}", stats.misses());
            println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }
    }

    println!("\n=== Policy Comparison ===");
    println!("FIFO   : Evicts oldest (deterministic)");
    println!("LRU    : Evicts least recently used (temporal locality)");
    println!("LFU    : Evicts least frequently used (frequency patterns)");
    println!("ARC    : Evicts based on adaptive score (hybrid)");
    println!("Random : Evicts randomly (baseline/simple)");
}
