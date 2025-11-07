// Comparative example showing thread-local vs global cache with parking_lot

use cachelito::cache;
use std::thread;
use std::time::Instant;

// Thread-local cache (no synchronization)
#[cache(limit = 50)]
fn compute_local(n: u64) -> u64 {
    // Simulate work
    (0..100).fold(n, |acc, x| acc.wrapping_add(x))
}

// Global cache with parking_lot::Mutex (synchronized)
#[cache(scope = "global", limit = 50)]
fn compute_global(n: u64) -> u64 {
    // Same computation
    (0..100).fold(n, |acc, x| acc.wrapping_add(x))
}

fn main() {
    println!("=== Thread-Local vs Global Cache Comparison ===\n");

    const NUM_THREADS: usize = 4;
    const OPS_PER_THREAD: u64 = 1000;

    // Thread-local benchmark
    println!("1. Thread-Local Cache (independent caches per thread):");
    let start = Instant::now();

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|thread_id| {
            thread::spawn(move || {
                for i in 0..OPS_PER_THREAD {
                    // Each thread caches its own subset
                    let key = (thread_id as u64 * 10) + (i % 10);
                    let _ = compute_local(key);
                }
                println!("   Thread {} completed", thread_id);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let local_time = start.elapsed();
    println!("   Total time: {:?}", local_time);
    println!("   Note: Each thread maintains its own cache (no sharing)\n");

    // Global cache benchmark
    println!("2. Global Cache (shared cache with parking_lot::Mutex):");
    let start = Instant::now();

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|thread_id| {
            thread::spawn(move || {
                for i in 0..OPS_PER_THREAD {
                    // All threads share the same cache
                    let key = i % 10; // Overlapping keys
                    let _ = compute_global(key);
                }
                println!("   Thread {} completed", thread_id);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let global_time = start.elapsed();
    println!("   Total time: {:?}", global_time);
    println!("   Note: All threads benefit from shared cache\n");

    // Analysis
    println!("=== Analysis ===");
    println!("Thread-Local benefits:");
    println!("  ✓ No synchronization overhead (faster)");
    println!("  ✓ No lock contention");
    println!("  ✗ No cache sharing (duplicate work across threads)");
    println!("  ✗ Higher memory usage (cache per thread)\n");

    println!("Global Cache benefits (with parking_lot::Mutex):");
    println!("  ✓ Cache sharing (less duplicate work)");
    println!("  ✓ Lower memory usage (single cache)");
    println!("  ✓ Efficient synchronization (parking_lot is optimized)");
    println!("  ✗ Lock contention under high concurrency");

    println!("\nparking_lot::Mutex advantages over std::sync::Mutex:");
    println!("  • 30-50% faster under contention");
    println!("  • No lock poisoning (simpler API)");
    println!("  • ~40x smaller memory footprint");
    println!("  • Fair scheduling prevents starvation");
}
