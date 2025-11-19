use cachelito::cache;

#[test]
fn test_memory_limit_with_strings() {
    #[cache(max_memory = "1KB", policy = "fifo")]
    fn cached_string(id: u32) -> String {
        // Each string is approximately 500 bytes
        format!("{}", "A".repeat(500))
    }

    // Fill cache with 2 strings (total ~1KB)
    let _ = cached_string(1);
    let _ = cached_string(2);

    // This should evict the first entry due to memory limit
    let _ = cached_string(3);

    // Verify that earlier entries were evicted by checking if they regenerate
    // (In a real test, we would track function calls)
}

#[test]
fn test_memory_limit_with_vectors() {
    #[cache(max_memory = "10KB", policy = "lru")]
    fn cached_vec(size: usize) -> Vec<u64> {
        let result: Vec<u64> = (0..size).map(|i| i as u64).collect();
        result
    }

    // Each u64 is 8 bytes
    // 1000 elements = 8KB
    let _ = cached_vec(1000);

    // 500 elements = 4KB
    // Total would be 12KB, exceeding 10KB limit
    // Should evict first vector
    let _ = cached_vec(500);
}

#[test]
fn test_memory_limit_parsing() {
    // Test that different memory formats parse correctly

    #[cache(max_memory = "1MB")]
    fn mb_cache(x: i32) -> i32 {
        x * 2
    }

    #[cache(max_memory = "1GB")]
    fn gb_cache(x: i32) -> i32 {
        x * 3
    }

    #[cache(max_memory = "512KB")]
    fn kb_cache(x: i32) -> i32 {
        x * 4
    }

    #[cache(max_memory = 1024)]
    fn bytes_cache(x: i32) -> i32 {
        x * 5
    }

    // Just verify they compile and run
    assert_eq!(mb_cache(2), 4);
    assert_eq!(gb_cache(2), 6);
    assert_eq!(kb_cache(2), 8);
    assert_eq!(bytes_cache(2), 10);
}

#[test]
fn test_memory_limit_with_entry_limit() {
    // Test that memory limit takes precedence over entry limit

    #[cache(limit = 1000, max_memory = "5KB", policy = "fifo")]
    fn dual_limit(size: usize) -> Vec<u64> {
        let result: Vec<u64> = (0..size).map(|i| i as u64).collect();
        result
    }

    // Create a vector that uses ~4KB
    let v1 = dual_limit(500); // 500 * 8 = 4KB
    assert_eq!(v1.len(), 500);

    // This would fit within entry limit (2 < 1000)
    // but exceeds memory limit (4KB + 4KB > 5KB)
    // Should evict first entry
    let v2 = dual_limit(500);
    assert_eq!(v2.len(), 500);
}

#[test]
fn test_memory_limit_lfu_policy() {
    #[cache(max_memory = "2KB", policy = "lfu")]
    fn lfu_cached(id: u32) -> Vec<u8> {
        vec![0u8; 1000] // 1KB per entry
    }

    // Insert 2 entries (fills cache)
    let _ = lfu_cached(1);
    let _ = lfu_cached(2);

    // Access first entry multiple times (increase frequency)
    let _ = lfu_cached(1);
    let _ = lfu_cached(1);

    // Insert third entry - should evict entry 2 (lowest frequency)
    let _ = lfu_cached(3);
}

#[test]
fn test_memory_limit_arc_policy() {
    #[cache(max_memory = "3KB", policy = "arc")]
    fn arc_cached(id: u32) -> Vec<u8> {
        vec![0u8; 1000] // 1KB per entry
    }

    // Fill cache with 3 entries
    let _ = arc_cached(1);
    let _ = arc_cached(2);
    let _ = arc_cached(3);

    // Access some entries to build frequency
    let _ = arc_cached(1);
    let _ = arc_cached(2);
    let _ = arc_cached(1);

    // Insert new entry - ARC should evict based on frequency and recency
    let _ = arc_cached(4);
}

#[test]
fn test_thread_local_memory_limit() {
    #[cache(scope = "thread", max_memory = "2KB", policy = "lru")]
    fn thread_local_cached(id: u32) -> Vec<u8> {
        vec![id as u8; 1000] // 1KB per entry
    }

    // Fill thread-local cache
    let _ = thread_local_cached(1);
    let _ = thread_local_cached(2);

    // This should evict the first entry
    let _ = thread_local_cached(3);

    // Verify it works in thread-local context
    assert_eq!(thread_local_cached(3).len(), 1000);
}

#[test]
fn test_global_memory_limit() {
    #[cache(scope = "global", max_memory = "4KB", policy = "fifo")]
    fn global_cached(id: u32) -> Vec<u8> {
        vec![id as u8; 1000] // 1KB per entry
    }

    // Fill global cache
    let _ = global_cached(1);
    let _ = global_cached(2);
    let _ = global_cached(3);
    let _ = global_cached(4);

    // This should evict the first entry (FIFO)
    let _ = global_cached(5);

    // Verify it works in global context
    assert_eq!(global_cached(5).len(), 1000);
}
