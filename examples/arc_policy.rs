//! Example demonstrating the ARC (Adaptive Replacement Cache) eviction policy.
//!
//! ARC is a self-tuning cache algorithm that dynamically balances between:
//! - Recency (LRU-like behavior): Recently accessed items
//! - Frequency (LFU-like behavior): Frequently accessed items
//!
//! This example shows how ARC adapts to different access patterns better than
//! pure LRU or LFU policies.

use cachelito::cache;

/// Compute factorial with ARC caching
#[cache(policy = "arc", limit = 10)]
fn factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

/// Expensive computation simulating database lookup
#[cache(policy = "arc", limit = 5, scope = "global")]
fn fetch_user_data(user_id: u32) -> String {
    println!("  [MISS] Fetching user data for ID: {}", user_id);
    std::thread::sleep(std::time::Duration::from_millis(100));
    format!("User data for ID: {}", user_id)
}

fn main() {
    println!("=== ARC (Adaptive Replacement Cache) Policy Demo ===\n");

    // Example 1: Factorial with mixed access patterns
    println!("Example 1: Factorial computation with mixed patterns");
    println!("Cache limit: 10 entries\n");

    // Sequential access (tests recency)
    println!("Sequential access pattern:");
    for i in 1..=5 {
        let result = factorial(i);
        println!("  factorial({}) = {}", i, result);
    }

    println!("\nRepeated access to same values (tests frequency):");
    for _ in 0..3 {
        factorial(3); // Frequently accessed
        factorial(5); // Frequently accessed
    }

    println!("\nNew values that should evict less frequently used ones:");
    for i in 6..=12 {
        let result = factorial(i);
        println!("  factorial({}) = {}", i, result);
    }

    println!("\nRe-accessing frequent values (should still be cached):");
    println!(
        "  factorial(3) = {} [likely cached due to high frequency]",
        factorial(3)
    );
    println!(
        "  factorial(5) = {} [likely cached due to high frequency]",
        factorial(5)
    );
    println!(
        "  factorial(1) = {} [may be evicted due to low frequency]",
        factorial(1)
    );

    // Example 2: User data with concurrent access
    println!("\n\n=== Example 2: User data fetching ===");
    println!("Cache limit: 5 entries\n");

    // Initial population
    println!("Initial access:");
    for id in 1..=5 {
        fetch_user_data(id);
    }

    // Frequent access to specific users
    println!("\nFrequently accessing users 2 and 4:");
    for _ in 0..3 {
        fetch_user_data(2);
        fetch_user_data(4);
    }

    // Add new users (should evict based on ARC score)
    println!("\nAdding new users (cache is full):");
    for id in 6..=8 {
        fetch_user_data(id);
    }

    // Check if frequent users are still cached
    println!("\nRe-accessing users:");
    println!("User 2 (frequently accessed):");
    fetch_user_data(2); // Should be cached

    println!("User 4 (frequently accessed):");
    fetch_user_data(4); // Should be cached

    println!("User 1 (accessed once, long ago):");
    fetch_user_data(1); // Likely evicted

    // Example 3: Scan-resistant behavior
    println!("\n\n=== Example 3: Scan-resistant behavior ===");

    #[cache(policy = "arc", limit = 5)]
    fn process_item(id: u32) -> String {
        println!("  Processing item {}", id);
        format!("Item {}", id)
    }

    // Establish some frequently used items
    println!("Establishing hot items:");
    for _ in 0..5 {
        process_item(1);
        process_item(2);
    }

    // Simulate a scan (sequential access of many items)
    println!("\nPerforming scan (many sequential accesses):");
    for id in 10..=20 {
        process_item(id);
    }

    // Check if hot items survived the scan
    println!("\nChecking if hot items survived the scan:");
    println!("Item 1 (hot):");
    process_item(1); // Should still be cached
    println!("Item 2 (hot):");
    process_item(2); // Should still be cached
    println!("Item 15 (from scan):");
    process_item(15); // Likely evicted

    println!("\n=== ARC Policy Benefits ===");
    println!("✓ Adapts to mixed workloads (recency + frequency)");
    println!("✓ Protects frequently used items from eviction");
    println!("✓ Handles sequential scans without polluting cache");
    println!("✓ Self-tuning - no manual parameter adjustment needed");
    println!("✓ O(1) operations for all cache access");
}
