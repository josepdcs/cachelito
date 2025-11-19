# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] - 2025-11-19

### Added

- **💾 Memory-Based Cache Limits**: New `max_memory` attribute for `#[cache]` and `#[cache_async]`
  - Example: `#[cache(max_memory = "100MB")]` or `#[cache(max_memory = 1048576)]`
  - Supports units: `KB`, `MB`, `GB` and raw bytes (integer literal)
  - Can be combined with `limit` (entry count); memory limit is enforced first
  - Implemented for: `ThreadLocalCache`, `GlobalCache`, and `AsyncGlobalCache`
  - Works with all eviction policies (FIFO, LRU, LFU, ARC)
  - Eviction loop continues until memory usage <= `max_memory`
- **MemoryEstimator Trait Integration**:
  - Trait now actively used for memory-based eviction decisions
  - Built-in implementations for primitives, `String`, `&str`, `Vec<T>`, `HashMap<K,V>`, `HashSet<T>`, tuples (2 & 3), `Option<T>`, `Result<T,E>`, `Arc<T>`, `Rc<T>`, `Box<T>`, slices `&[T]`, and `CacheEntry<R>`
  - Users can implement `MemoryEstimator` for custom types with heap allocations
- **Benchmarks Updated**:
  - Extended `cache_benchmark.rs` to include LFU and ARC policies
  - Added `memory_eviction` benchmark group to test `max_memory` behavior
- **New Tests**:
  - Sync memory limit tests: `tests/memory_limit_tests.rs` (strings, vectors, parsing units, combined limits, LFU, ARC, thread-local, global)
  - Async memory limit tests: `cachelito-async/tests/memory_limit_async_tests.rs` (basic, combined, LFU, ARC, parsing)
- **Example**:
  - `examples/memory_limit.rs` demonstrating `max_memory` with large string and vector values
- **Documentation**:
  - README updated with new Memory-Based Limits section
  - Added guidance on custom memory estimators and combined limits usage

### Changed

- **Constructor Signatures** (breaking for direct usage, transparent for macro users):
  - `ThreadLocalCache::new(&cache, &order, limit, max_memory, policy, ttl)`
  - `GlobalCache::new(&map, &order, limit, max_memory, policy, ttl [, stats])`
  - `AsyncGlobalCache::new(&dashmap, &order, limit, max_memory, policy, ttl [, stats])`
- **Macro Code Generation**:
  - Sync and async macros now pass `max_memory` to underlying cache constructors
  - Attribute parser extended to parse `max_memory` values
- **Eviction Logic**:
  - Augmented to perform repeated eviction passes until memory usage <= limit when `max_memory` specified
  - Falls back to entry count limiting when `max_memory` is `None`

### Migration

If you directly construct cache instances (not using the macros):
```rust
// Before (v0.9.0)
let cache = GlobalCache::new(&MAP, &ORDER, Some(100), EvictionPolicy::LRU, None);

// After (v0.10.0)
let cache = GlobalCache::new(&MAP, &ORDER, Some(100), None, EvictionPolicy::LRU, None);
// Or with memory limit
let cache = GlobalCache::new(&MAP, &ORDER, Some(100), Some(64 * 1024 * 1024), EvictionPolicy::LRU, None);
```
For macro users, no changes required; simply add `max_memory = "64MB"` where desired.

### Notes

- Memory estimation is approximate and does not include allocator overhead or fragmentation.
- For collections containing heap-allocated items (e.g., `Vec<String>`), implement a custom `MemoryEstimator` to include nested capacities.
- Combining `limit` and `max_memory` allows dual control: memory pressure eviction first, then entry count fallback.
- Using `Arc<T>` as return type remains recommended for very large values to avoid cloning overhead.

### Performance Considerations

- Memory check on insert is O(n) (sums all entry sizes); acceptable for moderate cache sizes.
- LFU & ARC memory eviction maintain their O(n) scan characteristics.
- Consider smaller `limit` when using very large objects to reduce eviction scan cost.

### Internal

- Added `MEMORY_LIMITS_IMPLEMENTATION.md` summarizing design & usage.
- Updated doctests referencing old constructor signatures.

## [0.9.0] - 2025-11-17

### Added

