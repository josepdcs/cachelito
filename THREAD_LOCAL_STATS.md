# Thread-Local Cache Statistics - Technical Explanation

## Question: Why aren't statistics available in thread-local caches?

**Short Answer**: They ARE available - they're just not accessible through `stats_registry::get()`.

## The Complete Picture

### ✅ What DOES Work

Thread-local caches (`ThreadLocalCache`) **DO have statistics**:

```rust
pub struct ThreadLocalCache<R: 'static> {
    pub cache: &'static LocalKey<RefCell<HashMap<String, CacheEntry<R>>>>,
    pub order: &'static LocalKey<RefCell<VecDeque<String>>>,
    pub limit: Option<usize>,
    pub policy: EvictionPolicy,
    pub ttl: Option<u64>,
    #[cfg(feature = "stats")]
    pub stats: CacheStats,  // ← Statistics ARE tracked!
}
```

**Statistics are tracked for:**

- Every `get()` call (hit or miss)
- Every cache access
- Hit/miss rates
- Total accesses

### ❌ What DOESN'T Work

You **cannot** access thread-local statistics via `stats_registry::get()`:

```rust
// This ONLY works for scope = "global"
#[cache]  // Default is thread-local
fn my_function(x: i32) -> i32 { x * 2 }

// This will return None for thread-local caches
stats_registry::get("my_function")  // ❌ Returns None
```

## Why This Limitation Exists

### 1. **Thread Isolation**

Thread-local caches are stored using `thread_local!` macro:

```rust
thread_local! {
    static CACHE: RefCell<HashMap<...>> = ...;
    static ORDER: RefCell<VecDeque<...>> = ...;
}
```

Each thread has its **own independent copy** of these statics. There's no way to access another thread's `thread_local!`
data.

### 2. **Macro Architecture**

When you write:

```rust
#[cache]
fn my_function(x: i32) -> i32 { ... }
```

The macro generates code like this:

```rust
fn my_function(x: i32) -> i32 {
    thread_local! {
        static CACHE: RefCell<...> = ...;
        static ORDER: RefCell<...> = ...;
    }

    let cache = ThreadLocalCache::new(&CACHE, &ORDER, ...);
    // cache.stats exists here but is local to this call
    ...
}
```

Each call to `my_function()` creates a **new** `ThreadLocalCache` instance, but it references the **same** thread-local
statics. However, the `stats` field is part of the `ThreadLocalCache` struct, not the static storage.

### 3. **Stats Registry Limitation**

The `stats_registry` needs a **static reference** that can be accessed from any thread:

```rust
pub fn register(name: &str, stats: &'static Lazy<CacheStats>) {
    // Needs &'static reference
}
```

For global caches, this works:

```rust
static STATS: Lazy<CacheStats> = ...;  // ✅ Can take &'static reference
stats_registry::register("name", & STATS);
```

For thread-local, we'd need:

```rust
thread_local! {
    static STATS: CacheStats = ...;  // ❌ Can't take &'static reference
}
// Can't register - no way to get &'static from thread_local!
```

## Solutions & Workarounds

### Solution 1: Use Global Scope ✅ Recommended

```rust
#[cache(scope = "global")]  // Add this
fn my_function(x: i32) -> i32 { x * 2 }

// Now this works!
stats_registry::get("my_function")  // ✅ Returns Some(stats)
```

**Pros:**

- Statistics accessible via `stats_registry`
- Can monitor across all threads
- Can compare different cache strategies

**Cons:**

- Slight synchronization overhead (uses `RwLock`)
- Cache shared across threads (may not be desired)

### Solution 2: Direct Access in Tests ✅ For Testing

```rust
#[test]
fn test_my_cache() {
    thread_local! {
        static CACHE: RefCell<HashMap<...>> = ...;
        static ORDER: RefCell<VecDeque<...>> = ...;
    }

    let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, ...);

    cache.insert("key", value);
    cache.get("key");

    // Direct access to stats
    assert_eq!(cache.stats.hits(), 1);  // ✅ Works!
}
```

**Pros:**

- Full access to statistics
- Perfect for unit tests
- No overhead

**Cons:**

- Only works when you create the cache yourself
- Can't use with `#[cache]` macro

### Solution 3: Accept the Limitation ✅ For Pure Performance

```rust
#[cache]  // Thread-local, no stats access
fn fast_function(x: i32) -> i32 { x * 2 }

// Just use the cache, don't worry about stats
```

**Pros:**

- Maximum performance (no synchronization)
- Each thread has independent cache
- Statistics still tracked (for internal use)

**Cons:**

- Can't monitor performance programmatically
- No visibility into cache effectiveness

## Summary

| Feature                          | Thread-Local                      | Global                           |
|----------------------------------|-----------------------------------|----------------------------------|
| Statistics tracked?              | ✅ Yes                             | ✅ Yes                            |
| Accessible via `stats_registry`? | ❌ No                              | ✅ Yes                            |
| Accessible in tests?             | ✅ Yes (direct)                    | ✅ Yes                            |
| Performance                      | ⚡ Fastest                         | 🔒 Synchronization overhead      |
| Use case                         | High-performance, thread-isolated | Cross-thread sharing, monitoring |

## Recommendation

- **Need stats monitoring?** → Use `scope = "global"`
- **Need maximum performance?** → Use thread-local (default)
- **Testing?** → Access `cache.stats` directly

## Example

See `examples/thread_local_stats_internals.rs` for a complete demonstration of how thread-local statistics work
internally.

```bash
cargo run --example thread_local_stats_internals --features stats
```

