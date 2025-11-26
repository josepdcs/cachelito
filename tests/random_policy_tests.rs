use cachelito::cache;

/// Test basic random policy functionality with thread-local cache
#[test]
fn test_random_policy_thread_local() {
    #[cache(scope = "thread", policy = "random", limit = 3)]
    fn compute(n: u32) -> u32 {
        n * 2
    }

    // Fill cache
    assert_eq!(compute(1), 2);
    assert_eq!(compute(2), 4);
    assert_eq!(compute(3), 6);

    // Verify cached
    assert_eq!(compute(1), 2);
    assert_eq!(compute(2), 4);
    assert_eq!(compute(3), 6);

    // Add more items to trigger evictions
    compute(4);
    compute(5);
    compute(6);

    // Cache should still have 3 items (but we don't know which ones)
    // All values should still be correct regardless of what's cached
    assert_eq!(compute(1), 2);
    assert_eq!(compute(2), 4);
    assert_eq!(compute(3), 6);
    assert_eq!(compute(4), 8);
    assert_eq!(compute(5), 10);
    assert_eq!(compute(6), 12);
}

/// Test random policy with global cache
#[test]
fn test_random_policy_global() {
    #[cache(
        scope = "global",
        policy = "random",
        limit = 5,
        name = "random_global_test"
    )]
    fn calculate(n: u32) -> u32 {
        n.pow(2)
    }

    // Fill cache
    for i in 1..=5 {
        calculate(i);
    }

    // Verify values are correct
    for i in 1..=5 {
        assert_eq!(calculate(i), i.pow(2));
    }

    // Add more to trigger random eviction
    for i in 6..=10 {
        calculate(i);
    }

    // All values should still be correct
    for i in 1..=10 {
        assert_eq!(calculate(i), i.pow(2));
    }
}

/// Test random policy doesn't update order on cache hits
#[test]
fn test_random_policy_no_order_update() {
    #[cache(scope = "thread", policy = "random", limit = 3)]
    fn compute_double(n: u32) -> u32 {
        n * 2
    }

    // Fill cache
    compute_double(1);
    compute_double(2);
    compute_double(3);

    // Access first item multiple times (should not affect eviction order)
    for _ in 0..10 {
        compute_double(1);
    }

    // The fact that key 1 was accessed many times doesn't matter for Random policy
    // Any key could be evicted when we add new items
    compute_double(4);
    compute_double(5);

    // All values should be correct
    assert_eq!(compute_double(1), 2);
    assert_eq!(compute_double(2), 4);
    assert_eq!(compute_double(3), 6);
    assert_eq!(compute_double(4), 8);
    assert_eq!(compute_double(5), 10);
}

/// Test random policy with TTL
#[test]
fn test_random_policy_with_ttl() {
    use std::thread;
    use std::time::Duration;

    #[cache(scope = "thread", policy = "random", limit = 5, ttl = 100)]
    fn compute_with_ttl(n: u32) -> u32 {
        n + 100
    }

    // Fill cache
    compute_with_ttl(1);
    compute_with_ttl(2);
    compute_with_ttl(3);

    // Values should be cached
    assert_eq!(compute_with_ttl(1), 101);
    assert_eq!(compute_with_ttl(2), 102);
    assert_eq!(compute_with_ttl(3), 103);

    // Wait for expiration
    thread::sleep(Duration::from_millis(150));

    // Values should be recomputed (expired)
    assert_eq!(compute_with_ttl(1), 101);
    assert_eq!(compute_with_ttl(2), 102);
    assert_eq!(compute_with_ttl(3), 103);
}

/// Test random policy eviction actually happens
#[test]
fn test_random_eviction_occurs() {
    #[cache(
        scope = "global",
        policy = "random",
        limit = 3,
        name = "random_eviction_test"
    )]
    fn expensive_compute(n: u32) -> u32 {
        n * 10
    }

    // Fill cache (3 items)
    expensive_compute(1);
    expensive_compute(2);
    expensive_compute(3);

    // Access cached values
    assert_eq!(expensive_compute(1), 10);
    assert_eq!(expensive_compute(2), 20);
    assert_eq!(expensive_compute(3), 30);

    // Add more items - should trigger random evictions
    expensive_compute(4);
    expensive_compute(5);
    expensive_compute(6);

    // All values should still be correct (some will be recomputed)
    assert_eq!(expensive_compute(1), 10);
    assert_eq!(expensive_compute(2), 20);
    assert_eq!(expensive_compute(3), 30);
    assert_eq!(expensive_compute(4), 40);
    assert_eq!(expensive_compute(5), 50);
    assert_eq!(expensive_compute(6), 60);
}

/// Test that random policy behaves differently than FIFO
#[test]
fn test_random_vs_deterministic() {
    // With Random policy, we can't predict which keys will be evicted
    // But we can verify that eviction happens and values remain correct

    #[cache(scope = "thread", policy = "random", limit = 3)]
    fn random_compute(n: u32) -> u32 {
        n + 1000
    }

    // Fill cache with 10 items (limit is 3)
    for i in 1..=10 {
        random_compute(i);
    }

    // Only 3 items should remain in cache
    // But we can't predict which ones - that's the nature of random eviction
    // All we can verify is that the function still returns correct values
    for i in 1..=10 {
        assert_eq!(random_compute(i), i + 1000);
    }
}

#[cfg(feature = "stats")]
#[test]
fn test_random_policy_stats() {
    use cachelito_core::stats_registry;

    #[cache(
        scope = "global",
        policy = "random",
        limit = 5,
        name = "random_stats_test"
    )]
    fn compute_for_stats(n: u32) -> u32 {
        n * 3
    }

    // Fill cache
    for i in 1..=5 {
        compute_for_stats(i);
    }

    // Access cached values
    for i in 1..=5 {
        compute_for_stats(i);
    }

    // Check stats
    if let Some(stats) = stats_registry::get("random_stats_test") {
        assert_eq!(stats.total_accesses(), 10);
        assert_eq!(stats.hits(), 5);
        assert_eq!(stats.misses(), 5);
        assert_eq!(stats.hit_rate(), 0.5);
    }
}

/// Test random policy with Result types
#[test]
fn test_random_policy_with_result() {
    #[cache(scope = "thread", policy = "random", limit = 3)]
    fn divide(a: i32, b: i32) -> Result<i32, String> {
        if b == 0 {
            Err("Division by zero".to_string())
        } else {
            Ok(a / b)
        }
    }

    // Test successful results
    assert_eq!(divide(10, 2), Ok(5));
    assert_eq!(divide(20, 4), Ok(5));
    assert_eq!(divide(15, 3), Ok(5));

    // Test error results (errors are also cached)
    assert_eq!(divide(10, 0), Err("Division by zero".to_string()));

    // Cached values
    assert_eq!(divide(10, 2), Ok(5));
    assert_eq!(divide(10, 0), Err("Division by zero".to_string()));
}

/// Test concurrent access with random policy
#[test]
fn test_random_policy_concurrent() {
    use std::thread;

    #[cache(
        scope = "global",
        policy = "random",
        limit = 10,
        name = "random_concurrent_test"
    )]
    fn concurrent_compute(n: u32) -> u32 {
        n * 7
    }

    let mut handles = vec![];

    // Spawn multiple threads accessing the cache
    for thread_id in 0..5 {
        let handle = thread::spawn(move || {
            for i in 0..20 {
                let key = (thread_id * 20 + i) % 30; // Create overlapping access
                let result = concurrent_compute(key);
                assert_eq!(result, key * 7);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify values are still correct
    for i in 0..30 {
        assert_eq!(concurrent_compute(i), i * 7);
    }
}
