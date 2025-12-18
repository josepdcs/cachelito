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
//! - 🎯 **Multiple eviction policies**: FIFO, LRU, LFU, ARC, Random, and TLRU
//! - ⏰ **TLRU with frequency_weight**: Fine-tune recency vs frequency balance (v0.15.0)
//! - 💾 **Memory-based limits**: Control cache size by memory usage
//! - ⏱️ **TTL support**: Automatic expiration of cached entries
//! - 📊 **Limit control**: Set maximum cache size by entry count or memory
//! - 🔍 **Result caching**: Only caches `Ok` values from `Result` types
//! - 🌐 **Global cache**: Shared across all tasks and threads
//! - ⚡ **Zero async overhead**: No `.await` needed for cache operations
//! - 📈 **Statistics**: Track hit/miss rates via `stats_registry`
//! - 🎛️ **Conditional caching**: Cache only valid results with `cache_if` predicates
//! - 🔥 **Smart invalidation**: Tag-based, event-driven, and conditional invalidation
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! cachelito-async = "0.1.0"
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
//! - `policy`: Eviction policy - `"fifo"`, `"lru"`, `"lfu"`, `"arc"`, `"random"`, or `"tlru"` (default: `"fifo"`)
//! - `ttl`: Time-to-live in seconds (default: none)
//! - `frequency_weight`: Weight factor for frequency in TLRU policy (default: 1.0)
//! - `name`: Custom cache identifier (default: function name)
//! - `max_memory`: Maximum memory usage (e.g., "100MB", default: none)
//! - `tags`: Tags for group invalidation (default: none)
//! - `events`: Events that trigger invalidation (default: none)
//! - `dependencies`: Cache dependencies (default: none)
//! - `invalidate_on`: Function to check if entry should be invalidated (default: none)
//! - `cache_if`: Function to determine if result should be cached (default: none)
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
