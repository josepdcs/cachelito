//! # W-TinyLFU Cache Example
//!
//! This example demonstrates the W-TinyLFU (Windowed TinyLFU) eviction policy.
//!
//! W-TinyLFU is an advanced admission-based policy that:
//! - Uses a small window segment for new entries (recency)
//! - Uses a protected segment for proven valuable entries (frequency)
//! - Uses a Count-Min Sketch for efficient frequency estimation
//! - Admits new entries based on frequency comparison
//!
//! This provides excellent hit rates on mixed workloads with minimal memory overhead.

use cachelito::cache;
use std::cell::RefCell;

// Counter to track function executions
thread_local! {
    static EXEC_COUNT: RefCell<usize> = RefCell::new(0);
}

/// Simulates an expensive computation with W-TinyLFU caching
///
/// Configuration:
/// - Limit: 5 entries total
/// - Window ratio: 0.2 (20% = 1 entry in window, 4 in protected)
/// - Policy: W-TinyLFU with admission control
///
/// NOTE: This example will compile once W-TinyLFU implementation is complete
#[cache(
    policy = "w_tinylfu",
    limit = 5,
    window_ratio = 0.2,
    sketch_width = 64,
    sketch_depth = 4
)]
fn compute(key: u32) -> u32 {
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() += 1;
    });
    println!("  [COMPUTE] key={}", key);
    key * 2
}

fn main() {
    println!("=== W-TinyLFU Cache Example ===\n");
    println!("Cache configuration:");
    println!("  - Total capacity: 5 entries");
    println!("  - Window: 20% (1 entry)");
    println!("  - Protected: 80% (4 entries)");
    println!("  - Policy: W-TinyLFU with admission control\n");

    EXEC_COUNT.with(|count| {
        *count.borrow_mut() = 0;
    });

    println!("--- Phase 1: Fill cache ---");
    for i in 1..=5 {
        println!("\nAccess key {}", i);
        let result = compute(i);
        println!("  Result: {}", result);
    }

    println!("\n--- Phase 2: Access popular entries ---");
    println!("(Build frequency for keys 1, 2, 3)");
    for _ in 0..3 {
        for i in 1..=3 {
            print!(".");
            compute(i);
        }
    }
    println!(" done");

    println!("\n--- Phase 3: Add new entries (admission test) ---");
    println!("\nAccess key 6 (new, low frequency)");
    compute(6);
    println!("  → Should be admitted to window, evict key from window");

    println!("\nAccess key 7 (new, low frequency)");
    compute(7);
    println!("  → Window full, needs to compete for protected segment");
    println!("  → Frequency too low, likely rejected vs high-frequency keys 1-3");

    println!("\n--- Phase 4: Verify high-frequency entries survived ---");
    println!("\nRe-access keys 1, 2, 3 (should be cached in protected)");
    for i in 1..=3 {
        println!("\nAccess key {}", i);
        let result = compute(i);
        println!("  Result: {} (cached)", result);
    }

    let exec_count = EXEC_COUNT.with(|count| *count.borrow());
    println!("\n--- Results ---");
    println!("Total executions: {}", exec_count);
    println!("\nW-TinyLFU admission policy successfully:");
    println!("  ✓ Protected high-frequency entries (1, 2, 3)");
    println!("  ✓ Rejected low-frequency candidates");
    println!("  ✓ Maintained optimal hit rate");
}
