/// Tests for edge cases in memory limit handling for async cache
///
/// This test file verifies that the async cache handles edge cases correctly,
/// particularly when values exceed the memory limit.
use cachelito_async::cache_async;
use cachelito_core::MemoryEstimator;
use tokio;


/// A type that allows us to control its memory size
#[derive(Debug, Clone)]
struct AsyncCustomSizedData {
    size: usize,
    value: String,
}

impl MemoryEstimator for AsyncCustomSizedData {
    fn estimate_memory(&self) -> usize {
        self.size
    }
}

/// Test that inserting a value larger than max_memory doesn't cause infinite loop
/// The value should simply not be cached
#[tokio::test]
async fn test_async_value_larger_than_max_memory() {
    #[cache_async(limit = 10, max_memory = 1024, policy = "lru")]
    async fn process_async_data(id: u64) -> AsyncCustomSizedData {
        AsyncCustomSizedData {
            size: 2048, // 2KB - larger than max_memory (1KB = 1024 bytes)
            value: format!("Async Data {}", id),
        }
    }

    // First call - value is too large, should not be cached
    let data1 = process_async_data(1).await;
    assert_eq!(data1.value, "Async Data 1");

    // Second call - should NOT be cached (should execute function again)
    let data2 = process_async_data(1).await;
    assert_eq!(data2.value, "Async Data 1");
}

/// Test that values at the boundary work correctly
#[tokio::test]
async fn test_async_value_at_max_memory_boundary() {
    #[cache_async(limit = 10, max_memory = 1024, policy = "fifo")]
    async fn process_boundary_data(id: u64) -> AsyncCustomSizedData {
        AsyncCustomSizedData {
            size: 1024, // Exactly 1KB - same as max_memory
            value: format!("Boundary {}", id),
        }
    }

    // This should work - value is exactly at the limit
    let data1 = process_boundary_data(1).await;
    assert_eq!(data1.value, "Boundary 1");

    // Second call should be cached
    let data2 = process_boundary_data(1).await;
    assert_eq!(data2.value, "Boundary 1");
}

/// Test that slightly smaller values work correctly
#[tokio::test]
async fn test_async_value_just_under_max_memory() {
    #[cache_async(limit = 10, max_memory = 1024, policy = "lru")]
    async fn process_small_data(id: u64) -> AsyncCustomSizedData {
        AsyncCustomSizedData {
            size: 512, // 512B - half of max_memory
            value: format!("Small {}", id),
        }
    }

    // These should be cached
    let data1 = process_small_data(1).await;
    assert_eq!(data1.value, "Small 1");

    let data2 = process_small_data(2).await;
    assert_eq!(data2.value, "Small 2");

    // Both should still be in cache since total is 1KB
    let data1_again = process_small_data(1).await;
    assert_eq!(data1_again.value, "Small 1");
}

/// Test eviction when adding a value that requires eviction
#[tokio::test]
async fn test_async_memory_eviction_on_insert() {
    #[cache_async(limit = 10, max_memory = 1024, policy = "lru")]
    async fn process_evicting_data(id: u64) -> AsyncCustomSizedData {
        AsyncCustomSizedData {
            size: 600, // 600B each
            value: format!("Evict {}", id),
        }
    }

    // First entry - 600B
    process_evicting_data(1).await;

    // Second entry - would be 1200B total, exceeds 1KB
    // Should evict first entry
    process_evicting_data(2).await;

    // Third entry - should work without hanging
    process_evicting_data(3).await;
}

/// Test LFU policy with memory limit edge case
#[tokio::test]
async fn test_async_lfu_with_large_value() {
    #[cache_async(limit = 5, max_memory = 2048, policy = "lfu")]
    async fn process_lfu_data(id: u64) -> AsyncCustomSizedData {
        AsyncCustomSizedData {
            size: 3000, // 3KB - larger than max_memory (2KB)
            value: format!("LFU {}", id),
        }
    }

    // Should not cache, but should not hang either
    let data1 = process_lfu_data(1).await;
    assert_eq!(data1.value, "LFU 1");
}

/// Test ARC policy with memory limit edge case
#[tokio::test]
async fn test_async_arc_with_large_value() {
    #[cache_async(limit = 5, max_memory = 2048, policy = "arc")]
    async fn process_arc_data(id: u64) -> AsyncCustomSizedData {
        AsyncCustomSizedData {
            size: 4096, // 4KB - larger than max_memory (2KB)
            value: format!("ARC {}", id),
        }
    }

    // Should not cache, but should not hang either
    let data1 = process_arc_data(1).await;
    assert_eq!(data1.value, "ARC 1");
}

/// Test concurrent inserts with oversized values
#[tokio::test]
async fn test_async_concurrent_large_values() {
    #[cache_async(limit = 10, max_memory = 1024, policy = "lru")]
    async fn process_concurrent_data(id: u64) -> AsyncCustomSizedData {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        AsyncCustomSizedData {
            size: 5000, // 5KB - much larger than max_memory
            value: format!("Concurrent {}", id),
        }
    }

    // Launch multiple concurrent tasks
    let handles: Vec<_> = (1..=5)
        .map(|i| tokio::spawn(async move { process_concurrent_data(i).await }))
        .collect();

    // Wait for all to complete - should not hang
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.value.starts_with("Concurrent"));
    }
}
