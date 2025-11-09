/// Example demonstrating custom cache names with the `name` attribute.
///
/// This shows how to use custom identifiers for caches in the statistics registry,
/// which is useful when you want more descriptive names or when caching multiple
/// versions of similar functions.
use cachelito::cache;


// API V1 - using custom name "api_v1"
#[cache(limit = 50, name = "api_v1")]
fn fetch_data(id: u32) -> String {
    println!("  [V1] Fetching data for ID {}", id);
    std::thread::sleep(std::time::Duration::from_millis(100));
    format!("V1 Data for ID {}", id)
}

// API V2 - using custom name "api_v2"
#[cache(limit = 50, name = "api_v2")]
fn fetch_data_v2(id: u32) -> String {
    println!("  [V2] Fetching enhanced data for ID {}", id);
    std::thread::sleep(std::time::Duration::from_millis(100));
    format!("V2 Enhanced Data for ID {}", id)
}

// User cache - using custom descriptive name
#[cache(limit = 100, name = "user_profile_cache")]
fn get_user_profile(user_id: u32) -> String {
    println!("  [Profile] Loading profile for user {}", user_id);
    std::thread::sleep(std::time::Duration::from_millis(150));
    format!("Profile of User {}", user_id)
}

// Default name - will use function name "expensive_computation"
#[cache(limit = 50)]
fn expensive_computation(n: u32) -> u32 {
    println!("  [Compute] Processing {}", n);
    std::thread::sleep(std::time::Duration::from_millis(200));
    n * n
}

fn main() {
    println!("=== Custom Cache Names Example ===\n");

    // Test API V1
    println!("--- Testing API V1 ---");
    fetch_data(1);
    fetch_data(1); // Hit
    fetch_data(2);
    fetch_data(1); // Hit

    // Test API V2
    println!("\n--- Testing API V2 ---");
    fetch_data_v2(1);
    fetch_data_v2(1); // Hit
    fetch_data_v2(2);

    // Test User Profile Cache
    println!("\n--- Testing User Profile Cache ---");
    get_user_profile(100);
    get_user_profile(100); // Hit
    get_user_profile(101);

    // Test Default Name
    println!("\n--- Testing Default Name (expensive_computation) ---");
    expensive_computation(5);
    expensive_computation(5); // Hit
    expensive_computation(10);

    // Display statistics
    #[cfg(feature = "stats")]
    {
        println!("\n=== Cache Statistics ===\n");

        // List all registered caches
        println!("Registered caches:");
        for name in cachelito::stats_registry::list() {
            println!("  - {}", name);
        }

        println!("\n--- Individual Statistics ---\n");

        // API V1 stats (using custom name)
        if let Some(stats) = cachelito::stats_registry::get("api_v1") {
            println!("📊 api_v1 (fetch_data):");
            println!("   Hits:   {}", stats.hits());
            println!("   Misses: {}", stats.misses());
            println!("   Total:  {}", stats.total_accesses());
            println!("   Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }

        // API V2 stats (using custom name)
        if let Some(stats) = cachelito::stats_registry::get("api_v2") {
            println!("\n📊 api_v2 (fetch_data_v2):");
            println!("   Hits:   {}", stats.hits());
            println!("   Misses: {}", stats.misses());
            println!("   Total:  {}", stats.total_accesses());
            println!("   Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }

        // User profile cache stats (using custom descriptive name)
        if let Some(stats) = cachelito::stats_registry::get("user_profile_cache") {
            println!("\n📊 user_profile_cache (get_user_profile):");
            println!("   Hits:   {}", stats.hits());
            println!("   Misses: {}", stats.misses());
            println!("   Total:  {}", stats.total_accesses());
            println!("   Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }

        // Default name stats (using function name)
        if let Some(stats) = cachelito::stats_registry::get("expensive_computation") {
            println!("\n📊 expensive_computation (default name):");
            println!("   Hits:   {}", stats.hits());
            println!("   Misses: {}", stats.misses());
            println!("   Total:  {}", stats.total_accesses());
            println!("   Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }

        // Compare performance
        println!("\n--- Performance Comparison ---\n");
        if let (Some(v1), Some(v2)) = (
            cachelito::stats_registry::get("api_v1"),
            cachelito::stats_registry::get("api_v2"),
        ) {
            println!(
                "API V1 vs V2 hit rates: {:.2}% vs {:.2}%",
                v1.hit_rate() * 100.0,
                v2.hit_rate() * 100.0
            );
        }
    }

    #[cfg(not(feature = "stats"))]
    {
        println!("\n💡 Enable 'stats' feature to see cache statistics");
        println!("   Run with: cargo run --example custom_cache_name --features stats");
    }
}
