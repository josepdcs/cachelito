# Cachelito Async

[![Crates.io](https://img.shields.io/crates/v/cachelito-async.svg)](https://crates.io/crates/cachelito-async)
[![Documentation](https://docs.rs/cachelito-async/badge.svg)](https://docs.rs/cachelito-async)
[![License](https://img.shields.io/crates/l/cachelito-async.svg)](https://github.com/josepdcs/cachelito/blob/main/LICENSE)

A flexible and efficient async caching library for Rust async/await functions.

## Features

- 🚀 **Lock-free caching** - Uses DashMap for concurrent access without blocking
- 🎯 **Multiple eviction policies** - FIFO, LRU, LFU, and ARC (Adaptive Replacement Cache)
- 💾 **Memory-based limits** - Control cache size by memory usage (v0.10.1)
- ⏰ **TTL support** - Automatic expiration of cached entries
- 📊 **Limit control** - Set maximum cache size by entry count or memory
- 🔍 **Result caching** - Only caches `Ok` values from `Result` types
- 🌐 **Global cache** - Shared across all tasks and threads
- ⚡ **Zero async overhead** - No `.await` needed for cache operations
- 📈 **Statistics** - Track cache hit/miss rates and performance metrics

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cachelito-async = "0.10.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use cachelito_async::cache_async;
use std::time::Duration;

#[cache_async]
async fn expensive_operation(x: u32) -> u32 {
    tokio::time::sleep(Duration::from_secs(1)).await;
    x * 2
}

#[tokio::main]
async fn main() {
    // First call: sleeps for 1 second
    let result = expensive_operation(5).await;

    // Second call: returns immediately from cache
    let result = expensive_operation(5).await;
}
```

## Examples

### Basic Async Caching

```rust
use cachelito_async::cache_async;

#[cache_async]
async fn fetch_user(id: u64) -> User {
    database::get_user(id).await
}
```

### Cache with Limit and LRU Policy

```rust
use cachelito_async::cache_async;

#[cache_async(limit = 100, policy = "lru")]
async fn fetch_data(key: String) -> Data {
    // Only 100 entries cached
    // Least recently used entries evicted first
    api::fetch(&key).await
}
```

### Cache with TTL (Time To Live)

```rust
use cachelito_async::cache_async;

#[cache_async(ttl = 60)]
async fn get_weather(city: String) -> Weather {
    // Cache expires after 60 seconds
    weather_api::fetch(&city).await
}
```

### Result Caching (Only Ok Values)

```rust
use cachelito_async::cache_async;

#[cache_async(limit = 50)]
async fn api_call(endpoint: String) -> Result<Response, Error> {
    // Only successful responses are cached
    // Errors are not cached and always re-executed
    make_request(&endpoint).await
}
```

### Cache Statistics

Track cache performance with built-in statistics:

```rust
use cachelito_async::{cache_async, stats_registry};

#[cache_async]
async fn compute(x: u32) -> u32 {
    x * x
}

#[cache_async(name = "my_cache")]
async fn custom(x: u32) -> u32 {
    x + 10
}

#[tokio::main]
async fn main() {
    // Make some calls
    compute(1).await;
    compute(1).await; // cache hit
    compute(2).await;

    // Get statistics
    if let Some(stats) = stats_registry::get("compute") {
        println!("Hits: {}", stats.hits());
        println!("Misses: {}", stats.misses());
        println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
    }

    // List all caches
    for name in stats_registry::list() {
        println!("Cache: {}", name);
    }
}
```

**Statistics Features:**

- Automatic tracking of hits and misses
- Hit/miss rates calculation
- Global registry for all async caches
- Custom cache naming with `name` attribute
- Thread-safe counters using `AtomicU64`

### Combining Features

```rust
use cachelito_async::cache_async;

#[cache_async(limit = 100, policy = "lru", ttl = 300)]
async fn complex_operation(x: i32, y: i32) -> Result<i32, Error> {
    // - Max 100 entries
    // - LRU eviction policy
    // - 5 minute TTL
    // - Only Ok values cached
    expensive_computation(x, y).await
}
```

## Macro Parameters

| Parameter | Type                | Default       | Description                           |
|-----------|---------------------|---------------|---------------------------------------|
| `limit`   | `usize`             | unlimited     | Maximum number of entries in cache    |
| `policy`  | `"fifo"` \| `"lru"` | `"fifo"`      | Eviction policy when limit is reached |
| `ttl`     | `u64`               | none          | Time-to-live in seconds               |
| `name`    | `String`            | function name | Custom cache identifier               |

## Eviction Policies

### LRU (Least Recently Used) - Default

- Evicts least recently accessed entries
- O(n) performance for cache hits (reordering)
- Best when access patterns matter
- Ideal for temporal locality workloads

### FIFO (First In, First Out)

- Evicts oldest entries first
- O(1) performance for all operations
- Best for simple use cases
- Predictable behavior

### LFU (Least Frequently Used)

- Evicts least frequently accessed entries
- O(n) performance for eviction (finding minimum frequency)
- O(1) performance for cache hits (increment counter)
- Best for workloads with "hot" data
- Popular items remain cached longer

**Policy Comparison:**

| Policy | Eviction | Cache Hit | Use Case |
|--------|----------|-----------|----------|
| **LRU** | O(1) | O(n) | Recent access matters |
| **FIFO** | O(1) | O(1) | Simple predictable caching |
| **LFU** | O(n) | O(1) | Frequency patterns matter |

## Performance

- **Lock-free**: Uses DashMap for excellent concurrent performance
- **No blocking**: Cache operations don't block the async executor
- **Minimal overhead**: No `.await` needed for cache lookups
- **Memory efficient**: Configurable limits prevent unbounded growth

## Thread Safety

All caches are thread-safe and can be safely shared across multiple tasks and threads. The underlying DashMap provides
excellent concurrent performance without traditional locks.

## Comparison with Sync Version

| Feature     | cachelito                       | cachelito-async       |
|-------------|---------------------------------|-----------------------|
| Functions   | Sync                            | Async                 |
| Storage     | Thread-local or Global (RwLock) | Global (DashMap)      |
| Concurrency | Mutex/RwLock                    | Lock-free             |
| Scope       | Thread or Global                | Always Global         |
| Best for    | CPU-bound, sync code            | I/O-bound, async code |

## Examples in Repository

- `async_basic.rs` - Basic async caching
- `async_lru.rs` - LRU eviction policy
- `async_concurrent.rs` - Concurrent task access
- `async_stats.rs` - Cache statistics tracking

Run examples with:

```bash
cargo run --example async_basic
cargo run --example async_lru
cargo run --example async_concurrent
cargo run --example async_stats
```

## Requirements

- Rust 1.70.0 or later
- Arguments must implement `Debug` for key generation
- Return type must implement `Clone` for cache storage

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

## Related Crates

- [`cachelito`](https://crates.io/crates/cachelito) - Sync version for regular functions
- [`cachelito-core`](https://crates.io/crates/cachelito-core) - Core caching primitives
- [`cachelito-macros`](https://crates.io/crates/cachelito-macros) - Sync procedural macros

