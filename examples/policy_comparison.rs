use cachelito::cache;
use std::sync::atomic::{AtomicU32, Ordering};

/// Example comparing FIFO, LRU, and LFU eviction policies
///
/// This example demonstrates how different eviction policies behave
/// with the same access pattern, helping you choose the right one.
///
/// Run with: cargo run --example policy_comparison

static FIFO_CALLS: AtomicU32 = AtomicU32::new(0);
static LRU_CALLS: AtomicU32 = AtomicU32::new(0);
static LFU_CALLS: AtomicU32 = AtomicU32::new(0);

#[cache(limit = 3, policy = "fifo")]
fn compute_fifo(n: u32) -> u32 {
    FIFO_CALLS.fetch_add(1, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(10));
    n * n
}

#[cache(limit = 3, policy = "lru")]
fn compute_lru(n: u32) -> u32 {
    LRU_CALLS.fetch_add(1, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(10));
    n * n
}

#[cache(limit = 3, policy = "lfu")]
fn compute_lfu(n: u32) -> u32 {
    LFU_CALLS.fetch_add(1, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(10));
    n * n
}

fn main() {
    println!("=== Eviction Policy Comparison ===\n");
    println!("Cache limit: 3 entries");
    println!("Access pattern: [1,2,3] → access 1 multiple times → add 4 → verify\n");

    // Reset counters
    FIFO_CALLS.store(0, Ordering::SeqCst);
    LRU_CALLS.store(0, Ordering::SeqCst);
    LFU_CALLS.store(0, Ordering::SeqCst);

    println!("--- FIFO (First In, First Out) ---");
    println!("Phase 1: Fill cache");
    compute_fifo(1);
    compute_fifo(2);
    compute_fifo(3);
    println!("  Cached: [1, 2, 3]");
    println!("  Calls: {}", FIFO_CALLS.load(Ordering::SeqCst));

    println!("Phase 2: Access item 1 five times");
    for _ in 0..5 {
        compute_fifo(1);
    }
    println!(
        "  Calls: {} (no new calls - cached)",
        FIFO_CALLS.load(Ordering::SeqCst)
    );

    println!("Phase 3: Add item 4 (triggers eviction)");
    compute_fifo(4);
    println!("  Evicted: 1 (oldest, even though recently used)");
    println!("  Cached: [2, 3, 4]");
    println!("  Calls: {}", FIFO_CALLS.load(Ordering::SeqCst));

    println!("Phase 4: Try to access item 1 again");
    compute_fifo(1);
    println!(
        "  Calls: {} (item 1 was evicted - new call!)\n",
        FIFO_CALLS.load(Ordering::SeqCst)
    );

    // LRU
    println!("--- LRU (Least Recently Used) ---");
    println!("Phase 1: Fill cache");
    compute_lru(1);
    compute_lru(2);
    compute_lru(3);
    println!("  Cached: [1, 2, 3]");
    println!("  Calls: {}", LRU_CALLS.load(Ordering::SeqCst));

    println!("Phase 2: Access item 1 five times");
    for _ in 0..5 {
        compute_lru(1);
    }
    println!("  Item 1 moved to 'most recently used' position");
    println!("  Order: [2, 3, 1] (by recency)");
    println!(
        "  Calls: {} (no new calls)",
        LRU_CALLS.load(Ordering::SeqCst)
    );

    println!("Phase 3: Add item 4 (triggers eviction)");
    compute_lru(4);
    println!("  Evicted: 2 (least recently used)");
    println!("  Cached: [3, 1, 4]");
    println!("  Calls: {}", LRU_CALLS.load(Ordering::SeqCst));

    println!("Phase 4: Try to access item 1 again");
    compute_lru(1);
    println!(
        "  Calls: {} (item 1 still cached!)\n",
        LRU_CALLS.load(Ordering::SeqCst)
    );

    // LFU
    println!("--- LFU (Least Frequently Used) ---");
    println!("Phase 1: Fill cache");
    compute_lfu(1);
    compute_lfu(2);
    compute_lfu(3);
    println!("  Cached: [1, 2, 3]");
    println!("  Frequency: all at 1");
    println!("  Calls: {}", LFU_CALLS.load(Ordering::SeqCst));

    println!("Phase 2: Access item 1 five times");
    for _ in 0..5 {
        compute_lfu(1);
    }
    println!("  Frequency: 1→6, 2→1, 3→1");
    println!(
        "  Calls: {} (no new calls)",
        LFU_CALLS.load(Ordering::SeqCst)
    );

    println!("Phase 3: Add item 4 (triggers eviction)");
    compute_lfu(4);
    println!("  Evicted: 2 or 3 (both have frequency 1)");
    println!("  Cached: [1, 3 or 2, 4]");
    println!("  Calls: {}", LFU_CALLS.load(Ordering::SeqCst));

    println!("Phase 4: Try to access item 1 again");
    compute_lfu(1);
    println!(
        "  Calls: {} (item 1 still cached due to high frequency!)\n",
        LFU_CALLS.load(Ordering::SeqCst)
    );

    println!("=== Summary ===\n");
    println!(
        "FIFO  - Total calls: {} (item 1 was evicted despite recent use)",
        FIFO_CALLS.load(Ordering::SeqCst)
    );
    println!(
        "LRU   - Total calls: {} (item 1 kept due to recent access)",
        LRU_CALLS.load(Ordering::SeqCst)
    );
    println!(
        "LFU   - Total calls: {} (item 1 kept due to high frequency)",
        LFU_CALLS.load(Ordering::SeqCst)
    );
    println!();
    println!("Choose based on your use case:");
    println!("  • FIFO: Simple, predictable (oldest out)");
    println!("  • LRU:  Recent access matters (temporal locality)");
    println!("  • LFU:  Frequency matters (keep 'hot' data)");
}
