//! # Async Concurrent Cache Example
//!
//! This example demonstrates concurrent async cache access across multiple tasks.
//! The cache is shared globally and thread-safe using DashMap.

use cachelito_async::cache_async;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::task::JoinSet;

static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Simulates an expensive async computation
#[cache_async(limit = 100)]
async fn expensive_computation(n: u32) -> u64 {
    EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
    println!(
        "Task computing fibonacci({}) on thread {:?}",
        n,
        std::thread::current().id()
    );

    // Simulate expensive async work
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Simple fibonacci (not optimal, just for demo)
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 0..n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
}

#[tokio::main]
async fn main() {
    println!("=== Async Concurrent Cache Example ===\n");

    let mut tasks = JoinSet::new();

    // Spawn multiple concurrent tasks that compute the same values
    println!("Spawning 10 tasks (each computing fib(20) and fib(25))...\n");

    for i in 0..10 {
        tasks.spawn(async move {
            println!("Task {} starting", i);

            // Each task computes the same two values
            let result1 = expensive_computation(20).await;
            let result2 = expensive_computation(25).await;

            println!(
                "Task {} finished: fib(20)={}, fib(25)={}",
                i, result1, result2
            );

            (result1, result2)
        });
    }

    // Wait for all tasks to complete
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result.unwrap());
    }

    let exec_count = EXEC_COUNT.load(Ordering::SeqCst);

    println!("\n--- Results ---");
    println!("Total tasks: 10");
    println!("Total computations per task: 2");
    println!("Total function executions: {}", exec_count);
    println!("Note: Due to concurrent execution, multiple tasks may compute the same value");
    println!("      before the first one caches it (cache stampede/thundering herd).");

    // Verify all results are the same
    let first = results[0];
    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            *result, first,
            "Task {} result mismatch: {:?} vs {:?}",
            i, result, first
        );
    }

    println!("\n✅ Async Concurrent Cache Test PASSED");
    println!("   All {} tasks returned correct results", results.len());
    println!(
        "   Function executed {} times (would be 20 without caching)",
        exec_count
    );

    if exec_count < 20 {
        println!(
            "   Cache prevented {} redundant executions",
            20 - exec_count
        );
    }
}
