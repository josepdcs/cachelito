use cachelito::cache;
use std::cell::RefCell;

// Counter to verify how many times the function executes
thread_local! {
    static EXEC_COUNT: RefCell<usize> = RefCell::new(0);
}

// Using default policy (FIFO) by not specifying policy parameter
#[cache(limit = 2)]
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

    println!("\n=== Testing Default Cache Policy (FIFO) ===\n");

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

    // Call 3: miss -> cache: [2,3] (evicts 1 because it was added first - FIFO)
    println!("Calling compute_square(3)...");
    let result = compute_square(3);
    assert_eq!(result, 9);
    println!("Result: {}\n", result);

    // Call 4: hit -> cache: [2,3] (order doesn't change with FIFO)
    println!("Calling compute_square(2)...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {} (should be cached)\n", result);

    // Call 5: miss -> cache: [3,4] (evicts 2 because it was added first - FIFO)
    // Note: With FIFO (default), accessing 2 in call 4 doesn't move it to the end
    println!("Calling compute_square(4)...");
    let result = compute_square(4);
    assert_eq!(result, 16);
    println!("Result: {}\n", result);

    // Call 6: hit -> cache: [3,4] (order doesn't change)
    println!("Calling compute_square(3)...");
    let result = compute_square(3);
    assert_eq!(result, 9);
    println!("Result: {} (should be cached)\n", result);

    // Call 7: miss -> 2 was evicted in call 5, cache: [4,2] (evicts 3 - FIFO)
    println!("Calling compute_square(2)...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {} (should NOT be cached)\n", result);

    // Verify execution count
    let exec_count = EXEC_COUNT.with(|count| *count.borrow());
    println!("Total executions: {}", exec_count);

    // Expected: 5 executions (1, 2, 3, 4, 2)
    // Hits: compute_square(2) in call 4, compute_square(3) in call 6
    assert_eq!(
        exec_count, 5,
        "Expected 5 function executions but got {}",
        exec_count
    );

    println!("\n✅ Default Policy (FIFO) Test PASSED");
    println!("   The default policy is FIFO when no policy is specified.");
}
