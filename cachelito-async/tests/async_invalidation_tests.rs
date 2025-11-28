//! Integration tests for async cache invalidation

use cachelito_async::cache_async;
use cachelito_core::{invalidate_by_dependency, invalidate_by_event, invalidate_by_tag};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

// Global mutex to ensure tests run sequentially to avoid interference
// from the shared InvalidationRegistry
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
}

#[tokio::test]
async fn test_tag_based_invalidation() {
    let _lock = TEST_MUTEX.lock().unwrap();

    use std::sync::atomic::AtomicUsize as Counter;
    static CALL_COUNT: Counter = Counter::new(0);

    #[cache_async(name = "test_tag_cache_1", tags = ["users", "profiles"])]
    async fn get_test_user(id: u64) -> User {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        User {
            id,
            name: format!("User {}", id),
        }
    }

    // Reset counter
    CALL_COUNT.store(0, Ordering::SeqCst);

    // First call - cache miss
    let user1 = get_test_user(1).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(user1.id, 1);

    // Second call - cache hit
    let user2 = get_test_user(1).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1); // No increment
    assert_eq!(user1, user2);

    // Invalidate by tag "users"
    let invalidated = invalidate_by_tag("users");
    assert!(invalidated > 0);

    // Third call - cache miss after invalidation
    let user3 = get_test_user(1).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2); // Incremented again
    assert_eq!(user1, user3);
}

#[tokio::test]
async fn test_event_based_invalidation() {
    let _lock = TEST_MUTEX.lock().unwrap();

    use std::sync::atomic::AtomicUsize as Counter;
    static CALL_COUNT: Counter = Counter::new(0);

    #[cache_async(
        name = "test_event_cache_1",
        events = ["user_updated", "profile_changed"]
    )]
    async fn get_test_user(id: u64) -> User {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        User {
            id,
            name: format!("User {}", id),
        }
    }

    // Reset counter
    CALL_COUNT.store(0, Ordering::SeqCst);

    // First call - cache miss
    let user1 = get_test_user(2).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    // Second call - cache hit
    let user2 = get_test_user(2).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(user1, user2);

    // Invalidate by event
    let invalidated = invalidate_by_event("user_updated");
    assert!(invalidated > 0);

    // Third call - cache miss after invalidation
    let user3 = get_test_user(2).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
    assert_eq!(user1, user3);
}

#[tokio::test]
async fn test_dependency_based_invalidation() {
    let _lock = TEST_MUTEX.lock().unwrap();

    use std::sync::atomic::AtomicUsize as Counter;
    static USER_CALL_COUNT: Counter = Counter::new(0);
    static PROFILE_CALL_COUNT: Counter = Counter::new(0);

    #[cache_async(name = "test_dep_user_cache", tags = ["users"])]
    async fn get_test_user(id: u64) -> User {
        USER_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        User {
            id,
            name: format!("User {}", id),
        }
    }

    #[cache_async(name = "test_dep_profile_cache", dependencies = ["test_dep_user_cache"])]
    async fn get_test_profile(id: u64) -> String {
        PROFILE_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        format!("Profile for user {}", id)
    }

    // Reset counters
    USER_CALL_COUNT.store(0, Ordering::SeqCst);
    PROFILE_CALL_COUNT.store(0, Ordering::SeqCst);

    // Call both functions - cache misses
    let _user = get_test_user(3).await;
    assert_eq!(USER_CALL_COUNT.load(Ordering::SeqCst), 1);

    let profile1 = get_test_profile(3).await;
    assert_eq!(PROFILE_CALL_COUNT.load(Ordering::SeqCst), 1);

    // Second calls - cache hits
    get_test_user(3).await;
    get_test_profile(3).await;
    assert_eq!(USER_CALL_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(PROFILE_CALL_COUNT.load(Ordering::SeqCst), 1);

    // Invalidate by dependency
    let invalidated = invalidate_by_dependency("test_dep_user_cache");
    assert!(invalidated > 0);

    // User cache should still be cached, but profile should be invalidated
    get_test_user(3).await;
    assert_eq!(USER_CALL_COUNT.load(Ordering::SeqCst), 1); // Still cached

    let profile2 = get_test_profile(3).await;
    assert_eq!(PROFILE_CALL_COUNT.load(Ordering::SeqCst), 2); // Recomputed
    assert_eq!(profile1, profile2);
}

#[tokio::test]
async fn test_multiple_tags() {
    let _lock = TEST_MUTEX.lock().unwrap();

    use std::sync::atomic::AtomicUsize as Counter;
    static CALL_COUNT: Counter = Counter::new(0);

    #[cache_async(name = "test_multi_tag_cache", tags = ["users", "profiles"])]
    async fn get_test_user(id: u64) -> User {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        User {
            id,
            name: format!("User {}", id),
        }
    }

    // Reset counter
    CALL_COUNT.store(0, Ordering::SeqCst);

    // First call
    get_test_user(4).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    // Cache hit
    get_test_user(4).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    // Invalidate by second tag "profiles"
    let invalidated = invalidate_by_tag("profiles");
    assert!(invalidated > 0);

    // Cache miss after invalidation
    get_test_user(4).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_multiple_events() {
    let _lock = TEST_MUTEX.lock().unwrap();

    use std::sync::atomic::AtomicUsize as Counter;
    static CALL_COUNT: Counter = Counter::new(0);

    #[cache_async(
        name = "test_multi_event_cache",
        events = ["user_updated", "profile_changed"]
    )]
    async fn get_test_user(id: u64) -> User {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        User {
            id,
            name: format!("User {}", id),
        }
    }

    // Reset counter
    CALL_COUNT.store(0, Ordering::SeqCst);

    // First call
    get_test_user(5).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    // Cache hit
    get_test_user(5).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    // Invalidate by second event "profile_changed"
    let invalidated = invalidate_by_event("profile_changed");
    assert!(invalidated > 0);

    // Cache miss after invalidation
    get_test_user(5).await;
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
}
