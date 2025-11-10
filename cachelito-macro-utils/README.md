# cachelito-macro-utils

Shared utilities for `cachelito` procedural macros.

This crate provides common parsing and code generation utilities used by both `cachelito-macros` (sync) and
`cachelito-async-macros` (async).

## Purpose

This crate eliminates code duplication between the sync and async macro implementations by providing shared
functionality for:

- Parsing macro attributes (`limit`, `policy`, `ttl`, `name`, `scope`)
- Generating cache key expressions
- Common data structures

## Usage

This crate is not meant to be used directly. It's an internal dependency of:

- `cachelito-macros` - Procedural macros for sync functions
- `cachelito-async-macros` - Procedural macros for async functions

## Public API

### Parsing Functions

- `parse_limit_attribute()` - Parse the `limit` attribute (returns `Some(usize)` or `None`)
- `parse_policy_attribute()` - Parse the `policy` attribute (returns string: `"fifo"` or `"lru"`)
- `parse_ttl_attribute()` - Parse the `ttl` attribute (returns `Some(u64)` or `None`)
- `parse_name_attribute()` - Parse the `name` attribute (returns `Option<String>`)
- `parse_scope_attribute()` - Parse the `scope` attribute (returns string: `"thread"` or `"global"`)

### Code Generation

- `generate_key_expr()` - Generate cache key expression using `Debug` formatting

### Data Structures

- `SyncCacheAttributes` - Struct holding parsed macro attributes with defaults for sync macros
- `AsyncCacheAttributes` - Struct holding parsed macro attributes with defaults for async macros

## Example

```rust
use cachelito_macro_utils::{parse_limit_attribute, parse_policy_attribute};

// In a procedural macro
for nv in parsed_args {
    if nv.path.is_ident("limit") {
        attrs.limit = parse_limit_attribute(&nv);
    } else if nv.path.is_ident("policy") {
        attrs.policy = parse_policy_attribute(&nv);
    }
}
```

## License

Licensed under Apache-2.0.

