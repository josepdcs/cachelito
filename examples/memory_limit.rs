use cachelito::cache;

/// Example demonstrating memory-based cache limits.
///
/// This example shows how to use the `max_memory` attribute to limit cache size
/// by memory usage instead of entry count.

#[cache(max_memory = "1MB", policy = "lru")]
fn generate_large_string(id: u32) -> String {
    println!("Generating large string for id: {}", id);
    // Generate a string of approximately 100KB
    "X".repeat(100_000) + &format!(" - ID: {}", id)
}

#[cache(max_memory = "500KB", policy = "fifo")]
fn create_vector(size: usize) -> Vec<u64> {
    println!("Creating vector of size: {}", size);
    let result: Vec<u64> = (0..size).map(|i| i as u64).collect();
    result
}

#[cache(limit = 100, max_memory = "2MB", policy = "lfu")]
fn process_data(key: String) -> Vec<String> {
    println!("Processing data for key: {}", key);
    // Create a moderately sized result
    vec![format!("Result for {}", key); 1000]
}

fn main() {
    println!("=== Memory-Based Cache Limits Example ===\n");

    // Example 1: Large strings with 1MB limit
    println!("1. Testing large string cache (max_memory = 1MB):");
    for i in 1..=15 {
        let result = generate_large_string(i);
        println!("  String {} length: {} bytes", i, result.len());
    }
    // Since each string is ~100KB, only about 10 strings should fit in 1MB
    // Earlier entries should be evicted

    println!("\n  Accessing earlier entries (may be evicted):");
    let _ = generate_large_string(1); // Likely evicted, will regenerate
    let _ = generate_large_string(14); // Likely still cached

    println!("\n2. Testing vector cache (max_memory = 500KB):");
    // Create vectors of different sizes
    let _ = create_vector(10_000); // ~80KB
    let _ = create_vector(20_000); // ~160KB
    let _ = create_vector(30_000); // ~240KB
    let _ = create_vector(40_000); // ~320KB
                                   // The 500KB limit will trigger eviction

    println!("\n3. Testing data processing cache (limit = 100, max_memory = 2MB):");
    // This cache has both entry count and memory limits
    // The more restrictive limit will be enforced
    for i in 1..=10 {
        let _ = process_data(format!("key_{}", i));
    }

    println!("\n=== Memory Limit Examples Complete ===");
    println!("Note: Memory limits ensure cache doesn't grow beyond specified size,");
    println!("      evicting entries according to the chosen policy.");
}
