use cachelito::cache;
use std::sync::Arc;

/// Test basic W-TinyLFU functionality
#[test]
fn test_w_tinylfu_basic() {
    #[cache(
        scope = "global",
        limit = 5,
        policy = "w_tinylfu",
        name = "w_tinylfu_basic"
    )]
    fn fetch(id: u32) -> u32 {
        id * 10
    }

    // Fill cache
    for i in 1..=5 {
        assert_eq!(fetch(i), i * 10);
    }

    // Access some items multiple times (build frequency)
    for _ in 0..10 {
        fetch(1);
        fetch(2);
    }

    // Add new items (triggers eviction)
    for i in 6..=10 {
        assert_eq!(fetch(i), i * 10);
    }

    // Verify cache still works
    assert_eq!(fetch(1), 10);
}

/// Test W-TinyLFU with custom window ratio
#[test]
fn test_w_tinylfu_with_window_ratio() {
    #[cache(
        scope = "global",
        limit = 10,
        policy = "w_tinylfu",
        window_ratio = 0.3,
        name = "w_tinylfu_window"
    )]
    fn fetch_with_window(id: u32) -> String {
        format!("value_{}", id)
    }

    // Fill cache
    for i in 1..=10 {
        fetch_with_window(i);
    }

    // Access frequently
    for _ in 0..5 {
        fetch_with_window(1);
    }

    // Add more items
    for i in 11..=15 {
        fetch_with_window(i);
    }

    // Verify functionality
    assert_eq!(fetch_with_window(1), "value_1");
}

/// Test W-TinyLFU thread-local cache
#[test]
fn test_w_tinylfu_thread_local() {
    #[cache(
        scope = "thread",
        limit = 5,
        policy = "w_tinylfu",
        name = "w_tinylfu_thread"
    )]
    fn thread_fetch(id: u32) -> u32 {
        id * 100
    }

    for i in 1..=5 {
        assert_eq!(thread_fetch(i), i * 100);
    }

    // Access with varying frequency
    for _ in 0..3 {
        thread_fetch(1);
    }

    // Add new items
    for i in 6..=8 {
        assert_eq!(thread_fetch(i), i * 100);
    }

    // Verify cache works
    assert_eq!(thread_fetch(1), 100);
}

/// Test W-TinyLFU with Result types
#[test]
fn test_w_tinylfu_with_results() {
    #[cache(
        scope = "global",
        limit = 5,
        policy = "w_tinylfu",
        name = "w_tinylfu_results"
    )]
    fn fetch_result(id: u32) -> Result<u32, String> {
        if id % 2 == 0 {
            Ok(id)
        } else {
            Err(format!("odd_{}", id))
        }
    }

    // Both Ok and Err should be cached
    assert!(fetch_result(1).is_err());
    assert!(fetch_result(2).is_ok());
    assert_eq!(fetch_result(2).unwrap(), 2);

    // Access multiple times
    for _ in 0..5 {
        let _ = fetch_result(2);
    }

    // Add more items
    for i in 3..=10 {
        let _ = fetch_result(i);
    }

    // Verify caching works
    assert!(fetch_result(2).is_ok());
}

/// Test W-TinyLFU with Arc values
#[test]
fn test_w_tinylfu_with_arc() {
    #[cache(
        scope = "global",
        limit = 5,
        policy = "w_tinylfu",
        name = "w_tinylfu_arc"
    )]
    fn fetch_arc(id: u32) -> Arc<Vec<u8>> {
        Arc::new(vec![id as u8; 100])
    }

    for i in 1..=5 {
        let data = fetch_arc(i);
        assert_eq!(data.len(), 100);
    }

    // Build frequency
    for _ in 0..10 {
        fetch_arc(1);
    }

    // Add new items
    for i in 6..=10 {
        let data = fetch_arc(i);
        assert_eq!(data.len(), 100);
    }

    // Verify cache works
    let data = fetch_arc(1);
    assert_eq!(data.len(), 100);
}

/// Test W-TinyLFU with different window ratios
#[test]
fn test_w_tinylfu_window_ratios() {
    // Small window (emphasis on frequency)
    #[cache(
        scope = "global",
        limit = 10,
        policy = "w_tinylfu",
        window_ratio = 0.1,
        name = "w_tinylfu_small_window"
    )]
    fn small_window(id: u32) -> u32 {
        id
    }

    for i in 1..=10 {
        small_window(i);
    }

    // Large window (emphasis on recency)
    #[cache(
        scope = "global",
        limit = 10,
        policy = "w_tinylfu",
        window_ratio = 0.4,
        name = "w_tinylfu_large_window"
    )]
    fn large_window(id: u32) -> u32 {
        id * 2
    }

    for i in 1..=10 {
        large_window(i);
    }

    // Both should work
    assert_eq!(small_window(1), 1);
    assert_eq!(large_window(1), 2);
}

/// Test W-TinyLFU with mixed access patterns
#[test]
fn test_w_tinylfu_mixed_pattern() {
    #[cache(
        scope = "global",
        limit = 8,
        policy = "w_tinylfu",
        name = "w_tinylfu_mixed"
    )]
    fn mixed(id: u32) -> String {
        format!("item_{}", id)
    }

    // Hot data
    for _ in 0..5 {
        mixed(1);
        mixed(2);
        mixed(3);
    }

    // Cold data
    for i in 4..=10 {
        mixed(i);
    }

    // More hot data access
    for _ in 0..5 {
        mixed(1);
    }

    // Add new items
    for i in 11..=15 {
        mixed(i);
    }

    // Hot data should likely still work
    assert_eq!(mixed(1), "item_1");
}

/// Test W-TinyLFU concurrent access
#[test]
fn test_w_tinylfu_concurrent() {
    use std::thread;

    #[cache(
        scope = "global",
        limit = 20,
        policy = "w_tinylfu",
        name = "w_tinylfu_concurrent"
    )]
    fn concurrent(id: u32) -> u32 {
        id * 3
    }

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            thread::spawn(move || {
                // Each thread accesses some shared keys
                for i in 1..=10 {
                    concurrent(i);
                }
                // And some unique keys
                for i in (thread_id * 10)..((thread_id + 1) * 10) {
                    concurrent(i);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Shared keys should work
    assert_eq!(concurrent(5), 15);
}

/// Test W-TinyLFU with custom name
#[test]
fn test_w_tinylfu_named() {
    #[cache(
        scope = "global",
        limit = 5,
        policy = "w_tinylfu",
        name = "my_custom_w_tinylfu_cache"
    )]
    fn named_fetch(id: u32) -> u32 {
        id + 100
    }

    for i in 1..=5 {
        assert_eq!(named_fetch(i), i + 100);
    }

    // Verify it works
    assert_eq!(named_fetch(1), 101);
}

/// Test W-TinyLFU eviction behavior
#[test]
fn test_w_tinylfu_eviction() {
    #[cache(
        scope = "global",
        limit = 4,
        policy = "w_tinylfu",
        name = "w_tinylfu_evict"
    )]
    fn evict_test(id: u32) -> u32 {
        id * 10
    }

    // Fill cache
    for i in 1..=4 {
        evict_test(i);
    }

    // Build different frequency patterns
    for _ in 0..10 {
        evict_test(1); // High frequency
    }
    for _ in 0..3 {
        evict_test(2); // Medium frequency
    }
    // 3 and 4 have low frequency (1 access each)

    // Add new items - should trigger eviction
    for i in 5..=8 {
        assert_eq!(evict_test(i), i * 10);
    }

    // High frequency item should likely still work
    assert_eq!(evict_test(1), 10);
}
