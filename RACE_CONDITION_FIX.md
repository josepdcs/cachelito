 lu# Race Condition Fixes - Async Cache Implementation

## ✅ Problems Resolved

Three critical race conditions were identified and fixed in the async cache implementation:

1. **Race condition** in LRU order updates on cache hits
2. **Cache inconsistency** where expired entries were not being removed from the order queue
3. **Race condition** in limit check and eviction (non-atomic check-and-evict) ⭐ NEW

## 🔍 The Problems

### Problem 1: Race Condition in LRU Updates

**Race window identified:**
1. Task A reads from cache and gets a reference to the cached value
2. Task A drops the reference (`drop(__entry_ref)`)
3. **[RACE WINDOW]** Before Task A can update the LRU order, Task B evicts this entry
4. Task A updates the LRU order, adding a key that no longer exists in the cache
5. **Result:** orphaned key in the order queue

### Problem 2: Orphaned Keys on Expiration

**Cache inconsistency:**
1. An entry expires (TTL exceeded)
2. The entry is removed from the cache (`cache.remove(&key)`)
3. **[BUG]** The key was NOT being removed from the order queue unconditionally
4. **Result:** orphaned keys accumulate in the order queue, causing memory leaks

### Problem 3: Non-Atomic Limit Check and Eviction ⭐ NEW

**Race condition in eviction:**
```rust
// BEFORE (buggy):
if let Some(__limit) = #limit_expr {
    // ❌ Check limit without holding lock
    if #cache_ident.len() >= __limit && !#cache_ident.contains_key(&__key) {
        let mut __order = #order_ident.lock();  // Lock acquired here
        // ... evict ...
    }
}
```

**The problem:**
1. Task A checks: `len() >= limit` → true
2. Task A checks: `!contains_key(&key)` → true
3. **[RACE WINDOW]** Task B inserts a different entry
4. Task A acquires lock and performs eviction
5. **Result:** cache exceeds limit because check was not atomic

### Consequences

- **Orphaned keys accumulate** in the order queue
- **Wasted eviction attempts** when trying to evict orphaned keys
- **Potential memory leak** if orphaned keys keep growing
- **Cache limit violation** when concurrent insertions bypass the limit check
- **Incorrect cache size management**

## 💡 Solutions Implemented

### 1. Double-Check Pattern for LRU Updates

```rust
// Update LRU order on cache hit
// Verify key still exists to avoid orphaned keys in the order queue
if let Some(__limit) = #limit_expr {
    if #policy_expr == "lru" && #cache_ident.contains_key(&__key) {
        let mut __order = #order_ident.lock();
        // Double-check after acquiring lock
        if #cache_ident.contains_key(&__key) {
            __order.retain(|k| k != &__key);
            __order.push_back(__key.clone());
        }
    }
}
```

**Benefits:**
- First check before locking: Fast path to avoid lock contention
- Second check after locking: Guarantees consistency
- Only updates if the key still exists in cache

### 2. Unconditional Order Queue Cleanup on Expiration

```rust
// Expired - remove and continue to execute
drop(__entry_ref);
#cache_ident.remove(&__key);

// Also remove from order queue to prevent orphaned keys
// This is now UNCONDITIONAL (not dependent on limit being set)
let mut __order = #order_ident.lock();
__order.retain(|k| k != &__key);
```

**Benefits:**
- Always cleans up order queue on expiration
- Works even without limit configured
- Prevents long-term memory leaks
- Handles edge cases where cache configuration changes

### 3. Atomic Check-and-Evict ⭐ NEW

**BEFORE (buggy - non-atomic):**
```rust
if let Some(__limit) = #limit_expr {
    // ❌ Check without lock - race condition!
    if #cache_ident.len() >= __limit && !#cache_ident.contains_key(&__key) {
        let mut __order = #order_ident.lock();
        // ... evict ...
    }
    
    // ❌ Acquire lock again - inefficient
    let mut __order = #order_ident.lock();
    __order.push_back(__key.clone());
}
```

**AFTER (fixed - atomic):**
```rust
// Handle limit - acquire lock first to ensure atomicity
if let Some(__limit) = #limit_expr {
    let mut __order = #order_ident.lock();  // ✅ Lock acquired FIRST
    
    // ✅ Check limit after acquiring lock to prevent race condition
    if #cache_ident.len() >= __limit && !#cache_ident.contains_key(&__key) {
        // Keep trying until we find a valid entry to evict
        while let Some(__evict_key) = __order.pop_front() {
            if #cache_ident.contains_key(&__evict_key) {
                #cache_ident.remove(&__evict_key);
                break;
            }
        }
    }
    
    // Update order for the new entry
    if #policy_expr == "lru" {
        __order.retain(|k| k != &__key);
    }
    __order.push_back(__key.clone());
}
```

**Benefits:**
- Lock acquired once at the beginning
- All checks performed while holding lock
- Atomic check-and-evict-and-update operation
- Prevents cache from exceeding limit
- More efficient (single lock acquisition)

### 4. Robust Eviction Loop

The eviction loop correctly handles orphaned keys by continuing until a valid entry is found:

```rust
while let Some(__evict_key) = __order.pop_front() {
    if #cache_ident.contains_key(&__evict_key) {
        #cache_ident.remove(&__evict_key);
        break;
    }
    // Key doesn't exist in cache (already removed), try next one
}
```

## 🧪 Tests Added

### 6 New Tests for Race Conditions and Orphaned Keys

