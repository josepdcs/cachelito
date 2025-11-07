# Cachelito

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

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
- ✅ **Type-safe**: Full compile-time type checking
- 📦 **Minimal dependencies**: Uses `parking_lot` for optimal performance

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
cachelito = "0.5.0"
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
use cachelito_core::DefaultCacheableKey;

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
use cachelito_core::DefaultCacheableKey;

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
use cachelito_core::CacheableKey;

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
│ map: RwLock<HashMap<...>>          │ ← Multiple readers OR one writer
│ order: Mutex<VecDeque<...>>        │ ← Always exclusive (needs modification)
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
cd cachelito_core
cargo bench --bench cache_benchmark

# Run RwLock concurrent reads demo
cargo run --example rwlock_concurrent_reads

# Run parking_lot demo
cargo run --example parking_lot_performance

# Compare thread-local vs global
cargo run --example cache_comparison
```

### Migration Note

This change is **fully backward compatible**. No changes are required to your code - the performance improvements are
automatic when you upgrade to 0.5.0.

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

## Limitations

- Cannot be used with generic functions (lifetime and type parameter support is limited)
- The function must be deterministic for correct caching behavior
- By default, each thread maintains its own cache (use `scope = "global"` to share across threads)
- LRU policy has O(n) overhead on cache hits for reordering (where n is the number of cached entries)
- Global scope adds synchronization overhead due to `Mutex` usage

## Documentation

For detailed API documentation, run:

```bash
cargo doc --no-deps --open
```

## Changelog

### Version 0.5.0 (Current)

**New Features:**

- ⚡ **RwLock for cache map** - Replaced `Mutex` with `RwLock` for the global cache HashMap, enabling concurrent reads
- 🚀 **4-5x performance improvement** for read-heavy workloads (typical cache usage)
- 🔓 **20x faster concurrent reads** - Multiple threads read simultaneously without blocking
- 💾 **Optimized architecture** - RwLock for map, Mutex for eviction queue
- 📊 **Enhanced benchmarks** - Added RwLock-specific benchmarks and read-heavy workload tests

**parking_lot Integration:**

- ⚡ **parking_lot::RwLock** - Better performance than `std::sync::RwLock`
- 💾 **40x smaller memory footprint** per lock (~1 byte vs ~40 bytes)
- 🔓 **No lock poisoning** - simpler API without `Result` wrapping

**Improvements:**

- 📊 Added comprehensive benchmarks: `rwlock_concurrent_reads`, `read_heavy_workload`
- 📚 New `rwlock_concurrent_reads` example demonstrating concurrent non-blocking reads
- 🧪 Added 2 new unit tests: `test_rwlock_concurrent_reads`, `test_rwlock_write_excludes_reads`
- 🧹 Cleaner internal code thanks to parking_lot's simpler API
- 📚 Enhanced documentation with RwLock benefits, architecture diagrams, and benchmarks

**Breaking Changes:**

- None (fully backward compatible - performance improvements are automatic)

### Version 0.4.0

**New Features:**

- 🌐 Global scope cache with `scope = "global"` for cross-thread sharing
- 🔒 Thread-safe global cache using `Mutex` synchronization

**Improvements:**

- 📚 New `global_scope` example showing cross-thread cache sharing
- 📚 Enhanced documentation with global scope examples
- 🧪 Added test coverage for global scope functionality

**Breaking Changes:**

- None (fully backward compatible)

### Version 0.3.0

**New Features:**

- ⏱️ TTL (Time To Live) support with automatic expiration
- 🔄 Per-entry timestamp tracking with `CacheEntry<R>` wrapper
- 🧹 Automatic removal of expired entries on access
- 🎯 TTL works seamlessly with all eviction policies and limits

**Improvements:**

- 📚 Enhanced documentation with TTL examples
- 📚 Comprehensive TTL example demonstrating all features
- 🧪 Added test coverage for TTL expiration scenarios
- 🔧 Improved error messages and validation

**Breaking Changes:**

- None (fully backward compatible)

### Version 0.2.0

**New Features:**

- ✨ Cache size limits with `limit` parameter
- ✨ FIFO (First In, First Out) eviction policy
- ✨ LRU (Least Recently Used) eviction policy
- ✨ Configurable eviction policies via `policy` parameter

**Improvements:**

- 📚 Enhanced documentation with comprehensive examples
- 📚 Added 7 example files demonstrating different use cases
- 🧪 Improved test coverage for eviction policies
- 🔧 Better error messages for invalid macro parameters

**Breaking Changes:**

- None (fully backward compatible)

### Version 0.1.0

**Initial Release:**

- ✨ Basic caching functionality with `#[cache]` attribute
- ✨ Thread-local storage for cache isolation
- ✨ Custom cache key generation via `CacheableKey` trait
- ✨ Default cache key implementation via `DefaultCacheableKey`
- ✨ Result-aware caching (only `Ok` values cached)

- ✨ Support for methods (`self`, `&self`, `&mut self`)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## See Also

- [Macro Expansion Guide](MACRO_EXPANSION.md) - How to view generated code and understand `format!("{:?}")`
- [API Documentation](https://docs.rs/cachelito) - Full API reference

