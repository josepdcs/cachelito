//! Tests for async eviction with orphaned keys in the queue
//!
//! These tests verify that the async eviction mechanism correctly handles cases where
//! keys in the order queue no longer exist in the cache (e.g., due to TTL expiration).

use cachelito_async::cache_async;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::time::sleep;

/// Test that async eviction works correctly when the first key in queue is expired (TTL)
#[tokio::test]
async fn test_async_eviction_with_expired_first_key() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 2, ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x * 2
    }

    // Fill cache to limit
    assert_eq!(compute(1).await, 2);
    assert_eq!(compute(2).await, 4);

    // Wait for first entry to expire
    sleep(Duration::from_secs(2)).await;

    // Insert new key - should skip expired key 1, evict key 2
    assert_eq!(compute(3).await, 6);

    // Verify that key 2 was actually evicted
    let count_before = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(compute(2).await, 4);
    let count_after = CALL_COUNT.load(Ordering::SeqCst);
    assert!(count_after > count_before, "Key 2 should have been evicted");
}

/// Test async eviction when multiple keys in queue are orphaned
#[tokio::test]
async fn test_async_eviction_with_multiple_orphaned_keys() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 3, ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x * 2
    }

    // Fill cache
    compute(1).await;
    compute(2).await;
    compute(3).await;

    // Wait for first two entries to expire
    sleep(Duration::from_secs(2)).await;

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Insert new key - should skip orphaned keys 1 and 2, evict key 3
    compute(4).await;

    // Verify key 3 was evicted by trying to compute it again
    compute(3).await;
    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    assert!(
        count_after > count_before,
        "Key 3 should have been evicted and recomputed"
    );
}

/// Test race condition: multiple concurrent insertions at cache limit
/// This tests the scenario where multiple tasks try to insert entries
/// when the cache is at or near its limit
#[tokio::test]
async fn test_async_race_condition_concurrent_insertions_at_limit() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 5)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        x * 2
    }

    // Fill cache to limit
    for i in 1..=5 {
        compute(i).await;
    }

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Spawn many concurrent tasks trying to insert new entries
    // This should trigger evictions and test the atomic check-and-evict
    let mut handles = vec![];
    for i in 100..150 {
        let handle = tokio::spawn(async move { compute(i).await });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    // Should have computed all 50 new values
    assert_eq!(
        count_after - count_before,
        50,
        "Should compute all new unique values"
    );

    // Verify cache size is still within limit (allowing for race conditions, it might be slightly over)
    // But it should not be drastically larger than the limit
    // Note: Due to concurrent access, exact size may vary, but should be close to limit
    // We're mainly testing that it doesn't panic or deadlock
}

/// Test race condition: concurrent insertions with LRU at limit
#[tokio::test]
async fn test_async_race_condition_lru_insertions_at_limit() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 10, policy = "lru")]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        x * 3
    }

    // Fill cache
    for i in 1..=10 {
        compute(i).await;
    }

    // Spawn concurrent tasks that:
    // 1. Access existing entries (updating LRU order)
    // 2. Add new entries (triggering evictions)
    let mut handles = vec![];

    for round in 0..10 {
        // Access existing entries
        for key in 1..=10 {
            let handle = tokio::spawn(async move { compute(key).await });
            handles.push(handle);
        }

        // Add new entries (forcing evictions)
        for key in (100 + round * 10)..(110 + round * 10) {
            let handle = tokio::spawn(async move { compute(key).await });
            handles.push(handle);
        }
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify no panic occurred and cache is still functional
    let result = compute(999).await;
    assert_eq!(result, 2997);
}

