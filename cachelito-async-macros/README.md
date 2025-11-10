# Cachelito Async Macros

[![Crates.io](https://img.shields.io/crates/v/cachelito-async-macros.svg)](https://crates.io/crates/cachelito-async-macros)
[![Documentation](https://docs.rs/cachelito-async-macros/badge.svg)](https://docs.rs/cachelito-async-macros)
[![License](https://img.shields.io/crates/l/cachelito-async-macros.svg)](https://github.com/josepdcs/cachelito/blob/main/LICENSE)

Procedural macros for automatic async function caching.

This crate provides the `#[cache_async]` procedural macro used by `cachelito-async`. You typically don't need to use
this crate directly - use `cachelito-async` instead.

## Usage

This crate is automatically included when you use `cachelito-async`:

```rust
use cachelito_async::cache_async;

#[cache_async]
async fn my_function(x: u32) -> u32 {
    x * 2
}
```

## Features

- Generates efficient async caching code
- Supports FIFO and LRU eviction policies
- TTL (time-to-live) support
- Result type handling (only caches Ok values)
- Custom cache naming

## Documentation

See the [cachelito-async](https://docs.rs/cachelito-async) crate for complete documentation and examples.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

