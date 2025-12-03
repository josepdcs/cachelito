//! # Cachelito Core
//!
//! Core traits and utilities for the Cachelito caching library.
//!
//! This module provides the fundamental building blocks for cache key generation,
//! thread-local cache management, global cache management, eviction policies,
//! invalidation strategies, and memory management.
//!
//! ## Features
//!
//! - **Cache Key Generation**: Flexible traits for custom or default cache keys
//! - **Thread-Local Storage**: Safe, lock-free caching using `thread_local!`
//! - **Global Cache**: Thread-safe cache shared across all threads using `parking_lot::RwLock`
//! - **Async Cache**: Lock-free async cache using `DashMap` for concurrent async operations
//! - **Eviction Policies**: Support for FIFO, LRU (default), LFU, ARC, and Random
//!   - **FIFO**: First In, First Out - simple and predictable
//!   - **LRU**: Least Recently Used - evicts least recently accessed entries
//!   - **LFU**: Least Frequently Used - evicts least frequently accessed entries
//!   - **ARC**: Adaptive Replacement Cache - self-tuning policy combining recency and frequency
//!   - **Random**: Random replacement - O(1) eviction with minimal overhead
//! - **Cache Limits**: Control cache size with entry count limits (`limit`) or memory limits (`max_memory`)
//! - **Memory Estimation**: `MemoryEstimator` trait for accurate memory usage tracking
//! - **TTL Support**: Time-to-live expiration for automatic cache invalidation
//! - **Result-Aware Caching**: Smart handling of `Result<T, E>` types
//! - **Smart Invalidation**: Tag-based, event-driven, and dependency-based cache invalidation
//! - **Conditional Invalidation**: Runtime invalidation with custom check functions
//! - **Statistics Tracking**: Optional hit/miss rate monitoring (requires `stats` feature)
//!
//! ## Module Organization
//!
//! The library is organized into focused modules:
//!
//! - [`cache_entry`] - Entry wrapper with timestamp and frequency tracking for TTL and LFU support
//! - [`eviction_policy`] - Eviction strategies: FIFO, LRU, LFU, ARC, and Random
//! - [`keys`] - Cache key generation traits and implementations
//! - [`thread_local_cache`] - Thread-local caching with zero synchronization overhead
//! - [`global_cache`] - Thread-safe global cache with `parking_lot::RwLock` for concurrent reads
//! - [`async_global_cache`] - Lock-free async cache using `DashMap`
//! - [`memory_estimator`] - Trait for estimating memory usage of cached values
//! - [`invalidation`] - Cache invalidation registry and strategies
//! - [`utils`] - Common utility functions for cache operations
//! - [`stats`] - Cache statistics tracking (optional, requires `stats` feature)
//! - [`stats_registry`] - Global statistics registry for querying cache metrics
//!
//! ## Invalidation Strategies
//!
//! The invalidation module provides multiple strategies for cache invalidation:
//!
//! - **Tag-based**: `invalidate_by_tag("user_data")` - Invalidate all caches with a specific tag
//! - **Event-driven**: `invalidate_by_event("user_updated")` - Invalidate based on application events
//! - **Dependency-based**: `invalidate_by_dependency("get_user")` - Cascade invalidation to dependent caches
//! - **Manual**: `invalidate_cache("cache_name")` - Direct cache invalidation
//! - **Conditional**: `invalidate_with("cache_name", |key| predicate)` - Selective invalidation with custom logic
//! - **Global conditional**: `invalidate_all_with(|cache_name, key| predicate)` - Apply check function across all caches
//!
//! ## Memory Management
//!
//! Cachelito supports both entry-count and memory-based limits:
//!
//! - **Entry limit**: `limit = 1000` - Maximum number of entries
//! - **Memory limit**: `max_memory = "100MB"` - Maximum memory usage
//! - **Custom estimators**: Implement `MemoryEstimator` for user-defined types
//!
//! ## Statistics (Optional)
//!
//! When compiled with the `stats` feature, cachelito tracks cache performance:
//!
//! - Hit/miss counts
//! - Hit rate percentage
//! - Total access count
//! - Per-cache statistics via `stats_registry::get("cache_name")`
//!
mod async_global_cache;
mod cache_entry;
mod eviction_policy;
mod global_cache;
mod keys;
mod memory_estimator;
mod thread_local_cache;

pub mod invalidation;
pub mod utils;

#[cfg(feature = "stats")]
mod stats;

#[cfg(feature = "stats")]
pub mod stats_registry;

pub use async_global_cache::AsyncGlobalCache;
pub use cache_entry::CacheEntry;
pub use eviction_policy::EvictionPolicy;
pub use global_cache::GlobalCache;
pub use invalidation::{
    invalidate_all_with, invalidate_by_dependency, invalidate_by_event, invalidate_by_tag,
    invalidate_cache, invalidate_with, InvalidationMetadata, InvalidationRegistry,
    InvalidationStrategy,
};
pub use keys::{CacheableKey, DefaultCacheableKey};
pub use memory_estimator::MemoryEstimator;
pub use thread_local_cache::ThreadLocalCache;

#[cfg(feature = "stats")]
pub use stats::CacheStats;
/// Cache scope: thread-local or global
///
/// This enum determines whether a cache is stored in thread-local storage
/// or in global static storage accessible by all threads.
///
/// # Variants
///
/// * `ThreadLocal` - Each thread has its own independent cache
/// * `Global` - Cache is shared across all threads with mutex protection
///
/// # Examples
///
/// ```
/// use cachelito_core::CacheScope;
///
/// let scope = CacheScope::ThreadLocal;
/// assert_eq!(scope, CacheScope::ThreadLocal);
///
/// let global = CacheScope::Global;
/// assert_eq!(global, CacheScope::Global);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheScope {
    ThreadLocal,
    Global,
}
