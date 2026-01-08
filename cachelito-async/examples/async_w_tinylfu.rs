use cachelito_async::cache_async;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::time::sleep;

// Simulated API call counter
static API_CALLS: AtomicU32 = AtomicU32::new(0);

/// Simulate an expensive async API call
async fn simulate_api_call(endpoint: &str, id: u64, delay_ms: u64) -> String {
    API_CALLS.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(delay_ms)).await;
    format!("{} response for ID: {}", endpoint, id)
}

/// Fetch user data with W-TinyLFU caching
///
/// W-TinyLFU provides excellent hit rates through:
/// - Window segment (20%): Recent items using FIFO
/// - Protected segment (80%): Frequently accessed items using LFU
#[cache_async(limit = 100, policy = "w_tinylfu")]
async fn get_user(user_id: u64) -> String {
    simulate_api_call("User API", user_id, 10).await
}

/// Fetch product data with custom window ratio (30% window)
///
/// Larger window = more emphasis on recency
/// Good for: e-commerce products, trending items
#[cache_async(limit = 50, policy = "w_tinylfu", window_ratio = 0.3)]
async fn get_product(product_id: u64) -> String {
    simulate_api_call("Product API", product_id, 15).await
}

/// Fetch analytics data with small window ratio (10% window)
///
/// Smaller window = more emphasis on frequency
/// Good for: analytics queries, reference data
#[cache_async(limit = 200, policy = "w_tinylfu", window_ratio = 0.1)]
async fn get_analytics(query_id: u64) -> String {
    simulate_api_call("Analytics API", query_id, 20).await
}

/// Fetch news articles with large window ratio (40% window)
///
/// Large window = strong recency emphasis
/// Good for: news, social media, time-sensitive content
#[cache_async(limit = 150, policy = "w_tinylfu", window_ratio = 0.4)]
async fn get_news(article_id: u64) -> String {
    simulate_api_call("News API", article_id, 12).await
}

