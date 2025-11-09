//! # Cachelito Async
//!
//! A flexible and efficient async caching library for Rust async functions.
//!
//! This crate provides automatic memoization for async functions through the `#[cache_async]` macro.
//! It uses [DashMap](https://docs.rs/dashmap) for lock-free concurrent caching, making it ideal
//! for high-concurrency async applications.
//!
//! ## Features
//!
//! - 🚀 **Lock-free caching**: Uses DashMap for concurrent access without blocking
//! - 🎯 **Multiple eviction policies**: FIFO and LRU
//! - ⏰ **TTL support**: Automatic expiration of cached entries
//! - 📊 **Limit control**: Set maximum cache size
//! - 🔍 **Result caching**: Only caches `Ok` values from `Result` types
//! - 🌐 **Global cache**: Shared across all tasks and threads
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! cachelito-async = "0.1"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```rust,ignore
//! use cachelito_async::cache_async;
//! use std::time::Duration;
//!
//! #[cache_async]
//! async fn expensive_operation(x: u32) -> u32 {
//!     tokio::time::sleep(Duration::from_secs(1)).await;
//!     x * 2
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // First call: sleeps for 1 second
//!     let result = expensive_operation(5).await;
//!     
//!     // Second call: returns immediately from cache
//!     let result = expensive_operation(5).await;
//! }
//! ```
//!
//! ### With Cache Limit and LRU Policy
//!
//! ```rust,ignore
//! use cachelito_async::cache_async;
//!
//! #[cache_async(limit = 100, policy = "lru")]
//! async fn fetch_user(id: u64) -> User {
//!     // Only 100 users cached at a time
//!     // Least recently used entries evicted first
//!     database::fetch_user(id).await
//! }
//! ```
//!
//! ### With TTL (Time To Live)
//!
//! ```rust,ignore
//! use cachelito_async::cache_async;
//!
//! #[cache_async(ttl = 60)]
//! async fn fetch_weather(city: String) -> Weather {
//!     // Cache expires after 60 seconds
//!     api::get_weather(&city).await
//! }
//! ```
//!
//! ### Result Caching (Only Ok Values)
//!
//! ```rust,ignore
//! use cachelito_async::cache_async;
//!
//! #[cache_async(limit = 50)]
//! async fn api_call(endpoint: String) -> Result<Response, Error> {
//!     // Only successful responses are cached
//!     // Errors are not cached and always re-executed
//!     make_request(&endpoint).await
//! }
//! ```
//!
//! ## Macro Parameters
//!
//! - `limit`: Maximum number of entries (default: unlimited)
//! - `policy`: Eviction policy - `"fifo"` or `"lru"` (default: `"fifo"`)
//! - `ttl`: Time-to-live in seconds (default: none)
//! - `name`: Custom cache identifier (default: function name)
//!
//! ## Performance
//!
//! - Uses DashMap for lock-free concurrent access
//! - No `.await` overhead for cache operations
//! - Minimal memory footprint with configurable limits
//! - O(1) cache lookups and insertions
//!
//! ## Thread Safety
//!
//! All caches are thread-safe and can be safely used across multiple tasks and threads.
//! The underlying DashMap provides excellent concurrent performance without traditional locks.
//!

// Re-export the macro
pub use cachelito_async_macros::cache_async;

// Re-export stats functionality from cachelito-core
pub use cachelito_core::{stats_registry, CacheStats};

// Re-export common dependencies that users might need
pub use dashmap;
pub use parking_lot;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::cache_async;
    pub use crate::stats_registry;
    pub use crate::CacheStats;
}
