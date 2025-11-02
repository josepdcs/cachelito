//! # Custom Type with Custom Cache Key Example
//!
//! This example demonstrates implementing a custom cache key for better
//! performance and more control over the cache key format.

use cachelito::cache;
use cachelito_core::CacheableKey;

/// Represents a user in the system.
///
/// This type demonstrates implementing a custom cache key for better
/// performance and more control over the cache key format.
#[derive(Debug, Clone)]
struct User {
    /// Unique user identifier
    id: u64,
    /// User's display name
    name: String,
    /// User's email address
    email: String,
}

// Custom cache key implementation for User
// This is more efficient than the default Debug-based approach
// as it only uses the ID and name without Debug formatting overhead
impl CacheableKey for User {
    fn to_cache_key(&self) -> String {
        format!("user:{}:{}", self.id, self.name)
    }
}

/// Fetches user profile data (simulated).
///
/// This function is cached using the custom cache key implementation.
///
/// # Arguments
///
/// * `user` - The user to fetch profile for
///
/// # Returns
///
/// A formatted profile string
#[cache]
fn get_user_profile(user: User) -> String {
    println!("Fetching profile for user: {}", user.name);
    format!("Profile: {} ({})", user.name, user.email)
}

fn main() {
    println!("=== Custom Type with Custom Cache Key Example ===\n");

    let user1 = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let user2 = User {
        id: 2,
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
    };

    println!("--- Testing User Profile Caching ---");

    // First call with user1: computes and caches the result
    println!("Fetching user1 profile (first time):");
    let profile1 = get_user_profile(user1.clone());
    println!("Result: {}\n", profile1);

    // Second call with user1: returns cached result (no "Fetching" message)
    println!("Getting user1 profile (should be cached):");
    let cached_profile1 = get_user_profile(user1.clone());
    println!("Result: {}\n", cached_profile1);

    // First call with user2: computes and caches the result
    println!("Fetching user2 profile (first time):");
    let profile2 = get_user_profile(user2.clone());
    println!("Result: {}\n", profile2);

    // Verify cache works correctly
    assert_eq!(profile1, cached_profile1);
    assert_eq!(profile1, "Profile: Alice (alice@example.com)");
    assert_eq!(profile2, "Profile: Bob (bob@example.com)");

    println!("✅ Custom Type with Custom Cache Key Test PASSED");
    println!("   Custom cache keys provide better performance than Debug-based keys.");
}
