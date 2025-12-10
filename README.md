# Cachelito

[![Crates.io](https://img.shields.io/crates/v/cachelito.svg)](https://crates.io/crates/cachelito)
[![Documentation](https://docs.rs/cachelito/badge.svg)](https://docs.rs/cachelito)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-brightgreen.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/josepdcs/cachelito/rust.yml?branch=main)](https://github.com/josepdcs/cachelito/actions)

A lightweight, thread-safe caching library for Rust that provides automatic memoization through procedural macros.

## Features

- 🚀 **Easy to use**: Simply add `#[cache]` attribute to any function or method
- 🌐 **Global scope by default**: Cache shared across all threads (use `scope = "thread"` for thread-local)
- ⚡ **High-performance synchronization**: Uses `parking_lot::RwLock` for global caches, enabling concurrent reads
- 🔒 **Thread-local option**: Optional thread-local storage with `scope = "thread"` for maximum performance
- 🎯 **Flexible key generation**: Supports custom cache key implementations
- 🎨 **Result-aware**: Intelligently caches only successful `Result::Ok` values
- 🗑️ **Cache entry limits**: Control growth with numeric `limit`
- 💾 **Memory-based limits (v0.10.0)**: New `max_memory = "100MB"` attribute for memory-aware eviction
- 📊 **Eviction policies**: FIFO, LRU (default), LFU *(v0.8.0)*, ARC *(v0.9.0)*, Random *(v0.11.0)*
- 🎯 **ARC (Adaptive Replacement Cache)**: Self-tuning policy combining recency & frequency
- 🎲 **Random Replacement**: O(1) eviction for baseline benchmarks and random access patterns
- ⏱️ **TTL support**: Time-to-live expiration for automatic cache invalidation
- 🔥 **Smart Invalidation (v0.12.0)**: Tag-based, event-driven, and dependency-based cache invalidation
- 🎯 **Conditional Invalidation (v0.13.0)**: Runtime invalidation with custom check functions and named invalidation checks
- 🎛️ **Conditional Caching (v0.14.0)**: Control when results are cached with `cache_if` predicate functions
- 📏 **MemoryEstimator trait**: Used internally for memory-based limits (customizable for user types)
- 📈 **Statistics (v0.6.0+)**: Track hit/miss rates via `stats` feature & `stats_registry`
- 🔮 **Async/await support (v0.7.0)**: Dedicated `cachelito-async` crate (lock-free DashMap)
- ✅ **Type-safe**: Full compile-time type checking
- 📦 **Minimal dependencies**: Uses `parking_lot` for optimal performance

## Quick Start

### For Synchronous Functions

Add this to your `Cargo.toml`:

```toml
[dependencies]
cachelito = "0.14.0"
# Or with statistics:
# cachelito = { version = "0.14.0", features = ["stats"] }
```

### For Async Functions

> **Note:** `cachelito-async` follows the same versioning as `cachelito` core (0.14.x).
```toml
[dependencies]
cachelito-async = "0.14.0"
tokio = { version = "1", features = ["full"] }
```

## Which Version Should I Use?

| Use Case                | Crate                            | Macro                           | Best For                                       |
|-------------------------|----------------------------------|---------------------------------|------------------------------------------------|
| **Sync functions**      | `cachelito`                      | `#[cache]`                      | CPU-bound computations                        |
| **Async functions**     | `cachelito-async`                | `#[cache_async]`                | I/O-bound / network operations                |
| **Thread-local cache**  | `cachelito`                      | `#[cache(scope = "thread")]`    | Per-thread isolated cache                     |
| **Global shared cache** | `cachelito` / `cachelito-async`  | `#[cache]` / `#[cache_async]`   | Cross-thread/task sharing                     |
| **High concurrency**    | `cachelito-async`                | `#[cache_async]`                | Many concurrent async tasks                   |
| **Statistics tracking** | `cachelito` (v0.6.0+)            | `#[cache]` + feature `stats`    | Performance monitoring                        |
| **Memory limits**       | `cachelito` (v0.10.0)            | `#[cache(max_memory = "64MB")]` | Large objects / controlled memory usage       |

**Quick Decision:**
- 🔄 Synchronous code? → Use `cachelito`
- ⚡ Async/await code? → Use `cachelito-async`
- 💾 Need memory-based eviction? → Use `cachelito` v0.10.0+

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

#### LRU (Least Recently Used) - Default

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using LRU eviction
#[cache(limit = 100, policy = "lru")]
fn expensive_computation(x: i32) -> i32 {
    // When cache is full, least recently accessed entry is evicted
    // Accessing a cached value moves it to the end of the queue
    x * x
}

// LRU is the default policy, so this is equivalent:
#[cache(limit = 100)]
fn another_computation(x: i32) -> i32 {
    x * x
}
```

#### FIFO (First In, First Out)

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using FIFO eviction
#[cache(limit = 100, policy = "fifo")]
fn expensive_computation(x: i32) -> i32 {
    // When cache is full, oldest entry is evicted
    x * x
}
```

#### LFU (Least Frequently Used)

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using LFU eviction
#[cache(limit = 100, policy = "lfu")]
fn expensive_computation(x: i32) -> i32 {
    // When cache is full, least frequently accessed entry is evicted
    // Each access increments the frequency counter
    x * x
}
```

#### ARC (Adaptive Replacement Cache)

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using ARC eviction
#[cache(limit = 100, policy = "arc")]
fn expensive_computation(x: i32) -> i32 {
    // Self-tuning cache that adapts between recency and frequency
    // Combines the benefits of LRU and LFU automatically
    // Best for mixed workloads with varying access patterns
    x * x
}
```

#### Random Replacement

```rust
use cachelito::cache;

// Cache with a limit of 100 entries using Random eviction
#[cache(limit = 100, policy = "random")]
fn expensive_computation(x: i32) -> i32 {
    // When cache is full, a random entry is evicted
    // Minimal overhead, useful for benchmarks and random access patterns
    x * x
}
```

**Policy Comparison:**

| Policy | Evicts                            | Best For                                  | Performance     |
|--------|-----------------------------------|-------------------------------------------|-----------------|
| **LRU** | Least recently accessed          | Temporal locality (recent items matter)   | O(n) on hit     |
| **FIFO** | Oldest inserted                 | Simple, predictable behavior              | O(1)            |
| **LFU** | Least frequently accessed        | Frequency patterns (popular items matter) | O(n) on evict   |
| **ARC** | Adaptive (recency + frequency)   | Mixed workloads, self-tuning              | O(n) on evict/hit |
| **Random** | Randomly selected              | Baseline benchmarks, random access        | O(1)            |

**Choosing the Right Policy:**

- **FIFO**: Simple, predictable, minimal overhead. Use when you just need basic caching.
- **LRU**: Best for most use cases with temporal locality (recent items are likely to be accessed again).
- **LFU**: Best when certain items are accessed much more frequently (like "hot" products in e-commerce).
- **ARC**: Best for workloads with mixed patterns - automatically adapts between recency and frequency.
- **Random**: Best for baseline benchmarks, truly random access patterns, or when minimizing overhead is critical.

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

By default, the cache is shared across all threads (global scope). Use `scope = "thread"` for thread-local caches where
each thread has its own independent cache:

```rust
use cachelito::cache;

// Global cache (default) - shared across all threads
#[cache(limit = 100)]
fn global_computation(x: i32) -> i32 {
    // Cache IS shared across all threads
    // Uses RwLock for thread-safe access
    x * x
}

// Thread-local cache - each thread has its own cache
#[cache(limit = 100, scope = "thread")]
fn thread_local_computation(x: i32) -> i32 {
    // Cache is NOT shared across threads
    // No synchronization overhead
    x * x
}
```

**When to use global scope (default):**

- ✅ **Cross-thread sharing**: All threads benefit from cached results
- ✅ **Statistics monitoring**: Full access to cache statistics via `stats_registry`
- ✅ **Expensive operations**: Computation cost outweighs synchronization overhead
- ✅ **Shared data**: Same function called with same arguments across threads

**When to use thread-local (`scope = "thread"`):**

- ✅ **Maximum performance**: No synchronization overhead
- ✅ **Thread isolation**: Each thread needs independent cache
- ✅ **Thread-specific data**: Different threads process different data

**Performance considerations:**

- **Global** (default): Uses `RwLock` for synchronization, allows concurrent reads
- **Thread-local**: No synchronization overhead, but cache is not shared

```rust
use cachelito::cache;
use std::thread;

#[cache(limit = 50)]  // Global by default
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

## Async/Await Support

Starting with version 0.7.0, Cachelito provides dedicated support for async/await functions through the
`cachelito-async` crate.

### Installation

```toml
[dependencies]
cachelito-async = "0.2.0"
tokio = { version = "1", features = ["full"] }
# or use async-std, smol, etc.
```

### Quick Example

```rust
use cachelito_async::cache_async;
use std::time::Duration;

#[cache_async(limit = 100, policy = "lru", ttl = 60)]
async fn fetch_user(id: u64) -> Result<User, Error> {
    // Expensive async operation (database, API call, etc.)
    let user = database::get_user(id).await?;
    Ok(user)
}

#[tokio::main]
async fn main() {
    // First call: fetches from database (~100ms)
    let user1 = fetch_user(42).await.unwrap();

    // Second call: returns cached result (instant)
    let user2 = fetch_user(42).await.unwrap();

    assert_eq!(user1.id, user2.id);
}
```

### Key Features of Async Cache

| Feature            | Sync (`#[cache]`)                       | Async (`#[cache_async]`)       |
|--------------------|-----------------------------------------|--------------------------------|
| **Scope**          | Global or Thread-local                  | **Always Global**              |
| **Storage**        | `RwLock<HashMap>` or `RefCell<HashMap>` | **`DashMap`** (lock-free)      |
| **Concurrency**    | `parking_lot::RwLock`                   | **Lock-free concurrent**       |
| **Best for**       | CPU-bound operations                    | **I/O-bound async operations** |
| **Blocking**       | May block on lock                       | **No blocking**                |
| **Policies**       | FIFO, LRU                               | FIFO, LRU                      |
| **TTL**            | ✅ Supported                             | ✅ Supported                    |

### Why DashMap for Async?

The async version uses [DashMap](https://docs.rs/dashmap) instead of traditional locks because:

- ✅ **Lock-free**: No blocking, perfect for async contexts
- ✅ **High concurrency**: Multiple tasks can access cache simultaneously
- ✅ **No async overhead**: Cache operations don't require `.await`
- ✅ **Thread-safe**: Safe to share across tasks and threads
- ✅ **Performance**: Optimized for high-concurrency scenarios

### Limitations

- **Always Global**: No thread-local option (not needed in async context)
- **Cache Stampede**: Multiple concurrent requests for the same key may execute simultaneously
  (consider using request coalescing patterns for production use)

### Complete Documentation

See the [`cachelito-async` README](cachelito-async/README.md) for:

- Detailed API documentation
- More examples (LRU, concurrent access, TTL)
- Performance considerations
- Migration guide from sync version

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

# Async examples (requires cachelito-async)
cargo run --example async_basic --manifest-path cachelito-async/Cargo.toml
cargo run --example async_lru --manifest-path cachelito-async/Cargo.toml
cargo run --example async_concurrent --manifest-path cachelito-async/Cargo.toml
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

Statistics are automatically tracked for global caches (default):

```rust
use cachelito::cache;

#[cache(limit = 100, policy = "lru")]  // Global by default
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

fn main() {
    // Get a snapshot of statistics for a function
    if let Some(stats) = stats_registry::get("my_function") {
        println!("Hits: {}", stats.hits());
        println!("Misses: {}", stats.misses());
    }

    // Get direct reference (no cloning)
    if let Some(stats) = stats_registry::get_ref("my_function") {
        println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
    }
}
```

#### List All Cached Functions

```rust
use cachelito::stats_registry;

fn main() { 
    // Get names of all registered cache functions 
    let functions = stats_registry::list();
    for name in functions { 
        if let Some(stats) = stats_registry::get(&name) {
            println!("{}: {} hits, {} misses", name, stats.hits(), stats.misses());
        }
    }
}
```

#### Reset Statistics

```rust
use cachelito::stats_registry;

fn main() {
    // Reset stats for a specific function
    if stats_registry::reset("my_function") {
        println!("Statistics reset successfully");
    }

    // Clear all registrations (useful for testing)
    stats_registry::clear();
}
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

#[cache(limit = 100)]  // Global by default
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

#[cache(limit = 50, policy = "lru")]  // Global by default
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

// API V1 - using custom name (global by default)
#[cache(limit = 50, name = "api_v1")]
fn fetch_data(id: u32) -> String {
    format!("V1 Data for ID {}", id)
}

// API V2 - using custom name (global by default)
#[cache(limit = 50, name = "api_v2")]
fn fetch_data_v2(id: u32) -> String {
    format!("V2 Data for ID {}", id)
}

fn main() {
    // Make some calls
    fetch_data(1);
    fetch_data(1);
    fetch_data_v2(2);
    fetch_data_v2(2);
    fetch_data_v2(3);
    // Access statistics using custom names
    #[cfg(feature = "stats")]
    {
        if let Some(stats) = cachelito::stats_registry::get("api_v1") {
            println!("V1 hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }
        if let Some(stats) = cachelito::stats_registry::get("api_v2") {
            println!("V2 hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }
    }
}
```

**Benefits:**

- **Descriptive names**: Use meaningful identifiers instead of function names
- **Multiple versions**: Track different implementations separately
- **Easier debugging**: Identify caches by purpose rather than function name
- **Better monitoring**: Compare performance of different cache strategies

**Default behavior:** If `name` is not provided, the function name is used as the identifier.

## Smart Cache Invalidation

Starting from version 0.12.0, Cachelito supports smart invalidation mechanisms beyond simple TTL expiration, providing fine-grained control over when and how cached entries are invalidated.

### Invalidation Strategies

Cachelito supports three complementary invalidation strategies:

1. **Tag-based invalidation**: Group related entries and invalidate them together
2. **Event-driven invalidation**: Trigger invalidation when specific events occur
3. **Dependency-based invalidation**: Cascade invalidation to dependent caches

### Tag-Based Invalidation

Use tags to group related cache entries and invalidate them together:

```rust
use cachelito::{cache, invalidate_by_tag};

#[cache(
    scope = "global",
    tags = ["user_data", "profile"],
    name = "get_user_profile"
)]
fn get_user_profile(user_id: u64) -> UserProfile {
    // Expensive database query
    fetch_user_from_db(user_id)
}

#[cache(
    scope = "global",
    tags = ["user_data", "settings"],
    name = "get_user_settings"
)]
fn get_user_settings(user_id: u64) -> UserSettings {
    fetch_settings_from_db(user_id)
}

// Later, when user data is updated:
invalidate_by_tag("user_data"); // Invalidates both functions
```

### Event-Driven Invalidation

Trigger cache invalidation based on application events:

```rust
use cachelito::{cache, invalidate_by_event};

#[cache(
    scope = "global",
    events = ["user_updated", "permissions_changed"],
    name = "get_user_permissions"
)]
fn get_user_permissions(user_id: u64) -> Vec<String> {
    fetch_permissions_from_db(user_id)
}

// When a permission changes:
invalidate_by_event("permissions_changed");

// When user profile is updated:
invalidate_by_event("user_updated");
```

### Dependency-Based Invalidation

Create cascading invalidation when dependent caches change:

```rust
use cachelito::{cache, invalidate_by_dependency};

#[cache(scope = "global", name = "get_user")]
fn get_user(user_id: u64) -> User {
    fetch_user_from_db(user_id)
}

#[cache(
    scope = "global",
    dependencies = ["get_user"],
    name = "get_user_dashboard"
)]
fn get_user_dashboard(user_id: u64) -> Dashboard {
    // This cache depends on get_user
    build_dashboard(user_id)
}

// When the user cache changes:
invalidate_by_dependency("get_user"); // Invalidates get_user_dashboard
```

### Combining Multiple Strategies

You can combine tags, events, and dependencies for maximum flexibility:

```rust
use cachelito::cache;

#[cache(
    scope = "global",
    tags = ["user_data", "dashboard"],
    events = ["user_updated"],
    dependencies = ["get_user_profile", "get_user_permissions"],
    name = "get_user_dashboard"
)]
fn get_user_dashboard(user_id: u64) -> Dashboard {
    // This cache can be invalidated by:
    // - Tag: invalidate_by_tag("user_data")
    // - Event: invalidate_by_event("user_updated")
    // - Dependency: invalidate_by_dependency("get_user_profile")
    build_dashboard(user_id)
}
```

### Manual Cache Invalidation

Invalidate specific caches by their name:

```rust
use cachelito::invalidate_cache;

// Invalidate a specific cache function
if invalidate_cache("get_user_profile") {
    println!("Cache invalidated successfully");
}
```

### Invalidation API

The invalidation API is simple and intuitive:

- `invalidate_by_tag(tag: &str) -> usize` - Returns the number of caches invalidated
- `invalidate_by_event(event: &str) -> usize` - Returns the number of caches invalidated
- `invalidate_by_dependency(dependency: &str) -> usize` - Returns the number of caches invalidated
- `invalidate_cache(cache_name: &str) -> bool` - Returns `true` if the cache was found and invalidated

### Benefits

- **Fine-grained control**: Invalidate only what needs to be invalidated
- **Event-driven**: React to application events automatically
- **Cascading updates**: Maintain consistency across dependent caches
- **Flexible grouping**: Use tags to organize related caches
- **Performance**: No overhead when invalidation attributes are not used

### Conditional Invalidation with Check Functions (v0.13.0)

For even more control, you can use custom check functions (predicates) to selectively invalidate cache entries based on runtime conditions:

#### Single Cache Conditional Invalidation

Invalidate specific entries in a cache based on custom logic:

```rust
use cachelito::{cache, invalidate_with};

#[cache(scope = "global", name = "get_user", limit = 1000)]
fn get_user(user_id: u64) -> User {
    fetch_user_from_db(user_id)
}

// Invalidate only users with ID > 1000
invalidate_with("get_user", |key| {
    key.parse::<u64>().unwrap_or(0) > 1000
});

// Invalidate users based on a pattern
invalidate_with("get_user", |key| {
    key.starts_with("admin_")
});
```

#### Global Conditional Invalidation

Apply a check function across all registered caches:

```rust
use cachelito::invalidate_all_with;

#[cache(scope = "global", name = "get_user")]
fn get_user(user_id: u64) -> User {
    fetch_user_from_db(user_id)
}

#[cache(scope = "global", name = "get_product")]
fn get_product(product_id: u64) -> Product {
    fetch_product_from_db(product_id)
}

// Invalidate all entries with numeric IDs >= 1000 across ALL caches
let count = invalidate_all_with(|_cache_name, key| {
    key.parse::<u64>().unwrap_or(0) >= 1000
});
println!("Applied check function to {} caches", count);
```

#### Complex Check Conditions

Use any Rust logic in your check functions:

```rust
use cachelito::invalidate_with;

// Invalidate entries where ID is divisible by 30
invalidate_with("get_user", |key| {
    key.parse::<u64>()
        .map(|id| id % 30 == 0)
        .unwrap_or(false)
});

// Invalidate entries matching a range
invalidate_with("get_product", |key| {
    if let Ok(id) = key.parse::<u64>() {
        id >= 100 && id < 1000
    } else {
        false
    }
});
```

#### Conditional Invalidation API

- `invalidate_with(cache_name: &str, check_fn: F) -> bool` 
  - Invalidates entries in a specific cache where `check_fn(key)` returns `true`
  - Returns `true` if the cache was found and the check function was applied
  
- `invalidate_all_with(check_fn: F) -> usize`
  - Invalidates entries across all caches where `check_fn(cache_name, key)` returns `true`
  - Returns the number of caches that had the check function applied

#### Use Cases for Conditional Invalidation

- **Time-based cleanup**: Invalidate entries older than a specific timestamp
- **Range-based invalidation**: Remove entries with IDs above/below thresholds
- **Pattern matching**: Invalidate entries matching specific key patterns
- **Selective cleanup**: Remove stale data based on business logic
- **Multi-cache coordination**: Apply consistent invalidation rules across caches

#### Performance Considerations

- **O(n) operation**: Conditional invalidation checks all keys in the cache
- **Lock acquisition**: Briefly holds write lock during key collection and removal
- **Automatic registration**: All global-scope caches support conditional invalidation
- **Thread-safe**: Safe to call from multiple threads concurrently

### Named Invalidation Check Functions (Macro Attribute)

For automatic validation on every cache access, you can specify an invalidation check function directly in the macro:

```rust
use cachelito::cache;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct User {
    id: u64,
    name: String,
    updated_at: Instant,
}

// Define invalidation check function
fn is_stale(_key: &String, value: &User) -> bool {
    // Return true if entry should be invalidated (is stale)
    value.updated_at.elapsed() > Duration::from_secs(3600)
}

// Use invalidation check as macro attribute
#[cache(
    scope = "global",
    name = "get_user",
    invalidate_on = is_stale
)]
fn get_user(user_id: u64) -> User {
    fetch_user_from_db(user_id)
}

// Check function is evaluated on EVERY cache access
let user = get_user(42); // Returns cached value only if !is_stale()
```

#### How Named Invalidation Checks Work

1. **Evaluated on every access**: The check function runs each time `get()` is called
2. **Signature**: `fn check_fn(key: &String, value: &T) -> bool`
3. **Return `true` to invalidate**: If the function returns `true`, the cached entry is considered stale
4. **Re-execution**: When stale, the function re-executes and the result is cached
5. **Works with all scopes**: Compatible with both `global` and `thread` scope

#### Common Check Function Patterns

```rust
// Time-based staleness
fn is_older_than_5min(_key: &String, val: &CachedData) -> bool {
    val.timestamp.elapsed() > Duration::from_secs(300)
}

// Key-based invalidation
fn is_admin_key(key: &String, _val: &Data) -> bool {
    key.contains("admin") // Note: keys are stored with Debug format
}

// Value-based validation
fn has_invalid_data(_key: &String, val: &String) -> bool {
    val.contains("ERROR") || val.is_empty()
}

// Complex conditions
fn needs_refresh(key: &String, val: &(u64, Instant)) -> bool {
    let (count, timestamp) = val;
    // Refresh if count > 1000 OR older than 1 hour
    *count > 1000 || timestamp.elapsed() > Duration::from_secs(3600)
}
```

#### Key Format Note

Cache keys are stored using Rust's Debug format (`{:?}`), which means string keys will have quotes. Use `contains()` instead of exact matching:

```rust
// ✅ Correct
fn check_admin(key: &String, _val: &T) -> bool {
    key.contains("admin")
}

// ❌ Won't work (key is "\"admin_123\"" not "admin_123")
fn check_admin(key: &String, _val: &T) -> bool {
    key.starts_with("admin")
}
```


### Complete Example

See [`examples/smart_invalidation.rs`](examples/smart_invalidation.rs) and [`examples/named_invalidation.rs`](examples/named_invalidation.rs) for complete working examples demonstrating all invalidation strategies.

### Conditional Caching with `cache_if` (v0.14.0)

By default, **all** function results are cached. The `cache_if` attribute allows you to control **when** results should be cached based on custom predicates. This is useful for:
- **Avoiding cache pollution**: Don't cache empty results, error states, or invalid data
- **Memory efficiency**: Only cache valuable results
- **Business logic**: Cache based on result characteristics

#### Basic Usage

```rust
use cachelito::cache;

// Only cache non-empty vectors
fn should_cache_non_empty(_key: &String, result: &Vec<String>) -> bool {
    !result.is_empty()
}

#[cache(scope = "global", limit = 100, cache_if = should_cache_non_empty)]
fn fetch_items(category: String) -> Vec<String> {
    // Simulate database query
    match category.as_str() {
        "electronics" => vec!["laptop".to_string(), "phone".to_string()],
        "empty_category" => vec![], // This won't be cached!
        _ => vec![],
    }
}

fn main() {
    // First call with "electronics" - computes and caches
    let items1 = fetch_items("electronics".to_string());
    // Second call - returns cached result
    let items2 = fetch_items("electronics".to_string());
    
    // First call with "empty_category" - computes but doesn't cache
    let items3 = fetch_items("empty_category".to_string());
    // Second call - computes again (not cached)
    let items4 = fetch_items("empty_category".to_string());
}
```

#### Common Patterns

**Don't cache None values:**
```rust
fn cache_some(_key: &String, result: &Option<User>) -> bool {
    result.is_some()
}

#[cache(scope = "thread", cache_if = cache_some)]
fn find_user(id: u32) -> Option<User> {
    database.find_user(id)
}
```

**Only cache successful HTTP responses:**
```rust
#[derive(Clone)]
struct ApiResponse {
    status: u16,
    body: String,
}

fn cache_success(_key: &String, response: &ApiResponse) -> bool {
    response.status >= 200 && response.status < 300
}

#[cache(scope = "global", limit = 50, cache_if = cache_success)]
fn api_call(url: String) -> ApiResponse {
    // Only 2xx responses will be cached
    make_http_request(url)
}
```

**Cache based on value size:**
```rust
fn cache_if_large(_key: &String, data: &Vec<u8>) -> bool {
    data.len() > 1024 // Only cache results larger than 1KB
}

#[cache(scope = "global", cache_if = cache_if_large)]
fn process_data(input: String) -> Vec<u8> {
    expensive_processing(input)
}
```

**Cache based on value criteria:**
```rust
fn cache_if_positive(_key: &String, value: &i32) -> bool {
    *value > 0
}

#[cache(scope = "thread", cache_if = cache_if_positive)]
fn compute(x: i32, y: i32) -> i32 {
    x + y // Only positive results will be cached
}
```

#### Async Support

The `cache_if` attribute also works with async functions:

```rust
use cachelito_async::cache_async;

fn should_cache_non_empty(_key: &String, result: &Vec<String>) -> bool {
    !result.is_empty()
}

#[cache_async(limit = 100, cache_if = should_cache_non_empty)]
async fn fetch_items_async(category: String) -> Vec<String> {
    // Async database query
    fetch_from_db_async(category).await
}
```

#### Combining with Result Types

When caching functions that return `Result<T, E>`, remember that:
1. **Err values are NEVER cached** (default behavior)
2. **`cache_if` applies ONLY to Ok values**

```rust
fn cache_valid_ok(_key: &String, result: &Result<String, String>) -> bool {
    matches!(result, Ok(data) if !data.is_empty())
}

#[cache(limit = 50, cache_if = cache_valid_ok)]
fn fetch_data(id: u32) -> Result<String, String> {
    match id {
        1..=10 => Ok(format!("Data {}", id)),   // ✅ Cached
        11..=20 => Ok(String::new()),           // ❌ Not cached (empty)
        _ => Err("Invalid ID".to_string()),     // ❌ Not cached (Err)
    }
}
```

#### Performance Impact

- **Zero overhead when not used**: If `cache_if` is not specified, there's no performance impact
- **Check runs after computation**: The predicate function is called AFTER computing the result but BEFORE caching
- **Should be fast**: Keep predicate functions simple and fast (they're called on every cache miss)
- **No lock contention**: The check happens before acquiring the cache lock

#### See Also

- [`examples/conditional_caching.rs`](examples/conditional_caching.rs) - Complete sync examples
- [`cachelito-async/examples/conditional_caching_async.rs`](cachelito-async/examples/conditional_caching_async.rs) - Async examples
- [`tests/conditional_caching_tests.rs`](tests/conditional_caching_tests.rs) - Test suite

## Limitations

- Cannot be used with generic functions (lifetime and type parameter support is limited)
- The function must be deterministic for correct caching behavior
- Cache is global by default (use `scope = "thread"` for thread-local isolation)
- LRU policy has O(n) overhead on cache hits for reordering (where n is the number of cached entries)
- Global scope adds synchronization overhead (though optimized with RwLock)
- Statistics are automatically available for global caches (default); thread-local caches track stats internally but
  they're not accessible via `stats_registry`

## Documentation

For detailed API documentation, run:

```bash
cargo doc --no-deps --open
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes.

### Latest Release: Version 0.13.0

**🎯 Conditional Invalidation with Custom Check Functions!**

Version 0.13.0 introduces powerful conditional invalidation, allowing you to selectively invalidate cache entries based on runtime conditions:

**New Features:**

- 🎯 **Conditional Invalidation** - Invalidate entries matching custom check functions (predicates)
- 🌐 **Global Conditional Invalidation Support** - Apply check functions across all registered caches
- 🔑 **Key-Based Filtering** - Match entries by key patterns, ranges, or any custom logic
- 🏷️ **Named Invalidation Check Functions** - Automatic validation on every cache access with `invalidate_on = function_name` attribute
- ⚡ **Automatic Registration** - All global-scope caches support conditional invalidation by default
- 🔒 **Thread-Safe Execution** - Safe concurrent check function execution
- 💡 **Flexible Conditions** - Use any Rust logic in your check functions

**Quick Start:**

```rust
use cachelito::{cache, invalidate_with, invalidate_all_with};

// Named invalidation check function (evaluated on every access)
fn is_stale(_key: &String, value: &User) -> bool {
    value.updated_at.elapsed() > Duration::from_secs(3600)
}

#[cache(scope = "global", name = "get_user", invalidate_on = is_stale)]
fn get_user(user_id: u64) -> User {
    fetch_user_from_db(user_id)
}

// Manual conditional invalidation
invalidate_with("get_user", |key| {
    key.parse::<u64>().unwrap_or(0) > 1000
});

// Global invalidation across all caches
invalidate_all_with(|_cache_name, key| {
    key.parse::<u64>().unwrap_or(0) >= 1000
});
```

**See also:**
- [`examples/conditional_invalidation.rs`](examples/conditional_invalidation.rs) - Manual conditional invalidation
- [`examples/named_invalidation.rs`](examples/named_invalidation.rs) - Named invalidation check functions

### Previous Release: Version 0.12.0

**🔥 Smart Cache Invalidation!**

Version 0.12.0 introduces intelligent cache invalidation mechanisms beyond simple TTL expiration:

**New Features:**

- 🏷️ **Tag-Based Invalidation** - Group related caches and invalidate them together
- 📡 **Event-Driven Invalidation** - Trigger invalidation when application events occur
- 🔗 **Dependency-Based Invalidation** - Cascade invalidation to dependent caches
- 🎯 **Manual Invalidation** - Invalidate specific caches by name
- 🔄 **Flexible Combinations** - Use tags, events, and dependencies together
- ⚡ **Zero Overhead** - No performance impact when not using invalidation
- 🔒 **Thread-Safe** - All operations are atomic and concurrent-safe

**Quick Start:**

```rust
use cachelito::{cache, invalidate_by_tag, invalidate_by_event};

// Tag-based grouping
#[cache(tags = ["user_data", "profile"], name = "get_user_profile")]
fn get_user_profile(user_id: u64) -> UserProfile {
    fetch_from_db(user_id)
}

// Event-driven invalidation
#[cache(events = ["user_updated"], name = "get_user_settings")]
fn get_user_settings(user_id: u64) -> Settings {
    fetch_settings(user_id)
}

// Invalidate all user_data caches
invalidate_by_tag("user_data");

// Invalidate on event
invalidate_by_event("user_updated");
```

**See also:** [`examples/smart_invalidation.rs`](examples/smart_invalidation.rs)

### Previous Release: Version 0.11.0

**🎲 Random Replacement Policy!**

Version 0.11.0 introduces the Random eviction policy for baseline benchmarking and simple use cases:

**New Features:**

- 🎲 **Random Eviction Policy** - Randomly evicts entries when cache is full
- ⚡ **O(1) Performance** - Constant-time eviction with no access tracking overhead
- 🔒 **Thread-Safe RNG** - Uses `fastrand` for fast, lock-free random selection
- 📊 **Minimal Overhead** - No order updates on cache hits (unlike LRU/ARC)
- 🎯 **Benchmark Baseline** - Ideal for comparing policy effectiveness
- 🔄 **All Cache Types** - Available in sync (thread-local & global) and async caches
- 📚 **Full Support** - Works with `limit`, `ttl`, and `max_memory` attributes

**Quick Start:**

```rust
// Simple random eviction - O(1) performance
#[cache(policy = "random", limit = 1000)]
fn baseline_cache(x: u64) -> u64 { x * x }

// Random with memory limit
#[cache(policy = "random", max_memory = "100MB")]
fn random_with_memory(key: String) -> Vec<u8> {
    vec![0u8; 1024]
}
```

**When to Use Random:**
- Baseline for performance benchmarks
- Truly random access patterns
- Simplicity preferred over optimization
- Reducing lock contention vs LRU/LFU

See the [Cache Limits and Eviction Policies](#cache-limits-and-eviction-policies) section for complete details.

---

### Previous Release: Version 0.10.0

**💾 Memory-Based Limits!**

Version 0.10.0 introduces memory-aware caching controls:

**New Features:**

- 💾 **Memory-Based Limits** - Control cache size by memory footprint
- 📏 **`max_memory` Attribute** - Specify memory limit (e.g. `max_memory = "100MB"`)
- 🔄 **Combined Limits** - Use both entry count and memory limits together
- ⚙️ **Custom Memory Estimation** - Implement `MemoryEstimator` for precise control
- 📊 **Improved Statistics** - Monitor memory usage and hit/miss rates together

**Breaking Changes:**

- **Default policy remains LRU** - No change, but now with memory limits!
- **MemoryEstimator usage** - Custom types with heap allocations must implement `MemoryEstimator`

**Quick Start:**

```rust
// Memory limit - eviction when total size exceeds 100MB
#[cache(max_memory = "100MB")]
fn large_object(id: u32) -> Vec<u8> {
    vec![0u8; 512 * 1024] // 512KB object
}

// Combined limits - max 500 entries OR 128MB
#[cache(limit = 500, max_memory = "128MB")]
fn compute(x: u64) -> u64 { x * x }
```

See the Memory-Based Limits section above for complete details.

---

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
