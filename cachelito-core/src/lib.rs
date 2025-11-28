//! # Cachelito Core
//!
//! Core traits and utilities for the Cachelito caching library.
//!
//! This module provides the fundamental building blocks for cache key generation,
//! thread-local cache management, global cache management, and eviction policies.
//!
//! ## Features
//!
//! - **Cache Key Generation**: Flexible traits for custom or default cache keys
//! - **Thread-Local Storage**: Safe, lock-free caching using `thread_local!`
//! - **Global Cache**: Thread-safe cache shared across all threads
//! - **Eviction Policies**: Support for FIFO (First In, First Out) and LRU (Least Recently Used)
//! - **Cache Limits**: Control memory usage with configurable size limits
//! - **TTL Support**: Time-to-live expiration for automatic cache invalidation
//! - **Result-Aware Caching**: Smart handling of `Result<T, E>` types
//!
//! ## Module Organization
//!
//! The library is organized into focused modules:
//!
//! - [`cache_entry`] - Entry wrapper with timestamp tracking for TTL support
//! - [`eviction_policy`] - FIFO and LRU eviction strategies
//! - [`keys`] - Cache key generation traits and implementations
//! - [`thread_local_cache`] - Thread-local caching with zero synchronization overhead
//! - [`global_cache`] - Thread-safe global cache with mutex protection
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
    invalidate_by_dependency, invalidate_by_event, invalidate_by_tag, InvalidationMetadata,
    InvalidationRegistry, InvalidationStrategy,
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