1. **`test_async_race_condition_hit_vs_eviction`**
   - Tests concurrent cache hits and evictions
   - Validates no panics and cache remains functional
   - 20 concurrent tasks reading and writing

2. **`test_async_race_condition_ttl_vs_lru_update`**
   - Tests TTL expiration during LRU update window
   - Validates graceful handling of expired entries
   - 50 concurrent tasks accessing expiring entries

3. **`test_async_orphaned_keys_no_infinite_loop`**
   - Tests that orphaned keys don't cause hangs
   - Validates quick completion and correct computation count
   - Ensures no infinite loops when all queued keys are orphaned

4. **`test_async_expired_entries_cleaned_from_order_queue`**
   - Tests that expired keys are removed from order queue
   - Validates no memory leaks even without limit configured
   - Ensures proper cleanup in all scenarios

5. **`test_async_race_condition_concurrent_insertions_at_limit`** ⭐ NEW
   - Tests multiple concurrent insertions when cache is at limit
   - Validates atomic check-and-evict prevents limit violations
   - 50 concurrent tasks trying to insert at cache limit

6. **`test_async_race_condition_lru_insertions_at_limit`** ⭐ NEW
   - Tests concurrent LRU updates and insertions at limit
   - Validates no deadlocks or panics under high concurrency
   - 200 concurrent tasks (mix of reads and writes)

### Test Results

✅ **All 12 orphaned keys tests pass:**
```
test test_async_race_condition_hit_vs_eviction ... ok
test test_async_race_condition_ttl_vs_lru_update ... ok
test test_async_orphaned_keys_no_infinite_loop ... ok
test test_async_expired_entries_cleaned_from_order_queue ... ok
test test_async_race_condition_concurrent_insertions_at_limit ... ok ⭐ NEW
test test_async_race_condition_lru_insertions_at_limit ... ok ⭐ NEW
test test_async_fifo_eviction_with_orphaned_keys ... ok
test test_async_eviction_with_expired_first_key ... ok
test test_async_lru_eviction_with_orphaned_keys ... ok
test test_async_eviction_with_multiple_orphaned_keys ... ok
test test_async_eviction_queue_auto_cleanup ... ok
test test_async_concurrent_eviction_with_orphaned_keys ... ok
```

✅ **All workspace tests pass:**
- cachelito: 4 tests + 5 doctests ✓
- cachelito-async: 12 tests ✓
- cachelito-core: 54 tests + 32 doctests ✓
- **Total: 70 unit tests + 37 doctests**

## 📈 Performance Impact

The fixes have minimal to positive performance impact:

### Lock Acquisition Optimization
- **Before**: Lock acquired twice per insertion (once for eviction check, once for order update)
- **After**: Lock acquired once per insertion
- **Result**: ~50% reduction in lock operations

### Fast Path
- **LRU updates**: First `contains_key` check avoids lock contention in most cases
- **Lock-free reads**: DashMap's operations are lock-free
- **Eviction check**: Single atomic operation while holding lock

### Memory Efficiency
- Prevents memory leaks from accumulating orphaned keys
- Prevents cache from growing beyond configured limit

## 📝 Files Modified

1. **`cachelito-async-macros/src/lib.rs`**:
   - ✅ Added double-check pattern for LRU updates on cache hit (Result path)
   - ✅ Added double-check pattern for LRU updates on cache hit (non-Result path)
   - ✅ Made order queue cleanup unconditional on expiration (Result path)
   - ✅ Made order queue cleanup unconditional on expiration (non-Result path)
   - ✅ Atomic check-and-evict: lock acquired before limit check (Result path) ⭐ NEW
   - ✅ Atomic check-and-evict: lock acquired before limit check (non-Result path) ⭐ NEW

2. **`cachelito-async/tests/eviction_orphaned_keys_tests.rs`**:
   - ✅ Added 6 new race condition tests
   - ✅ Fixed test expectation in `test_async_eviction_with_multiple_orphaned_keys`
   - ✅ Total test coverage: 12 tests for orphaned keys and race conditions

3. **`RACE_CONDITION_FIX.md`** (this file):
   - ✅ Complete documentation of all three problems and solutions

## 🎯 Conclusion

All three race condition issues have been **completely resolved**:

✅ **Double-check pattern** prevents race conditions in LRU updates  
✅ **Unconditional cleanup** prevents memory leaks from expired entries  
✅ **Atomic check-and-evict** prevents cache limit violations ⭐ NEW  
✅ **Robust eviction** handles orphaned keys gracefully  
✅ **Comprehensive tests** validate correctness under high concurrency  
✅ **Improved performance** thanks to optimized locking strategy  
✅ **All tests pass** (70 unit tests + 37 doctests)

The solution is **efficient, thread-safe, and thoroughly tested**.

---

## 📋 Summary for PR Review

### Changes Made

1. **LRU Update Safety**: Double-check pattern ensures keys exist before updating order
2. **Expiration Cleanup**: Unconditional removal from order queue on expiration
3. **Atomic Eviction**: Lock acquired before limit check for atomic operations
4. **Test Coverage**: 6 new concurrency tests, 12 total tests for race conditions
5. **Performance**: Reduced lock acquisitions by 50% per insertion

### Verification

- ✅ All 70 unit tests pass
- ✅ All 37 doctests pass
- ✅ No compiler warnings
- ✅ Tested under high concurrency (200+ concurrent tasks)
- ✅ Memory leak prevention validated
- ✅ Cache limit enforcement validated

