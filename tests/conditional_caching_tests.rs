// Tests for conditional caching with cache_if attribute
//
// These tests verify that the cache_if predicate function controls
// whether results are cached or not.

use cachelito::cache;
use std::sync::atomic::{AtomicU32, Ordering};

// Test 1: Only cache non-empty vectors
fn should_cache_non_empty(_key: &String, result: &Vec<i32>) -> bool {
    !result.is_empty()
}

static CALL_COUNT_1: AtomicU32 = AtomicU32::new(0);

#[cache(scope = "global", cache_if = should_cache_non_empty)]
fn get_numbers(count: usize) -> Vec<i32> {
    CALL_COUNT_1.fetch_add(1, Ordering::SeqCst);
    if count == 0 {
        vec![]
    } else {
        (0..count as i32).collect()
    }
}

#[test]
fn test_cache_if_empty_not_cached() {
    CALL_COUNT_1.store(0, Ordering::SeqCst);

    // Empty results should not be cached
    let result1 = get_numbers(0);
    assert_eq!(result1, vec![]);
    assert_eq!(CALL_COUNT_1.load(Ordering::SeqCst), 1);

    let result2 = get_numbers(0);
    assert_eq!(result2, vec![]);
    // Should execute again (not cached)
    assert_eq!(CALL_COUNT_1.load(Ordering::SeqCst), 2);

    // Non-empty results should be cached
    let result3 = get_numbers(3);
    assert_eq!(result3, vec![0, 1, 2]);
    assert_eq!(CALL_COUNT_1.load(Ordering::SeqCst), 3);

    let result4 = get_numbers(3);
    assert_eq!(result4, vec![0, 1, 2]);
    // Should use cache (not execute again)
    assert_eq!(CALL_COUNT_1.load(Ordering::SeqCst), 3);
}

// Test 2: Only cache Some values
fn cache_some(_key: &String, result: &Option<i32>) -> bool {
    result.is_some()
}

static CALL_COUNT_2: AtomicU32 = AtomicU32::new(0);

#[cache(scope = "thread", cache_if = cache_some)]
fn find_value(id: i32) -> Option<i32> {
    CALL_COUNT_2.fetch_add(1, Ordering::SeqCst);
    if id > 0 {
        Some(id * 2)
    } else {
        None
    }
}

#[test]
fn test_cache_if_none_not_cached() {
    CALL_COUNT_2.store(0, Ordering::SeqCst);

    // None values should not be cached
    let result1 = find_value(-5);
    assert_eq!(result1, None);
    assert_eq!(CALL_COUNT_2.load(Ordering::SeqCst), 1);

    let result2 = find_value(-5);
    assert_eq!(result2, None);
    // Should execute again (not cached)
    assert_eq!(CALL_COUNT_2.load(Ordering::SeqCst), 2);

    // Some values should be cached
    let result3 = find_value(10);
    assert_eq!(result3, Some(20));
    assert_eq!(CALL_COUNT_2.load(Ordering::SeqCst), 3);

    let result4 = find_value(10);
    assert_eq!(result4, Some(20));
    // Should use cache (not execute again)
    assert_eq!(CALL_COUNT_2.load(Ordering::SeqCst), 3);
}

// Test 3: Only cache successful results (not errors)
fn cache_success(_key: &String, result: &Result<String, String>) -> bool {
    result.is_ok()
}

static CALL_COUNT_3: AtomicU32 = AtomicU32::new(0);

#[cache(scope = "global", cache_if = cache_success)]
fn fetch_data(id: i32) -> Result<String, String> {
    CALL_COUNT_3.fetch_add(1, Ordering::SeqCst);
    if id > 0 {
        Ok(format!("Data for id {}", id))
    } else {
        Err("Invalid ID: must be positive".to_string())
    }
}

#[test]
fn test_cache_if_errors_not_cached() {
    CALL_COUNT_3.store(0, Ordering::SeqCst);

    // Error results should not be cached
    let result1 = fetch_data(-1);
    assert!(result1.is_err());
    assert_eq!(result1.unwrap_err(), "Invalid ID: must be positive");
    assert_eq!(CALL_COUNT_3.load(Ordering::SeqCst), 1);

    let result2 = fetch_data(-1);
    assert!(result2.is_err());
    // Should execute again (not cached)
    assert_eq!(CALL_COUNT_3.load(Ordering::SeqCst), 2);

    // Another error with different key
    let result3 = fetch_data(0);
    assert!(result3.is_err());
    assert_eq!(CALL_COUNT_3.load(Ordering::SeqCst), 3);

    let result4 = fetch_data(0);
    assert!(result4.is_err());
    // Should execute again (not cached)
    assert_eq!(CALL_COUNT_3.load(Ordering::SeqCst), 4);

    // Successful results should be cached
    let result5 = fetch_data(42);
    assert!(result5.is_ok());
    assert_eq!(result5.unwrap(), "Data for id 42");
    assert_eq!(CALL_COUNT_3.load(Ordering::SeqCst), 5);

    let result6 = fetch_data(42);
    assert!(result6.is_ok());
    // Should use cache (not execute again)
    assert_eq!(CALL_COUNT_3.load(Ordering::SeqCst), 5);
}

// Test 4: Only cache positive values
fn cache_positive(_key: &String, value: &i32) -> bool {
    *value > 0
}

static CALL_COUNT_4: AtomicU32 = AtomicU32::new(0);

#[cache(scope = "global", limit = 100, cache_if = cache_positive)]
fn compute(x: i32, y: i32) -> i32 {
    CALL_COUNT_4.fetch_add(1, Ordering::SeqCst);
    x + y
}

#[test]
fn test_cache_if_negative_not_cached() {
    CALL_COUNT_4.store(0, Ordering::SeqCst);

    // Negative results should not be cached
    let result1 = compute(-10, 5);
    assert_eq!(result1, -5);
    assert_eq!(CALL_COUNT_4.load(Ordering::SeqCst), 1);

    let result2 = compute(-10, 5);
    assert_eq!(result2, -5);
    // Should execute again (not cached)
    assert_eq!(CALL_COUNT_4.load(Ordering::SeqCst), 2);

    // Positive results should be cached
    let result3 = compute(10, 5);
    assert_eq!(result3, 15);
    assert_eq!(CALL_COUNT_4.load(Ordering::SeqCst), 3);

    let result4 = compute(10, 5);
    assert_eq!(result4, 15);
    // Should use cache (not execute again)
    assert_eq!(CALL_COUNT_4.load(Ordering::SeqCst), 3);

    // Zero should not be cached
    let result5 = compute(0, 0);
    assert_eq!(result5, 0);
    assert_eq!(CALL_COUNT_4.load(Ordering::SeqCst), 4);

    let result6 = compute(0, 0);
    assert_eq!(result6, 0);
    // Should execute again (not cached)
    assert_eq!(CALL_COUNT_4.load(Ordering::SeqCst), 5);
}
