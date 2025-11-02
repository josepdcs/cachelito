use cachelito_macros::cache;
use std::cell::RefCell;

// Counter to verify how many times the function executes
thread_local! {
    static EXEC_COUNT: RefCell<usize> = RefCell::new(0);
}

#[cache(limit = 2, policy = "lru")]
fn compute_square(x: u32) -> u32 {
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() += 1;
    });
    println!("Executing compute_square({x})");
    x * x
}

fn main() {
    // Reset counter
    EXEC_COUNT.with(|count| {
        *count.borrow_mut() = 0;
    });

    println!("\n=== Testing LRU Cache Policy ===\n");

    // Call 1: miss -> cache: [1]
    println!("Calling compute_square(1)...");
    let result = compute_square(1);
    assert_eq!(result, 1);
    println!("Result: {}\n", result);

    // Call 2: miss -> cache: [1,2]
    println!("Calling compute_square(2)...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {}\n", result);

    // Call 3: miss -> cache: [2,3] (evicts 1 because limit=2)
    println!("Calling compute_square(3)...");
    let result = compute_square(3);
    assert_eq!(result, 9);
    println!("Result: {}\n", result);

    // Call 4: hit -> cache: [3,2] (2 moves to end because LRU)
    println!("Calling compute_square(2)...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {} (should be cached)\n", result);

    // Call 5: miss -> cache: [2,4] (evicts 3, not 2, because 2 was recently used)
    println!("Calling compute_square(4)...");
    let result = compute_square(4);
    assert_eq!(result, 16);
    println!("Result: {}\n", result);

    // Call 6: hit -> cache: [2,4] (4 stays at the end)
    println!("Calling compute_square(4)...");
    let result = compute_square(4);
    assert_eq!(result, 16);
    println!("Result: {} (should be cached)\n", result);

    // Call 7: hit -> cache: [4,2] (2 moves to end because used)
    println!("Calling compute_square(2)...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {} (should be cached)\n", result);

    // Call 8: miss -> cache: [2,1] (evicts 4, because 2 is more recent)
    println!("Calling compute_square(1)...");
    let result = compute_square(1);
    assert_eq!(result, 1);
    println!("Result: {}\n", result);

    // Call 9: miss -> 4 was evicted, cache: [1,4]
    println!("Calling compute_square(4)...");
    let result = compute_square(4);
    assert_eq!(result, 16);
    println!("Result: {}\n", result);

    // Verify execution count
    let exec_count = EXEC_COUNT.with(|count| *count.borrow());
    println!("Total executions: {}", exec_count);

    // Expected: 6 executions (1, 2, 3, 4, 1, 4)
    // Hits: compute_square(2) in call 4, compute_square(4) in call 6, compute_square(2) in call 7
    assert_eq!(
        exec_count, 6,
        "Expected 6 function executions but got {}",
        exec_count
    );

    println!("\n✅ LRU Policy Test PASSED");
}