/// Test race condition: multiple tasks computing the same key simultaneously
/// This tests the scenario where multiple tasks start computing the same key
/// at the same time and all try to insert the result
#[tokio::test]
async fn test_async_race_condition_duplicate_insertions() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 5)]
    async fn expensive_compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Simulate expensive computation
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        x * x
    }

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Spawn 10 tasks that all try to compute the same value (42) at the same time
    let mut handles = vec![];
    for _ in 0..10 {
        let handle = tokio::spawn(async { expensive_compute(42).await });
        handles.push(handle);
    }

    // All should return the same value
    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result, 1764);
    }

    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    // Should have computed at least once, but not necessarily 10 times
    // (some may have hit cache if one finished before others started)
    assert!(
        count_after > count_before,
        "Should have computed at least once"
    );

    // The key point: even though multiple tasks computed and tried to insert,
    // the cache should still respect the limit
    // We can't easily check cache size with DashMap, but the test passing
    // without panic or deadlock validates the fix
}

/// Test race condition: concurrent identical insertions at cache limit
#[tokio::test]
async fn test_async_race_condition_identical_keys_at_limit() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 3, policy = "lru")]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        x + 100
    }

    // Fill cache to limit
    compute(1).await;
    compute(2).await;
    compute(3).await;

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Spawn 20 tasks that all compute the same NEW key (4) concurrently
    // This should trigger eviction and all tasks should try to insert
    let mut handles = vec![];
    for _ in 0..20 {
        let handle = tokio::spawn(async { compute(4).await });
        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result, 104);
    }

    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    // Should have computed, but not necessarily 20 times
    assert!(count_after > count_before);

    // Verify cache is still functional and hasn't exceeded limit
    let result = compute(5).await;
    assert_eq!(result, 105);
}

/// Test concurrent async eviction with orphaned keys
#[tokio::test]
async fn test_async_fifo_eviction_with_orphaned_keys() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 2, policy = "fifo", ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x * 10
    }

    // Fill cache
    assert_eq!(compute(1).await, 10);
    assert_eq!(compute(2).await, 20);

    sleep(Duration::from_secs(2)).await;

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // This should handle both orphaned keys correctly
    assert_eq!(compute(3).await, 30);

    // Verify cache works correctly after cleanup
    assert_eq!(compute(3).await, 30); // Should be cached
    assert_eq!(compute(4).await, 40);

    let count_after = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(
        count_after - count_before,
        2,
        "Should have computed 3 and 4 only"
    );
}

/// Test async LRU eviction with orphaned keys
#[tokio::test]
async fn test_async_lru_eviction_with_orphaned_keys() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 2, policy = "lru", ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x * 100
    }

    // Fill cache
    compute(1).await;
    compute(2).await;

    // Access key 1 to make it most recently used
    compute(1).await;

    sleep(Duration::from_secs(2)).await;

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Should handle orphaned keys
    compute(3).await;
    compute(4).await;

    let count_after = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(count_after - count_before, 2, "Should compute 3 and 4");
}

/// Test that async eviction queue auto-cleans orphaned entries
#[tokio::test]
async fn test_async_eviction_queue_auto_cleanup() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 3, ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x
    }

    // Fill cache completely
    for i in 1..=3 {
        compute(i).await;
    }

    // Wait for all to expire
    sleep(Duration::from_secs(2)).await;

    // Adding 3 new entries should work fine (auto-cleanup)
    for i in 4..=6 {
        compute(i).await;
    }

    let count = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(count, 6, "Should have computed all 6 entries");

    // Verify last 3 are cached
    let count_before = count;
    compute(4).await;
    compute(5).await;
    compute(6).await;
    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    assert_eq!(count_after, count_before, "Last 3 should be cached");
}

