use cachelito_async::cache_async;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::time::sleep;

// Counter to track function calls
static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

/// Simulate an expensive database query
async fn expensive_database_query(user_id: u64) -> Result<String, String> {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);

    // Simulate network delay
    sleep(Duration::from_millis(100)).await;

    if user_id > 0 {
        Ok(format!("User data for ID {}", user_id))
    } else {
        Err("Invalid user ID".to_string())
    }
}

/// Basic W-TinyLFU async cache
///
/// Default configuration:
/// - Window segment: 20%
/// - Protected segment: 80%
#[cache_async(limit = 10, policy = "w_tinylfu")]
async fn get_user_basic(user_id: u64) -> Result<String, String> {
    expensive_database_query(user_id).await
}

/// W-TinyLFU with emphasis on recency (larger window)
///
/// Good for frequently changing data like news or social media
#[cache_async(limit = 10, policy = "w_tinylfu", window_ratio = 0.4)]
async fn get_trending_content(content_id: u64) -> String {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(50)).await;
    format!("Trending content #{}", content_id)
}

/// W-TinyLFU with emphasis on frequency (smaller window)
///
/// Good for stable data like reference tables or configuration
#[cache_async(limit = 10, policy = "w_tinylfu", window_ratio = 0.1)]
async fn get_reference_data(key: u64) -> String {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(50)).await;
    format!("Reference data for {}", key)
}

#[tokio::main]
async fn main() {
    println!("=== Basic Async W-TinyLFU Examples ===\n");

    // Example 1: Basic usage
    println!("Example 1: Basic W-TinyLFU Cache");
    println!("---------------------------------");
    CALL_COUNT.store(0, Ordering::SeqCst);

    // First calls - cache miss (calls expensive function)
    println!("First access (cache miss):");
    for i in 1..=5 {
        match get_user_basic(i).await {
            Ok(data) => println!("  {}", data),
            Err(e) => println!("  Error: {}", e),
        }
    }
    println!("Function calls: {}", CALL_COUNT.load(Ordering::SeqCst));

    // Second calls - cache hit (returns cached value)
    println!("\nSecond access (cache hit):");
    for i in 1..=5 {
        match get_user_basic(i).await {
            Ok(data) => println!("  {}", data),
            Err(e) => println!("  Error: {}", e),
        }
    }
    println!(
        "Function calls: {} (no new calls!)",
        CALL_COUNT.load(Ordering::SeqCst)
    );

    // Example 2: Hot vs Cold data
    println!("\n\nExample 2: Hot vs Cold Data Pattern");
    println!("------------------------------------");
    CALL_COUNT.store(0, Ordering::SeqCst);

    // Access users 1 and 2 frequently (hot data)
    println!("Building hot data (users 1, 2):");
    for _ in 0..10 {
        let _ = get_user_basic(1).await;
        let _ = get_user_basic(2).await;
    }
    println!("Function calls: {}", CALL_COUNT.load(Ordering::SeqCst));

    // Access users 3-12 once (cold data)
    println!("\nAccessing cold data (users 3-12):");
    for i in 3..=12 {
        let _ = get_user_basic(i).await;
    }
    let calls_after_cold = CALL_COUNT.load(Ordering::SeqCst);
    println!("Function calls: {}", calls_after_cold);

    // Re-access hot data - should still be cached!
    println!("\nRe-accessing hot data (should be cached):");
    let _ = get_user_basic(1).await;
    let _ = get_user_basic(2).await;
    let final_calls = CALL_COUNT.load(Ordering::SeqCst);

    if final_calls == calls_after_cold {
        println!("✓ Hot data protected! Still cached despite cold data influx.");
    } else {
        println!("✗ Hot data was evicted.");
    }
    println!("Final function calls: {}", final_calls);

    // Example 3: Different window ratios
    println!("\n\nExample 3: Window Ratio Comparison");
    println!("-----------------------------------");

    // Large window (recency-focused)
    println!("Trending content (40% window - recency focus):");
    CALL_COUNT.store(0, Ordering::SeqCst);
    for i in 1..=10 {
        get_trending_content(i).await;
    }
    // Access recent items
    for i in 8..=10 {
        get_trending_content(i).await;
    }
    println!(
        "  Calls for trending: {}",
        CALL_COUNT.load(Ordering::SeqCst)
    );

    // Small window (frequency-focused)
    println!("\nReference data (10% window - frequency focus):");
    CALL_COUNT.store(0, Ordering::SeqCst);
    for i in 1..=10 {
        get_reference_data(i).await;
    }
    // Access most frequent items
    for _ in 0..5 {
        get_reference_data(1).await;
        get_reference_data(2).await;
    }
    println!(
        "  Calls for reference: {}",
        CALL_COUNT.load(Ordering::SeqCst)
    );

    // Example 4: Concurrent tasks
    println!("\n\nExample 4: Concurrent Async Tasks");
    println!("----------------------------------");
    CALL_COUNT.store(0, Ordering::SeqCst);

    #[cache_async(limit = 5, policy = "w_tinylfu")]
    async fn shared_cache(id: u64) -> u64 {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        sleep(Duration::from_millis(10)).await;
        id * 2
    }

    // Spawn 3 concurrent tasks accessing the same cache
    let task1 = tokio::spawn(async {
        for i in 1..=5 {
            shared_cache(i).await;
        }
    });

    let task2 = tokio::spawn(async {
        for i in 1..=5 {
            shared_cache(i).await;
        }
    });

    let task3 = tokio::spawn(async {
        for i in 1..=5 {
            shared_cache(i).await;
        }
    });

    // Wait for all tasks
    task1.await.unwrap();
    task2.await.unwrap();
    task3.await.unwrap();

    let concurrent_calls = CALL_COUNT.load(Ordering::SeqCst);
    println!("3 tasks × 5 items = 15 accesses");
    println!("Actual function calls: {}", concurrent_calls);
    println!("Cached accesses: {}", 15 - concurrent_calls);
    println!("✓ W-TinyLFU efficiently handles concurrent async tasks!");

    // Summary
    println!("\n=== Summary ===");
    println!("W-TinyLFU async cache provides:");
    println!("  ✓ Excellent hit rates on mixed workloads");
    println!("  ✓ Protection for frequently accessed data");
    println!("  ✓ Configurable window_ratio:");
    println!("    • Small (0.1): Frequency-focused");
    println!("    • Default (0.2): Balanced");
    println!("    • Large (0.4): Recency-focused");
    println!("  ✓ Async/await native support");
    println!("  ✓ Thread-safe with DashMap");
}
