//! Example demonstrating smart cache invalidation with async functions
//!
//! This example shows how to use tags, events, and dependencies to invalidate
//! async cached functions in a fine-grained manner.

use cachelito_async::cache_async;
use cachelito_core::{invalidate_by_dependency, invalidate_by_event, invalidate_by_tag};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
struct UserProfile {
    id: u64,
    name: String,
    email: String,
    role: String,
}

/// Fetch user profile - tagged with "user_data" and "profile"
/// Triggered by "user_updated" event
#[cache_async(
    name = "get_user_profile",
    tags = ["user_data", "profile"],
    events = ["user_updated"]
)]
async fn get_user_profile(user_id: u64) -> UserProfile {
    println!("  [Computing] get_user_profile({})", user_id);
    sleep(Duration::from_millis(100)).await;
    UserProfile {
        id: user_id,
        name: format!("User {}", user_id),
        email: format!("user{}@example.com", user_id),
        role: "user".to_string(),
    }
}

/// Fetch user permissions - tagged with "user_data"
/// Triggered by "user_updated" and "permissions_changed" events
#[cache_async(
    name = "get_user_permissions",
    tags = ["user_data"],
    events = ["user_updated", "permissions_changed"]
)]
async fn get_user_permissions(user_id: u64) -> Vec<String> {
    println!("  [Computing] get_user_permissions({})", user_id);
    sleep(Duration::from_millis(100)).await;
    vec![
        "read".to_string(),
        "write".to_string(),
        format!("user:{}", user_id),
    ]
}

/// Fetch user dashboard - depends on get_user_profile
#[cache_async(
    name = "get_user_dashboard",
    dependencies = ["get_user_profile"]
)]
async fn get_user_dashboard(user_id: u64) -> String {
    println!("  [Computing] get_user_dashboard({})", user_id);
    sleep(Duration::from_millis(100)).await;
    format!("Dashboard for user {}", user_id)
}

/// Fetch user settings - tagged with all categories
#[cache_async(
    name = "get_user_settings",
    tags = ["user_data", "settings"],
    events = ["user_updated", "settings_changed"],
    dependencies = ["get_user_profile"]
)]
async fn get_user_settings(user_id: u64) -> String {
    println!("  [Computing] get_user_settings({})", user_id);
    sleep(Duration::from_millis(100)).await;
    format!("Settings for user {}", user_id)
}

#[tokio::main]
async fn main() {
    println!("=== Async Cache Invalidation Demo ===\n");

    // 1. Initial calls (will compute)
    println!("1. Initial calls (will compute):");
    let _profile = get_user_profile(1).await;
    println!("   Profile: {:?}", _profile);
    let _perms = get_user_permissions(1).await;
    println!("   Permissions: {:?}", _perms);
    let _dashboard = get_user_dashboard(1).await;
    println!("   Dashboard: {}", _dashboard);
    let _settings = get_user_settings(1).await;
    println!("   Settings: {}\n", _settings);

    // 2. Cached calls (instant)
    println!("2. Cached calls (instant):");
    get_user_profile(1).await;
    get_user_permissions(1).await;
    get_user_dashboard(1).await;
    get_user_settings(1).await;
    println!("   All calls returned instantly from cache\n");

    // 3. Invalidate by tag
    println!("3. Invalidate by tag 'user_data':");
    let count = invalidate_by_tag("user_data");
    println!("   Invalidated {} caches\n", count);

    println!("4. Calls after tag invalidation:");
    get_user_profile(1).await; // Will recompute (has tag "user_data")
    get_user_permissions(1).await; // Will recompute (has tag "user_data")
    get_user_dashboard(1).await; // Still cached (no tag "user_data")
    get_user_settings(1).await; // Will recompute (has tag "user_data")
    println!();

    // Re-cache everything
    get_user_profile(1).await;
    get_user_permissions(1).await;
    get_user_dashboard(1).await;
    get_user_settings(1).await;

    // 4. Invalidate by event
    println!("5. Invalidate by event 'user_updated':");
    let count = invalidate_by_event("user_updated");
    println!("   Invalidated {} caches\n", count);

    println!("6. Calls after event invalidation:");
    get_user_profile(1).await; // Will recompute (listens to "user_updated")
    get_user_permissions(1).await; // Will recompute (listens to "user_updated")
    get_user_dashboard(1).await; // Still cached (doesn't listen to this event)
    get_user_settings(1).await; // Will recompute (listens to "user_updated")
    println!();

    // Re-cache everything
    get_user_profile(1).await;
    get_user_permissions(1).await;
    get_user_dashboard(1).await;
    get_user_settings(1).await;

    // 5. Invalidate by dependency
    println!("7. Invalidate by dependency 'get_user_profile':");
    let count = invalidate_by_dependency("get_user_profile");
    println!("   Invalidated {} caches\n", count);

    println!("8. Calls after dependency invalidation:");
    get_user_profile(1).await; // Still cached (it's the dependency, not dependent)
    get_user_permissions(1).await; // Still cached (doesn't depend on get_user_profile)
    get_user_dashboard(1).await; // Will recompute (depends on get_user_profile)
    get_user_settings(1).await; // Will recompute (depends on get_user_profile)
    println!();

    println!("=== Demo Complete ===");
}
