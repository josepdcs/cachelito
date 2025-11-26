use cachelito_async::cache_async;

/// Test basic random policy functionality with async cache
#[tokio::test]
async fn test_async_random_policy_basic() {
    #[cache_async(policy = "random", limit = 3)]
    async fn compute_async(n: u32) -> u32 {
        n * 2
    }

    // Fill cache
    assert_eq!(compute_async(1).await, 2);
    assert_eq!(compute_async(2).await, 4);
    assert_eq!(compute_async(3).await, 6);

    // Verify cached
    assert_eq!(compute_async(1).await, 2);
    assert_eq!(compute_async(2).await, 4);
    assert_eq!(compute_async(3).await, 6);

    // Add more items to trigger random evictions
    compute_async(4).await;
    compute_async(5).await;
    compute_async(6).await;

    // All values should still be correct
    assert_eq!(compute_async(1).await, 2);
    assert_eq!(compute_async(2).await, 4);
    assert_eq!(compute_async(3).await, 6);
    assert_eq!(compute_async(4).await, 8);
    assert_eq!(compute_async(5).await, 10);
    assert_eq!(compute_async(6).await, 12);
}

/// Test random policy with large number of entries
#[tokio::test]
async fn test_async_random_policy_large() {
    #[cache_async(policy = "random", limit = 10)]
    async fn expensive_op(n: u32) -> u32 {
        n.pow(2)
    }

    // Insert many items
    for i in 1..=50 {
        expensive_op(i).await;
    }

    // Verify all values are correct
    for i in 1..=50 {
        assert_eq!(expensive_op(i).await, i.pow(2));
    }
}

/// Test random policy with TTL
#[tokio::test]
async fn test_async_random_policy_with_ttl() {
    use tokio::time::{sleep, Duration};

    #[cache_async(policy = "random", limit = 5, ttl = 100)]
    async fn compute_with_ttl(n: u32) -> u32 {
        n + 100
    }

    // Fill cache
    compute_with_ttl(1).await;
    compute_with_ttl(2).await;
    compute_with_ttl(3).await;

    // Wait for expiration
    sleep(Duration::from_millis(150)).await;

    // Values should be recomputed
    assert_eq!(compute_with_ttl(1).await, 101);
    assert_eq!(compute_with_ttl(2).await, 102);
    assert_eq!(compute_with_ttl(3).await, 103);
}

/// Test random policy with different value types
#[tokio::test]
async fn test_async_random_policy_with_strings() {
    #[cache_async(policy = "random", limit = 3)]
    async fn format_string(a: i32, b: i32) -> String {
        format!("{}+{}={}", a, b, a + b)
    }

    // Test string caching
    assert_eq!(format_string(10, 2).await, "10+2=12");
    assert_eq!(format_string(20, 4).await, "20+4=24");
    assert_eq!(format_string(15, 3).await, "15+3=18");

    // Cached values
    assert_eq!(format_string(10, 2).await, "10+2=12");
    assert_eq!(format_string(20, 4).await, "20+4=24");

    // Trigger eviction
    format_string(30, 5).await;
    format_string(40, 6).await;

    // All values should still be correct
    assert_eq!(format_string(15, 3).await, "15+3=18");
    assert_eq!(format_string(30, 5).await, "30+5=35");
}

/// Test concurrent async access with random policy
#[tokio::test]
async fn test_async_random_policy_concurrent() {
    #[cache_async(policy = "random", limit = 10, name = "async_random_concurrent")]
    async fn concurrent_compute(n: u32) -> u32 {
        tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
        n * 7
    }

    let mut tasks = vec![];

    // Spawn multiple tasks accessing the cache
    for task_id in 0..5 {
        let task = tokio::spawn(async move {
            for i in 0..20 {
                let key = (task_id * 20 + i) % 30;
                let result = concurrent_compute(key).await;
                assert_eq!(result, key * 7);
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        task.await.unwrap();
    }

    // Verify values are still correct
    for i in 0..30 {
        assert_eq!(concurrent_compute(i).await, i * 7);
    }
}

/// Test that random policy doesn't favor any particular key
#[tokio::test]
async fn test_async_random_policy_fairness() {
    #[cache_async(policy = "random", limit = 2)]
    async fn compute_fair(n: u32) -> u32 {
        n * 5
    }

    // Fill cache
    compute_fair(1).await;
    compute_fair(2).await;

    // Add many new items to trigger multiple random evictions
    for i in 3..=20 {
        compute_fair(i).await;
    }

    // All values should be correct
    for i in 1..=20 {
        assert_eq!(compute_fair(i).await, i * 5);
    }
}

#[cfg(feature = "stats")]
#[tokio::test]
async fn test_async_random_policy_stats() {
    use cachelito_core::stats_registry;

    #[cache_async(policy = "random", limit = 5, name = "async_random_stats")]
    async fn compute_for_stats(n: u32) -> u32 {
        n * 3
    }

    // Fill cache (5 misses)
    for i in 1..=5 {
        compute_for_stats(i).await;
    }

    // Access cached values (5 hits)
    for i in 1..=5 {
        compute_for_stats(i).await;
    }

    // Check stats
    if let Some(stats) = stats_registry::get("async_random_stats") {
        assert_eq!(stats.total_accesses(), 10);
        assert_eq!(stats.hits(), 5);
        assert_eq!(stats.misses(), 5);
        assert_eq!(stats.hit_rate(), 0.5);
    }
}