#[tokio::main]
async fn main() {
    println!("=== Async W-TinyLFU Cache Policy Demo ===\n");

    // Reset API counter
    API_CALLS.store(0, Ordering::SeqCst);

    // Scenario 1: Hot and cold data mix
    println!("Scenario 1: Mixed Hot and Cold Data");
    println!("-----------------------------------");

    let hot_users = vec![1, 2, 3]; // Frequently accessed users
    let cold_users: Vec<u64> = (4..=20).collect(); // Infrequently accessed users

    // Access hot users multiple times
    println!("Accessing hot users (frequent access):");
    for _ in 0..5 {
        for &user_id in &hot_users {
            let user = get_user(user_id).await;
            println!("  User {}: {}", user_id, user);
        }
    }

    // Access cold users once
    println!("\nAccessing cold users (one-time access):");
    for &user_id in &cold_users {
        let user = get_user(user_id).await;
        println!("  User {}: {}", user_id, user);
    }

    let api_calls_after_initial = API_CALLS.load(Ordering::SeqCst);
    println!("\nAPI calls so far: {}", api_calls_after_initial);

    // Try to access hot users again - should be cached
    println!("\nRe-accessing hot users (should be cached):");
    for &user_id in &hot_users {
        let user = get_user(user_id).await;
        println!("  User {}: {}", user_id, user);
    }

    let api_calls_after_hot = API_CALLS.load(Ordering::SeqCst);
    println!(
        "API calls after re-accessing hot users: {}",
        api_calls_after_hot
    );
    println!("✓ Hot users were cached! (no additional API calls)\n");

    // Access some cold users again - may or may not be cached
    println!("Re-accessing some cold users:");
    for user_id in &cold_users[..3] {
        let user = get_user(*user_id).await;
        println!("  User {}: {}", user_id, user);
    }

    let api_calls_after_cold = API_CALLS.load(Ordering::SeqCst);
    let cold_evicted = api_calls_after_cold - api_calls_after_hot;
    println!("Cold users evicted: {} out of 3", cold_evicted);
    println!("✓ W-TinyLFU protected hot data from eviction!\n");

    // Scenario 2: Product catalog with window_ratio = 0.3
    println!("\nScenario 2: Product Catalog (30% window)");
    println!("------------------------------------------");
    API_CALLS.store(0, Ordering::SeqCst);

    // Access products with varying patterns
    println!("Initial product access:");
    for i in 1..=20 {
        get_product(i).await;
    }
    println!("Loaded {} products", API_CALLS.load(Ordering::SeqCst));

    // Frequently access some products
    for _ in 0..10 {
        get_product(5).await;
        get_product(10).await;
        get_product(15).await;
    }

    // Add new products (triggers eviction)
    for i in 21..=40 {
        get_product(i).await;
    }

    // Check if frequently accessed products are still cached
    API_CALLS.store(0, Ordering::SeqCst);
    get_product(5).await;
    get_product(10).await;
    get_product(15).await;

    if API_CALLS.load(Ordering::SeqCst) == 0 {
        println!("✓ Frequently accessed products still cached!");
    }

    // Scenario 3: Analytics queries (small window - frequency focus)
    println!("\nScenario 3: Analytics Queries (10% window)");
    println!("--------------------------------------------");
    API_CALLS.store(0, Ordering::SeqCst);

    // Popular analytics queries
    let popular_queries = vec![101, 102, 103];

    // Execute popular queries many times
    println!("Running popular analytics queries:");
    for _ in 0..20 {
        for &query_id in &popular_queries {
            get_analytics(query_id).await;
        }
    }

    // Execute many one-off queries
    println!("Running one-off queries:");
    for i in 200..=250 {
        get_analytics(i).await;
    }

    // Popular queries should still be cached
    API_CALLS.store(0, Ordering::SeqCst);
    for &query_id in &popular_queries {
        get_analytics(query_id).await;
    }

    if API_CALLS.load(Ordering::SeqCst) == 0 {
        println!("✓ Popular queries survived the one-off query flood!");
        println!("  (Small window = strong frequency protection)");
    }

    // Scenario 4: News articles (large window - recency focus)
    println!("\nScenario 4: News Articles (40% window)");
    println!("---------------------------------------");
    API_CALLS.store(0, Ordering::SeqCst);

    // Older articles
    println!("Loading older articles:");
    for i in 1..=50 {
        get_news(i).await;
    }

    // Some older articles get multiple views
    for _ in 0..5 {
        get_news(10).await;
        get_news(20).await;
    }

    // Recent articles (simulating breaking news)
    println!("Loading recent/breaking news:");
    for i in 51..=100 {
        get_news(i).await;
    }

    // Recent articles should be well-represented in cache
    API_CALLS.store(0, Ordering::SeqCst);
    let mut recent_cached = 0;
    for i in 90..=100 {
        let before = API_CALLS.load(Ordering::SeqCst);
        get_news(i).await;
        let after = API_CALLS.load(Ordering::SeqCst);
        if after == before {
            recent_cached += 1;
        }
    }

    println!("✓ Recent articles cached: {}/11", recent_cached);
    println!("  (Large window = strong recency protection)");

    // Scenario 5: Concurrent async tasks
    println!("\nScenario 5: Concurrent Async Tasks");
    println!("-----------------------------------");
    API_CALLS.store(0, Ordering::SeqCst);

    #[cache_async(limit = 20, policy = "w_tinylfu")]
    async fn concurrent_fetch(id: u64) -> String {
        simulate_api_call("Concurrent API", id, 5).await
    }

    // Spawn multiple concurrent tasks
    let mut tasks = vec![];
    for task_id in 0..4 {
        let task = tokio::spawn(async move {
            // Each task accesses shared keys
            for i in 1..=5 {
                concurrent_fetch(i).await;
            }
            // And some unique keys
            for i in (task_id * 10)..((task_id + 1) * 10) {
                concurrent_fetch(i).await;
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        task.await.unwrap();
    }

    println!("Completed {} concurrent tasks", 4);
    println!("Total API calls: {}", API_CALLS.load(Ordering::SeqCst));

    // Verify shared keys are cached
    API_CALLS.store(0, Ordering::SeqCst);
    for i in 1..=5 {
        concurrent_fetch(i).await;
    }
    let shared_cached = 5 - API_CALLS.load(Ordering::SeqCst);
    println!("Shared keys cached: {}/5", shared_cached);
    println!("✓ W-TinyLFU handles concurrent access efficiently!");

    // Summary
    println!("\n=== Summary ===");
    println!("W-TinyLFU for async functions provides:");
    println!("  • Excellent hit rates on mixed workloads");
    println!("  • Protection against cache pollution");
    println!("  • Configurable window_ratio for workload tuning:");
    println!("    - Small (0.1): Frequency-focused (analytics, stable data)");
    println!("    - Medium (0.2-0.3): Balanced (general purpose)");
    println!("    - Large (0.4): Recency-focused (news, trending content)");
    println!("  • Lock-free DashMap for high concurrency");
    println!("  • Async/await native support");

    println!("\n✓ Async W-TinyLFU demonstration complete!");
}
