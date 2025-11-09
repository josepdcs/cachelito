# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-01-09

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

