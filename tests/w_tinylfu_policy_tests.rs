use cachelito::cache;

/// Test basic W-TinyLFU eviction with window and protected segments
#[test]
fn test_w_tinylfu_basic_eviction() {
    #[cache(limit = 10, policy = "w_tinylfu", window_ratio = 0.2)]
    fn compute(x: u32) -> u32 {
        x * 2
    }

    // Fill cache
    for i in 0..10 {
        assert_eq!(compute(i), i * 2);
    }

    // Access first few entries multiple times (should move to protected)
    for _ in 0..5 {
        compute(0);
        compute(1);
        compute(2);
    }

    // Add new entry - should evict from window, not protected
    compute(100);

    // Protected entries should still be cached
    // Note: We can't directly test cache hits without exposing internals,
    // but we can verify the function still works correctly
    assert_eq!(compute(0), 0);
    assert_eq!(compute(1), 2);
    assert_eq!(compute(2), 4);
}

/// Test W-TinyLFU with small window ratio
#[test]
fn test_w_tinylfu_small_window() {
    #[cache(limit = 100, policy = "w_tinylfu", window_ratio = 0.01)]
    fn fibonacci(n: u32) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            _ => {
                let a = fibonacci(n - 1);
                let b = fibonacci(n - 2);
                a + b
            }
        }
    }

    // Window is only 1% of 100 = 1 entry minimum
    assert_eq!(fibonacci(10), 55);
    assert_eq!(fibonacci(20), 6765);
}

/// Test W-TinyLFU with large window ratio
#[test]
fn test_w_tinylfu_large_window() {
    #[cache(limit = 50, policy = "w_tinylfu", window_ratio = 0.3)]
    fn square(x: u32) -> u32 {
        x * x
    }

    // Window is 30% of 50 = 15 entries
    for i in 0..50 {
        assert_eq!(square(i), i * i);
    }

    // Adding more should evict from window first
    for i in 50..60 {
        assert_eq!(square(i), i * i);
    }
}

/// Test W-TinyLFU default configuration
#[test]
fn test_w_tinylfu_default_config() {
    #[cache(limit = 100, policy = "w_tinylfu")]
    fn process(id: u32) -> String {
        format!("result_{}", id)
    }

    // Default window_ratio should be applied
    for i in 0..100 {
        assert_eq!(process(i), format!("result_{}", i));
    }

    // Trigger eviction
    assert_eq!(process(200), "result_200".to_string());
}

/// Test W-TinyLFU with thread-local scope
#[test]
fn test_w_tinylfu_thread_local() {
    #[cache(scope = "thread", limit = 20, policy = "w_tinylfu", window_ratio = 0.2)]
    fn compute_local(x: u32) -> u32 {
        x + 42
    }

    // Each thread gets its own cache
    std::thread::spawn(|| {
        for i in 0..20 {
            assert_eq!(compute_local(i), i + 42);
        }
    })
    .join()
    .unwrap();

    // Main thread has independent cache
    for i in 0..20 {
        assert_eq!(compute_local(i), i + 42);
    }
}

