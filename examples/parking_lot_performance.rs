// Example demonstrating parking_lot::Mutex performance benefits
// in a multi-threaded global cache scenario

use cachelito::cache;
use cachelito_core::MemoryEstimator;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct ExpensiveData {
    id: u64,
    payload: Vec<u8>,
}

impl MemoryEstimator for ExpensiveData {
    fn estimate_memory(&self) -> usize {
        size_of::<Self>() + self.payload.capacity()
    }
}

// Global scope cache using parking_lot::Mutex internally
#[cache(scope = "global", limit = 100, policy = "lru")]
fn fetch_data(id: u64) -> Arc<ExpensiveData> {
    // Simulate expensive operation
    thread::sleep(Duration::from_millis(10));

    Arc::new(ExpensiveData {
        id,
        payload: vec![0u8; 1024], // 1KB of data
    })
}

fn main() {
    println!("=== Cachelito with parking_lot::Mutex Demo ===\n");

    // Scenario 1: Multiple threads accessing the same cached data
    println!("Scenario 1: Concurrent cache hits (10 threads, same key)");
    let start = Instant::now();

    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            thread::spawn(move || {
                // All threads request the same data
                let data = fetch_data(42);
                println!("  Thread {} got data with id: {}", thread_id, data.id);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("  Time: {:?}\n", start.elapsed());

    // Scenario 2: High contention with mixed reads/writes
    println!("Scenario 2: High contention (8 threads, 100 ops each)");
    let start = Instant::now();

    let handles: Vec<_> = (0..8)
        .map(|thread_id| {
            thread::spawn(move || {
                for i in 0..100 {
                    // Mix of cache hits and misses
                    let key = (thread_id * 10 + i % 10) as u64;
                    let _ = fetch_data(key);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("  Time: {:?}", start.elapsed());
    println!("  (parking_lot::Mutex provides better performance under contention)\n");

    // Scenario 3: Demonstrating no lock poisoning
    println!("Scenario 3: No lock poisoning with parking_lot");
    println!("  With std::sync::Mutex, a panic while holding the lock would poison it.");
    println!("  parking_lot::Mutex doesn't have lock poisoning - simpler and faster!");
    println!("  Our API doesn't need to handle Result from lock() calls.\n");

    // Scenario 4: Memory efficiency
    println!("Scenario 4: Memory efficiency");
    println!("  std::sync::Mutex size: ~40 bytes");
    println!("  parking_lot::Mutex size: ~1 byte");
    println!("  With many cached functions, this adds up!\n");

    println!("=== Benefits of parking_lot::Mutex ===");
    println!("  ✓ Better performance under contention");
    println!("  ✓ No lock poisoning (simpler API)");
    println!("  ✓ Smaller memory footprint");
    println!("  ✓ Fair locking algorithm");
    println!("  ✓ Adaptive spinning for short critical sections");
}
