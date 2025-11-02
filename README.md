# Cachelito

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A lightweight, thread-safe caching library for Rust that provides automatic memoization through procedural macros.

## Features

- 🚀 **Easy to use**: Simply add `#[cache]` attribute to any function or method
- 🔒 **Thread-safe**: Uses `thread_local!` storage for cache isolation
- 🎯 **Flexible key generation**: Supports custom cache key implementations
- 🎨 **Result-aware**: Intelligently caches only successful `Result::Ok` values
- 🗑️ **Cache limits**: Control memory usage with configurable cache size limits
- 📊 **Eviction policies**: Choose between FIFO (First In, First Out) and LRU (Least Recently Used)
- ✅ **Type-safe**: Full compile-time type checking
- 📦 **Zero runtime dependencies**: Uses only Rust standard library for runtime

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
cachelito = "0.2.0"
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

## How It Works

The `#[cache]` macro generates code that:

1. Creates a thread-local cache using `thread_local!` and `RefCell<HashMap>`
2. Creates a thread-local order queue using `VecDeque` for eviction tracking
3. Builds a cache key from function arguments using `CacheableKey::to_cache_key()`
4. Checks the cache before executing the function body
5. Stores the result in the cache after execution
6. For `Result<T, E>` types, only caches `Ok` values
7. When cache limit is reached, evicts entries according to the configured policy:
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

- **Cache key generation**: Uses `CacheableKey::to_cache_key()` method. The default implementation uses `Debug`
  formatting, which may be slow for complex types. Consider implementing `CacheableKey` directly for better performance.
- **Thread-local storage**: Each thread has its own cache, so cached data is not shared across threads. This means no
  locks or synchronization overhead.
- **Memory usage**: Without a limit, the cache grows unbounded. Use the `limit` parameter to control memory usage.
- **Eviction policy overhead**:
    - **FIFO**: Minimal overhead, O(1) eviction
    - **LRU**: Slightly higher overhead due to reordering on access, O(n) for reordering but still efficient
- **Cache hit performance**: O(1) hash map lookup, with LRU having an additional O(n) reordering cost on hits

## Limitations

- Cannot be used with generic functions (lifetime and type parameter support is limited)
- The function must be deterministic for correct caching behavior
- Each thread maintains its own cache (data is not shared across threads)
- LRU policy has O(n) overhead on cache hits for reordering (where n is the number of cached entries)

## Documentation

For detailed API documentation, run:

```bash
cargo doc --no-deps --open
```

## Changelog

### Version 0.2.0 (Current)

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