- **🎯 ARC Eviction Policy**: New Adaptive Replacement Cache policy
    - `#[cache(policy = "arc")]` - Self-tuning cache that adapts between recency and frequency
    - Combines benefits of LRU (recency) and LFU (frequency) dynamically
    - Eviction score: `frequency × recency_weight` (lower score = evicted first)
    - Ideal for mixed workloads with varying access patterns
    - Scan-resistant: protects frequently accessed items from sequential scans
    - Eviction: O(n) complexity (see find_arc_eviction_key which iterates through all entries)
    - Cache hit: O(n) complexity (moves key to end of queue)
    - Available for both sync and async versions
- **Examples**:
    - `examples/arc_policy.rs` - Comprehensive ARC policy demonstration
    - Shows adaptive behavior with mixed access patterns
    - Demonstrates scan-resistance
- **Tests**:
    - `tests/arc_policy_tests.rs` - Complete ARC test suite (11 tests)
    - Tests for basic caching, limit enforcement, frequency tracking, recency tracking
    - Tests for adaptive behavior, global scope, scan resistance
- **Documentation**:
    - Updated `EvictionPolicy` enum with ARC variant
    - Added ARC to policy comparison table
    - Enhanced macro validation to accept "arc" policy

### Changed

- **Policy validation**: Updated to accept "fifo", "lru", "lfu", or "arc"
- **Cache implementations**: All cache types now support ARC eviction
    - `ThreadLocalCache`: ARC support with frequency + recency tracking
    - `GlobalCache`: ARC support with parking_lot synchronization
    - `AsyncGlobalCache`: ARC support with DashMap
- **Performance table**: Added ARC characteristics (O(1) for all operations)

## [0.8.0] - 2025-11-14

### Added

- **🔥 LFU Eviction Policy**: New Least Frequently Used eviction policy
    - `#[cache(policy = "lfu")]` - Evicts entries based on access frequency
    - Frequency counter tracking for each cache entry
    - Ideal for workloads where popular items should remain cached
    - Available for both sync (`cachelito`) and async (`cachelito-async`) versions
- **📏 MemoryEstimator Trait**: Foundation for memory-based cache limits (v0.9.0)
    - New `MemoryEstimator` trait for estimating value memory size
    - Implementations for common types (`String`, `Vec`, `Option`, `Result`, tuples, etc.)
    - Allows custom memory estimation for user types
    - Example: `examples/memory_estimation.rs`
    - Prepares infrastructure for future `max_memory` parameter
- **Frequency Tracking**: 
    - `CacheEntry` now includes a `frequency` field (u64)
    - `increment_frequency()` method for updating access counts
    - Saturating addition prevents overflow
- **Enhanced Eviction Logic**:
    - Sync: Updated `ThreadLocalCache` and `GlobalCache` for LFU support
    - Async: Updated DashMap structure to store `(value, timestamp, frequency)`
    - LFU eviction scans all entries to find minimum frequency (O(n))
- **Examples**: 
    - `examples/lfu.rs` - Demonstrates LFU policy behavior
    - `examples/memory_estimation.rs` - Shows MemoryEstimator usage
    - `cachelito-async/examples/async_lfu.rs` - Async LFU example
- **Tests**:
    - `tests/lfu_tests.rs` - Comprehensive LFU test suite (5 tests)
    - `cachelito-async/tests/lfu_tests.rs` - Async LFU tests (4 tests)
- **Documentation**:
    - Updated `EvictionPolicy` enum documentation
    - Added LFU to policy comparison table
    - Performance characteristics for each policy

### Changed

- **Default eviction policy changed from FIFO to LRU**
    - LRU provides better cache effectiveness for most use cases
    - FIFO and LFU remain available as explicit options
- **Macro validation**: Updated to accept "fifo", "lru", or "lfu" policies
- **Policy comparison table**: Added performance characteristics
- **README.md**: Updated eviction policies section with LFU examples
- **🏗️ Async Architecture Refactoring**:
    - Created `AsyncGlobalCache` struct in `cachelito-core`
    - Moved cache logic from macro code to testable Rust code
    - Reduced macro complexity by ~48% (135 lines removed)
    - Improved maintainability and consistency with sync version
    - No breaking changes - public API remains the same

### Improved

- **Code Organization**:
    - Async cache logic now in `cachelito-core/src/async_global_cache.rs`
    - Consistent architecture between sync and async versions
    - Easier to test, maintain, and extend
- **Testability**:
    - Added 2 unit tests for `AsyncGlobalCache`
    - Can now test async cache logic independently of macro code

### Technical Details

- **LRU update on cache hit**: Moves entry to end of order queue (O(n))
- **LFU update on cache hit**: Increments frequency counter (O(1))
- **LFU eviction**: Scans all entries to find minimum frequency (O(n))
- **Frequency reset**: New entries start with frequency = 0
- **TTL interaction**: Expired entries reset frequency on re-insertion

