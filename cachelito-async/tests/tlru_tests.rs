use cachelito_async::cache_async;
use tokio::time::{sleep, Duration};

#[cache_async(policy = "tlru", limit = 5, ttl = 2)]
async fn async_compute(n: u32) -> u32 {
    n * 2
}

#[cache_async(policy = "tlru", limit = 3, ttl = 1)]
async fn async_compute_short_ttl(n: u32) -> u32 {
    n * 3
}

#[tokio::test]
async fn test_tlru_basic_async() {
    // Fill cache to limit
    for i in 0..5 {
        async_compute(i).await;
    }

    // All should be cached
    for i in 0..5 {
        assert_eq!(async_compute(i).await, i * 2);
    }

    // Add one more, should evict based on TLRU score
    async_compute(10).await;

    // The new one should be cached
    assert_eq!(async_compute(10).await, 20);
}

#[tokio::test]
async fn test_tlru_frequency_matters_async() {
    #[cache_async(policy = "tlru", limit = 3, ttl = 10)]
    async fn freq_compute(n: u32) -> u32 {
        n * 4
    }

    // Fill cache
    freq_compute(1).await;
    freq_compute(2).await;
    freq_compute(3).await;

    // Access first two multiple times (increase frequency)
    for _ in 0..5 {
        freq_compute(1).await;
        freq_compute(2).await;
    }

    // Add new entry, should evict freq_compute(3) (low frequency)
    freq_compute(4).await;

    // Items 1 and 2 should still be cached (high frequency)
    assert_eq!(freq_compute(1).await, 4);
    assert_eq!(freq_compute(2).await, 8);
    assert_eq!(freq_compute(4).await, 16);
}

#[tokio::test]
async fn test_tlru_age_matters_with_ttl_async() {
    #[cache_async(policy = "tlru", limit = 3, ttl = 1)]
    async fn age_compute(n: u32) -> u32 {
        n * 5
    }

    // Fill cache
    age_compute(1).await;
    age_compute(2).await;

    // Sleep to make first entries older
    sleep(Duration::from_millis(600)).await;

    age_compute(3).await;

    // Add new entry, older one should be prioritized for eviction
    age_compute(4).await;

    assert_eq!(age_compute(4).await, 20);
}

#[tokio::test]
async fn test_tlru_with_expiration_async() {
    #[cache_async(policy = "tlru", limit = 5, ttl = 1)]
    async fn expiring_compute(n: u32) -> u32 {
        n * 6
    }

    // Cache value
    expiring_compute(1).await;
    assert_eq!(expiring_compute(1).await, 6);

    // Wait for expiration
    sleep(Duration::from_secs(2)).await;

    // Should be expired and recalculated
    assert_eq!(expiring_compute(1).await, 6);
}

#[tokio::test]
async fn test_tlru_evicts_approaching_ttl_async() {
    #[cache_async(policy = "tlru", limit = 2, ttl = 1)]
    async fn ttl_evict(n: u32) -> u32 {
        n * 7
    }

    // Add first entry
    ttl_evict(1).await;

    // Sleep close to TTL
    sleep(Duration::from_millis(800)).await;

    // Add second entry (fresher)
    ttl_evict(2).await;

    // Increase frequency of entry 2
    ttl_evict(2).await;

    // Add third entry, should evict entry 1 (older, approaching TTL)
    ttl_evict(3).await;

    // Entry 2 should still be there (fresher, higher frequency)
    assert_eq!(ttl_evict(2).await, 14);
    assert_eq!(ttl_evict(3).await, 21);
}

#[tokio::test]
async fn test_tlru_no_ttl_behaves_like_arc_async() {
    #[cache_async(policy = "tlru", limit = 3)]
    async fn no_ttl_compute(n: u32) -> u32 {
        n * 8
    }

    // Fill cache
    no_ttl_compute(1).await;
    no_ttl_compute(2).await;
    no_ttl_compute(3).await;

    // Increase frequency of 1 and 2
    for _ in 0..3 {
        no_ttl_compute(1).await;
        no_ttl_compute(2).await;
    }

    // Add new entry, should evict entry 3 (lowest frequency)
    no_ttl_compute(4).await;

    // Check that high frequency entries remain
    assert_eq!(no_ttl_compute(1).await, 8);
    assert_eq!(no_ttl_compute(2).await, 16);
    assert_eq!(no_ttl_compute(4).await, 32);
}

#[tokio::test]
async fn test_tlru_concurrent_access() {
    #[cache_async(policy = "tlru", limit = 5, ttl = 3)]
    async fn concurrent_compute(n: u32) -> u32 {
        n * 10
    }

    // Spawn multiple tasks accessing the cache concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            tokio::spawn(async move {
                for j in 0..5 {
                    concurrent_compute(i % 5 + j).await;
                }
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify some cached values
    assert_eq!(concurrent_compute(0).await, 0);
    assert_eq!(concurrent_compute(1).await, 10);
}

