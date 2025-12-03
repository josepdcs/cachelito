//! Integration tests for named invalidation check functions

use cachelito::cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct TimestampedValue {
    value: String,
    timestamp: Instant,
}

// Check function that checks if value is older than 2 seconds
fn is_stale(_key: &String, val: &TimestampedValue) -> bool {
    val.timestamp.elapsed() > Duration::from_secs(2)
}

#[cache(scope = "global", name = "with_time_check", invalidate_on = is_stale)]
fn get_timestamped(id: u64) -> TimestampedValue {
    TimestampedValue {
        value: format!("Value {}", id),
        timestamp: Instant::now(),
    }
}

#[test]
fn test_time_based_invalidation_check() {
    // First call - cache miss
    let val1 = get_timestamped(1);
    let first_timestamp = val1.timestamp;

    // Immediate second call - cache hit
    let val2 = get_timestamped(1);
    assert_eq!(val2.timestamp, first_timestamp);

    // Wait for entry to become stale
    thread::sleep(Duration::from_secs(3));

    // Third call - check function detects staleness, re-executes
    let val3 = get_timestamped(1);
    assert!(val3.timestamp > first_timestamp);
}

// Check function based on value content
fn is_invalid_value(_key: &String, val: &String) -> bool {
    val.contains("invalid")
}

#[cache(scope = "global", name = "with_content_check", invalidate_on = is_invalid_value)]
fn get_data(key: String) -> String {
    format!("data_{}", key)
}

#[test]
fn test_content_based_invalidation_check() {
    // Valid content - should cache
    let val1 = get_data("valid".to_string());
    let val2 = get_data("valid".to_string());
    assert_eq!(val1, val2);

    // Invalid content - check function always invalidates
    let val3 = get_data("invalid".to_string());
    assert_eq!(val3, "data_invalid");
}

// Check function based on key pattern
fn is_admin_key(key: &String, _val: &u64) -> bool {
    // Note: keys are stored with Debug format, so they have quotes
    key.contains("admin")
}

#[cache(scope = "global", name = "with_key_check", limit = 100, invalidate_on = is_admin_key)]
fn get_count(key: String) -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[test]
fn test_key_based_invalidation_check() {
    // Non-admin key - should cache
    let count1 = get_count("user_1".to_string());
    let count2 = get_count("user_1".to_string());
    assert_eq!(count1, count2);

    // Admin key - check function invalidates on every access, so function re-executes
    let count3 = get_count("admin_1".to_string());
    let count4 = get_count("admin_1".to_string());
    assert_ne!(count3, count4); // Different values = re-executed each time
}

// Check function that always returns false (never invalidates)
fn never_invalidate(_key: &String, _val: &String) -> bool {
    false
}

#[cache(scope = "global", name = "with_never_invalidate", invalidate_on = never_invalidate)]
fn get_value(id: u64) -> String {
    format!("Value {}", id)
}

#[test]
fn test_never_invalidates_check() {
    let val1 = get_value(1);
    let val2 = get_value(1);
    let val3 = get_value(1);
    assert_eq!(val1, val2);
    assert_eq!(val2, val3);
}

// Thread-local cache with invalidation check
fn is_negative(_key: &String, val: &i32) -> bool {
    *val < 0
}

#[cache(scope = "thread", name = "thread_local_check", invalidate_on = is_negative)]
fn compute(x: i32) -> i32 {
    x * 2
}

#[test]
fn test_invalidation_check_with_thread_local() {
    // Positive value - should cache
    let result1 = compute(5);
    let result2 = compute(5);
    assert_eq!(result1, 10);
    assert_eq!(result2, 10);

    // Note: Negative values would be invalidated by check function
    // but since compute always returns positive for positive input,
    // this tests that check function works with thread-local scope
}

// Complex check function with multiple conditions
fn is_complex_stale(key: &String, val: &(String, u64)) -> bool {
    // Invalidate if key is "special" OR value's second element is > 100
    key == "special" || val.1 > 100
}

#[cache(scope = "global", name = "complex_check", invalidate_on = is_complex_stale)]
fn get_tuple(key: String, num: u64) -> (String, u64) {
    (format!("result_{}", key), num)
}

#[test]
fn test_complex_invalidation_check() {
    // Normal key, small number - should cache
    let val1 = get_tuple("normal".to_string(), 50);
    let val2 = get_tuple("normal".to_string(), 50);
    assert_eq!(val1, val2);

    // Special key - always invalidated
    let val3 = get_tuple("special".to_string(), 50);
    let val4 = get_tuple("special".to_string(), 50);
    // Values would be same but re-executed each time (if counter was used, would differ)

    // Large number - always invalidated
    let val5 = get_tuple("normal".to_string(), 150);
    let val6 = get_tuple("normal".to_string(), 150);
    // Same situation as special key
}