## [0.7.0] - 2025-01-10

### Added

- **🔮 Async/Await Support**: New `cachelito-async` crate for async functions
    - `#[cache_async]` macro for async function memoization
    - Lock-free concurrent caching using [DashMap](https://docs.rs/dashmap)
    - Support for `limit`, `policy` (FIFO/LRU), `ttl`, and `name` attributes
    - Always global scope - cache shared across all tasks and threads
    - Zero blocking - cache operations don't require `.await`
    - Optimized for I/O-bound async operations
    - **Statistics support**: Automatic hit/miss tracking for async caches
- **New crate**: `cachelito-async` (v0.1.0)
    - Dedicated async caching with `cache_async` procedural macro
    - DashMap-based storage for lock-free concurrent access
    - Thread-safe across tasks and threads
    - Built-in statistics via `stats_registry`
    - Examples: `async_basic`, `async_lru`, `async_concurrent`, `async_stats`
- **New crate**: `cachelito-async-macros` (v0.1.0)
    - Procedural macro implementation for async caching
    - Same attribute syntax as sync version
    - LRU order tracking on cache hits
    - Result-aware caching (only caches `Ok` values)
    - Automatic stats registration and tracking
- **New crate**: `cachelito-macro-utils` (v0.7.0)
    - Shared utilities for sync and async macro implementations
    - Eliminates code duplication
    - Common parsing functions for attributes
    - Improved maintainability
- **Documentation**:
    - Comprehensive README for `cachelito-async`
    - Comparison table: sync vs async caching
    - Migration guide and best practices
    - Performance considerations for async contexts

### Changed

- Updated main README with async support section
- Added async examples to workspace
- Enhanced documentation with async/await use cases

### Technical Details

- **Storage**: `DashMap<String, (ReturnType, u64)>` for values and timestamps
- **Eviction**: `parking_lot::Mutex<VecDeque<String>>` for FIFO/LRU tracking
- **Key generation**: Uses `Debug` formatting (same as sync version)
- **Concurrency**: Lock-free reads and writes via DashMap sharding
- **LRU**: Order updated on both cache hits and misses

## [0.6.0] - 2025-11-09

### Added

- **Cache Statistics**: New `stats` feature flag for tracking cache performance metrics
- **Stats Registry**: Centralized statistics management via `cachelito::stats_registry`
    - `stats_registry::get(name)` - Get statistics snapshot for a cached function
    - `stats_registry::get_ref(name)` - Get direct reference to statistics
    - `stats_registry::list()` - List all registered cache functions
    - `stats_registry::reset(name)` - Reset statistics for a specific function
    - `stats_registry::clear()` - Clear all statistics registrations
- **Custom Cache Names**: New optional `name` attribute for `#[cache]` macro
    - `#[cache(name = "identifier")]` - Give caches custom identifiers in the stats registry
    - Useful for versioning APIs, descriptive names, and better monitoring
    - Defaults to function name if not provided
- **CacheStats metrics**:
    - `hits()` - Number of successful cache lookups
    - `misses()` - Number of cache misses
    - `total_accesses()` - Total cache access count
    - `hit_rate()` - Ratio of hits to total accesses
    - `miss_rate()` - Ratio of misses to total accesses
    - `reset()` - Reset counters to zero
- **Thread-safe statistics**: Using `AtomicU64` for concurrent access
- **Automatic registration**: Global-scoped caches automatically register their statistics
- New examples: `cache_stats`, `concurrent_stats`, `test_stats_simple`, `custom_cache_name`
- Comprehensive test coverage for statistics functionality (91 tests total)
- New module: `cachelito-core/src/stats_registry.rs`
- New module: `cachelito-core/src/stats.rs`
- New integration tests: `tests/custom_name_tests.rs`

### Changed

- **Global scope is now the default** - Cache is shared across threads by default
    - Use `scope = "thread"` explicitly if you need thread-local caches
    - Better integration with statistics (automatically accessible via `stats_registry`)
    - More intuitive behavior for most use cases
- Statistics are automatically tracked for all caches (global by default)
- Enhanced documentation with statistics usage examples and best practices
- Updated README with comprehensive statistics section

### Fixed

- None

### Breaking Changes

- **Default scope changed from `thread` to `global`**
    - If you need the old behavior (thread-local caches), add `scope = "thread"` to your `#[cache]` attributes
    - Migration: `#[cache]` → `#[cache(scope = "thread")]` (if you want thread-local)
    - Most users won't need to change anything as global scope is more useful in most scenarios

### Notes

- Statistics are only accessible via `stats_registry` for global-scoped caches
- Thread-local caches track statistics internally but don't expose them via the registry
- Statistics add minimal overhead (atomic operations only)
- Feature must be explicitly enabled: `cachelito = { version = "0.6.0", features = ["stats"] }`

## [0.5.0] - 2025-01-07

### Added

- **RwLock for cache map**: Replaced `Mutex` with `RwLock` for the global cache HashMap, enabling concurrent reads
- **Enhanced performance**: 4-5x performance improvement for read-heavy workloads (typical cache usage)
- **Concurrent reads**: 20x faster concurrent reads - multiple threads read simultaneously without blocking
- **Optimized architecture**: RwLock for map, Mutex for eviction queue
- **parking_lot::RwLock integration**: Better performance than `std::sync::RwLock`
- **Smaller memory footprint**: 40x smaller per lock (~1 byte vs ~40 bytes)
- **No lock poisoning**: Simpler API without `Result` wrapping
- New benchmarks: `rwlock_concurrent_reads`, `read_heavy_workload`
- New example: `rwlock_concurrent_reads` demonstrating concurrent non-blocking reads
- New unit tests: `test_rwlock_concurrent_reads`, `test_rwlock_write_excludes_reads`

### Changed

- **Idiomatically renamed crates**: `cachelito_core` → `cachelito-core`, `cachelito_macros` → `cachelito-macros`
- Cleaner internal code thanks to parking_lot's simpler API
- Enhanced documentation with RwLock benefits, architecture diagrams, and benchmarks

### Fixed

- None

### Breaking Changes

- None (fully backward compatible - performance improvements are automatic)

## [0.4.0] - 2024-12-15

### Added

- **Global scope cache**: Added `scope = "global"` attribute for cross-thread cache sharing
- Thread-safe global cache using `Mutex` synchronization
- New example: `global_scope` showing cross-thread cache sharing
- Test coverage for global scope functionality

### Changed

- Enhanced documentation with global scope examples

### Fixed

- None

### Breaking Changes

- None (fully backward compatible)

## [0.3.0] - 2024-11-20

### Added

- **TTL (Time To Live) support**: Automatic expiration of cache entries
- Per-entry timestamp tracking with `CacheEntry<R>` wrapper
- Automatic removal of expired entries on access
- TTL works seamlessly with all eviction policies and limits
- Comprehensive TTL example demonstrating all features
- Test coverage for TTL expiration scenarios

### Changed

- Enhanced documentation with TTL examples
- Improved error messages and validation

### Fixed

- None

### Breaking Changes

- None (fully backward compatible)

## [0.2.0] - 2024-10-10

### Added

- **Cache size limits**: Control memory usage with `limit` parameter
- **FIFO eviction policy**: First In, First Out eviction strategy
- **LRU eviction policy**: Least Recently Used eviction strategy
- Configurable eviction policies via `policy` parameter
- 7 comprehensive example files demonstrating different use cases:
    - `custom_type_default_key`
    - `custom_type_custom_key`
    - `result_caching`
    - `cache_limit`
    - `lru`
    - `fifo`
    - `fifo_default`
- Improved test coverage for eviction policies

### Changed

- Enhanced documentation with comprehensive examples
- Better error messages for invalid macro parameters

### Fixed

- None

### Breaking Changes

- None (fully backward compatible)

## [0.1.0] - 2024-09-01

### Added

- Initial release
- Basic caching functionality with `#[cache]` attribute
- Thread-local storage for cache isolation
- Custom cache key generation via `CacheableKey` trait
- Default cache key implementation via `DefaultCacheableKey`
- Result-aware caching (only `Ok` values cached)
- Support for methods (`self`, `&self`, `&mut self`)
- Comprehensive documentation and examples

[0.5.0]: https://github.com/josepdcs/cachelito/compare/v0.4.0...v0.5.0

[0.4.0]: https://github.com/josepdcs/cachelito/compare/v0.3.0...v0.4.0

[0.3.0]: https://github.com/josepdcs/cachelito/compare/v0.2.0...v0.3.0

[0.2.0]: https://github.com/josepdcs/cachelito/compare/v0.1.0...v0.2.0

[0.1.0]: https://github.com/josepdcs/cachelito/releases/tag/v0.1.0