#[tokio::test]
async fn test_tlru_recency_vs_frequency_async() {
    #[cache_async(policy = "tlru", limit = 3, ttl = 5)]
    async fn balanced_compute(n: u32) -> u32 {
        n * 9
    }

    // Fill cache
    balanced_compute(1).await;
    balanced_compute(2).await;

    // Make entry 1 very frequent
    for _ in 0..10 {
        balanced_compute(1).await;
    }

    balanced_compute(3).await;

    // Add new entry
    balanced_compute(4).await;

    // Entry 1 should remain (high frequency despite age)
    assert_eq!(balanced_compute(1).await, 9);
}

#[tokio::test]
async fn test_tlru_low_frequency_weight_async() {
    // Low frequency_weight (0.3) means frequency has less impact
    // Recency and age matter more
    #[cache_async(policy = "tlru", limit = 3, ttl = 10, frequency_weight = 0.3)]
    async fn low_freq_compute(n: u32) -> u32 {
        n * 100
    }

    // Fill cache
    low_freq_compute(1).await;
    low_freq_compute(2).await;
    low_freq_compute(3).await;

    // Make entry 1 very frequent
    for _ in 0..10 {
        low_freq_compute(1).await;
    }

    // Wait to age entry 1
    sleep(Duration::from_millis(100)).await;

    // Add new entry (cache is full)
    low_freq_compute(4).await;

    // With low frequency_weight, entry 1 might be evicted
    // despite high frequency because recency matters more
    assert_eq!(low_freq_compute(4).await, 400);
}

#[tokio::test]
async fn test_tlru_high_frequency_weight_async() {
    // High frequency_weight (1.5) means frequency has more impact
    // Popular entries are protected from eviction
    #[cache_async(policy = "tlru", limit = 3, ttl = 10, frequency_weight = 1.5)]
    async fn high_freq_compute(n: u32) -> u32 {
        n * 200
    }

    // Fill cache
    high_freq_compute(1).await;
    high_freq_compute(2).await;
    high_freq_compute(3).await;

    // Make entry 1 very frequent
    for _ in 0..10 {
        high_freq_compute(1).await;
    }

    // Wait to age entry 1
    sleep(Duration::from_millis(100)).await;

    // Add new entry (cache is full)
    high_freq_compute(4).await;

    // With high frequency_weight, entry 1 should remain
    // because its high frequency protects it
    assert_eq!(high_freq_compute(1).await, 200);
    assert_eq!(high_freq_compute(4).await, 800);
}

#[tokio::test]
async fn test_tlru_frequency_weight_comparison_async() {
    // Test with default frequency_weight (no parameter)
    #[cache_async(policy = "tlru", limit = 2, ttl = 5)]
    async fn default_compute(n: u32) -> u32 {
        n * 50
    }

    // Test with custom frequency_weight
    #[cache_async(policy = "tlru", limit = 2, ttl = 5, frequency_weight = 2.0)]
    async fn custom_compute(n: u32) -> u32 {
        n * 60
    }

    // Fill both caches
    default_compute(1).await;
    default_compute(2).await;
    custom_compute(1).await;
    custom_compute(2).await;

    // Increase frequency for entry 1 in both
    for _ in 0..5 {
        default_compute(1).await;
        custom_compute(1).await;
    }

    sleep(Duration::from_millis(50)).await;

    // Add new entries
    default_compute(3).await;
    custom_compute(3).await;

    // Both should work correctly with their respective weights
    assert_eq!(default_compute(3).await, 150);
    assert_eq!(custom_compute(3).await, 180);
}

#[tokio::test]
async fn test_tlru_concurrent_with_frequency_weight_async() {
    #[cache_async(policy = "tlru", limit = 5, ttl = 3, frequency_weight = 1.2)]
    async fn concurrent_freq_compute(n: u32) -> u32 {
        n * 15
    }

    // Spawn multiple tasks with different access patterns
    let mut handles = vec![];

    // Task 1: Access entry 1 frequently
    for _ in 0..5 {
        handles.push(tokio::spawn(async {
            concurrent_freq_compute(1).await;
        }));
    }

    // Task 2: Access various entries
    for i in 2..=6 {
        handles.push(tokio::spawn(async move {
            concurrent_freq_compute(i).await;
        }));
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Entry 1 should remain cached due to high frequency and frequency_weight > 1.0
    assert_eq!(concurrent_freq_compute(1).await, 15);
}
