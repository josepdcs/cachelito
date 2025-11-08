# Cachelito

[![Crates.io](https://img.shields.io/crates/v/cachelito.svg)](https://crates.io/crates/cachelito)
[![Documentation](https://docs.rs/cachelito/badge.svg)](https://docs.rs/cachelito)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-brightgreen.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/josepdcs/cachelito/rust.yml?branch=main)](https://github.com/josepdcs/cachelito/actions)

A lightweight, thread-safe caching library for Rust that provides automatic memoization through procedural macros.

## Features

- 🚀 **Easy to use**: Simply add `#[cache]` attribute to any function or method
- 🔒 **Thread-safe**: Uses `thread_local!` storage for cache isolation by default
- 🌐 **Global scope**: Optional global cache shared across all threads with `scope = "global"`
- ⚡ **High-performance synchronization**: Uses `parking_lot::RwLock` for global caches, enabling concurrent reads
- 🎯 **Flexible key generation**: Supports custom cache key implementations
- 🎨 **Result-aware**: Intelligently caches only successful `Result::Ok` values
- 🗑️ **Cache limits**: Control memory usage with configurable cache size limits
- 📊 **Eviction policies**: Choose between FIFO (First In, First Out) and LRU (Least Recently Used)
- ⏱️ **TTL support**: Time-to-live expiration for automatic cache invalidation
- 📈 **Statistics**: Track cache hit/miss rates and performance metrics (with `stats` feature)
- ✅ **Type-safe**: Full compile-time type checking
- 📦 **Minimal dependencies**: Uses `parking_lot` for optimal performance

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
cachelito = "0.6.0"

# Optional: Enable statistics tracking
cachelito = { version = "0.6.0", features = ["stats"] }
```

## Usage

### Basic Function Caching

```rust
use cachelito::cache;

#[cache]
fn fibonacci(n: u32) -> u64 {
    if n <= 1 {
        return n as u64;
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    // First call computes the result
    let result1 = fibonacci(10);

    // Second call returns cached result instantly
    let result2 = fibonacci(10);

    assert_eq!(result1, result2);
}
```

### Caching with Methods

The `#[cache]` attribute also works with methods:

```rust
use cachelito::cache;
use cachelito::DefaultCacheableKey;

#[derive(Debug, Clone)]
struct Calculator {
    precision: u32,
}

impl DefaultCacheableKey for Calculator {}

impl Calculator {
    #[cache]
    fn compute(&self, x: f64, y: f64) -> f64 {
        // Expensive computation
        x.powf(y) * self.precision as f64
    }
}
```

### Custom Cache Keys

For complex types, you can implement custom cache key generation:

#### Option 1: Use Default Debug-based Key

```rust
use cachelito::DefaultCacheableKey;

#[derive(Debug, Clone)]
struct Product {
    id: u32,
    name: String,
}

// Enable default cache key generation based on Debug
impl DefaultCacheableKey for Product {}
```

#### Option 2: Custom Key Implementation

```rust
use cachelito::CacheableKey;

#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
}

// More efficient custom key implementation
impl CacheableKey for User {
    fn to_cache_key(&self) -> String {
        format!("user:{}", self.id)
    }
}
```

### Caching Result Types

Functions returning `Result<T, E>` only cache successful results:

```rust
use cachelito::cache;

#[cache]
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("Division by zero".to_string())
    } else {
        Ok(a / b)
    }
}

fn main() {
    // Ok results are cached
    let _ = divide(10, 2); // Computes and caches Ok(5)
    let _ = divide(10, 2); // Returns cached Ok(5)

    // Err results are NOT cached (will retry each time)
    let _ = divide(10, 0); // Returns Err, not cached
    let _ = divide(10, 0); // Computes again, returns Err
}
```

### Cache Limits and Eviction Policies

Control memory usage by setting cache limits and choosing an eviction policy:

#### FIFO (First In, First Out) - Default

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using FIFO eviction
#[cache(limit = 100, policy = "fifo")]
fn expensive_computation(x: i32) -> i32 {
    // When cache is full, oldest entry is evicted
    x * x
}

// FIFO is the default policy, so this is equivalent:
#[cache(limit = 100)]
fn another_computation(x: i32) -> i32 {
    x * x
}
```

#### LRU (Least Recently Used)

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using LRU eviction
#[cache(limit = 100, policy = "lru")]
fn expensive_computation(x: i32) -> i32 {
    // When cache is full, least recently accessed entry is evicted
    // Accessing a cached value moves it to the end of the queue
    x * x
}
```

**Key Differences:**

- **FIFO**: Evicts the oldest inserted entry, regardless of usage
- **LRU**: Evicts the least recently accessed entry, keeping frequently used items longer

### Time-To-Live (TTL) Expiration

Set automatic expiration times for cached entries:

```rust
use cachelito::cache;

// Cache entries expire after 60 seconds
#[cache(ttl = 60)]
fn fetch_user_data(user_id: u32) -> UserData {
    // Entries older than 60 seconds are automatically removed
    // when accessed
    fetch_from_database(user_id)
}

// Combine TTL with limits and policies
#[cache(limit = 100, policy = "lru", ttl = 300)]
fn api_call(endpoint: &str) -> Result<Response, Error> {
    // Max 100 entries, LRU eviction, 5 minute TTL
    make_http_request(endpoint)
}
```

**Benefits:**

- **Automatic expiration**: Old data is automatically removed
- **Per-entry tracking**: Each entry has its own timestamp
- **Lazy eviction**: Expired entries removed on access
- **Works with policies**: Compatible with FIFO and LRU

### Global Scope Cache

By default, each thread has its own cache (thread-local). Use `scope = "global"` to share the cache across all threads:

```rust
use cachelito::cache;

// Thread-local cache (default) - each thread has its own cache
#[cache(limit = 100)]
fn thread_local_computation(x: i32) -> i32 {
    // Cache is NOT shared across threads
    x * x
}

// Global cache - shared across all threads
#[cache(limit = 100, scope = "global")]
fn global_computation(x: i32) -> i32 {
    // Cache IS shared across all threads
    // Uses Mutex for thread-safe access
    x * x
}
```

**When to use global scope:**

- **Cross-thread sharing**: When you want all threads to benefit from cached results
- **Expensive operations**: When the cost of computation outweighs the synchronization overhead
- **Shared data**: When the same function is called with the same arguments across multiple threads

**Performance considerations:**

- **Thread-local** (default): No synchronization overhead, but cache is not shared
- **Global**: Uses `Mutex` for synchronization, adds overhead but shares cache across threads

```rust
use cachelito::cache;
use std::thread;

#[cache(scope = "global", limit = 50)]
fn expensive_api_call(endpoint: &str) -> String {
    // This expensive call is cached globally
    // All threads benefit from the same cache
    format!("Response from {}", endpoint)
}

fn main() {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                // All threads share the same cache
                // First thread computes, others get cached result
                expensive_api_call("/api/users")
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
```

### Performance with Large Values

The cache clones values on every `get` operation. For large values (big structs, vectors, strings), this can be
expensive. Wrap your return values in `Arc<T>` to share ownership without copying data:

#### Problem: Expensive Cloning

```rust
use cachelito::cache;

#[derive(Clone, Debug)]
struct LargeData {
    payload: Vec<u8>, // Could be megabytes of data
    metadata: String,
}

#[cache(limit = 100)]
fn process_data(id: u32) -> LargeData {
    LargeData {
        payload: vec![0u8; 1_000_000], // 1MB of data
        metadata: format!("Data for {}", id),
    }
}

fn main() {
    // First call: computes and caches (1MB allocation)
    let data1 = process_data(42);

    // Second call: clones the ENTIRE 1MB! (expensive)
    let data2 = process_data(42);
}
```

#### Solution: Use Arc<T>

```rust
use cachelito::cache;
use std::sync::Arc;

#[derive(Debug)]
struct LargeData {
    payload: Vec<u8>,
    metadata: String,
}

// Return Arc instead of the value directly
#[cache(limit = 100)]
fn process_data(id: u32) -> Arc<LargeData> {
    Arc::new(LargeData {
        payload: vec![0u8; 1_000_000], // 1MB of data
        metadata: format!("Data for {}", id),
    })
}

fn main() {
    // First call: computes and caches Arc (1MB allocation)
    let data1 = process_data(42);

    // Second call: clones only the Arc pointer (cheap!)
    // The 1MB payload is NOT cloned
    let data2 = process_data(42);

    // Both Arc point to the same underlying data
    assert!(Arc::ptr_eq(&data1, &data2));
}
```

#### Real-World Example: Caching Parsed Data

```rust
use cachelito::cache;
use std::sync::Arc;

#[derive(Debug)]
struct ParsedDocument {
    title: String,
    content: String,
    tokens: Vec<String>,
    word_count: usize,
}

// Cache expensive parsing operations
#[cache(limit = 50, policy = "lru", ttl = 3600)]
fn parse_document(file_path: &str) -> Arc<ParsedDocument> {
    // Expensive parsing operation
    let content = std::fs::read_to_string(file_path).unwrap();
    let tokens: Vec<String> = content
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    Arc::new(ParsedDocument {
        title: extract_title(&content),
        content,
        word_count: tokens.len(),
        tokens,
    })
}

fn analyze_document(path: &str) {
    // First access: parses file (expensive)
    let doc = parse_document(path);
    println!("Title: {}", doc.title);

    // Subsequent accesses: returns Arc clone (cheap)
    let doc2 = parse_document(path);
    println!("Words: {}", doc2.word_count);

    // The underlying ParsedDocument is shared, not cloned
}
```

#### When to Use Arc<T>

**Use Arc<T> when:**

- ✅ Values are large (>1KB)
- ✅ Values contain collections (Vec, HashMap, String)
- ✅ Values are frequently accessed from cache
- ✅ Multiple parts of your code need access to the same data

**Don't need Arc<T> when:**

- ❌ Values are small primitives (i32, f64, bool)
- ❌ Values are rarely accessed from cache
- ❌ Clone is already cheap (e.g., types with `Copy` trait)

#### Combining Arc with Global Scope

For maximum efficiency with multi-threaded applications:

```rust
use cachelito::cache;
use std::sync::Arc;
use std::thread;

#[cache(scope = "global", limit = 100, policy = "lru")]
fn fetch_user_profile(user_id: u64) -> Arc<UserProfile> {
    // Expensive database or API call
    Arc::new(UserProfile::fetch_from_db(user_id))
}

fn main() {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                // All threads share the global cache
                // Cloning Arc is cheap across threads
                let profile = fetch_user_profile(42);
                println!("User: {}", profile.name);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
```

**Benefits:**

- 🚀 Only one database/API call across all threads
- 💾 Minimal memory overhead (Arc clones are just pointer + ref count)
- 🔒 Thread-safe sharing with minimal synchronization cost
- ⚡ Fast cache access with no data copying

## Synchronization with parking_lot

Starting from version **0.5.0**, Cachelito uses [`parking_lot`](https://crates.io/crates/parking_lot) for
synchronization in global scope caches. The implementation uses **RwLock for the cache map** and **Mutex for the
eviction queue**, providing optimal performance for read-heavy workloads.

### Why parking_lot + RwLock?

**RwLock Benefits (for the cache map):**

- **Concurrent reads**: Multiple threads can read simultaneously without blocking
- **4-5x faster** for read-heavy workloads (typical for caches)
- **Perfect for 90/10 read/write ratio** (common in cache scenarios)
- Only writes acquire exclusive lock

**parking_lot Advantages over std::sync:**

- **30-50% faster** under high contention scenarios
- **Adaptive spinning** for short critical sections (faster than kernel-based locks)
- **Fair scheduling** prevents thread starvation
- **No lock poisoning** - simpler API without `Result` wrapping
- **~40x smaller** memory footprint per lock (~1 byte vs ~40 bytes)

### Architecture

```
GlobalCache Structure:
┌─────────────────────────────────────┐
│ map: RwLock<HashMap<...>>           │ ← Multiple readers OR one writer
│ order: Mutex<VecDeque<...>>         │ ← Always exclusive (needs modification)
└─────────────────────────────────────┘

Read Operation (cache hit):
Thread 1 ──┐
Thread 2 ──┼──> RwLock.read() ──> ✅ Concurrent, no blocking
Thread 3 ──┘

Write Operation (cache miss):
Thread 1 ──> RwLock.write() ──> ⏳ Exclusive access
```

### Benchmark Results

Performance comparison on concurrent cache access:

**Mixed workload** (8 threads, 100 operations, 90% reads / 10% writes):

```
Thread-Local Cache:      1.26ms  (no synchronization baseline)
Global + RwLock:         1.84ms  (concurrent reads)
Global + Mutex only:     ~3.20ms (all operations serialized)
std::sync::RwLock:       ~2.80ms (less optimized)

Improvement: RwLock is ~74% faster than Mutex for read-heavy workloads
```

**Pure concurrent reads** (20 threads, 100 reads each):

```
With RwLock:    ~2ms   (all threads read simultaneously)
With Mutex:     ~40ms  (threads wait in queue)

20x improvement for concurrent reads!
```

### Code Simplification

With `parking_lot`, the internal code is cleaner:

```rust
// Read operation (concurrent with RwLock)
let value = self .map.read().get(key).cloned();

// Write operation (exclusive)
self .map.write().insert(key, value);
```

### Running the Benchmarks

You can run the included benchmarks to see the performance on your hardware:

```bash
# Run cache benchmarks (includes RwLock concurrent reads)
cd cachelito-core
cargo bench --bench cache_benchmark

# Run RwLock concurrent reads demo
cargo run --example rwlock_concurrent_reads

# Run parking_lot demo
cargo run --example parking_lot_performance

# Compare thread-local vs global
cargo run --example cache_comparison
```

## How It Works

The `#[cache]` macro generates code that:

1. Creates a thread-local cache using `thread_local!` and `RefCell<HashMap>`
2. Creates a thread-local order queue using `VecDeque` for eviction tracking
3. Wraps cached values in `CacheEntry` to track insertion timestamps
4. Builds a cache key from function arguments using `CacheableKey::to_cache_key()`
5. Checks the cache before executing the function body
6. Validates TTL expiration if configured, removing expired entries
7. Stores the result in the cache after execution
8. For `Result<T, E>` types, only caches `Ok` values
9. When cache limit is reached, evicts entries according to the configured policy:
    - **FIFO**: Removes the oldest inserted entry
    - **LRU**: Removes the least recently accessed entry

## Examples

The library includes several comprehensive examples demonstrating different features:

### Run Examples

```bash
# Basic caching with custom types (default cache key)
cargo run --example custom_type_default_key

# Custom cache key implementation
cargo run --example custom_type_custom_key

# Result type caching (only Ok values cached)
cargo run --example result_caching

# Cache limits with LRU policy
cargo run --example cache_limit

# LRU eviction policy
cargo run --example lru

# FIFO eviction policy
cargo run --example fifo

# Default policy (FIFO)
cargo run --example fifo_default

# TTL (Time To Live) expiration
cargo run --example ttl

# Global scope cache (shared across threads)
cargo run --example global_scope
```

### Example Output (LRU Policy):

```
=== Testing LRU Cache Policy ===

Calling compute_square(1)...
Executing compute_square(1)
Result: 1

Calling compute_square(2)...
Executing compute_square(2)
Result: 4

Calling compute_square(3)...
Executing compute_square(3)
Result: 9

Calling compute_square(2)...
Result: 4 (should be cached)

Calling compute_square(4)...
Executing compute_square(4)
Result: 16

...

Total executions: 6
✅ LRU Policy Test PASSED
```

## Performance Considerations

- **Thread-local storage** (default): Each thread has its own cache, so cached data is not shared across threads. This
  means no locks or synchronization overhead.
- **Global scope**: When using `scope = "global"`, the cache is shared across all threads using a `Mutex`. This adds
  synchronization overhead but allows cache sharing.
- **Memory usage**: Without a limit, the cache grows unbounded. Use the `limit` parameter to control memory usage.
- **Cache key generation**: Uses `CacheableKey::to_cache_key()` method. The default implementation uses `Debug`
  formatting, which may be slow for complex types. Consider implementing `CacheableKey` directly for better performance.
- **Value cloning**: The cache clones values on every access. For large values (>1KB), wrap them in `Arc<T>` to avoid
  expensive clones. See the [Performance with Large Values](#performance-with-large-values) section for details.
- **Cache hit performance**: O(1) hash map lookup, with LRU having an additional O(n) reordering cost on hits
    - **FIFO**: Minimal overhead, O(1) eviction
    - **LRU**: Slightly higher overhead due to reordering on access, O(n) for reordering but still efficient

## Cache Statistics

**Available since v0.6.0** with the `stats` feature flag.

Track cache performance metrics including hit/miss rates and access counts. Statistics are automatically collected for
global-scoped caches and can be queried programmatically.

### Enabling Statistics

Add the `stats` feature to your `Cargo.toml`:

```toml
[dependencies]
cachelito = { version = "0.6.0", features = ["stats"] }
```

### Basic Usage

Statistics are automatically tracked for caches with `scope = "global"`:

```rust
use cachelito::cache;

#[cache(scope = "global", limit = 100, policy = "lru")]
fn expensive_operation(x: i32) -> i32 {
    // Simulate expensive work
    std::thread::sleep(std::time::Duration::from_millis(100));
    x * x
}

fn main() {
    // Make some calls
    expensive_operation(5);  // Miss - computes
    expensive_operation(5);  // Hit - cached
    expensive_operation(10); // Miss - computes
    expensive_operation(5);  // Hit - cached

    // Access statistics using the registry
    #[cfg(feature = "stats")]
    if let Some(stats) = cachelito::stats_registry::get("expensive_operation") {
        println!("Total accesses: {}", stats.total_accesses());
        println!("Cache hits:     {}", stats.hits());
        println!("Cache misses:   {}", stats.misses());
        println!("Hit rate:       {:.2}%", stats.hit_rate() * 100.0);
        println!("Miss rate:      {:.2}%", stats.miss_rate() * 100.0);
    }
}
```

Output:

```
Total accesses: 4
Cache hits:     2
Cache misses:   2
Hit rate:       50.00%
Miss rate:      50.00%
```

### Statistics Registry API

The `stats_registry` module provides centralized access to all cache statistics:

#### Get Statistics

```rust
use cachelito::stats_registry;

// Get a snapshot of statistics for a function
if let Some(stats) = stats_registry::get("my_function") {
println ! ("Hits: {}", stats.hits());
println ! ("Misses: {}", stats.misses());
}

// Get direct reference (no cloning)
if let Some(stats) = stats_registry::get_ref("my_function") {
println ! ("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
}
```

#### List All Cached Functions

```rust
use cachelito::stats_registry;

// Get names of all registered cache functions
let functions = stats_registry::list();
for name in functions {
if let Some(stats) = stats_registry::get( & name) {
println ! ("{}: {} hits, {} misses", name, stats.hits(), stats.misses());
}
}
```

#### Reset Statistics

```rust
use cachelito::stats_registry;

// Reset stats for a specific function
if stats_registry::reset("my_function") {
println ! ("Statistics reset successfully");
}

// Clear all registrations (useful for testing)
stats_registry::clear();
```

### Statistics Metrics

The `CacheStats` struct provides the following metrics:

- `hits()` - Number of successful cache lookups
- `misses()` - Number of cache misses (computation required)
- `total_accesses()` - Total number of get operations
- `hit_rate()` - Ratio of hits to total accesses (0.0 to 1.0)
- `miss_rate()` - Ratio of misses to total accesses (0.0 to 1.0)
- `reset()` - Reset all counters to zero

### Concurrent Statistics Example

Statistics are thread-safe and work correctly with concurrent access:

```rust
use cachelito::cache;
use std::thread;

#[cache(scope = "global", limit = 100)]
fn compute(n: u32) -> u32 {
    n * n
}

fn main() {
    // Spawn multiple threads
    let handles: Vec<_> = (0..5)
        .map(|_| {
            thread::spawn(|| {
                for i in 0..20 {
                    compute(i);
                }
            })
        })
        .collect();

    // Wait for completion
    for handle in handles {
        handle.join().unwrap();
    }

    // Check statistics
    #[cfg(feature = "stats")]
    if let Some(stats) = cachelito::stats_registry::get("compute") {
        println!("Total accesses: {}", stats.total_accesses());
        println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        // Expected: ~80% hit rate since first thread computes,
        // others find values in cache
    }
}
```

### Monitoring Cache Performance

Use statistics to monitor and optimize cache performance:

```rust
use cachelito::{cache, stats_registry};

#[cache(scope = "global", limit = 50, policy = "lru")]
fn api_call(endpoint: &str) -> String {
    // Expensive API call
    format!("Data from {}", endpoint)
}

fn monitor_cache_health() {
    #[cfg(feature = "stats")]
    if let Some(stats) = stats_registry::get("api_call") {
        let hit_rate = stats.hit_rate();

        if hit_rate < 0.5 {
            eprintln!("⚠️ Low cache hit rate: {:.2}%", hit_rate * 100.0);
            eprintln!("Consider increasing cache limit or adjusting TTL");
        } else if hit_rate > 0.9 {
            println!("✅ Excellent cache performance: {:.2}%", hit_rate * 100.0);
        }

        println!("Cache stats: {} hits / {} total",
                 stats.hits(), stats.total_accesses());
    }
}
```

### Custom Cache Names

Use the `name` attribute to give your caches custom identifiers in the statistics registry:

```rust
use cachelito::cache;

// API V1 - using custom name
#[cache(scope = "global", limit = 50, name = "api_v1")]
fn fetch_data(id: u32) -> String {
    format!("V1 Data for ID {}", id)
}

// API V2 - using custom name
#[cache(scope = "global", limit = 50, name = "api_v2")]
fn fetch_data_v2(id: u32) -> String {
    format!("V2 Data for ID {}", id)
}

// Access statistics using custom names
#[cfg(feature = "stats")]
{
if let Some(stats) = cachelito::stats_registry::get("api_v1") {
println ! ("V1 hit rate: {:.2}%", stats.hit_rate() * 100.0);
}
if let Some(stats) = cachelito::stats_registry::get("api_v2") {
println ! ("V2 hit rate: {:.2}%", stats.hit_rate() * 100.0);
}
}
```

**Benefits:**

- **Descriptive names**: Use meaningful identifiers instead of function names
- **Multiple versions**: Track different implementations separately
- **Easier debugging**: Identify caches by purpose rather than function name
- **Better monitoring**: Compare performance of different cache strategies

**Default behavior:** If `name` is not provided, the function name is used as the identifier.

### Important Notes

- **Global scope only**: Statistics are only available via `stats_registry` for caches with `scope = "global"`
- **Thread-local statistics**: Thread-local caches (default) **DO track statistics** internally via
  the `ThreadLocalCache::stats` field, but these are **NOT accessible via `stats_registry::get()`**
  due to architectural limitations. See [THREAD_LOCAL_STATS.md](THREAD_LOCAL_STATS.md) for a detailed explanation.
- **Performance**: Statistics use atomic operations (minimal overhead)
- **Feature flag**: Statistics are only compiled when the `stats` feature is enabled

**Why thread-local stats aren't in `stats_registry`:**

- Each thread has its own independent cache and statistics
- Thread-local statics (`thread_local!`) cannot be registered in a global registry
- To access statistics programmatically, use `scope = "global"`
- Thread-local stats are still useful for testing and internal debugging

## Limitations

- Cannot be used with generic functions (lifetime and type parameter support is limited)
- The function must be deterministic for correct caching behavior
- By default, each thread maintains its own cache (use `scope = "global"` to share across threads)
- LRU policy has O(n) overhead on cache hits for reordering (where n is the number of cached entries)
- Global scope adds synchronization overhead due to `Mutex` usage
- Statistics are only available for global-scoped caches (thread-local statistics are tracked but not externally
  accessible)

## Documentation

For detailed API documentation, run:

```bash
cargo doc --no-deps --open
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes.

### Latest Release: Version 0.6.0

**Highlights:**

- 📈 **Cache Statistics** - Track hit/miss rates and performance metrics with the `stats` feature
- 🎯 **Stats Registry** - Centralized API for querying statistics: `stats_registry::get("function_name")`
- 🏷️ **Custom Cache Names** - Use `name` attribute to give caches custom identifiers: `#[cache(name = "my_cache")]`
- 🔍 **Performance Monitoring** - Monitor cache effectiveness with detailed metrics
- ⚡ **Thread-safe Statistics** - Atomic counters for concurrent access
- 📊 **Rich Metrics** - Access hits, misses, total accesses, hit rate, and miss rate
- 🧹 **Statistics Management** - Reset, clear, and list all cached functions

**Statistics Features:**

```rust
// Enable with feature flag
cachelito = { version = "0.6.0", features = ["stats"] }

// Access statistics with default name (function name)
if let Some(stats) = cachelito::stats_registry::get("my_function") {
println ! ("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
}

// Or use a custom name for better organization
#[cache(scope = "global", name = "api_cache")]
fn fetch_data() -> Data { ... }

if let Some(stats) = cachelito::stats_registry::get("api_cache") {
println ! ("API cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
}
```

For full details, see the [complete changelog](CHANGELOG.md).

### Previous Release: Version 0.5.0

**Highlights:**

- ⚡ **RwLock for concurrent reads** - 4-5x faster for read-heavy workloads
- 🚀 **20x improvement** for pure concurrent reads
- 💾 **40x smaller memory footprint** with parking_lot
- 📊 **Enhanced benchmarks** and examples
- 🔧 **Idiomatic crate naming** (`cachelito-core`, `cachelito-macros`)

For full details, see the [complete changelog](CHANGELOG.md).

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## See Also

- [CHANGELOG](CHANGELOG.md) - Detailed version history and release notes
- [Macro Expansion Guide](MACRO_EXPANSION.md) - How to view generated code and understand `format!("{:?}")`
- [Thread-Local Statistics](THREAD_LOCAL_STATS.md) - Why thread-local cache stats aren't in `stats_registry` and how
  they work
- [API Documentation](https://docs.rs/cachelito) - Full API reference

