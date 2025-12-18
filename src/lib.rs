//! # Cachelito
//!
//! A lightweight, thread-safe caching library for Rust that provides automatic memoization
//! through procedural macros.
//!
//! ## Features
//!
//! - **Easy to use**: Simply add `#[cache]` attribute to any function or method
//! - **Global scope by default**: Cache shared across all threads (use `scope = "thread"` for thread-local)
//! - **High-performance synchronization**: Uses `parking_lot::RwLock` for global caches
//! - **Thread-local option**: Optional thread-local storage for maximum performance
//! - **Multiple eviction policies**: FIFO, LRU, LFU, ARC, Random, and TLRU
//! - **TLRU with frequency_weight**: Fine-tune recency vs frequency balance (v0.15.0)
//! - **Flexible key generation**: Supports custom cache key implementations
//! - **Result-aware**: Intelligently caches only successful `Result::Ok` values
//! - **Cache limits**: Control size with `limit` (entry count) or `max_memory` (memory-based)
//! - **TTL support**: Time-to-live expiration for automatic cache invalidation
//! - **Statistics**: Track hit/miss rates via `stats` feature
//! - **Smart invalidation**: Tag-based, event-driven, and conditional invalidation
//! - **Conditional caching**: Cache only valid results with `cache_if` predicates
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
