use cachelito_async::cache_async;
use cachelito_core::MemoryEstimator;
use tokio::time::Duration;

/// A data type that implements MemoryEstimator
/// This test ensures that the cache works with MemoryEstimator when max_memory is specified
#[derive(Debug, Clone)]
struct LargeData {
    id: u64,
    payload: Vec<u8>,
}

impl MemoryEstimator for LargeData {
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>() + self.payload.capacity()
    }
}

/// Test that cache_async uses insert_with_memory when max_memory is specified
#[cache_async(max_memory = "10KB", policy = "lru")]
async fn fetch_large_data(id: u64) -> LargeData {
    tokio::time::sleep(Duration::from_millis(10)).await;
    LargeData {
        id,
        payload: vec![0u8; 1024], // 1KB of data
    }
}

/// Test with both limit and max_memory
#[cache_async(limit = 100, max_memory = "50KB", policy = "fifo")]
async fn fetch_data_with_both_limits(id: u64) -> LargeData {
    tokio::time::sleep(Duration::from_millis(5)).await;
    LargeData {
        id,
        payload: vec![0u8; 512], // 512 bytes
    }
}

#[tokio::test]
async fn test_async_cache_with_memory_limit() {
    // First call - cache miss
    let data1 = fetch_large_data(1).await;
    assert_eq!(data1.id, 1);
    assert_eq!(data1.payload.len(), 1024);

    // Second call - cache hit
    let data2 = fetch_large_data(1).await;
    assert_eq!(data2.id, 1);
    assert_eq!(data2.payload.len(), 1024);

    // Different argument - cache miss
    let data3 = fetch_large_data(2).await;
    assert_eq!(data3.id, 2);
    assert_eq!(data3.payload.len(), 1024);
}

#[tokio::test]
async fn test_async_cache_memory_eviction() {
    // Fill cache with data that exceeds memory limit
    // max_memory is 10KB, each entry is ~1KB, so only ~10 entries should fit
    for i in 1..=15 {
        fetch_large_data(i).await;
    }

    // Cache should have evicted older entries to stay within memory limit
    // We can't directly check memory usage, but we can verify it still works
    let data = fetch_large_data(15).await;
    assert_eq!(data.id, 15);
}

#[tokio::test]
async fn test_async_cache_with_both_limits() {
    // Test that both entry count and memory limits work together
    for i in 1..=50 {
        fetch_data_with_both_limits(i).await;
    }

    // Verify cache still works
    let data = fetch_data_with_both_limits(50).await;
    assert_eq!(data.id, 50);
    assert_eq!(data.payload.len(), 512);
}

#[tokio::test]
async fn test_async_cache_memory_limit_lru_eviction() {
    // Add several entries
    for i in 1..=5 {
        fetch_large_data(i).await;
    }

    // Access some entries to make them recently used (LRU policy)
    fetch_large_data(1).await;
    fetch_large_data(2).await;

    // Add more entries to trigger eviction
    for i in 6..=12 {
        fetch_large_data(i).await;
    }

    // Recently accessed entries might still be in cache (depending on memory limit)
    // But we can verify the cache still works correctly
    let data = fetch_large_data(12).await;
    assert_eq!(data.id, 12);
}
