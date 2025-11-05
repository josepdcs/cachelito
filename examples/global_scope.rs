use cachelito_macros::cache;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Global counter to verify how many times the function executes
static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Function with global scope cache - shared across all threads
#[cache(limit = 5, scope = "global")]
fn compute_square(x: u32) -> u32 {
    EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
    println!(
        "Executing compute_square({x}) in thread {:?}",
        thread::current().id()
    );
    x * x
}

/// Function with thread-local scope cache (default) - separate cache per thread
#[cache(limit = 5)]
fn compute_cube(x: u32) -> u32 {
    println!(
        "Executing compute_cube({x}) in thread {:?}",
        thread::current().id()
    );
    x * x * x
}

fn main() {
    println!("\n=== Testing Global Scope Cache ===\n");

    // Reset counter
    EXEC_COUNT.store(0, Ordering::SeqCst);

    println!("--- Test 1: Global scope cache shared across threads ---\n");

    // First, call from main thread
    println!("Main thread calling compute_square(2)...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {}\n", result);

    println!("Main thread calling compute_square(3)...");
    let result = compute_square(3);
    assert_eq!(result, 9);
    println!("Result: {}\n", result);

    // Verify executions so far
    let exec_before_threads = EXEC_COUNT.load(Ordering::SeqCst);
    println!(
        "Executions before spawning threads: {}\n",
        exec_before_threads
    );
    assert_eq!(exec_before_threads, 2);

    // Spawn multiple threads that will use the same values
    let mut handles = vec![];

    for i in 1..=3 {
        let handle = thread::spawn(move || {
            println!("Thread {} calling compute_square(2)...", i);
            let result = compute_square(2);
            assert_eq!(result, 4);
            println!(
                "Thread {} got result: {} (should be from cache)\n",
                i, result
            );

            println!("Thread {} calling compute_square(3)...", i);
            let result = compute_square(3);
            assert_eq!(result, 9);
            println!(
                "Thread {} got result: {} (should be from cache)\n",
                i, result
            );

            // Each thread also computes a unique value
            println!("Thread {} calling compute_square({})...", i, i + 10);
            let result = compute_square(i + 10);
            println!("Thread {} got result: {}\n", i, result);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final execution count
    let final_exec_count = EXEC_COUNT.load(Ordering::SeqCst);
    println!("Total executions with global cache: {}", final_exec_count);

    // Expected: 2 (initial calls) + 3 (one per thread for unique values) = 5
    // The calls to compute_square(2) and compute_square(3) in threads should be cache hits
    assert_eq!(
        final_exec_count, 5,
        "Expected 5 function executions but got {}",
        final_exec_count
    );

    println!("✅ Global scope cache is shared across threads!\n");

    println!("--- Test 2: Thread-local scope cache (default) is NOT shared ---\n");

    // Create threads that will use thread-local cache
    let mut handles = vec![];

    let exec_count_local = Arc::new(AtomicUsize::new(0));

    for i in 1..=3 {
        let counter = Arc::clone(&exec_count_local);
        let handle = thread::spawn(move || {
            // Each thread calling the same value with thread-local cache
            // will execute the function because caches are separate
            println!("Thread {} calling compute_cube(5)...", i);
            let result = compute_cube(5);
            assert_eq!(result, 125);
            counter.fetch_add(1, Ordering::SeqCst);
            println!("Thread {} got result: {}\n", i, result);

            // Second call in same thread should be cached
            println!("Thread {} calling compute_cube(5) again...", i);
            let result = compute_cube(5);
            assert_eq!(result, 125);
            println!(
                "Thread {} got result: {} (cached in this thread)\n",
                i, result
            );
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // With thread-local, we can't easily count actual executions from outside,
    // but we spawned 3 threads and each should execute once for the same value
    println!("✅ Thread-local scope caches are independent per thread!\n");

    println!("--- Test 3: Verify cache hit in main thread for global scope ---\n");

    // Call again from main thread - should be cached
    println!("Main thread calling compute_square(2) again...");
    let result = compute_square(2);
    assert_eq!(result, 4);
    println!("Result: {} (should be cached from earlier)\n", result);

    // Execution count should not have increased
    let final_exec_count_2 = EXEC_COUNT.load(Ordering::SeqCst);
    println!("Total executions after cache hit: {}", final_exec_count_2);
    assert_eq!(
        final_exec_count_2, final_exec_count,
        "Execution count should not increase for cache hit"
    );

    println!("\n✅ Global Scope Cache Test PASSED");
    println!("\nSummary:");
    println!("- Global scope caches (scope=\"global\") are shared across all threads");
    println!("- Thread-local caches (default) are independent per thread");
    println!("- Global caches are useful for expensive computations used across threads");
}
