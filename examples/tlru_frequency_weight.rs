//! Example demonstrating TLRU with frequency_weight parameter.
//!
//! The frequency_weight parameter allows you to adjust the importance of
//! access frequency in the TLRU eviction algorithm.
//!
//! - frequency_weight = 0.3: Low weight on frequency (more emphasis on recency and age)
//! - frequency_weight = 1.0: Normal weight (default behavior)
//! - frequency_weight = 2.0: High weight on frequency

use cachelito::cache;
use std::thread;
use std::time::Duration;

/// Cache with TLRU and low frequency weight (0.3)
/// This means frequency has less impact on eviction decisions
#[cache(policy = "tlru", limit = 3, ttl = 10, frequency_weight = 0.3)]
fn low_frequency_weight(n: u32) -> u32 {
    println!("💻 Computing low_frequency_weight({})", n);
    n * 2
}

/// Cache with TLRU and default frequency weight (no parameter = default behavior)
#[cache(policy = "tlru", limit = 3, ttl = 10)]
fn default_frequency_weight(n: u32) -> u32 {
    println!("💻 Computing default_frequency_weight({})", n);
    n * 2
}

/// Cache with TLRU and high frequency weight (1.5)
/// This means frequency has more impact on eviction decisions
#[cache(policy = "tlru", limit = 3, ttl = 10, frequency_weight = 1.5)]
fn high_frequency_weight(n: u32) -> u32 {
    println!("💻 Computing high_frequency_weight({})", n);
    n * 2
}

fn main() {
    println!("=== TLRU frequency_weight Demo ===\n");

    // Example 1: Low frequency weight (0.3)
    println!("--- Example 1: Low Frequency Weight (0.3) ---");
    println!("Frequency has LESS impact on eviction\n");

    // Fill cache
    low_frequency_weight(1);
    low_frequency_weight(2);
    low_frequency_weight(3);

    // Increase frequency of entry 1 significantly
    println!("\n📊 Accessing entry 1 many times...");
    for _ in 0..10 {
        low_frequency_weight(1);
    }

    // Add new entry - with low frequency weight, entry 1 might still be evicted
    // because frequency has less impact
    println!("\n➕ Adding entry 4 (cache is full)...");
    low_frequency_weight(4);

    println!("\n✓ Checking which entries remain:");
    low_frequency_weight(1); // May or may not be cached
    low_frequency_weight(2);
    low_frequency_weight(4);

    // Example 2: Default frequency weight
    println!("\n\n--- Example 2: Default Frequency Weight ---");
    println!("Standard TLRU behavior\n");

    default_frequency_weight(1);
    default_frequency_weight(2);
    default_frequency_weight(3);

    println!("\n📊 Accessing entry 1 many times...");
    for _ in 0..10 {
        default_frequency_weight(1);
    }

    println!("\n➕ Adding entry 4 (cache is full)...");
    default_frequency_weight(4);

    println!("\n✓ Checking which entries remain:");
    default_frequency_weight(1); // More likely to be cached
    default_frequency_weight(2);
    default_frequency_weight(4);

    // Example 3: High frequency weight (1.5)
    println!("\n\n--- Example 3: High Frequency Weight (1.5) ---");
    println!("Frequency has MORE impact on eviction\n");

    high_frequency_weight(1);
    high_frequency_weight(2);
    high_frequency_weight(3);

    println!("\n📊 Accessing entry 1 many times...");
    for _ in 0..10 {
        high_frequency_weight(1);
    }

    println!("\n➕ Adding entry 4 (cache is full)...");
    high_frequency_weight(4);

    println!("\n✓ Checking which entries remain:");
    high_frequency_weight(1); // Very likely to be cached
    high_frequency_weight(2);
    high_frequency_weight(4);

    // Example 4: Comparing behavior with age factor
    println!("\n\n--- Example 4: Age Factor with Different Weights ---");

    println!("\n🕐 Creating entries with different ages...");
    high_frequency_weight(10);
    thread::sleep(Duration::from_millis(500));

    high_frequency_weight(20);
    high_frequency_weight(30);

    println!("\n📊 Accessing all entries equally...");
    high_frequency_weight(10);
    high_frequency_weight(20);
    high_frequency_weight(30);

    println!("\n➕ Adding new entry...");
    high_frequency_weight(40);

    println!("\n✓ With high frequency weight, older entry (10) should be evicted");
    println!("   because age factor decreases its score more than with low weight");

    println!("\n\n🎯 Summary:");
    println!("   - Low weight (0.3): Emphasizes recency and age over frequency");
    println!("   - Default: Balanced approach");
    println!("   - High weight (1.5): Emphasizes frequency over other factors");
    println!("   - Formula: score = frequency^weight × position × age_factor");
}
