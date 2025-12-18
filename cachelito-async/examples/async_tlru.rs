//! Example demonstrating TLRU (Time-aware Least Recently Used) eviction policy with async functions.
//!
//! TLRU combines three factors to make eviction decisions:
//! - **Recency**: How recently the entry was accessed
//! - **Frequency**: How often the entry has been accessed (adjustable with frequency_weight)
//! - **Age**: How close the entry is to its TTL expiration
//!
//! The `frequency_weight` parameter allows you to control the importance of access frequency:
//! - `frequency_weight = 0.3` → Low weight on frequency (emphasizes recency & age)
//! - `frequency_weight = None` → Default balanced behavior
//! - `frequency_weight = 1.5` → High weight on frequency (emphasizes popular items)
//!
//! This example shows TLRU working with async/await functions for concurrent operations.

use cachelito_async::cache_async;
use tokio::time::{sleep, Duration, Instant};

/// Async cache with TLRU policy and 2-second TTL
#[cache_async(policy = "tlru", limit = 5, ttl = 2)]
async fn fetch_weather_data(city: String) -> String {
    println!("🌤️  [ASYNC] Fetching weather data for {}", city);
    sleep(Duration::from_millis(100)).await; // Simulate API call
    format!("Weather data for {}", city)
}

/// Async cache with TLRU policy without TTL (behaves like ARC)
#[cache_async(policy = "tlru", limit = 3)]
async fn compute_expensive(n: u64) -> u64 {
    println!("💻 [ASYNC] Computing expensive operation for {}", n);
    sleep(Duration::from_millis(50)).await; // Simulate expensive computation
    n * n
}

/// Async cache with TLRU and short TTL for high-frequency updates
#[cache_async(policy = "tlru", limit = 4, ttl = 1)]
async fn get_stock_price(symbol: String) -> f64 {
    println!("📈 [ASYNC] Fetching stock price for {}", symbol);
    sleep(Duration::from_millis(30)).await; // Simulate market data API
    100.0 + (symbol.len() as f64 * 5.0) // Mock calculation
}

/// Time-sensitive async data cache
#[cache_async(policy = "tlru", limit = 2, ttl = 2)]
async fn time_sensitive_data(id: u32) -> String {
    println!("⏱️  [ASYNC] Fetching time-sensitive data for {}", id);
    sleep(Duration::from_millis(40)).await;
    format!("Data-{}", id)
}

/// Async cache for user session data
#[cache_async(policy = "tlru", limit = 100, ttl = 300)]
async fn get_user_session(user_id: u64) -> String {
    println!("👤 [ASYNC] Fetching user session for ID: {}", user_id);
    sleep(Duration::from_millis(20)).await; // Simulate database query
    format!("Session data for user {}", user_id)
}

/// Async cache with LOW frequency_weight (0.3) - emphasizes recency and age over frequency
#[cache_async(policy = "tlru", limit = 3, ttl = 5, frequency_weight = 0.3)]
async fn fetch_news_low_freq(topic: String) -> String {
    println!("📰 [LOW FREQ WEIGHT] Fetching news for: {}", topic);
    sleep(Duration::from_millis(30)).await;
    format!("News about {}", topic)
}

/// Async cache with DEFAULT frequency_weight (no parameter) - balanced approach
#[cache_async(policy = "tlru", limit = 3, ttl = 5)]
async fn fetch_news_default(topic: String) -> String {
    println!("📰 [DEFAULT] Fetching news for: {}", topic);
    sleep(Duration::from_millis(30)).await;
    format!("News about {}", topic)
}

/// Async cache with HIGH frequency_weight (1.5) - emphasizes frequency over recency
#[cache_async(policy = "tlru", limit = 3, ttl = 5, frequency_weight = 1.5)]
async fn fetch_news_high_freq(topic: String) -> String {
    println!("📰 [HIGH FREQ WEIGHT] Fetching news for: {}", topic);
    sleep(Duration::from_millis(30)).await;
    format!("News about {}", topic)
}

