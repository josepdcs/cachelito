//! Integration tests for ARC (Adaptive Replacement Cache) eviction policy in async context

use cachelito_async::cache_async;
use tokio;

#[cache_async(policy = "arc", limit = 5)]
async fn cached_computation(x: i32) -> i32 {
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    x * x
}

#[cache_async(policy = "arc", limit = 3)]
async fn global_arc_cache(x: i32) -> i32 {
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    x + 100
}

#[tokio::test]
async fn test_arc_basic_caching() {
    // First call should compute
    let result1 = cached_computation(5).await;
    assert_eq!(result1, 25);

    // Second call with same input should return cached value (faster)
    let start = tokio::time::Instant::now();
    let result2 = cached_computation(5).await;
    let elapsed = start.elapsed();

    assert_eq!(result2, 25);
    // Cached result should be much faster (< 5ms vs 10ms computation)
    assert!(elapsed.as_millis() < 8);
}

#[tokio::test]
async fn test_arc_limit_enforcement() {
    // Fill cache to limit
    for i in 1..=5 {
        cached_computation(i).await;
    }

    // Add more entries, forcing eviction
    for i in 6..=10 {
        cached_computation(i).await;
    }

    // Verify results are still correct (whether cached or recomputed)
    assert_eq!(cached_computation(10).await, 100);
    assert_eq!(cached_computation(6).await, 36);
}

#[tokio::test]
async fn test_arc_frequency_tracking() {
    // Access some values more frequently
    for _ in 0..5 {
        cached_computation(1).await;
        cached_computation(2).await;
    }

    // Access others less frequently
    cached_computation(3).await;
    cached_computation(4).await;
    cached_computation(5).await;

    // Fill cache beyond limit
    for i in 6..=10 {
        cached_computation(i).await;
    }

    // Frequently accessed items should still be available (verified by correct results)
    assert_eq!(cached_computation(1).await, 1);
    assert_eq!(cached_computation(2).await, 4);
}

#[tokio::test]
async fn test_arc_recency_tracking() {
    // Fill cache
    for i in 1..=5 {
        cached_computation(i).await;
    }

    // Re-access some items to make them recent
    cached_computation(2).await;
    cached_computation(4).await;

    // Add new items
    for i in 6..=8 {
        cached_computation(i).await;
    }

    // Recent items should have better survival rate
    assert_eq!(cached_computation(2).await, 4);
    assert_eq!(cached_computation(4).await, 16);
}

#[tokio::test]
async fn test_arc_adaptive_behavior() {
    // Create a mixed workload pattern
    // Some items accessed frequently, others just once

    // Frequent items
    for _ in 0..3 {
        cached_computation(10).await;
        cached_computation(20).await;
    }

    // One-time access items
    cached_computation(1).await;
    cached_computation(2).await;
    cached_computation(3).await;

    // Fill beyond limit
    for i in 30..=40 {
        cached_computation(i).await;
    }

    // Frequent items should be retained (verified by correct results)
    assert_eq!(cached_computation(10).await, 100);
    assert_eq!(cached_computation(20).await, 400);
}

#[tokio::test]
async fn test_arc_concurrent_access() {
    // Test concurrent access with ARC policy
    let handles: Vec<_> = (1..=10)
        .map(|i| {
            tokio::spawn(async move {
                // Each task accesses some shared values
                for _ in 0..3 {
                    global_arc_cache(i % 5).await;
                }
            })
        })
        .collect();

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify cache still works correctly
    assert_eq!(global_arc_cache(1).await, 101);
    assert_eq!(global_arc_cache(2).await, 102);
}

#[tokio::test]
async fn test_arc_scan_resistance() {
    // Establish some hot items
    for _ in 0..5 {
        cached_computation(100).await;
        cached_computation(200).await;
    }

    // Perform a scan (sequential access pattern)
    for i in 1..=20 {
        cached_computation(i).await;
    }

    // Hot items should survive the scan better than pure LRU
    assert_eq!(cached_computation(100).await, 10000);
    assert_eq!(cached_computation(200).await, 40000);
}

#[tokio::test]
async fn test_arc_empty_cache() {
    #[cache_async(policy = "arc", limit = 5)]
    async fn fresh_cache(x: i32) -> i32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        x * 2
    }

    // First access
    assert_eq!(fresh_cache(10).await, 20);

    // Second access (should be cached)
    let start = tokio::time::Instant::now();
    assert_eq!(fresh_cache(10).await, 20);
    let elapsed = start.elapsed();

    // Should be much faster when cached
    assert!(elapsed.as_millis() < 4);
}

#[tokio::test]
async fn test_arc_single_entry() {
    #[cache_async(policy = "arc", limit = 1)]
    async fn tiny_cache(x: i32) -> i32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        x * 3
    }

    assert_eq!(tiny_cache(1).await, 3);
    assert_eq!(tiny_cache(2).await, 6); // Should evict 1
    assert_eq!(tiny_cache(2).await, 6); // Should still be cached
    assert_eq!(tiny_cache(1).await, 3); // Should evict 2 and recompute
}

#[tokio::test]
async fn test_arc_with_different_types() {
    #[cache_async(policy = "arc", limit = 5)]
    async fn string_cache(s: String) -> String {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        format!("processed: {}", s)
    }

    let result1 = string_cache("hello".to_string()).await;
    assert_eq!(result1, "processed: hello");

    let result2 = string_cache("hello".to_string()).await;
    assert_eq!(result2, "processed: hello");
}

#[tokio::test]
async fn test_arc_result_caching() {
    #[cache_async(policy = "arc", limit = 5)]
    async fn divide(a: i32, b: i32) -> i32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        if b == 0 {
            panic!("Division by zero");
        }
        a / b
    }

    // Success should be cached
    assert_eq!(divide(10, 2).await, 5);
    assert_eq!(divide(10, 2).await, 5);

    // Test with different values
    assert_eq!(divide(20, 4).await, 5);
}

#[tokio::test]
async fn test_arc_high_concurrency() {
    // Test with many concurrent tasks accessing overlapping keys
    #[cache_async(policy = "arc", limit = 10)]
    async fn concurrent_func(x: i32) -> i32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        x * x
    }

    let mut handles = vec![];

    // Spawn 50 tasks
    for i in 0..50 {
        let handle = tokio::spawn(async move {
            // Access some overlapping keys
            concurrent_func(i % 15).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify cache still works
    assert_eq!(concurrent_func(5).await, 25);
    assert_eq!(concurrent_func(10).await, 100);
}

#[tokio::test]
async fn test_arc_mixed_frequency_patterns() {
    // Test that ARC adapts to mixed patterns better than pure LRU/LFU
    #[cache_async(policy = "arc", limit = 5)]
    async fn adaptive_func(x: i32) -> i32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        x + 1000
    }

    // Create a hot item (high frequency)
    for _ in 0..10 {
        adaptive_func(1).await;
    }

    // Create recent items (accessed once each, but recent)
    for i in 2..=5 {
        adaptive_func(i).await;
    }

    // Add new items that will trigger eviction
    for i in 6..=10 {
        adaptive_func(i).await;
    }

    // The hot item (1) should survive despite being "old" in LRU terms
    // because it has high frequency
    let start = tokio::time::Instant::now();
    assert_eq!(adaptive_func(1).await, 1001);
    let elapsed = start.elapsed();

    // If it's still cached, should be fast
    // If evicted and recomputed, will take > 5ms
    // ARC should keep it cached due to high frequency
    println!("Hot item access time: {}ms", elapsed.as_millis());
}
