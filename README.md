# Cachelito

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A lightweight, thread-safe caching library for Rust that provides automatic memoization through procedural macros.

## Features

- 🚀 **Easy to use**: Simply add `#[cache]` attribute to any function or method
- 🔒 **Thread-safe**: Uses `thread_local!` storage for cache isolation
- 🎯 **Flexible key generation**: Supports custom cache key implementations
- 🎨 **Result-aware**: Intelligently caches only successful `Result::Ok` values
- ✅ **Type-safe**: Full compile-time type checking
- 📦 **Zero runtime dependencies**: Uses only Rust standard library for runtime

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
cachelito = "0.1.0"
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

## How It Works

The `#[cache]` macro generates code that:

1. Creates a thread-local cache using `thread_local!` and `RefCell<HashMap>`
2. Builds a cache key from function arguments using `CacheableKey::to_cache_key()`
3. Checks the cache before executing the function body
4. Stores the result in the cache after execution
5. For `Result<T, E>` types, only caches `Ok` values

## Examples

Run the included example:

```bash
cargo run --example main
```

Output:

```
=== Cachelito Example ===

--- Testing Product Price Caching ---
Calculating price for Product { id: 1, name: "Book" }
First call: 12
Second call (cached): 12

--- Testing Result Caching ---
Running risky operation for 2
Result 1: Ok(4)
Result 2 (cached): Ok(4)
Running risky operation for 3
Result 3 (error, not cached): Err("Odd number: 3")
Running risky operation for 3
Result 4 (error again): Err("Odd number: 3")
```

## Performance Considerations

- **Cache key generation**: Uses `CacheableKey::to_cache_key()` method. The default implementation uses `Debug`
  formatting, which may be slow for complex types. Consider implementing `CacheableKey` directly for better performance.
- **Thread-local storage**: Each thread has its own cache, so cached data is not shared across threads. This means no
  locks or synchronization overhead.
- **Memory usage**: The cache grows unbounded. For long-running applications with many different input combinations,
  consider implementing cache eviction strategies.

## Limitations

- Cannot be used with generic functions (lifetime and type parameter support is limited)
- The function must be deterministic for correct caching behavior
- Cache size is unbounded (no automatic eviction)
- Each thread maintains its own cache (data is not shared across threads)

## Documentation

For detailed API documentation, run:

```bash
cargo doc --no-deps --open
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## See Also

- [Macro Expansion Guide](MACRO_EXPANSION.md) - How to view generated code and understand `format!("{:?}")`
- [API Documentation](https://docs.rs/cachelito) - Full API reference

