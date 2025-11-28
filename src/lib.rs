//! # Cachelito
//!
//! A lightweight, thread-safe caching library for Rust that provides automatic memoization
//! through procedural macros.
//!
//! ## Features
//!
//! - **Easy to use**: Simply add `#[cache]` attribute to any function or method
//! - **Thread-safe**: Uses `thread_local!` storage for cache isolation
//! - **Flexible key generation**: Supports custom cache key implementations
//! - **Result-aware**: Intelligently caches only successful `Result::Ok` values
//! - **Type-safe**: Full compile-time type checking
//!
//! ## Quick Start
//!
//! Add the `#[cache]` attribute to any function you want to memoize:
//!
//! ```rust
//! use cachelito::cache;
//!
//! #[cache]
//! fn fibonacci(n: u32) -> u64 {
//!     if n <= 1 {
//!         return n as u64;
//!     }
//!     fibonacci(n - 1) + fibonacci(n - 2)
//! }
//!
//! // First call computes the result
//! let result1 = fibonacci(10);
//! // Second call returns cached result instantly
//! let result2 = fibonacci(10);
//! assert_eq!(result1, result2);
//! ```
//!
//! ## Custom Cache Keys
//!
//! For complex types, you can implement custom cache key generation:
//!
//! ```rust
//! use cachelito::cache;
//! use cachelito_core::{CacheableKey, DefaultCacheableKey};
//!
//! #[derive(Debug, Clone)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! // Option 1: Use default Debug-based key
//! impl DefaultCacheableKey for User {}
//!
//! // Note: You can also implement CacheableKey directly instead of DefaultCacheableKey
//! // for better performance, but not both at the same time
//! ```
//!
//! Or with a custom implementation:
//!
//! ```rust
//! use cachelito::cache;
//! use cachelito_core::CacheableKey;
//!
//! #[derive(Debug, Clone)]
//! struct UserId {
//!     id: u64,
//!     name: String,
//! }
//!
//! // Custom key implementation (more efficient than Debug-based)
//! impl CacheableKey for UserId {
//!     fn to_cache_key(&self) -> String {
//!         format!("user:{}", self.id)
//!     }
//! }
//! ```
//!
//! ## Caching with Methods
//!
//! The `#[cache]` attribute also works with methods:
//!
//! ```rust
//! use cachelito::cache;
//! use cachelito_core::DefaultCacheableKey;
//!
//! #[derive(Debug, Clone)]
//! struct Calculator;
//!
//! impl DefaultCacheableKey for Calculator {}
//!
//! impl Calculator {
//!     #[cache]
//!     fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! }
//! ```
//!
//! ## Error Handling
//!
//! Functions returning `Result<T, E>` only cache successful results:
//!
//! ```rust
//! use cachelito::cache;
//!
//! #[cache]
//! fn divide(a: i32, b: i32) -> Result<i32, String> {
//!     if b == 0 {
//!         Err("Division by zero".to_string())
//!     } else {
//!         Ok(a / b)
//!     }
//! }
//!
//! // Ok results are cached
//! let _ = divide(10, 2);
//! // Err results are NOT cached
//! let _ = divide(10, 0);
//! ```

pub use cachelito_core::*;
pub use cachelito_macros::cache;

/// Invalidate all caches associated with a specific tag
///
/// This function triggers invalidation of all caches that have been
/// registered with the given tag.
///
/// # Arguments
///
/// * `tag` - The tag to invalidate
///
/// # Returns
///
/// The number of caches that were invalidated
///
/// # Examples
///
/// ```rust
/// use cachelito::{cache, invalidate_by_tag};
///
/// // Later, when data changes:
/// invalidate_by_tag("user_data");
/// ```
pub fn invalidate_by_tag(tag: &str) -> usize {
    InvalidationRegistry::global().invalidate_by_tag(tag)
}

/// Invalidate all caches associated with a specific event
///
/// This function triggers invalidation of all caches that have been
/// configured to listen to the given event.
///
/// # Arguments
///
/// * `event` - The event that occurred
///
/// # Returns
///
/// The number of caches that were invalidated
///
/// # Examples
///
/// ```rust
/// use cachelito::{cache, invalidate_by_event};
///
/// // When an event occurs:
/// invalidate_by_event("user_updated");
/// ```
pub fn invalidate_by_event(event: &str) -> usize {
    InvalidationRegistry::global().invalidate_by_event(event)
}

/// Invalidate all caches that depend on a specific dependency
///
/// This function triggers cascade invalidation of all caches that
/// have declared a dependency on the given function/cache.
///
/// # Arguments
///
/// * `dependency` - The dependency that changed
///
/// # Returns
///
/// The number of caches that were invalidated
///
/// # Examples
///
/// ```rust
/// use cachelito::{cache, invalidate_by_dependency};
///
/// // When a dependency changes:
/// invalidate_by_dependency("get_user_permissions");
/// ```
pub fn invalidate_by_dependency(dependency: &str) -> usize {
    InvalidationRegistry::global().invalidate_by_dependency(dependency)
}

/// Invalidate a specific cache by its name
///
/// This function invalidates a single cache identified by its name.
///
/// # Arguments
///
/// * `cache_name` - The name of the cache to invalidate
///
/// # Returns
///
/// `true` if the cache was found and invalidated, `false` otherwise
///
/// # Examples
///
/// ```rust
/// use cachelito::{cache, invalidate_cache};
///
/// // Invalidate a specific cache:
/// invalidate_cache("get_user_profile");
/// ```
pub fn invalidate_cache(cache_name: &str) -> bool {
    InvalidationRegistry::global().invalidate_cache(cache_name)
}
