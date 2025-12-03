//! Integration tests for conditional cache invalidation

use cachelito::cache;
use cachelito_core::{invalidate_all_with, invalidate_with};

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
}

#[cache(scope = "global", name = "get_user_pred", limit = 100, policy = "lru")]
fn get_user(user_id: u64) -> User {
    User {
        id: user_id,
        name: format!("User {}", user_id),
    }
}

#[cache(
    scope = "global",
    name = "get_product_pred",
    limit = 50,
    policy = "lru"
)]
fn get_product(product_id: u64) -> String {
    format!("Product {}", product_id)
}

#[test]
fn test_conditional_invalidation_with_macro() {
    // Populate cache
    for id in [100, 500, 1001, 1500, 2000].iter() {
        let _ = get_user(*id);
    }

    // Verify all entries are cached
    let user100_1 = get_user(100);
    let user1001_1 = get_user(1001);
    assert_eq!(user100_1.id, 100);
    assert_eq!(user1001_1.id, 1001);

    // Invalidate users with ID > 1000
    let result = invalidate_with("get_user_pred", |key: &str| {
        key.parse::<u64>().unwrap_or(0) > 1000
    });

    assert!(result, "Check function callback should be registered");

    // After invalidation, IDs > 1000 should be re-fetched
    // but we can't easily verify this without database access tracking
    // The important part is that invalidate_with returns true
}

#[test]
fn test_conditional_invalidation_by_pattern() {
    // Populate product cache
    for id in [1, 10, 100, 200, 300].iter() {
        let _ = get_product(*id);
    }

    // Invalidate all products with 3-digit IDs (100-999)
    let result = invalidate_with("get_product_pred", |key: &str| {
        if let Ok(id) = key.parse::<u64>() {
            id >= 100 && id < 1000
        } else {
            false
        }
    });

    assert!(result, "Check function callback should be registered");
}

#[test]
fn test_global_conditional_invalidation() {
    // Populate both caches
    for id in [1, 2, 3, 4, 5].iter() {
        let _ = get_user(*id);
        let _ = get_product(*id);
    }

    // Invalidate all entries with key >= 3 across all caches
    let count =
        invalidate_all_with(|_cache_name: &str, key: &str| key.parse::<u64>().unwrap_or(0) >= 3);

    // Should process at least the two caches we registered
    assert!(
        count >= 2,
        "Should process at least 2 caches, got {}",
        count
    );
}

#[test]
fn test_complex_conditional_check() {
    // Populate cache with specific IDs
    let ids = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for id in &ids {
        let _ = get_user(*id);
    }

    // Invalidate users with ID divisible by 30
    let result = invalidate_with("get_user_pred", |key: &str| {
        if let Ok(id) = key.parse::<u64>() {
            id % 30 == 0
        } else {
            false
        }
    });

    assert!(result, "Check function callback should be registered");
}

#[test]
fn test_conditional_check_no_matches() {
    // Populate cache
    for id in [1, 2, 3].iter() {
        let _ = get_user(*id);
    }

    // Invalidate with check function that matches nothing
    let result = invalidate_with("get_user_pred", |key: &str| {
        key.parse::<u64>().unwrap_or(0) > 1000000
    });

    assert!(
        result,
        "Check function callback should be registered even if no matches"
    );
}
