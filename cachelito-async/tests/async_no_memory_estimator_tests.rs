use cachelito_async::cache_async;
use std::sync::Arc;
use tokio::time::Duration;

/// A simple data type that does NOT implement MemoryEstimator
/// This test ensures that the cache works without MemoryEstimator when max_memory is not specified
#[derive(Debug, Clone)]
struct SimpleData {
    id: u64,
    value: String,
}

/// Test that cache_async works without MemoryEstimator when max_memory is not specified
#[cache_async(limit = 10, policy = "lru")]
async fn fetch_simple_data(id: u64) -> SimpleData {
    tokio::time::sleep(Duration::from_millis(10)).await;
    SimpleData {
        id,
        value: format!("Data {}", id),
    }
}

/// Test that cache_async works with Arc without MemoryEstimator when max_memory is not specified
#[cache_async(limit = 10, policy = "fifo")]
async fn fetch_arc_data(id: u64) -> Arc<SimpleData> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Arc::new(SimpleData {
        id,
        value: format!("Arc Data {}", id),
    })
}

#[tokio::test]
async fn test_async_cache_without_memory_estimator() {
    // First call - cache miss
    let data1 = fetch_simple_data(1).await;
    assert_eq!(data1.id, 1);
    assert_eq!(data1.value, "Data 1");

    // Second call - cache hit (should be instant)
    let data2 = fetch_simple_data(1).await;
    assert_eq!(data2.id, 1);
    assert_eq!(data2.value, "Data 1");

    // Different argument - cache miss
    let data3 = fetch_simple_data(2).await;
    assert_eq!(data3.id, 2);
    assert_eq!(data3.value, "Data 2");
}

#[tokio::test]
async fn test_async_cache_arc_without_memory_estimator() {
    // First call - cache miss
    let data1 = fetch_arc_data(10).await;
    assert_eq!(data1.id, 10);
    assert_eq!(data1.value, "Arc Data 10");

    // Second call - cache hit
    let data2 = fetch_arc_data(10).await;
    assert_eq!(data2.id, 10);
    assert_eq!(data2.value, "Arc Data 10");
}

#[tokio::test]
async fn test_async_cache_limit_without_memory_estimator() {
    // Fill the cache beyond its limit
    for i in 1..=15 {
        fetch_simple_data(i).await;
    }

    // The cache should have evicted older entries (FIFO policy has limit=10)
    // We can't directly inspect the cache size from here,
    // but we can verify it still works correctly
    let data = fetch_simple_data(15).await;
    assert_eq!(data.id, 15);
}
