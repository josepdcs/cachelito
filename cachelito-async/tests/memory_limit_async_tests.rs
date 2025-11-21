use cachelito_async::cache_async;

#[tokio::test]
async fn test_async_memory_limit() {
    #[cache_async(max_memory = "2KB", policy = "lru")]
    async fn cached_data(id: u32) -> Vec<u8> {
        // Each vec is ~1KB
        vec![id as u8; 1000]
    }

    // Fill cache with 2 entries
    let _ = cached_data(1).await;
    let _ = cached_data(2).await;

    // This should evict the least recently used entry
    let _ = cached_data(3).await;

    // Verify result is correct
    let result = cached_data(3).await;
    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], 3);
}

#[tokio::test]
async fn test_async_memory_limit_with_entry_limit() {
    #[cache_async(limit = 100, max_memory = "3KB", policy = "fifo")]
    async fn dual_limit_async(size: usize) -> Vec<u64> {
        let result: Vec<u64> = (0..size).map(|i| i as u64).collect();
        result
    }

    // Create vectors that fit within entry limit but not memory limit
    let v1 = dual_limit_async(300).await; // ~2.4KB
    assert_eq!(v1.len(), 300);

    let v2 = dual_limit_async(200).await; // ~1.6KB
    assert_eq!(v2.len(), 200);

    // Total would be ~4KB, exceeding 3KB limit
    // First entry should be evicted
}

#[tokio::test]
async fn test_async_memory_limit_lfu() {
    #[cache_async(max_memory = "4KB", policy = "lfu")]
    async fn lfu_async_cached(id: u32) -> Vec<u8> {
        vec![id as u8; 1000] // 1KB each
    }

    // Fill cache
    let _ = lfu_async_cached(1).await;
    let _ = lfu_async_cached(2).await;
    let _ = lfu_async_cached(3).await;
    let _ = lfu_async_cached(4).await;

    // Access some entries multiple times
    let _ = lfu_async_cached(1).await;
    let _ = lfu_async_cached(1).await;
    let _ = lfu_async_cached(2).await;

    // Insert new entry - should evict least frequently used
    let _ = lfu_async_cached(5).await;
}

#[tokio::test]
async fn test_async_memory_limit_arc() {
    #[cache_async(max_memory = "3KB", policy = "arc")]
    async fn arc_async_cached(id: u32) -> Vec<u8> {
        vec![id as u8; 1000] // 1KB each
    }

    // Fill cache
    let _ = arc_async_cached(1).await;
    let _ = arc_async_cached(2).await;
    let _ = arc_async_cached(3).await;

    // Access pattern
    let _ = arc_async_cached(1).await;
    let _ = arc_async_cached(2).await;
    let _ = arc_async_cached(1).await;

    // Insert new - ARC evicts based on adaptive algorithm
    let _ = arc_async_cached(4).await;
}

#[tokio::test]
async fn test_async_memory_parsing() {
    #[cache_async(max_memory = "1MB")]
    async fn mb_async(x: i32) -> i32 {
        x * 2
    }

    #[cache_async(max_memory = "1GB")]
    async fn gb_async(x: i32) -> i32 {
        x * 3
    }

    #[cache_async(max_memory = "512KB")]
    async fn kb_async(x: i32) -> i32 {
        x * 4
    }

    assert_eq!(mb_async(2).await, 4);
    assert_eq!(gb_async(2).await, 6);
    assert_eq!(kb_async(2).await, 8);
}
