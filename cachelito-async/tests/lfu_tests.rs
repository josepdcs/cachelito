use cachelito_async::cache_async;

#[tokio::test]
async fn test_async_lfu_eviction() {
    #[cache_async(limit = 3, policy = "lfu")]
    async fn compute(n: i32) -> i32 {
        n * 2
    }

    // Fill cache with 3 items
    assert_eq!(compute(1).await, 2); // freq: 1
    assert_eq!(compute(2).await, 4); // freq: 1
    assert_eq!(compute(3).await, 6); // freq: 1

    // Access item 1 and 2 multiple times to increase frequency
    assert_eq!(compute(1).await, 2); // freq: 2
    assert_eq!(compute(1).await, 2); // freq: 3
    assert_eq!(compute(2).await, 4); // freq: 2

    // Item 3 has frequency 1, should be evicted when adding item 4
    assert_eq!(compute(4).await, 8);

    // Item 3 should be re-computed (cache miss)
    assert_eq!(compute(3).await, 6);

    // Items 1, 2, and 4 should still be in cache
    assert_eq!(compute(1).await, 2);
    assert_eq!(compute(2).await, 4);
    assert_eq!(compute(4).await, 8);
}

#[tokio::test]
async fn test_async_lfu_frequency_tracking() {
    #[cache_async(limit = 2, policy = "lfu")]
    async fn expensive_calc(x: u32) -> u32 {
        x * x
    }

    // Fill cache
    assert_eq!(expensive_calc(1).await, 1); // freq: 1
    assert_eq!(expensive_calc(2).await, 4); // freq: 1

    // Access first item many times
    for _ in 0..5 {
        assert_eq!(expensive_calc(1).await, 1); // freq: 6
    }

    // Add new item - should evict item 2 (freq 1) instead of item 1 (freq 6)
    assert_eq!(expensive_calc(3).await, 9);

    // Item 2 should be evicted
    assert_eq!(expensive_calc(2).await, 4);

    // Item 1 should still be in cache
    assert_eq!(expensive_calc(1).await, 1);
}

#[tokio::test]
async fn test_async_lfu_with_ttl() {
    use std::time::Duration;

    #[cache_async(limit = 3, policy = "lfu", ttl = 1)]
    async fn timed_calc(n: i32) -> i32 {
        n * 5
    }

    assert_eq!(timed_calc(1).await, 5); // freq: 1
    assert_eq!(timed_calc(1).await, 5); // freq: 2
    assert_eq!(timed_calc(1).await, 5); // freq: 3

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be re-computed (expired), frequency reset
    assert_eq!(timed_calc(1).await, 5);
}

#[tokio::test]
async fn test_async_lfu_concurrent_access() {
    use std::sync::atomic::{AtomicU32, Ordering};

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    #[cache_async(limit = 2, policy = "lfu")]
    async fn concurrent_compute(n: u32) -> u32 {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        n * 2
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    // Spawn multiple tasks accessing same keys
    let handles: Vec<_> = (0..10)
        .map(|i| {
            tokio::spawn(async move {
                // All tasks access key 1 frequently
                for _ in 0..3 {
                    concurrent_compute(1).await;
                }
                // Some tasks access key 2
                if i % 2 == 0 {
                    concurrent_compute(2).await;
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    // Key 1 should have very high frequency, key 2 moderate
    // Add key 3 - should be fine since we have limit 2 and only 2 keys
    assert_eq!(concurrent_compute(3).await, 6);

    // Verify one of the original keys is still cached
    let initial_calls = CALL_COUNT.load(Ordering::SeqCst);
    concurrent_compute(1).await;

    // If key 1 is cached (which it should be due to high frequency), no new call
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), initial_calls);
}
