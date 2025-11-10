use cachelito_async::cache_async;
use std::sync::atomic::{AtomicUsize, Ordering};

static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cache_async]
async fn simple(x: u32) -> u32 {
    EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
    x + 1
}

#[tokio::main]
async fn main() {
    // First call
    let r1 = simple(42).await;
    assert_eq!(r1, 43);

    // Second call - should use cache
    let r2 = simple(42).await;
    assert_eq!(r2, 43);

    // Third call - different arg
    let r3 = simple(100).await;
    assert_eq!(r3, 101);

    let count = EXEC_COUNT.load(Ordering::SeqCst);
    println!("Execution count: {} (expected: 2)", count);

    if count == 2 {
        println!("✅ Cache is working!");
    } else {
        println!("❌ Cache NOT working! Got {} executions", count);
    }
}
