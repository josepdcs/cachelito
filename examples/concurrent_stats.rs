use cachelito::cache;

/// Example demonstrating concurrent cache statistics with global scope.
///
/// This shows how statistics are tracked across multiple threads
/// accessing the same global cache (default behavior).

#[cache(limit = 100, policy = "lru")] // Global by default
fn compute_factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else if n > 20 {
        // Prevent overflow - factorial(21) already overflows u64
        panic!("Factorial too large (max 20)");
    } else {
        n.checked_mul(compute_factorial(n - 1))
            .expect("Factorial overflow")
    }
}

fn main() {
    println!("=== Concurrent Cache Statistics Example ===\n");

    #[cfg(feature = "stats")]
    {
        // Reset stats at the start
        cachelito::stats_registry::reset("compute_factorial");

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

        let stats = cachelito::stats_registry::get("compute_factorial").unwrap();

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

        let stats_before = cachelito::stats_registry::get("compute_factorial").unwrap();
        println!("\nBefore reset:");
        println!("  Hits: {}", stats_before.hits());

        cachelito::stats_registry::reset("compute_factorial");

        let stats_after = cachelito::stats_registry::get("compute_factorial").unwrap();
        println!("\nAfter reset:");
        println!("  Hits: {}", stats_after.hits());
        println!("  Misses: {}", stats_after.misses());

        // Make some calls after reset (use smaller numbers to avoid overflow)
        println!("\nMaking a few cached calls:");
        compute_factorial(10); // Hit (already in cache)
        compute_factorial(15); // Hit
        compute_factorial(5); // Hit (already in cache)

        let final_stats = cachelito::stats_registry::get("compute_factorial").unwrap();
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
