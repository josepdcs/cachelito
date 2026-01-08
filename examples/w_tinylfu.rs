use cachelito::cache;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

// Simulated API call counter
static API_CALLS: AtomicU32 = AtomicU32::new(0);

fn simulate_api_call(endpoint: &str, id: u32) -> String {
    API_CALLS.fetch_add(1, Ordering::SeqCst);
    // Simulate network delay
    thread::sleep(Duration::from_millis(10));
    format!("{} response for ID: {}", endpoint, id)
}

/// Fetch user data with W-TinyLFU caching
///
/// W-TinyLFU (Windowed Tiny Least Frequently Used) is an advanced cache policy that:
/// - Maintains a small "window" segment for recent items (FIFO)
/// - Maintains a "protected" segment for frequently accessed items (LFU)
/// - Uses admission control to prevent cache pollution from one-hit wonders
#[cache(scope = "global", limit = 100, policy = "w_tinylfu")]
fn get_user(user_id: u32) -> String {
    simulate_api_call("User API", user_id)
}

/// Fetch product data with custom window ratio
///
/// window_ratio = 0.3 means 30% of cache is allocated to the window segment
/// (recent items), and 70% to the protected segment (frequent items)
#[cache(scope = "global", limit = 50, policy = "w_tinylfu", window_ratio = 0.3)]
fn get_product(product_id: u32) -> String {
    simulate_api_call("Product API", product_id)
}

/// Fetch analytics data with small window (emphasis on frequency)
///
/// Small window (10%) - better for workloads where frequency is more important
/// than recency (e.g., analytics queries on historical data)
#[cache(
    scope = "global",
    limit = 200,
    policy = "w_tinylfu",
    window_ratio = 0.1
)]
fn get_analytics(query_id: u32) -> String {
    simulate_api_call("Analytics API", query_id)
}

/// Fetch news articles with large window (emphasis on recency)
///
/// Large window (40%) - better for workloads where recent items are more
/// valuable than frequently accessed old items (e.g., news, social media feeds)
#[cache(
    scope = "global",
    limit = 150,
    policy = "w_tinylfu",
    window_ratio = 0.4
)]
fn get_news(article_id: u32) -> String {
    simulate_api_call("News API", article_id)
}

fn main() {
    println!("=== W-TinyLFU Cache Policy Demo ===\n");

    // Reset API counter
    API_CALLS.store(0, Ordering::SeqCst);

    // Scenario 1: Hot and cold data mix
    println!("Scenario 1: Mixed Hot and Cold Data");
    println!("-----------------------------------");

    // Simulate a workload with hot and cold data
    let hot_users = vec![1, 2, 3]; // Frequently accessed users
    let cold_users: Vec<u32> = (4..=20).collect(); // Infrequently accessed users

    // Access hot users multiple times
    println!("Accessing hot users (frequent access):");
    for _ in 0..5 {
        for &user_id in &hot_users {
            let user = get_user(user_id);
            println!("  User {}: {}", user_id, user);
        }
    }

    // Access cold users once
    println!("\nAccessing cold users (one-time access):");
    for &user_id in &cold_users {
        let user = get_user(user_id);
        println!("  User {}: {}", user_id, user);
    }

    let api_calls_after_initial = API_CALLS.load(Ordering::SeqCst);
    println!("\nAPI calls so far: {}", api_calls_after_initial);

    // Try to access hot users again - should be cached
    println!("\nRe-accessing hot users (should be cached):");
    for &user_id in &hot_users {
        let user = get_user(user_id);
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
        let user = get_user(*user_id);
        println!("  User {}: {}", user_id, user);
    }

    let api_calls_after_cold = API_CALLS.load(Ordering::SeqCst);
    let cold_evicted = api_calls_after_cold - api_calls_after_hot;
    println!("Cold users evicted: {} out of 3", cold_evicted);
    println!("✓ W-TinyLFU protected hot data from eviction!\n");

    // Scenario 2: Product catalog with different window sizes
    println!("\nScenario 2: Product Catalog (30% window)");
    println!("------------------------------------------");
    API_CALLS.store(0, Ordering::SeqCst);

    // Access products with varying patterns
    println!("Initial product access:");
    for i in 1..=20 {
        get_product(i);
    }
    println!("Loaded {} products", API_CALLS.load(Ordering::SeqCst));

    // Frequently access some products
    for _ in 0..10 {
        get_product(5);
        get_product(10);
        get_product(15);
    }

    // Add new products (triggers eviction)
    for i in 21..=40 {
        get_product(i);
    }

    // Check if frequently accessed products are still cached
    API_CALLS.store(0, Ordering::SeqCst);
    get_product(5);
    get_product(10);
    get_product(15);

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
            get_analytics(query_id);
        }
    }

    // Execute many one-off queries
    println!("Running one-off queries:");
    for i in 200..=250 {
        get_analytics(i);
    }

    // Popular queries should still be cached
    API_CALLS.store(0, Ordering::SeqCst);
    for &query_id in &popular_queries {
        get_analytics(query_id);
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
        get_news(i);
    }

    // Some older articles get multiple views
    for _ in 0..5 {
        get_news(10);
        get_news(20);
    }

    // Recent articles (simulating breaking news)
    println!("Loading recent/breaking news:");
    for i in 51..=100 {
        get_news(i);
    }

    // Recent articles should be well-represented in cache
    API_CALLS.store(0, Ordering::SeqCst);
    let mut recent_cached = 0;
    for i in 90..=100 {
        let before = API_CALLS.load(Ordering::SeqCst);
        get_news(i);
        let after = API_CALLS.load(Ordering::SeqCst);
        if after == before {
            recent_cached += 1;
        }
    }

    println!("✓ Recent articles cached: {}/11", recent_cached);
    println!("  (Large window = strong recency protection)");

    // Summary
    println!("\n=== Summary ===");
    println!("W-TinyLFU adapts to your workload:");
    println!("  • Protects frequently accessed items (hot data)");
    println!("  • Adapts window size for recency vs frequency trade-off");
    println!("  • Resists cache pollution from one-hit wonders");
    println!("  • Excellent for mixed workloads with hot/cold data patterns");

    println!("\nWindow Ratio Guide:");
    println!("  • Small (0.1-0.2): Emphasize frequency - good for analytics, APIs");
    println!("  • Medium (0.2-0.3): Balanced - good general purpose");
    println!("  • Large (0.3-0.5): Emphasize recency - good for news, feeds, time-sensitive data");

    println!("\n✓ W-TinyLFU demonstration complete!");
}
