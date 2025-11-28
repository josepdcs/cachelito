use cachelito::cache;
use cachelito::{invalidate_by_dependency, invalidate_by_event, invalidate_by_tag};

/// User profile data
#[derive(Debug, Clone)]
struct UserProfile {
    id: u64,
    name: String,
    email: String,
    role: String,
}

/// Get user profile with tag-based invalidation
#[cache(
    scope = "global",
    limit = 100,
    policy = "lru",
    tags = ["user_data", "profile"],
    name = "get_user_profile"
)]
fn get_user_profile(user_id: u64) -> UserProfile {
    println!("  [Computing] get_user_profile({})", user_id);
    // Simulate expensive operation
    std::thread::sleep(std::time::Duration::from_millis(100));

    UserProfile {
        id: user_id,
        name: format!("User {}", user_id),
        email: format!("user{}@example.com", user_id),
        role: "user".to_string(),
    }
}

/// Get user permissions with event-based invalidation
#[cache(
    scope = "global",
    limit = 50,
    policy = "lru",
    events = ["user_updated", "permissions_changed"],
    name = "get_user_permissions"
)]
fn get_user_permissions(user_id: u64) -> Vec<String> {
    println!("  [Computing] get_user_permissions({})", user_id);
    std::thread::sleep(std::time::Duration::from_millis(50));

    vec![
        "read".to_string(),
        "write".to_string(),
        format!("user:{}", user_id),
    ]
}

/// Get user dashboard data (depends on other functions)
#[cache(
    scope = "global",
    limit = 50,
    policy = "lru",
    dependencies = ["get_user_profile", "get_user_permissions"],
    tags = ["dashboard"],
    name = "get_user_dashboard"
)]
fn get_user_dashboard(user_id: u64) -> String {
    println!("  [Computing] get_user_dashboard({})", user_id);
    std::thread::sleep(std::time::Duration::from_millis(75));

    format!("Dashboard for user {}", user_id)
}

/// Get user settings with multiple invalidation strategies
#[cache(
    scope = "global",
    limit = 100,
    policy = "lru",
    tags = ["user_data", "settings"],
    events = ["user_updated"],
    dependencies = ["get_user_profile"],
    name = "get_user_settings"
)]
fn get_user_settings(user_id: u64) -> String {
    println!("  [Computing] get_user_settings({})", user_id);
    std::thread::sleep(std::time::Duration::from_millis(60));

    format!("Settings for user {}", user_id)
}

fn main() {
    println!("=== Cache Invalidation Demo ===\n");

    // Initial calls - will compute
    println!("1. Initial calls (will compute):");
    let profile1 = get_user_profile(1);
    println!("   Profile: {:?}", profile1);
    let perms1 = get_user_permissions(1);
    println!("   Permissions: {:?}", perms1);
    let dashboard1 = get_user_dashboard(1);
    println!("   Dashboard: {}", dashboard1);
    let settings1 = get_user_settings(1);
    println!("   Settings: {}", settings1);

    println!("\n2. Cached calls (instant):");
    let _profile2 = get_user_profile(1);
    let _perms2 = get_user_permissions(1);
    let _dashboard2 = get_user_dashboard(1);
    let _settings2 = get_user_settings(1);
    println!("   All calls returned instantly from cache");

    // Tag-based invalidation
    println!("\n3. Invalidate by tag 'user_data':");
    let count = invalidate_by_tag("user_data");
    println!("   Invalidated {} caches", count);

    println!("\n4. Calls after tag invalidation:");
    let _profile3 = get_user_profile(1); // Will compute (tagged with "user_data")
    let _perms3 = get_user_permissions(1); // Still cached (not tagged)
    let _dashboard3 = get_user_dashboard(1); // Still cached
    let _settings3 = get_user_settings(1); // Will compute (tagged with "user_data")

    // Event-based invalidation
    println!("\n5. Invalidate by event 'user_updated':");
    let count = invalidate_by_event("user_updated");
    println!("   Invalidated {} caches", count);

    println!("\n6. Calls after event invalidation:");
    let _perms4 = get_user_permissions(1); // Will compute (listens to "user_updated")
    let _settings4 = get_user_settings(1); // Will compute (listens to "user_updated")

    // Dependency-based invalidation
    println!("\n7. Invalidate by dependency 'get_user_profile':");
    let count = invalidate_by_dependency("get_user_profile");
    println!("   Invalidated {} caches", count);

    println!("\n8. Calls after dependency invalidation:");
    let _dashboard4 = get_user_dashboard(1); // Will compute (depends on get_user_profile)
    let _settings5 = get_user_settings(1); // Will compute (depends on get_user_profile)
    let _profile4 = get_user_profile(1); // Still cached (no dependencies on itself)

    println!("\n=== Demo Complete ===");
}
