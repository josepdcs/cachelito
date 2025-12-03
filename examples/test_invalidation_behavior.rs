use cachelito::cache;
use std::sync::atomic::{AtomicU64, Ordering};

fn is_admin(key: &String, val: &u64) -> bool {
    let result = key.contains("admin");
    println!("is_admin('{}', {}) -> {}", key, val, result);
    result
}

#[cache(scope = "global", name = "test_pred", invalidate_on = is_admin)]
fn get_count(key: String) -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    println!("get_count('{}') called, returning {}", key, count);
    count
}

fn main() {
    println!("=== Testing invalidation check behavior ===\n");

    println!("Call 1 with admin_1:");
    let val1 = get_count("admin_1".to_string());
    println!("Result: {}\n", val1);

    println!("Call 2 with admin_1 (check function should invalidate):");
    let val2 = get_count("admin_1".to_string());
    println!("Result: {}\n", val2);

    println!("Call 3 with admin_1:");
    let val3 = get_count("admin_1".to_string());
    println!("Result: {}\n", val3);
}
