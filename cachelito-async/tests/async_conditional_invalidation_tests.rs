//! Integration tests for async conditional cache invalidation

use cachelito_async::cache_async;
use cachelito_core::{invalidate_all_with, invalidate_with};

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
}

#[cache_async(name = "get_user_async_pred", limit = 100, policy = "lru")]
async fn get_user(user_id: u64) -> User {
    User {
        id: user_id,
        name: format!("User {}", user_id),
    }
}

#[cache_async(name = "get_product_async_pred", limit = 50, policy = "lru")]
async fn get_product(product_id: u64) -> String {
    format!("Product {}", product_id)
}

#[tokio::test]
async fn test_async_conditional_invalidation() {
    // Populate cache
    for id in [100, 500, 1001, 1500, 2000].iter() {
        let _ = get_user(*id).await;
    }

    // Invalidate users with ID > 1000
    let result = invalidate_with("get_user_async_pred", |key: &str| {
        key.parse::<u64>().unwrap_or(0) > 1000
    });

    assert!(
        result,
        "Invalidate callback should be registered for async cache"
    );
}

#[tokio::test]
async fn test_async_conditional_by_pattern() {
    // Populate product cache
    for id in [1, 10, 100, 200, 300].iter() {
        let _ = get_product(*id).await;
    }

    // Invalidate all products with 3-digit IDs (100-999)
    let result = invalidate_with("get_product_async_pred", |key: &str| {
        if let Ok(id) = key.parse::<u64>() {
            id >= 100 && id < 1000
        } else {
            false
        }
    });

    assert!(
        result,
        "Invalidate callback should be registered for async cache"
    );
}

#[tokio::test]
async fn test_async_global_conditional_invalidation() {
    // Populate both caches
    for id in [1, 2, 3, 4, 5].iter() {
        let _ = get_user(*id).await;
        let _ = get_product(*id).await;
    }

    // Invalidate all entries with key >= 3 across all caches
    let count =
        invalidate_all_with(|_cache_name: &str, key: &str| key.parse::<u64>().unwrap_or(0) >= 3);

    // Should process at least the two async caches we registered
    assert!(
        count >= 2,
        "Should process at least 2 caches, got {}",
        count
    );
}

#[tokio::test]
async fn test_async_complex_conditional_check() {
    // Populate cache with specific IDs
    let ids = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for id in &ids {
        let _ = get_user(*id).await;
    }

    // Invalidate users with ID divisible by 30
    let result = invalidate_with("get_user_async_pred", |key: &str| {
        if let Ok(id) = key.parse::<u64>() {
            id % 30 == 0
        } else {
            false
        }
    });

    assert!(
        result,
        "Invalidate callback should be registered for async cache"
    );
}
