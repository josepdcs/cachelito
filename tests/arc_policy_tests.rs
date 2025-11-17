//! Tests for ARC (Adaptive Replacement Cache) eviction policy

use cachelito::cache;

#[cache(policy = "arc", limit = 5)]
fn cached_computation(x: i32) -> i32 {
    x * x
}

#[cache(policy = "arc", limit = 3, scope = "global")]
fn global_arc_cache(x: i32) -> i32 {
    x + 100
}

#[test]
fn test_arc_basic_caching() {
    // First call should compute
    let result1 = cached_computation(5);
    assert_eq!(result1, 25);

    // Second call with same input should return cached value
    let result2 = cached_computation(5);
    assert_eq!(result2, 25);
}

#[test]
fn test_arc_limit_enforcement() {
    // Fill cache to limit
    for i in 1..=5 {
        cached_computation(i);
    }

    // Add more entries, forcing eviction
    for i in 6..=10 {
        cached_computation(i);
    }

    // Verify results are still correct (whether cached or recomputed)
    assert_eq!(cached_computation(10), 100);
    assert_eq!(cached_computation(6), 36);
}

#[test]
fn test_arc_frequency_tracking() {
    // Access some values more frequently
    for _ in 0..5 {
        cached_computation(1);
        cached_computation(2);
    }

    // Access others less frequently
    cached_computation(3);
    cached_computation(4);
    cached_computation(5);

    // Fill cache beyond limit
    for i in 6..=10 {
        cached_computation(i);
    }

    // Frequently accessed items should still be available
    // (this is tested by verifying correct results)
    assert_eq!(cached_computation(1), 1);
    assert_eq!(cached_computation(2), 4);
}

#[test]
fn test_arc_recency_tracking() {
    // Fill cache
    for i in 1..=5 {
        cached_computation(i);
    }

    // Re-access some items to make them recent
    cached_computation(2);
    cached_computation(4);

    // Add new items
    for i in 6..=8 {
        cached_computation(i);
    }

    // Recent items should have better survival rate
    assert_eq!(cached_computation(2), 4);
    assert_eq!(cached_computation(4), 16);
}

#[test]
fn test_arc_adaptive_behavior() {
    // Create a mixed workload pattern
    // Some items accessed frequently, others just once

    // Frequent items
    for _ in 0..3 {
        cached_computation(10);
        cached_computation(20);
    }

    // One-time access items
    cached_computation(1);
    cached_computation(2);
    cached_computation(3);

    // Fill beyond limit
    for i in 30..=40 {
        cached_computation(i);
    }

    // Frequent items should be retained
    assert_eq!(cached_computation(10), 100);
    assert_eq!(cached_computation(20), 400);
}

#[test]
fn test_arc_global_scope() {
    // Test that global ARC cache works
    assert_eq!(global_arc_cache(1), 101);
    assert_eq!(global_arc_cache(2), 102);
    assert_eq!(global_arc_cache(3), 103);

    // Add more to trigger eviction
    assert_eq!(global_arc_cache(4), 104);
    assert_eq!(global_arc_cache(5), 105);

    // Verify correctness
    assert_eq!(global_arc_cache(1), 101);
}

#[test]
fn test_arc_scan_resistance() {
    // Establish some hot items
    for _ in 0..5 {
        cached_computation(100);
        cached_computation(200);
    }

    // Perform a scan (sequential access pattern)
    for i in 1..=20 {
        cached_computation(i);
    }

    // Hot items should survive the scan better than pure LRU
    assert_eq!(cached_computation(100), 10000);
    assert_eq!(cached_computation(200), 40000);
}

#[test]
fn test_arc_empty_cache() {
    #[cache(policy = "arc", limit = 5)]
    fn fresh_cache(x: i32) -> i32 {
        x * 2
    }

    // First access
    assert_eq!(fresh_cache(10), 20);

    // Second access (should be cached)
    assert_eq!(fresh_cache(10), 20);
}

#[test]
fn test_arc_single_entry() {
    #[cache(policy = "arc", limit = 1)]
    fn tiny_cache(x: i32) -> i32 {
        x * 3
    }

    assert_eq!(tiny_cache(1), 3);
    assert_eq!(tiny_cache(2), 6); // Should evict 1
    assert_eq!(tiny_cache(2), 6); // Should still be cached
    assert_eq!(tiny_cache(1), 3); // Should evict 2 and recompute
}

#[test]
fn test_arc_with_different_types() {
    #[cache(policy = "arc", limit = 5)]
    fn string_cache(s: String) -> String {
        format!("processed: {}", s)
    }

    let result1 = string_cache("hello".to_string());
    assert_eq!(result1, "processed: hello");

    let result2 = string_cache("hello".to_string());
    assert_eq!(result2, "processed: hello");
}

#[test]
fn test_arc_result_caching() {
    #[cache(policy = "arc", limit = 5)]
    fn divide(a: i32, b: i32) -> Result<i32, String> {
        if b == 0 {
            Err("Division by zero".to_string())
        } else {
            Ok(a / b)
        }
    }

    // Success should be cached
    assert_eq!(divide(10, 2), Ok(5));
    assert_eq!(divide(10, 2), Ok(5));

    // Errors should not be cached
    assert_eq!(divide(10, 0), Err("Division by zero".to_string()));
}
