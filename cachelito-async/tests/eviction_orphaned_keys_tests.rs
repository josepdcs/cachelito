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

    // Verify key 3 was evicted
    compute(3).await;
    let count_after = CALL_COUNT.load(Ordering::SeqCst);

    assert!(count_after > count_before, "Key 3 should have been evicted");
}

/// Test async FIFO eviction with orphaned keys
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