#[tokio::main]
async fn main() {
    println!("=== Async TLRU Policy Demo ===\n");

    // Example 1: Basic async TLRU with TTL
    println!("--- Example 1: Async Weather Cache with TTL ---");
    let cities = vec!["Madrid", "Barcelona", "Valencia", "Seville", "Bilbao"];

    // Fill the cache asynchronously
    for city in &cities {
        fetch_weather_data(city.to_string()).await;
    }

    println!("\n✅ Cache filled with {} cities", cities.len());

    // Access some cities to increase their frequency
    println!("\n📊 Increasing frequency for Madrid and Barcelona...");
    for _ in 0..3 {
        fetch_weather_data("Madrid".to_string()).await;
        fetch_weather_data("Barcelona".to_string()).await;
    }

    // Wait a bit to make some entries older
    sleep(Duration::from_millis(500)).await;

    // Add new city - should evict based on TLRU score
    println!("\n➕ Adding new city (Malaga)...");
    fetch_weather_data("Malaga".to_string()).await;

    // High frequency cities should still be cached
    println!("\n✓ Checking if high-frequency cities are still cached:");
    fetch_weather_data("Madrid".to_string()).await; // Should be cached
    fetch_weather_data("Barcelona".to_string()).await; // Should be cached

    // Example 2: TTL expiration with async
    println!("\n\n--- Example 2: Async TTL Expiration ---");
    fetch_weather_data("Zaragoza".to_string()).await;
    println!("✓ Cached Zaragoza weather");

    println!("\n⏳ Waiting 3 seconds for TTL to expire...");
    sleep(Duration::from_secs(3)).await;

    println!("🔄 Accessing expired entry:");
    fetch_weather_data("Zaragoza".to_string()).await; // Should refetch

    // Example 3: Concurrent async operations
    println!("\n\n--- Example 3: Concurrent Async Operations ---");

    // Spawn multiple concurrent tasks
    let mut handles = vec![];

    for i in 0..5 {
        handles.push(tokio::spawn(async move { compute_expensive(i).await }));
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    println!("\n📈 Increasing frequency for operation 1 with concurrent calls...");
    let mut concurrent_handles = vec![];

    for _ in 0..10 {
        concurrent_handles.push(tokio::spawn(async { compute_expensive(1).await }));
    }

    for handle in concurrent_handles {
        handle.await.unwrap();
    }

    println!("\n➕ Adding new operation...");
    compute_expensive(10).await; // Should evict based on TLRU

    println!("\n✓ High-frequency operation should be cached:");
    compute_expensive(1).await; // Should be cached (no computation message)

    // Example 4: High-concurrency stock price updates
    println!("\n\n--- Example 4: High-Concurrency Stock Price Cache ---");

    let stocks = vec!["AAPL", "GOOGL", "MSFT", "AMZN", "TSLA"];

    println!("📊 Fetching initial stock prices concurrently...");

    let stock_handles: Vec<_> = stocks
        .iter()
        .map(|&stock| {
            let symbol = stock.to_string();
            tokio::spawn(async move { get_stock_price(symbol).await })
        })
        .collect();

    for handle in stock_handles {
        handle.await.unwrap();
    }

    // Simulate high-frequency trading bot accessing AAPL
    println!("\n💹 Simulating high-frequency access to AAPL...");
    let mut trading_handles = vec![];

    for i in 0..20 {
        trading_handles.push(tokio::spawn(async move {
            sleep(Duration::from_millis(i * 10)).await;
            get_stock_price("AAPL".to_string()).await
        }));
    }

    for handle in trading_handles {
        handle.await.unwrap();
    }

    println!("\n⏳ Waiting for some prices to age...");
    sleep(Duration::from_millis(800)).await;

    println!("\n➕ Adding new stocks...");
    get_stock_price("NVDA".to_string()).await;
    get_stock_price("META".to_string()).await;

    println!("\n✓ High-frequency stock (AAPL) should still be cached:");
    get_stock_price("AAPL".to_string()).await; // Should be cached

    // Example 5: User session management with TLRU
    println!("\n\n--- Example 5: User Session Management ---");

    println!("👥 Simulating user sessions...");

    // Create sessions for multiple users
    for user_id in 1..=5 {
        get_user_session(user_id).await;
    }

    // Simulate active users accessing their sessions frequently
    println!("\n🔄 Active users (1, 2, 3) accessing sessions...");
    for _ in 0..5 {
        get_user_session(1).await;
        get_user_session(2).await;
        get_user_session(3).await;
    }

    // New users join, triggering eviction
    println!("\n➕ New users joining...");
    get_user_session(6).await;
    get_user_session(7).await;

    println!("\n✓ Active user sessions should still be cached:");
    get_user_session(1).await; // Should be cached (high frequency)
    get_user_session(2).await; // Should be cached (high frequency)

    // Example 6: Age factor demonstration
    println!("\n\n--- Example 6: Age Factor with Time-Sensitive Data ---");

    println!("📥 Adding first entry...");
    time_sensitive_data(1).await;

    println!("⏳ Waiting 1 second to age the first entry...");
    sleep(Duration::from_secs(1)).await;

    println!("📥 Adding second entry (fresher)...");
    time_sensitive_data(2).await;

    // Both entries accessed, but with different frequencies
    println!("🔄 Accessing both entries...");
    time_sensitive_data(1).await;
    time_sensitive_data(2).await;
    time_sensitive_data(2).await; // Entry 2 has higher frequency

    println!("\n➕ Adding third entry with cache full...");
    time_sensitive_data(3).await;

    println!("\n✓ Newer/higher-frequency entries should be cached:");
    time_sensitive_data(2).await; // Should be cached
    time_sensitive_data(3).await; // Should be cached

    // Performance comparison
    println!("\n\n--- Performance Metrics ---");
    let start = Instant::now();

    for _ in 0..100 {
        compute_expensive(1).await; // All cached
    }

    let cached_time = start.elapsed();
    println!("✓ 100 cached async lookups: {:?}", cached_time);

    // Concurrent performance test
    println!("\n🚀 Testing concurrent access performance...");
    let concurrent_start = Instant::now();

    let perf_handles: Vec<_> = (0..50)
        .map(|_| tokio::spawn(async { compute_expensive(1).await }))
        .collect();

    for handle in perf_handles {
        handle.await.unwrap();
    }

    let concurrent_time = concurrent_start.elapsed();
    println!("✓ 50 concurrent cached lookups: {:?}", concurrent_time);

    // Example 7: Frequency Weight Demonstration
    println!("\n\n--- Example 7: Frequency Weight Impact ---");
    println!("Comparing LOW (0.3), DEFAULT, and HIGH (1.5) frequency weights\n");

    // Test LOW frequency weight (0.3) - recency matters more
    println!("🔹 LOW frequency_weight (0.3) - Recency over Frequency:");
    println!("   Filling cache with 3 topics...");
    fetch_news_low_freq("Tech".to_string()).await;
    fetch_news_low_freq("Sports".to_string()).await;
    fetch_news_low_freq("Politics".to_string()).await;

    println!("   Accessing 'Tech' many times to increase frequency...");
    for _ in 0..10 {
        fetch_news_low_freq("Tech".to_string()).await;
    }

    sleep(Duration::from_millis(200)).await;

    println!("   Adding 'Business' (cache full)...");
    fetch_news_low_freq("Business".to_string()).await;

    println!("   ✓ With LOW weight, 'Tech' might be evicted despite high frequency");
    println!("     because recent entries are prioritized more.\n");

    // Test DEFAULT frequency weight
    println!("🔹 DEFAULT frequency_weight - Balanced approach:");
    println!("   Filling cache with 3 topics...");
    fetch_news_default("Tech".to_string()).await;
    fetch_news_default("Sports".to_string()).await;
    fetch_news_default("Politics".to_string()).await;

    println!("   Accessing 'Tech' many times to increase frequency...");
    for _ in 0..10 {
        fetch_news_default("Tech".to_string()).await;
    }

    sleep(Duration::from_millis(200)).await;

    println!("   Adding 'Business' (cache full)...");
    fetch_news_default("Business".to_string()).await;

    println!("   ✓ With DEFAULT weight, balanced eviction based on all factors.\n");

    // Test HIGH frequency weight (1.5) - frequency matters more
    println!("🔹 HIGH frequency_weight (1.5) - Frequency over Recency:");
    println!("   Filling cache with 3 topics...");
    fetch_news_high_freq("Tech".to_string()).await;
    fetch_news_high_freq("Sports".to_string()).await;
    fetch_news_high_freq("Politics".to_string()).await;

    println!("   Accessing 'Tech' many times to increase frequency...");
    for _ in 0..10 {
        fetch_news_high_freq("Tech".to_string()).await;
    }

    sleep(Duration::from_millis(200)).await;

    println!("   Adding 'Business' (cache full)...");
    fetch_news_high_freq("Business".to_string()).await;

    println!("   Checking if 'Tech' is still cached...");
    fetch_news_high_freq("Tech".to_string()).await;

    println!("   ✓ With HIGH weight, 'Tech' is more likely to remain cached");
    println!("     because frequency has more impact on eviction score.\n");

    // Concurrent test with frequency_weight
    println!("🚀 Concurrent access with HIGH frequency_weight:");
    let mut high_freq_handles = vec![];

    for i in 0..20 {
        let topic = if i % 5 == 0 {
            "Tech".to_string() // Access Tech frequently
        } else {
            format!("Topic-{}", i)
        };

        high_freq_handles.push(tokio::spawn(
            async move { fetch_news_high_freq(topic).await },
        ));
    }

    for handle in high_freq_handles {
        handle.await.unwrap();
    }

    println!("   ✓ 'Tech' should be cached due to high access frequency\n");

    println!("📊 Frequency Weight Summary:");
    println!(
        "   • frequency_weight = 0.3  → Emphasizes recency & age (good for time-sensitive data)"
    );
    println!("   • frequency_weight = None → Balanced approach (default TLRU behavior)");
    println!("   • frequency_weight = 1.5  → Emphasizes frequency (good for popular content)");
    println!("   • Formula: score = frequency^weight × position × age_factor");

    println!("\n🎯 Async TLRU provides optimal balance for:");
    println!("   - Concurrent async operations (lock-free with DashMap)");
    println!("   - Temporal locality (LRU behavior)");
    println!("   - Access frequency (LFU behavior)");
    println!("   - Time-based relevance (age factor)");
    println!("   - High-throughput async workloads");
    println!("   - Customizable frequency impact with frequency_weight");
}
