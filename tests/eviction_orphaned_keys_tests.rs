//! Tests for eviction with orphaned keys in the queue
//!
//! These tests verify that the eviction mechanism correctly handles cases where
//! keys in the order queue no longer exist in the cache (e.g., due to TTL expiration).

use cachelito::cache;
use std::thread;
use std::time::Duration;

/// Test that eviction works correctly when the first key in queue is expired (TTL)
#[test]
fn test_eviction_with_expired_first_key() {
    static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[cache(limit = 2, ttl = 1)]
    fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        x * 2
    }

    // Fill cache to limit
    assert_eq!(compute(1), 2); // Cache: [1]
    assert_eq!(compute(2), 4); // Cache: [1, 2]

    // Wait for first entry to expire
    thread::sleep(Duration::from_secs(2));

    // Now cache should have: queue=[1, 2] but cache only has {2}
    // key 1 is orphaned (expired)

    // Insert new key - should evict successfully even though first key (1) is expired
    assert_eq!(compute(3), 6); // Should evict key 1 (expired), then key 2, insert 3

    // Verify that key 2 was actually evicted by forcing recomputation
    let count_before = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(compute(2), 4); // Should recompute (was evicted)
    let count_after = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        count_after > count_before,
        "Key 2 should have been evicted and recomputed"
    );
}

/// Test eviction when multiple keys in queue are orphaned
#[test]
fn test_eviction_with_multiple_orphaned_keys() {
    static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[cache(limit = 3, ttl = 1)]
    fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        x * 2
    }

    // Fill cache
    compute(1); // Cache: [1]
    compute(2); // Cache: [1, 2]
    compute(3); // Cache: [1, 2, 3]

    // Wait for first two entries to expire
    thread::sleep(Duration::from_secs(2));

    // Now queue=[1, 2, 3] but cache only has {3}
    // keys 1 and 2 are orphaned

    let count_before = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    // Insert new key - should skip orphaned keys 1 and 2, evict key 3
    compute(4); // Cache: [4]

    // Verify key 3 was evicted
    compute(3); // Should recompute
    let count_after = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    assert!(
        count_after > count_before,
        "Key 3 should have been evicted and recomputed"
    );
}

/// Test FIFO eviction with orphaned keys
#[test]
fn test_fifo_eviction_with_orphaned_keys() {
    static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[cache(limit = 2, policy = "fifo", ttl = 1)]
    fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        x * 10
    }

    // Fill cache
    assert_eq!(compute(1), 10); // Queue: [1]
    assert_eq!(compute(2), 20); // Queue: [1, 2]

    thread::sleep(Duration::from_secs(2));

    // Both expired, but queue still has [1, 2]
    let count_before = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    // This should handle both orphaned keys correctly
    assert_eq!(compute(3), 30); // Queue: [3]

    // Verify cache works correctly after cleanup
    assert_eq!(compute(3), 30); // Should be cached (no new computation)
    assert_eq!(compute(4), 40); // Queue: [3, 4]

    let count_after = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        count_after - count_before,
        2,
        "Should have computed 3 and 4 only"
    );
}

/// Test LRU eviction with orphaned keys
#[test]
fn test_lru_eviction_with_orphaned_keys() {
    static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[cache(limit = 2, policy = "lru", ttl = 1)]
    fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        x * 100
    }

    // Fill cache
    compute(1); // Queue: [1]
    compute(2); // Queue: [1, 2]

    // Access key 1 to make it most recently used
    compute(1); // Queue: [2, 1]

    thread::sleep(Duration::from_secs(2));

    // Both expired, queue has [2, 1]
    let count_before = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    // Should handle orphaned keys in LRU order
    compute(3);
    compute(4);

    // Verify both 3 and 4 were computed (not from cache)
    let count_after = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(count_after - count_before, 2, "Should compute 3 and 4");
}

/// Test that eviction queue auto-cleans orphaned entries
#[test]
fn test_eviction_queue_auto_cleanup() {
    static CALL_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[cache(limit = 3, ttl = 1)]
    fn compute(x: u32) -> u32 {
        CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        x
    }

    // Fill cache completely
    for i in 1..=3 {
        compute(i);
    }

    // Wait for all to expire
    thread::sleep(Duration::from_secs(2));

    // Queue has [1, 2, 3] but all are orphaned
    // Adding 3 new entries should work fine (auto-cleanup)
    for i in 4..=6 {
        compute(i);
    }

    let count = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(count, 6, "Should have computed all 6 entries");

    // Verify last 3 are cached
    let count_before = count;
    compute(4);
    compute(5);
    compute(6);
    let count_after = CALL_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    assert_eq!(
        count_after, count_before,
        "Last 3 should be cached (no new computations)"
    );
}
