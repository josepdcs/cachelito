use cachelito::cache;

/// Example demonstrating the LFU (Least Frequently Used) eviction policy.
///
/// In LFU caching, when the cache is full, the entry that has been accessed
/// the least number of times is evicted. This is useful for scenarios where
/// frequently accessed items should remain in the cache.
///
/// Run with: cargo run --example lfu
// Database simulation - tracks calls to demonstrate cache behavior
use std::sync::atomic::{AtomicU32, Ordering};

static DB_CALLS: AtomicU32 = AtomicU32::new(0);

fn simulate_db_query(user_id: u32) -> String {
    DB_CALLS.fetch_add(1, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(100));
    format!("User data for ID {}", user_id)
}

/// Cached database query with LFU eviction policy
///
/// With limit=3, only 3 entries can be cached at once.
/// When a 4th entry is added, the least frequently accessed entry is evicted.
#[cache(limit = 3, policy = "lfu")]
fn get_user_data(user_id: u32) -> String {
    simulate_db_query(user_id)
}

fn main() {
    println!("=== LFU (Least Frequently Used) Cache Policy Demo ===\n");

    // Reset counter
    DB_CALLS.store(0, Ordering::SeqCst);

    println!("Phase 1: Fill cache with 3 users");
    println!("  Fetching user 1... {}", get_user_data(1));
    println!("  Fetching user 2... {}", get_user_data(2));
    println!("  Fetching user 3... {}", get_user_data(3));
    println!("  DB calls so far: {}\n", DB_CALLS.load(Ordering::SeqCst));

    println!("Phase 2: Access user 1 and 2 multiple times (increase frequency)");
    for i in 0..3 {
        println!("  Access {}: User 1 = {}", i + 1, get_user_data(1));
    }
    for i in 0..2 {
        println!("  Access {}: User 2 = {}", i + 1, get_user_data(2));
    }
    println!(
        "  DB calls so far: {} (should still be 3 - all cache hits)\n",
        DB_CALLS.load(Ordering::SeqCst)
    );

    println!("Frequency summary:");
    println!("  User 1: accessed 4 times (1 initial + 3 additional)");
    println!("  User 2: accessed 3 times (1 initial + 2 additional)");
    println!("  User 3: accessed 1 time (only initial)");
    println!();

    println!("Phase 3: Add user 4 (cache is full, will evict least frequently used)");
    println!("  Fetching user 4... {}", get_user_data(4));
    println!(
        "  DB calls: {} (new call for user 4)\n",
        DB_CALLS.load(Ordering::SeqCst)
    );

    println!("Phase 4: Verify which user was evicted");
    println!("  Fetching user 3... {}", get_user_data(3));
    let calls_after_3 = DB_CALLS.load(Ordering::SeqCst);
    println!(
        "  DB calls: {} (user 3 was evicted - new call made)",
        calls_after_3
    );

    println!("  Fetching user 1... {}", get_user_data(1));
    println!(
        "  DB calls: {} (user 1 still cached - no new call)\n",
        DB_CALLS.load(Ordering::SeqCst)
    );

    println!("Phase 5: Add user 5 (will evict user 4, which has lowest frequency)");
    println!("  Fetching user 5... {}", get_user_data(5));
    println!("  DB calls: {}\n", DB_CALLS.load(Ordering::SeqCst));

    println!("Phase 6: Final verification");
    println!("  Fetching user 4... {}", get_user_data(4));
    println!(
        "  DB calls: {} (user 4 was evicted - new call made)",
        DB_CALLS.load(Ordering::SeqCst)
    );

    println!("  Fetching user 1... {}", get_user_data(1));
    println!(
        "  DB calls: {} (user 1 still cached due to high frequency)\n",
        DB_CALLS.load(Ordering::SeqCst)
    );

    println!("=== Summary ===");
    println!("LFU eviction policy ensures that frequently accessed items remain");
    println!("in the cache longer, even if they were accessed long ago.");
    println!("Total DB calls: {}", DB_CALLS.load(Ordering::SeqCst));
    println!("\nCompare with FIFO (examples/fifo.rs) and LRU (examples/lru.rs)");
}
