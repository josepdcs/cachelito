use cachelito::cache;

/// Example demonstrating concurrent cache statistics with global scope.
///
/// This shows how statistics are tracked across multiple threads
/// accessing the same global cache.

#[cache(scope = "global", limit = 100, policy = "lru")]
fn compute_factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        n * compute_factorial(n - 1)
    }
}

fn main() {
    println!("=== Concurrent Cache Statistics Example ===\n");

    #[cfg(feature = "stats")]
    {
        // Reset stats at the start
        compute_factorial_stats().reset();

        println!("Spawning 5 threads, each computing factorials...\n");

        let handles: Vec<_> = (0..5)
            .map(|thread_id| {
                std::thread::spawn(move || {
                    println!("Thread {} starting", thread_id);

                    // Each thread computes factorials from 1 to 20
                    for n in 1..=20 {
                        let result = compute_factorial(n);
                        if thread_id == 0 && n % 5 == 0 {
                            println!("  Thread {}: factorial({}) = {}", thread_id, n, result);
                        }
                    }

                    println!("Thread {} finished", thread_id);
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        println!("\n=== Final Statistics ===");

        let stats = compute_factorial_stats();

        println!("\n📊 Cache Performance:");
        println!("  Total accesses: {}", stats.total_accesses());
        println!("  Cache hits:     {}", stats.hits());
        println!("  Cache misses:   {}", stats.misses());
        println!("  Hit rate:       {:.2}%", stats.hit_rate() * 100.0);
        println!("  Miss rate:      {:.2}%", stats.miss_rate() * 100.0);

        println!("\n💡 Analysis:");
        println!("  - First thread computed 20 values (20 misses)");
        println!("  - Remaining 4 threads found all values in cache (80 hits)");
        println!("  - Expected: ~80% hit rate");
        println!("  - Actual:   {:.2}% hit rate", stats.hit_rate() * 100.0);

        let expected_hits = 80.0;
        let expected_misses = 20.0;
        let expected_hit_rate = expected_hits / (expected_hits + expected_misses);

        if (stats.hit_rate() - expected_hit_rate).abs() < 0.01 {
            println!("  ✅ Performance matches expectations!");
        } else {
            println!("  ⚠️  Performance differs from expectations");
            println!("     (Due to thread scheduling variations)");
        }

        // Demonstrate statistics reset
        println!("\n=== Testing Statistics Reset ===");

        let stats_before = compute_factorial_stats();
        println!("\nBefore reset:");
        println!("  Hits: {}", stats_before.hits());

        stats_before.reset();

        let stats_after = compute_factorial_stats();
        println!("\nAfter reset:");
        println!("  Hits: {}", stats_after.hits());
        println!("  Misses: {}", stats_after.misses());

        // Make some calls after reset
        println!("\nMaking a few cached calls:");
        compute_factorial(10); // Hit (already in cache)
        compute_factorial(15); // Hit
        compute_factorial(25); // Miss (new value)

        let final_stats = compute_factorial_stats();
        println!("\nNew statistics:");
        println!("  Hits: {}", final_stats.hits());
        println!("  Misses: {}", final_stats.misses());
        println!("  Hit rate: {:.2}%", final_stats.hit_rate() * 100.0);
    }

    #[cfg(not(feature = "stats"))]
    {
        println!("⚠️  Stats feature is not enabled!");
        println!("Run with: cargo run --example concurrent_stats --features stats");
    }
}
