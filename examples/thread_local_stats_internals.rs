/// Example demonstrating that thread-local caches DO track statistics internally.
///
/// This example shows that ThreadLocalCache has a `stats` field and tracks
/// hit/miss rates, but these statistics are only accessible when you have
/// direct access to the cache object (like in tests), not through stats_registry.
///
/// **Key Takeaway**: Thread-local statistics exist and work, but are not
/// accessible via the public `stats_registry` API due to architectural limitations.

#[cfg(feature = "stats")]
fn main() {
    use cachelito_core::{CacheEntry, EvictionPolicy, ThreadLocalCache};
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};

    println!("=== Thread-Local Cache Statistics - Internal Access ===\n");

    // Define thread-local storage
    thread_local! {
        static CACHE: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
        static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    }

    // Create a thread-local cache
    let cache = ThreadLocalCache::new(&CACHE, &ORDER, Some(5), EvictionPolicy::LRU, None);

    println!("Making cache calls...\n");

    // First access - miss
    cache.insert("key1", 100);
    println!("Inserted key1=100");

    // Second access - hit
    if let Some(value) = cache.get("key1") {
        println!("Retrieved key1={} (HIT)", value);
    }

    // Third access - miss
    cache.insert("key2", 200);
    println!("Inserted key2=200");

    // Fourth access - hit
    if let Some(value) = cache.get("key1") {
        println!("Retrieved key1={} (HIT)", value);
    }

    // Fifth access - miss
    cache.insert("key3", 300);
    println!("Inserted key3=300");

    // Access statistics directly from the cache object
    println!("\n📊 Thread-Local Cache Statistics (Direct Access):");
    println!("  Total accesses: {}", cache.stats.total_accesses());
    println!("  Hits:           {}", cache.stats.hits());
    println!("  Misses:         {}", cache.stats.misses());
    println!("  Hit rate:       {:.2}%", cache.stats.hit_rate() * 100.0);
    println!("  Miss rate:      {:.2}%", cache.stats.miss_rate() * 100.0);

    println!("\n✅ As you can see, thread-local caches DO track statistics!");
    println!("   They're just not accessible via stats_registry::get()");

    // Demonstrate that stats work correctly
    println!("\n--- Testing Statistics Accuracy ---\n");

    cache.stats.reset();
    println!("Reset statistics\n");

    // Make predictable calls - get() is what tracks stats
    cache.insert("test1", 1);
    cache.get("nonexistent"); // Miss
    cache.get("test1"); // Hit
    cache.get("test1"); // Hit
    cache.insert("test2", 2);
    cache.get("test1"); // Hit

    println!("Made 4 get() calls: 3 hits, 1 miss");
    println!("\nStatistics:");
    println!("  Hits:   {} (expected: 3)", cache.stats.hits());
    println!("  Misses: {} (expected: 1)", cache.stats.misses());
    println!("  Total:  {} (expected: 4)", cache.stats.total_accesses());

    assert_eq!(cache.stats.hits(), 3);
    assert_eq!(cache.stats.misses(), 1);
    assert_eq!(cache.stats.total_accesses(), 4);

    println!("\n✅ Statistics are accurate!");

    // Demonstrate multi-threading
    println!("\n--- Thread Isolation ---\n");

    let handle = std::thread::spawn(|| {
        thread_local! {
            static CACHE2: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
            static ORDER2: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
        }

        let cache2 = ThreadLocalCache::new(&CACHE2, &ORDER2, None, EvictionPolicy::FIFO, None);

        cache2.insert("thread2_key", 999);
        cache2.get("thread2_key");

        println!("Thread 2 statistics:");
        println!("  Hits:   {}", cache2.stats.hits());
        println!("  Misses: {}", cache2.stats.misses());
    });

    handle.join().unwrap();

    println!("\nMain thread statistics (unchanged by other thread):");
    println!("  Hits:   {}", cache.stats.hits());
    println!("  Misses: {}", cache.stats.misses());

    println!("\n✅ Each thread has independent statistics!");

    println!("\n=== Summary ===\n");
    println!("✅ Thread-local caches HAVE statistics (stats field)");
    println!("✅ Statistics are tracked accurately");
    println!("✅ Each thread has independent statistics");
    println!("❌ NOT accessible via stats_registry::get() (architectural limitation)");
    println!("✅ Accessible in tests and when you have direct cache object access");
    println!("\n💡 Use scope = \"global\" if you need stats_registry access");
}

#[cfg(not(feature = "stats"))]
fn main() {
    println!("⚠️  This example requires the 'stats' feature!");
    println!("Run with: cargo run --example thread_local_stats_internals --features stats");
}