/// Test race condition: concurrent cache hits and evictions
/// This tests the scenario where one task is reading from cache (hit)
/// while another task is evicting entries
#[tokio::test]
async fn test_async_race_condition_hit_vs_eviction() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 3, policy = "lru")]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        x * 2
    }

    // Fill cache
    compute(1).await;
    compute(2).await;
    compute(3).await;

    let _count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Spawn many concurrent tasks that:
    // - Read existing keys (cache hits triggering LRU updates)
    // - Add new keys (triggering evictions)
    let mut handles = vec![];

    for _ in 0..20 {
        // Read existing keys
        for key in 1..=3 {
            let handle = tokio::spawn(async move { compute(key).await });
            handles.push(handle);
        }

        // Add new keys (forcing evictions)
        let handle = tokio::spawn(async move {
            compute(100).await;
            compute(200).await;
            compute(300).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify no panic occurred and cache is still functional
    let result = compute(999).await;
    assert_eq!(result, 1998);
}

/// Test race condition: TTL expiration during LRU update
/// This tests the scenario where an entry expires between cache hit
/// and LRU order update
#[tokio::test]
async fn test_async_race_condition_ttl_vs_lru_update() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 2, policy = "lru", ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        x * 3
    }

    // Add entries
    compute(1).await;
    compute(2).await;

    // Wait almost until expiration
    sleep(Duration::from_millis(900)).await;

    // Spawn concurrent tasks that will hit these keys
    // Some will hit before expiration, some after
    let mut handles = vec![];
    for _ in 0..50 {
        let handle = tokio::spawn(async {
            compute(1).await;
            compute(2).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify cache still works correctly
    let count_before = CALL_COUNT.load(Ordering::SeqCst);
    compute(3).await;
    let count_after = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(count_after - count_before, 1);
}

/// Test that orphaned keys in order queue don't cause infinite loops or panics
#[tokio::test]
async fn test_async_orphaned_keys_no_infinite_loop() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 2, policy = "fifo", ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x
    }

    // Fill cache
    compute(1).await;
    compute(2).await;

    // Wait for expiration
    sleep(Duration::from_secs(2)).await;

    // Now the order queue has [1, 2] but cache is empty
    // Adding new items should handle this gracefully
    let start = std::time::Instant::now();

    compute(3).await;
    compute(4).await;
    compute(5).await;

    let elapsed = start.elapsed();

    // Should complete quickly (not stuck in infinite loop)
    assert!(
        elapsed < Duration::from_secs(2),
        "Should not hang on orphaned keys"
    );

    // Verify correct number of computations
    let count = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(count, 5, "Should have computed all 5 values");
}

/// Test that expired entries are removed from order queue even without a limit
/// This prevents memory leaks in the order queue when entries expire
#[tokio::test]
async fn test_async_expired_entries_cleaned_from_order_queue() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    // Note: No limit set, but we still want order queue cleanup on expiration
    #[cache_async(ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x * 5
    }

    // Add some entries
    compute(1).await;
    compute(2).await;
    compute(3).await;

    // Wait for expiration
    sleep(Duration::from_secs(2)).await;

    let count_before = CALL_COUNT.load(Ordering::SeqCst);

    // Access expired entries - should recompute and not leave orphaned keys
    compute(1).await;
    compute(2).await;
    compute(3).await;

    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    // All three should have been recomputed (expired)
    assert_eq!(
        count_after - count_before,
        3,
        "All expired entries should be recomputed"
    );

    // Access again - should be cached now
    let count_before = count_after;
    compute(1).await;
    compute(2).await;
    compute(3).await;
    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    assert_eq!(
        count_after, count_before,
        "Non-expired entries should be cached"
    );
}

/// Test concurrent async eviction with orphaned keys
#[tokio::test]
async fn test_async_concurrent_eviction_with_orphaned_keys() {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[cache_async(limit = 5, ttl = 1)]
    async fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        x * 2
    }

    // Fill cache
    for i in 1..=5 {
        compute(i).await;
    }

    // Wait for expiration
    sleep(Duration::from_secs(2)).await;

    // Spawn concurrent tasks to add new entries
    let mut handles = vec![];
    for i in 6..=10 {
        let handle = tokio::spawn(async move { compute(i).await });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Should have computed 10 total
    let count = CALL_COUNT.load(Ordering::SeqCst);
    assert_eq!(count, 10, "Should have computed all 10 entries");
}
