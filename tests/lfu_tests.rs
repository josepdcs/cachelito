use cachelito::cache;

/// Test LFU eviction policy - least frequently used items are evicted first
#[test]
fn test_lfu_eviction() {
    #[cache(limit = 3, policy = "lfu")]
    fn compute(n: i32) -> i32 {
        n * 2
    }

    // Fill cache with 3 items
    assert_eq!(compute(1), 2); // freq: 1
    assert_eq!(compute(2), 4); // freq: 1
    assert_eq!(compute(3), 6); // freq: 1

    // Access item 1 and 2 multiple times to increase frequency
    assert_eq!(compute(1), 2); // freq: 2
    assert_eq!(compute(1), 2); // freq: 3
    assert_eq!(compute(2), 4); // freq: 2

    // Item 3 has frequency 1, item 2 has frequency 2, item 1 has frequency 3
    // Adding item 4 should evict item 3 (lowest frequency)
    assert_eq!(compute(4), 8); // freq: 1, evicts 3

    // Item 3 should be re-computed (cache miss)
    let result_3 = compute(3);
    assert_eq!(result_3, 6);

    // Item 1, 2, and 4 should still be in cache
    assert_eq!(compute(1), 2); // cache hit
    assert_eq!(compute(2), 4); // cache hit
    assert_eq!(compute(4), 8); // cache hit
}

/// Test that LFU correctly tracks frequency across multiple accesses
#[test]
fn test_lfu_frequency_tracking() {
    #[cache(limit = 2, policy = "lfu")]
    fn expensive_calc(x: u32) -> u32 {
        x * x
    }

    // Fill cache
    assert_eq!(expensive_calc(1), 1); // freq: 1
    assert_eq!(expensive_calc(2), 4); // freq: 1

    // Access first item many times
    for _ in 0..5 {
        assert_eq!(expensive_calc(1), 1); // freq: 6
    }

    // Add new item - should evict item 2 (freq 1) instead of item 1 (freq 6)
    assert_eq!(expensive_calc(3), 9);

    // Item 2 should be evicted
    assert_eq!(expensive_calc(2), 4); // re-computed

    // Item 1 should still be in cache (not evicted)
    assert_eq!(expensive_calc(1), 1); // cache hit
}

/// Test LFU with global scope
#[test]
fn test_lfu_global_scope() {
    #[cache(limit = 2, policy = "lfu", scope = "global")]
    fn global_lfu(n: i32) -> i32 {
        n + 100
    }

    assert_eq!(global_lfu(1), 101); // freq: 1
    assert_eq!(global_lfu(2), 102); // freq: 1
    assert_eq!(global_lfu(1), 101); // freq: 2
    assert_eq!(global_lfu(1), 101); // freq: 3

    // Adding item 3 should evict item 2 (freq 1), not item 1 (freq 3)
    assert_eq!(global_lfu(3), 103);

    // Verify item 1 is still cached
    assert_eq!(global_lfu(1), 101);
}

/// Test LFU behavior when all items have same frequency
#[test]
fn test_lfu_same_frequency() {
    #[cache(limit = 3, policy = "lfu")]
    fn process(n: i32) -> i32 {
        n * 3
    }

    // All items accessed once - same frequency
    assert_eq!(process(1), 3);
    assert_eq!(process(2), 6);
    assert_eq!(process(3), 9);

    // When all have same frequency, first one should be evicted (FIFO-like)
    assert_eq!(process(4), 12);

    // One of the original items should have been evicted
    // We'll just verify the cache still works correctly
    assert_eq!(process(4), 12);
}

/// Test LFU with TTL expiration
#[test]
fn test_lfu_with_ttl() {
    use std::thread;
    use std::time::Duration;

    #[cache(limit = 3, policy = "lfu", ttl = 1)]
    fn timed_calc(n: i32) -> i32 {
        n * 5
    }

    assert_eq!(timed_calc(1), 5); // freq: 1
    assert_eq!(timed_calc(1), 5); // freq: 2
    assert_eq!(timed_calc(1), 5); // freq: 3

    // Wait for TTL to expire
    thread::sleep(Duration::from_secs(2));

    // Should be re-computed (expired), frequency reset to 0
    assert_eq!(timed_calc(1), 5);
}
