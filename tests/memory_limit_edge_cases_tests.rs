/// Tests for edge cases in memory limit handling
///
/// This test file verifies that the cache handles edge cases correctly,
/// particularly when values exceed the memory limit.
use cachelito::cache;
use cachelito_core::MemoryEstimator;


/// A type that allows us to control its memory size
#[derive(Debug, Clone)]
struct CustomSizedData {
    size: usize,
    value: String,
}

impl MemoryEstimator for CustomSizedData {
    fn estimate_memory(&self) -> usize {
        self.size
    }
}

/// Test that inserting a value larger than max_memory doesn't cause infinite loop
/// The value should simply not be cached
#[test]
fn test_value_larger_than_max_memory_thread_local() {
    #[cache(limit = 10, max_memory = "1KB", policy = "lru")]
    fn process_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 2048, // 2KB - larger than max_memory (1KB)
            value: format!("Data {}", id),
        }
    }

    // First call - value is too large, should not be cached
    let data1 = process_data(1);
    assert_eq!(data1.value, "Data 1");

    // Second call - should NOT be cached (should execute function again)
    // We can't directly verify this without instrumenting the function,
    // but at least we verify it doesn't crash or hang
    let data2 = process_data(1);
    assert_eq!(data2.value, "Data 1");
}

/// Test that inserting a value larger than max_memory in global scope doesn't cause infinite loop
#[test]
fn test_value_larger_than_max_memory_global() {
    #[cache(scope = "global", limit = 10, max_memory = 500, policy = "lru")]
    fn process_global_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 1024, // 1KB - larger than max_memory (500 bytes)
            value: format!("Global Data {}", id),
        }
    }

    // First call - value is too large, should not be cached
    let data1 = process_global_data(1);
    assert_eq!(data1.value, "Global Data 1");

    // Second call - should NOT be cached
    let data2 = process_global_data(1);
    assert_eq!(data2.value, "Global Data 1");
}

/// Test that values at the boundary work correctly
#[test]
fn test_value_at_max_memory_boundary() {
    #[cache(limit = 10, max_memory = "1KB", policy = "fifo")]
    fn process_boundary_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 1024, // Exactly 1KB - same as max_memory
            value: format!("Boundary {}", id),
        }
    }

    // This should work - value is exactly at the limit
    let data1 = process_boundary_data(1);
    assert_eq!(data1.value, "Boundary 1");

    // Second call should be cached
    let data2 = process_boundary_data(1);
    assert_eq!(data2.value, "Boundary 1");
}

/// Test that slightly smaller values work correctly
#[test]
fn test_value_just_under_max_memory() {
    #[cache(limit = 10, max_memory = "1KB", policy = "lru")]
    fn process_small_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 512, // 512B - half of max_memory
            value: format!("Small {}", id),
        }
    }

    // These should be cached
    let data1 = process_small_data(1);
    assert_eq!(data1.value, "Small 1");

    let data2 = process_small_data(2);
    assert_eq!(data2.value, "Small 2");

    // Both should still be in cache since total is 1KB
    let data1_again = process_small_data(1);
    assert_eq!(data1_again.value, "Small 1");
}

/// Test eviction when adding a value that requires eviction
#[test]
fn test_memory_eviction_on_insert() {
    #[cache(limit = 10, max_memory = "1KB", policy = "lru")]
    fn process_evicting_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 600, // 600B each
            value: format!("Evict {}", id),
        }
    }

    // First entry - 600B
    process_evicting_data(1);

    // Second entry - would be 1200B total, exceeds 1KB
    // Should evict first entry
    process_evicting_data(2);

    // Third entry - should work without hanging
    process_evicting_data(3);
}

/// Test LFU policy with memory limit edge case
#[test]
fn test_lfu_with_large_value() {
    #[cache(limit = 5, max_memory = "2KB", policy = "lfu")]
    fn process_lfu_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 3000, // 3KB - larger than max_memory
            value: format!("LFU {}", id),
        }
    }

    // Should not cache, but should not hang either
    let data1 = process_lfu_data(1);
    assert_eq!(data1.value, "LFU 1");
}

/// Test ARC policy with memory limit edge case
#[test]
fn test_arc_with_large_value() {
    #[cache(limit = 5, max_memory = "2KB", policy = "arc")]
    fn process_arc_data(id: u64) -> CustomSizedData {
        CustomSizedData {
            size: 4096, // 4KB - larger than max_memory
            value: format!("ARC {}", id),
        }
    }

    // Should not cache, but should not hang either
    let data1 = process_arc_data(1);
    assert_eq!(data1.value, "ARC 1");
}