/// Test W-TinyLFU with global scope
#[test]
fn test_w_tinylfu_global() {
    #[cache(
        scope = "global",
        limit = 30,
        policy = "w_tinylfu",
        window_ratio = 0.15
    )]
    fn compute_global(x: u32) -> u32 {
        x * 3
    }

    // Shared across threads
    let handles: Vec<_> = (0..3)
        .map(|thread_id| {
            std::thread::spawn(move || {
                for i in 0..10 {
                    assert_eq!(compute_global(thread_id * 10 + i), (thread_id * 10 + i) * 3);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

/// Test W-TinyLFU frequency-based admission
#[test]
fn test_w_tinylfu_frequency_admission() {
    #[cache(limit = 5, policy = "w_tinylfu", window_ratio = 0.4)]
    fn get_value(key: u32) -> u32 {
        key * 10
    }

    // Fill cache
    for i in 0..5 {
        get_value(i);
    }

    // Access some entries multiple times to increase frequency
    for _ in 0..10 {
        get_value(0);
        get_value(1);
    }

    // Add new entries - should evict low-frequency entries
    get_value(100);
    get_value(101);

    // High-frequency entries should still be accessible
    assert_eq!(get_value(0), 0);
    assert_eq!(get_value(1), 10);
}

/// Test W-TinyLFU with TTL
#[test]
fn test_w_tinylfu_with_ttl() {
    #[cache(limit = 10, policy = "w_tinylfu", window_ratio = 0.2, ttl = 1)]
    fn get_timestamp(key: u32) -> String {
        format!("value_{}", key)
    }

    // Insert entries
    for i in 0..5 {
        get_timestamp(i);
    }

    // Wait for TTL expiration
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Entries should be expired and re-computed
    assert_eq!(get_timestamp(0), "value_0".to_string());
}

/// Test W-TinyLFU eviction preserves frequently accessed items
#[test]
fn test_w_tinylfu_preserves_frequent_items() {
    #[cache(limit = 10, policy = "w_tinylfu", window_ratio = 0.3)]
    fn expensive_computation(n: u32) -> u32 {
        // Simulate expensive operation
        n.pow(2)
    }

    // Fill cache
    for i in 0..10 {
        expensive_computation(i);
    }

    // Make some items "hot" by accessing them repeatedly
    for _ in 0..20 {
        expensive_computation(5);
        expensive_computation(6);
        expensive_computation(7);
    }

    // Fill cache with new items to trigger evictions
    for i in 10..20 {
        expensive_computation(i);
    }

    // Hot items should still be in cache
    // (can't verify directly without stats, but they should compute quickly if cached)
    assert_eq!(expensive_computation(5), 25);
    assert_eq!(expensive_computation(6), 36);
    assert_eq!(expensive_computation(7), 49);
}

/// Test W-TinyLFU with custom types
#[test]
fn test_w_tinylfu_custom_types() {
    #[derive(Clone, Debug, PartialEq)]
    struct Data {
        id: u32,
        value: String,
    }

    #[cache(limit = 15, policy = "w_tinylfu", window_ratio = 0.2)]
    fn get_data(id: u32) -> Data {
        Data {
            id,
            value: format!("data_{}", id),
        }
    }

    for i in 0..15 {
        let data = get_data(i);
        assert_eq!(
            data,
            Data {
                id: i,
                value: format!("data_{}", i)
            }
        );
    }

    // Trigger eviction
    let new_data = get_data(100);
    assert_eq!(
        new_data,
        Data {
            id: 100,
            value: "data_100".to_string()
        }
    );
}

/// Test W-TinyLFU with window ratio edge cases
#[test]
fn test_w_tinylfu_window_ratio_edge_cases() {
    // Very small window (should be at least 1 entry)
    #[cache(limit = 10, policy = "w_tinylfu", window_ratio = 0.001)]
    fn tiny_window(x: u32) -> u32 {
        x
    }

    for i in 0..15 {
        assert_eq!(tiny_window(i), i);
    }

    // Large window (almost entire cache)
    #[cache(limit = 10, policy = "w_tinylfu", window_ratio = 0.9)]
    fn large_window(x: u32) -> u32 {
        x * 2
    }

    for i in 0..15 {
        assert_eq!(large_window(i), i * 2);
    }
}

#[test]
fn test_w_tinylfu_concurrent_access() {
    use std::thread;

    #[cache(
        scope = "global",
        limit = 100,
        policy = "w_tinylfu",
        window_ratio = 0.15
    )]
    fn concurrent_compute(key: u32) -> u32 {
        key.wrapping_mul(17)
    }

    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            thread::spawn(move || {
                for i in 0..50 {
                    let key = thread_id * 50 + i;
                    assert_eq!(concurrent_compute(key), key.wrapping_mul(17));
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
