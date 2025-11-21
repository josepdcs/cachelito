/// Example demonstrating async cache without MemoryEstimator
///
/// This example shows how the async cache can be used without implementing
/// the MemoryEstimator trait when max_memory is not specified.
///
/// Run with: cargo run --example async_basic --features async
use cachelito_async::cache_async;
use std::sync::Arc;
use tokio::time::Duration;


#[derive(Debug, Clone)]
#[allow(dead_code)]
struct User {
    id: u64,
    name: String,
    email: String,
}

/// Simple async function with cache (no MemoryEstimator required)
#[cache_async(limit = 100, policy = "lru")]
async fn fetch_user(id: u64) -> User {
    // Simulate database query
    println!("Fetching user {} from database...", id);
    tokio::time::sleep(Duration::from_millis(100)).await;

    User {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    }
}

/// Async function returning Arc (no MemoryEstimator required)
#[cache_async(limit = 50, policy = "fifo")]
async fn fetch_user_arc(id: u64) -> Arc<User> {
    println!("Fetching Arc user {} from database...", id);
    tokio::time::sleep(Duration::from_millis(100)).await;

    Arc::new(User {
        id,
        name: format!("Arc User {}", id),
        email: format!("arc.user{}@example.com", id),
    })
}

#[tokio::main]
async fn main() {
    println!("=== Async Cache Without MemoryEstimator ===\n");

    // First call - cache miss (will fetch from "database")
    println!("1. First call for user 1:");
    let user1 = fetch_user(1).await;
    println!("   Result: {:?}\n", user1);

    // Second call - cache hit (instant)
    println!("2. Second call for user 1 (should be cached):");
    let start = std::time::Instant::now();
    let user1_cached = fetch_user(1).await;
    let elapsed = start.elapsed();
    println!("   Result: {:?}", user1_cached);
    println!("   Elapsed: {:?} (should be < 1ms)\n", elapsed);

    // Different user - cache miss
    println!("3. First call for user 2:");
    let user2 = fetch_user(2).await;
    println!("   Result: {:?}\n", user2);

    // Test Arc version
    println!("4. Fetching Arc user 10:");
    let arc_user = fetch_user_arc(10).await;
    println!("   Result: {:?}\n", arc_user);

    println!("5. Fetching Arc user 10 again (cached):");
    let start = std::time::Instant::now();
    let arc_user_cached = fetch_user_arc(10).await;
    let elapsed = start.elapsed();
    println!("   Result: {:?}", arc_user_cached);
    println!("   Elapsed: {:?} (should be < 1ms)\n", elapsed);

    // Verify Arc points to same data
    println!("6. Arc reference counting:");
    println!("   Strong count: {}", Arc::strong_count(&arc_user_cached));

    println!("\n=== Done ===");
}
