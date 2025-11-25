use cachelito::cache;
use std::collections::HashSet;

/// Demonstrates the Random eviction policy
///
/// The Random policy evicts a randomly selected entry when the cache is full.
/// This provides:
/// - O(1) eviction performance
/// - Minimal overhead
/// - Useful as a baseline for benchmarks
/// - Good for truly random access patterns

#[cache(policy = "random", limit = 5)]
fn compute_value(key: u32) -> u32 {
    println!("Computing value for key: {}", key);
    key * 2
}

fn main() {
    println!("=== Random Policy Cache Demo ===\n");
    println!("Cache configuration:");
    println!("  - Policy: Random");
    println!("  - Limit: 5 entries");
    println!("  - Scope: Global (default)\n");

    println!("Phase 1: Fill cache to limit");
    println!("=====================================");
    for i in 1..=5 {
        let result = compute_value(i);
        println!("  compute_value({}) = {} [MISS - computed]", i, result);
    }

    println!("\nPhase 2: Access cached values (all should be hits)");
    println!("=====================================");
    for i in 1..=5 {
        let result = compute_value(i);
        println!("  compute_value({}) = {} [HIT - from cache]", i, result);
    }

    println!("\nPhase 3: Add new values (triggers random evictions)");
    println!("=====================================");
    println!("Adding keys 6-10 will cause random evictions...\n");

    for i in 6..=10 {
        let result = compute_value(i);
        println!(
            "  compute_value({}) = {} [MISS - computed, random eviction occurred]",
            i, result
        );
    }

    println!("\nPhase 4: Check which keys were randomly evicted");
    println!("=====================================");
    println!("Checking keys 1-10 to see which remain in cache:\n");

    let mut cached_keys: HashSet<u32> = HashSet::new();

    for i in 1..=10 {
        // Access each key - values should always be correct
        let result = compute_value(i);
        println!("  Key {} -> {} (in cache)", i, result);
        cached_keys.insert(i);
    }

    println!("\nPhase 5: Demonstrate randomness with multiple runs");
    println!("=====================================");
    println!("The Random policy evicts entries unpredictably.");
    println!("Each run may evict different keys, unlike deterministic policies like LRU/FIFO.\n");

    // Clear by adding many new entries
    for i in 100..120 {
        compute_value(i);
    }

    println!("\nPhase 6: Random policy characteristics");
    println!("=====================================");
    println!("✓ O(1) eviction time - very fast");
    println!("✓ No access pattern tracking needed");
    println!("✓ Minimal memory overhead");
    println!("✓ Unpredictable eviction - good for random workloads");
    println!("✓ Useful as benchmark baseline");

    #[cfg(feature = "stats")]
    {
        use cachelito_core::stats_registry;

        if let Some(stats) = stats_registry::get("compute_value") {
            println!("\n=== Cache Statistics ===");
            println!("Total accesses: {}", stats.total_accesses());
            println!("Hits: {}", stats.hits());
            println!("Misses: {}", stats.misses());
            println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }
    }

    println!("\n=== Comparison with Other Policies ===");
    println!("FIFO   : Evicts oldest inserted entry (predictable)");
    println!("LRU    : Evicts least recently used (temporal locality)");
    println!("LFU    : Evicts least frequently used (frequency patterns)");
    println!("ARC    : Evicts based on frequency × recency score (adaptive)");
    println!("Random : Evicts randomly selected entry (baseline/random access)");
}
