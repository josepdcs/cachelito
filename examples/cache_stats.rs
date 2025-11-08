use cachelito::cache;

/// Example demonstrating cache statistics tracking.
///
/// This example shows how to use the `stats` feature to monitor
/// cache hit/miss rates and performance metrics.

#[cache(limit = 5, policy = "lru")]
fn expensive_computation(n: u32) -> u32 {
    println!("Computing for n = {}", n);
    // Simulate expensive work
    std::thread::sleep(std::time::Duration::from_millis(100));
    n * n
}

#[cache(scope = "global", limit = 10, policy = "fifo")]
fn shared_computation(x: i32, y: i32) -> i32 {
    println!("Computing {} + {}", x, y);
    std::thread::sleep(std::time::Duration::from_millis(50));
    x + y
}

fn main() {
    println!("=== Cache Statistics Example ===\n");

    // Example 1: Thread-local cache statistics
    println!("--- Thread-Local Cache ---");

    // First calls - cache misses
    println!("\nFirst set of calls (cache misses):");
    expensive_computation(5);
    expensive_computation(10);
    expensive_computation(15);

    // Repeated calls - cache hits
    println!("\nRepeated calls (cache hits):");
    expensive_computation(5);
    expensive_computation(10);
    expensive_computation(15);

    // New values - cache misses
    println!("\nNew values (cache misses):");
    expensive_computation(20);
    expensive_computation(25);

    // Get statistics
    #[cfg(feature = "stats")]
    {
        let stats = expensive_computation_stats();
        println!("\n📊 Thread-Local Cache Statistics:");
        println!("  Total accesses: {}", stats.total_accesses());
        println!("  Hits:           {}", stats.hits());
        println!("  Misses:         {}", stats.misses());
        println!("  Hit rate:       {:.2}%", stats.hit_rate() * 100.0);
        println!("  Miss rate:      {:.2}%", stats.miss_rate() * 100.0);
    }

    // Example 2: Global cache statistics
    println!("\n--- Global Cache ---");

    println!("\nFirst calls (cache misses):");
    shared_computation(1, 2);
    shared_computation(3, 4);
    shared_computation(5, 6);

    println!("\nRepeated calls (cache hits):");
    shared_computation(1, 2);
    shared_computation(3, 4);
    shared_computation(5, 6);

    println!("\nMixed calls:");
    shared_computation(1, 2); // Hit
    shared_computation(7, 8); // Miss
    shared_computation(3, 4); // Hit

    #[cfg(feature = "stats")]
    {
        let stats = shared_computation_stats();
        println!("\n📊 Global Cache Statistics:");
        println!("  Total accesses: {}", stats.total_accesses());
        println!("  Hits:           {}", stats.hits());
        println!("  Misses:         {}", stats.misses());
        println!("  Hit rate:       {:.2}%", stats.hit_rate() * 100.0);
        println!("  Miss rate:      {:.2}%", stats.miss_rate() * 100.0);
    }

    // Example 3: Reset statistics
    #[cfg(feature = "stats")]
    {
        println!("\n--- Reset Statistics ---");

        let stats_before = expensive_computation_stats();
        println!("\nBefore reset:");
        println!(
            "  Hits: {}, Misses: {}",
            stats_before.hits(),
            stats_before.misses()
        );

        stats_before.reset();

        let stats_after = expensive_computation_stats();
        println!("\nAfter reset:");
        println!(
            "  Hits: {}, Misses: {}",
            stats_after.hits(),
            stats_after.misses()
        );

        // Make some new calls after reset
        println!("\nNew calls after reset:");
        expensive_computation(5); // Hit
        expensive_computation(30); // Miss

        let stats_final = expensive_computation_stats();
        println!("\nStatistics after new calls:");
        println!(
            "  Hits: {}, Misses: {}",
            stats_final.hits(),
            stats_final.misses()
        );
    }

    // Example 4: Monitoring eviction impact
    #[cfg(feature = "stats")]
    {
        println!("\n--- Eviction Impact on Hit Rate ---");

        shared_computation_stats().reset();

        // Fill cache (limit = 10)
        println!("\nFilling cache with 10 items:");
        for i in 0..10 {
            shared_computation(i, i + 1);
        }

        let stats1 = shared_computation_stats();
        println!(
            "After filling - Hit rate: {:.2}%",
            stats1.hit_rate() * 100.0
        );

        // Access existing items (all hits)
        println!("\nAccessing existing items:");
        for i in 0..10 {
            shared_computation(i, i + 1);
        }

        let stats2 = shared_computation_stats();
        println!("After hits - Hit rate: {:.2}%", stats2.hit_rate() * 100.0);

        // Add new items causing evictions
        println!("\nAdding 5 more items (causing evictions):");
        for i in 10..15 {
            shared_computation(i, i + 1);
        }

        let stats3 = shared_computation_stats();
        println!(
            "After evictions - Hit rate: {:.2}%",
            stats3.hit_rate() * 100.0
        );

        // Try to access evicted items
        println!("\nTrying to access potentially evicted items:");
        for i in 0..5 {
            shared_computation(i, i + 1);
        }

        let stats_final = shared_computation_stats();
        println!("\nFinal statistics:");
        println!("  Total accesses: {}", stats_final.total_accesses());
        println!("  Hits:           {}", stats_final.hits());
        println!("  Misses:         {}", stats_final.misses());
        println!("  Hit rate:       {:.2}%", stats_final.hit_rate() * 100.0);
    }

    #[cfg(not(feature = "stats"))]
    {
        println!("\n⚠️  Stats feature is not enabled!");
        println!("Run with: cargo run --example cache_stats --features stats");
    }
}
