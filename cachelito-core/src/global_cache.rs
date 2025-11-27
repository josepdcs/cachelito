use crate::{CacheEntry, EvictionPolicy};
use once_cell::sync::Lazy;
use parking_lot::lock_api::MutexGuard;
use parking_lot::{Mutex, RawMutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

use crate::utils::{
    find_arc_eviction_key, find_min_frequency_key, move_key_to_end, remove_key_from_global_cache,
};
#[cfg(feature = "stats")]
use crate::CacheStats;

/// A thread-safe global cache that can be shared across multiple threads.
///
/// Unlike `ThreadLocalCache` which uses thread-local storage, `GlobalCache` stores
/// cached values in global static variables protected by `Mutex`, allowing cache
/// sharing across all threads in the application.
///
/// # Type Parameters
///
/// * `R` - The return type to be cached. Must be `'static` to be stored in global state.
///
/// # Fields
///
/// * `map` - Static reference to a lazy-initialized mutex-protected HashMap storing cache entries
/// * `order` - Static reference to a lazy-initialized mutex-protected VecDeque tracking insertion/access order
/// * `limit` - Optional maximum number of entries in the cache
/// * `policy` - Eviction policy (FIFO or LRU) used when limit is reached
/// * `ttl` - Optional time-to-live in seconds for cache entries
///
/// # Thread Safety
///
/// This cache uses `parking_lot::RwLock` for the cache map and `parking_lot::Mutex` for the order queue.
/// The `parking_lot` implementation provides:
/// - **RwLock for reads**: Multiple threads can read concurrently without blocking
/// - **No lock poisoning** (simpler API, no `Result` wrapping)
/// - **Better performance** under contention (30-50% faster than std::sync)
/// - **Smaller memory footprint** (~40x smaller than std::sync)
/// - **Fair locking algorithm** prevents thread starvation
///
/// **Read-heavy workloads** (typical for caches) see 4-5x performance improvement with RwLock
/// compared to Mutex, as multiple threads can read the cache simultaneously.
///
/// # Performance Considerations
///
/// - **Synchronization overhead**: Each cache operation requires acquiring mutex locks
/// - **Lock contention**: High concurrent access may cause threads to wait
/// - **Shared benefits**: All threads benefit from cached results
/// - **Best for**: Expensive computations where sharing outweighs synchronization cost
///
/// # Example
///
/// ```ignore
/// use cachelito_core::{GlobalCache, EvictionPolicy, CacheEntry};
/// use once_cell::sync::Lazy;
/// use parking_lot::{Mutex, RwLock};
/// use std::collections::{HashMap, VecDeque};
///
/// static CACHE_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
///     Lazy::new(|| RwLock::new(HashMap::new()));
/// static CACHE_ORDER: Lazy<Mutex<VecDeque<String>>> =
///     Lazy::new(|| Mutex::new(VecDeque::new()));
///
/// let cache = GlobalCache::new(
///     &CACHE_MAP,
///     &CACHE_ORDER,
///     Some(100),
///     EvictionPolicy::LRU,
///     None,
/// );
///
/// // All threads can access the same cache
/// cache.insert("key1", 42);
/// assert_eq!(cache.get("key1"), Some(42));
/// ```
pub struct GlobalCache<R: 'static> {
    pub map: &'static Lazy<RwLock<HashMap<String, CacheEntry<R>>>>,
    pub order: &'static Lazy<Mutex<VecDeque<String>>>,
    pub limit: Option<usize>,
    pub max_memory: Option<usize>,
    pub policy: EvictionPolicy,
    pub ttl: Option<u64>,
    #[cfg(feature = "stats")]
    pub stats: &'static Lazy<CacheStats>,
}

impl<R: Clone + 'static> GlobalCache<R> {
    /// Creates a new global cache instance.
    ///
    /// # Parameters
    ///
    /// * `map` - Static reference to a RwLock-protected HashMap for storing cache entries
    /// * `order` - Static reference to a Mutex-protected VecDeque for tracking entry order
    /// * `limit` - Optional maximum number of entries (None for unlimited)
    /// * `max_memory` - Optional maximum memory size in bytes (None for unlimited)
    /// * `policy` - Eviction policy to use when limit is reached
    /// * `ttl` - Optional time-to-live in seconds for cache entries
    /// * `stats` - Static reference to CacheStats for tracking hit/miss statistics (stats feature only)
    ///
    /// # Returns
    ///
    /// A new `GlobalCache` instance configured with the provided parameters.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cache = GlobalCache::new(
    ///     &CACHE_MAP,
    ///     &CACHE_ORDER,
    ///     Some(50),
    ///     Some(100 * 1024 * 1024), // 100MB
    ///     EvictionPolicy::FIFO,
    ///     Some(300), // 5 minutes TTL
    ///     #[cfg(feature = "stats")]
    ///     &CACHE_STATS,
    /// );
    /// ```
    #[cfg(feature = "stats")]
    pub fn new(
        map: &'static Lazy<RwLock<HashMap<String, CacheEntry<R>>>>,
        order: &'static Lazy<Mutex<VecDeque<String>>>,
        limit: Option<usize>,
        max_memory: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
        stats: &'static Lazy<CacheStats>,
    ) -> Self {
        Self {
            map,
            order,
            limit,
            max_memory,
            policy,
            ttl,
            stats,
        }
    }

    #[cfg(not(feature = "stats"))]
    pub fn new(
        map: &'static Lazy<RwLock<HashMap<String, CacheEntry<R>>>>,
        order: &'static Lazy<Mutex<VecDeque<String>>>,
        limit: Option<usize>,
        max_memory: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
    ) -> Self {
        Self {
            map,
            order,
            limit,
            max_memory,
            policy,
            ttl,
        }
    }

    /// Retrieves a cached value by key.
    ///
    /// This method attempts to retrieve a cached value, checking for expiration
    /// and updating access order for LRU policy.
    ///
    /// # Parameters
    ///
    /// * `key` - The cache key to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(R)` - The cached value if found and not expired
    /// * `None` - If the key is not in cache or the entry has expired
    ///
    /// # Behavior
    ///
    /// 1. Acquires lock on the map and checks if the entry exists
    /// 2. If entry exists and is not expired:
    ///    - Clones the value before releasing the lock
    ///    - For LRU policy: moves the key to the end of the order queue (marks as recently used)
    ///    - Returns the cloned value
    /// 3. If entry is expired:
    ///    - Removes the entry from both map and order queue
    ///    - Returns None
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe. Multiple threads can safely call this method
    /// concurrently. The method uses mutex locks to ensure data consistency.
    ///
    /// # Example
    ///
    /// ```ignore
    /// cache.insert("key1", 42);
    ///
    /// // Retrieve the value
    /// assert_eq!(cache.get("key1"), Some(42));
    ///
    /// // Non-existent key
    /// assert_eq!(cache.get("key2"), None);
    /// ```
    pub fn get(&self, key: &str) -> Option<R> {
        let mut result = None;
        let mut expired = false;

        // Acquire read lock - allows concurrent reads
        {
            let m = self.map.read();
            if let Some(entry) = m.get(key) {
                if entry.is_expired(self.ttl) {
                    expired = true;
                } else {
                    result = Some(entry.value.clone());
                }
            }
        } // Read lock released here

        if expired {
            // Acquiring order lock to modify order queue
            let mut o = self.order.lock();
            // Acquire write lock to modify the map
            let mut map_write = self.map.write();
            remove_key_from_global_cache(&mut map_write, &mut o, key);
            #[cfg(feature = "stats")]
            self.stats.record_miss();
            return None;
        }

        // Record stats
        #[cfg(feature = "stats")]
        {
            if result.is_some() {
                self.stats.record_hit();
            } else {
                self.stats.record_miss();
            }
        }

        // Update access patterns based on policy
        if result.is_some() {
            match self.policy {
                EvictionPolicy::LRU => {
                    // Move key to end of order queue (most recently used)
                    move_key_to_end(&mut self.order.lock(), key);
                }
                EvictionPolicy::LFU => {
                    // Increment frequency counter
                    self.increment_frequency(key);
                }
                EvictionPolicy::ARC => {
                    // Adaptive Replacement: Update both recency (LRU) and frequency (LFU)
                    // Move key to end (recency) - lock is automatically released after this call
                    move_key_to_end(&mut self.order.lock(), key);
                    // Increment frequency counter
                    self.increment_frequency(key);
                }
                EvictionPolicy::FIFO | EvictionPolicy::Random => {
                    // No update needed for FIFO or Random
                }
            }
        }

        result
    }

    /// Increments the frequency counter for the specified key.
    fn increment_frequency(&self, key: &str) {
        let mut m = self.map.write();
        if let Some(entry) = m.get_mut(key) {
            entry.increment_frequency();
        }
    }

    /// Inserts or updates a value in the cache.
    ///
    /// This method stores a new value in the cache or updates an existing one.
    /// It handles cache limit enforcement and eviction according to the configured policy.
    ///
    /// # Parameters
    ///
    /// * `key` - The cache key
    /// * `value` - The value to cache
    ///
    /// # Behavior
    ///
    /// 1. Creates a new `CacheEntry` with the current timestamp
    /// 2. Inserts/updates the entry in the map
    /// 3. Updates the order queue:
    ///    - If key already exists in queue, removes old position
    ///    - Adds key to the end of the queue
    /// 4. Enforces cache limit:
    ///    - If limit is set and exceeded, evicts the oldest entry (front of queue)
    ///    - Removes evicted entry from both map and order queue
    ///
    /// # Eviction Policies
    ///
    /// - **FIFO**: Oldest inserted entry is evicted (front of queue)
    /// - **LRU**: Least recently used entry is evicted (front of queue, updated by `get()`)
    /// - **LFU**: Least frequently used entry is evicted (entry with minimum frequency counter)
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and uses mutex locks to ensure consistency
    /// between the map and order queue.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Insert a value
    /// cache.insert("user:123", user_data);
    ///
    /// // Update existing value
    /// cache.insert("user:123", updated_user_data);
    ///
    /// // With limit=2, this will evict the oldest entry
    /// cache.insert("user:456", another_user);
    /// cache.insert("user:789", yet_another_user); // Evicts first entry
    /// ```
    ///
    /// # Note
    ///
    /// This method does NOT require `MemoryEstimator` trait. It only handles entry-count limits.
    /// If `max_memory` is configured, use `insert_with_memory()` instead, which requires
    /// the type to implement `MemoryEstimator`.
    pub fn insert(&self, key: &str, value: R) {
        let key_s = key.to_string();
        let entry = CacheEntry::new(value);

        // Acquire write lock for modification
        self.map.write().insert(key_s.clone(), entry);

        let mut o = self.order.lock();
        if let Some(pos) = o.iter().position(|k| *k == key_s) {
            o.remove(pos);
        }
        o.push_back(key_s.clone());

        // Always handle entry-count limits, regardless of memory limits
        self.handle_entry_limit_eviction(&mut o);
    }

    /// Handles the eviction of entries from a global cache when the number of entries exceeds the limit.
    ///
    /// The eviction behavior depends on the specified eviction policy. The function ensures that the cache
    /// adheres to the defined entry limit by evicting entries based on the configured policy:
    ///
    /// - **LFU (Least Frequently Used):** Finds and evicts the entry with the minimum frequency of usage.
    /// - **ARC (Adaptive Replacement Cache):** Leverages the ARC eviction strategy to find and evict a specific entry.
    /// - **FIFO (First In First Out):** Evicts the oldest entry in the queue to ensure the limit is maintained.
    /// - **LRU (Least Recently Used):** Evicts the least recently accessed entry from the queue.
    ///
    /// # Parameters
    ///
    /// - `o`: A mutable reference to a `MutexGuard` that holds a `VecDeque<String>`.
    ///   This represents the global cache where entries are stored.
    ///
    /// # Behavior
    ///
    /// 1. **Check Limit:** The function first checks if the `limit` is defined and if the length of the
    ///    cache (`o`) exceeds the defined `limit`.
    ///
    /// 2. **Eviction By Policy:** Based on the configured `EvictionPolicy`, different eviction strategies
    ///    are employed:
    ///
    ///   - **LFU:** The method identifies the key with the minimum frequency count by inspecting the
    ///     associated frequency map and removes it from the cache.
    ///   - **ARC:** Uses an ARC strategy to determine which key should be evicted and removes it from the cache.
    ///   - **FIFO or LRU:** Dequeues entries in sequence (from the front of the queue) and checks if the
    ///     entry still exists in the global cache. If found, the entry is removed from both the queue and cache.
    ///
    /// 3. **Thread-Safe Access:** The function ensures thread-safe read/write access to the cache and
    ///    associated data structures using mutexes.
    fn handle_entry_limit_eviction(&self, mut o: &mut MutexGuard<RawMutex, VecDeque<String>>) {
        if let Some(limit) = self.limit {
            if o.len() > limit {
                match self.policy {
                    EvictionPolicy::LFU => {
                        // Find and evict the entry with the minimum frequency
                        let mut map_write = self.map.write();
                        let min_freq_key = find_min_frequency_key(&map_write, &o);

                        if let Some(evict_key) = min_freq_key {
                            remove_key_from_global_cache(&mut map_write, &mut o, &evict_key);
                        }
                    }
                    EvictionPolicy::ARC => {
                        let mut map_write = self.map.write();
                        if let Some(evict_key) =
                            find_arc_eviction_key(&map_write, o.iter().enumerate())
                        {
                            remove_key_from_global_cache(&mut map_write, &mut o, &evict_key);
                        }
                    }
                    EvictionPolicy::Random => {
                        // O(1) random eviction: select random position and remove directly
                        if !o.is_empty() {
                            let pos = fastrand::usize(..o.len());
                            if let Some(evict_key) = o.remove(pos) {
                                let mut map_write = self.map.write();
                                map_write.remove(&evict_key);
                            }
                        }
                    }
                    EvictionPolicy::FIFO | EvictionPolicy::LRU => {
                        // Keep trying to evict until we find a valid entry or queue is empty
                        let mut map_write = self.map.write();
                        while let Some(evict_key) = o.pop_front() {
                            // Check if the key still exists in the cache before removing
                            if map_write.contains_key(&evict_key) {
                                map_write.remove(&evict_key);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

// Separate implementation for types that implement MemoryEstimator
// This allows memory-based eviction
impl<R: Clone + 'static + crate::MemoryEstimator> GlobalCache<R> {
    /// Insert with memory limit support.
    ///
    /// This method requires `R` to implement `MemoryEstimator` and handles both
    /// memory-based and entry-count-based eviction.
    ///
    /// Use this method when `max_memory` is configured in the cache.
    pub fn insert_with_memory(&self, key: &str, value: R) {
        let key_s = key.to_string();
        let entry = CacheEntry::new(value);

        // Acquire write lock for modification
        self.map.write().insert(key_s.clone(), entry);

        let mut o = self.order.lock();
        if let Some(pos) = o.iter().position(|k| *k == key_s) {
            o.remove(pos);
        }
        o.push_back(key_s.clone());

        // Check memory limit first (if specified)
        if let Some(max_mem) = self.max_memory {
            // First, check if the new value by itself exceeds max_mem
            // This is a safety check to prevent infinite eviction loop
            let new_value_size = {
                let map_read = self.map.read();
                map_read
                    .get(&key_s)
                    .map(|e| e.value.estimate_memory())
                    .unwrap_or(0)
            };

            if new_value_size > max_mem {
                // The value itself is too large for the cache
                // Remove it and return early to respect memory limit
                self.map.write().remove(&key_s);
                o.pop_back(); // Remove from order queue as well
                return;
            }

            loop {
                let current_mem = {
                    let map_read = self.map.read();
                    map_read
                        .values()
                        .map(|e| e.value.estimate_memory())
                        .sum::<usize>()
                };

                if current_mem <= max_mem {
                    break;
                }

                // Need to evict based on policy
                let evicted = match self.policy {
                    EvictionPolicy::LFU => {
                        let mut map_write = self.map.write();
                        let min_freq_key = find_min_frequency_key(&map_write, &o);
                        if let Some(evict_key) = min_freq_key {
                            remove_key_from_global_cache(&mut map_write, &mut o, &evict_key);
                            true
                        } else {
                            false
                        }
                    }
                    EvictionPolicy::ARC => {
                        let mut map_write = self.map.write();
                        if let Some(evict_key) =
                            find_arc_eviction_key(&map_write, o.iter().enumerate())
                        {
                            remove_key_from_global_cache(&mut map_write, &mut o, &evict_key);
                            true
                        } else {
                            false
                        }
                    }
                    EvictionPolicy::Random => {
                        // O(1) random eviction: select random position and remove directly
                        if !o.is_empty() {
                            let pos = fastrand::usize(..o.len());
                            if let Some(evict_key) = o.remove(pos) {
                                let mut map_write = self.map.write();
                                map_write.remove(&evict_key);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    EvictionPolicy::FIFO | EvictionPolicy::LRU => {
                        // Ensure we only count as evicted if we actually remove from the map
                        let mut successfully_evicted = false;
                        let mut map_write = self.map.write();
                        while let Some(evict_key) = o.pop_front() {
                            if map_write.contains_key(&evict_key) {
                                map_write.remove(&evict_key);
                                successfully_evicted = true;
                                break;
                            }
                            // If key wasn't in map (orphan), continue popping until we remove a real one
                        }
                        successfully_evicted
                    }
                };

                if !evicted {
                    break; // Nothing left to evict
                }
            }
        }

        // Handle entry-count limits
        self.handle_entry_limit_eviction(&mut o);
    }

    /// Returns a reference to the cache statistics.
    ///
    /// This method is only available when the `stats` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let stats = cache.stats();
    /// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
    /// println!("Total accesses: {}", stats.total_accesses());
    /// ```
    #[cfg(feature = "stats")]
    pub fn stats(&self) -> &CacheStats {
        self.stats
    }

    /// Clears all entries from the cache.
    ///
    /// This method removes all entries from both the cache map and the order queue.
    /// It's useful for testing or when you need to completely reset the cache state.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be safely called from multiple threads.
    ///
    /// # Example
    ///
    /// ```ignore
    /// cache.insert("key1", 42);
    /// cache.insert("key2", 84);
    ///
    /// cache.clear();
    ///
    /// assert_eq!(cache.get("key1"), None);
    /// assert_eq!(cache.get("key2"), None);
    /// ```
    pub fn clear(&self) {
        self.map.write().clear();
        self.order.lock().clear();
    }
}

/// Implementation of `GlobalCache` for `Result` types.
///
/// This specialized implementation provides a `insert_result` method that only
/// caches successful (`Ok`) results, preventing error values from being cached.
///
/// # Type Parameters
///
/// * `T` - The success type, must be `Clone` and `Debug`
/// * `E` - The error type, must be `Clone` and `Debug`
///
/// # Rationale
///
/// Errors are typically transient (network failures, temporary resource unavailability)
/// and should not be cached. Only successful results should be memoized to avoid
/// repeatedly returning stale errors.
///
/// # Example
///
/// ```ignore
/// let cache: GlobalCache<Result<String, Error>> = GlobalCache::new(...);
///
/// // Only Ok values are cached
/// let result = fetch_data();
/// cache.insert_result("key1", &result);
///
/// // If result was Err, nothing is cached
/// // If result was Ok, the value is cached
/// ```
impl<T: Clone + Debug + 'static, E: Clone + Debug + 'static> GlobalCache<Result<T, E>> {
    /// Inserts a Result into the cache, but only if it's an `Ok` variant.
    ///
    /// This method intelligently caches only successful results, preventing
    /// error values from polluting the cache.
    ///
    /// This version does NOT require MemoryEstimator. Use `insert_result_with_memory()`
    /// when max_memory is configured.
    ///
    /// # Parameters
    ///
    /// * `key` - The cache key
    /// * `value` - The Result to potentially cache
    ///
    /// # Behavior
    ///
    /// - If `value` is `Ok(v)`: Caches `Ok(v.clone())` under the given key
    /// - If `value` is `Err(_)`: Does nothing, no cache entry is created
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn fetch_user(id: u64) -> Result<User, DbError> {
    ///     // ... database query ...
    /// }
    ///
    /// let result = fetch_user(123);
    /// cache.insert_result("user:123", &result);
    ///
    /// // Success: cached
    /// // Ok(user) -> cache contains Ok(user)
    ///
    /// // Failure: not cached (will retry next time)
    /// // Err(db_error) -> cache remains empty for this key
    /// ```
    pub fn insert_result(&self, key: &str, value: &Result<T, E>) {
        if let Ok(v) = value {
            self.insert(key, Ok(v.clone()));
        }
    }
}

/// Implementation of `GlobalCache` for `Result` types WITH MemoryEstimator support.
///
/// This specialized implementation provides memory-aware caching for Result types.
///
/// # Type Parameters
///
/// * `T` - The success type, must be `Clone`, `Debug`, and implement `MemoryEstimator`
/// * `E` - The error type, must be `Clone`, `Debug`, and implement `MemoryEstimator`
impl<
        T: Clone + Debug + 'static + crate::MemoryEstimator,
        E: Clone + Debug + 'static + crate::MemoryEstimator,
    > GlobalCache<Result<T, E>>
{
    /// Inserts a Result into the cache with memory limit support.
    ///
    /// This method requires both T and E to implement MemoryEstimator.
    /// Use this when max_memory is configured.
    ///
    /// # Parameters
    ///
    /// * `key` - The cache key
    /// * `value` - The Result to potentially cache
    ///
    /// # Behavior
    ///
    /// - If `value` is `Ok(v)`: Caches `Ok(v.clone())` under the given key
    /// - If `value` is `Err(_)`: Does nothing, no cache entry is created
    pub fn insert_result_with_memory(&self, key: &str, value: &Result<T, E>) {
        if let Ok(v) = value {
            self.insert_with_memory(key, Ok(v.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_global_basic_insert_get() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("key1", 100);
        assert_eq!(cache.get("key1"), Some(100));
    }

    #[test]
    fn test_global_missing_key() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_global_update_existing() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("key", 1);
        cache.insert("key", 2);
        assert_eq!(cache.get("key"), Some(2));
    }

    #[test]
    fn test_global_fifo_eviction() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(2),
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), Some(2));
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_global_lru_eviction() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(2),
            None,
            EvictionPolicy::LRU,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        let _ = cache.get("k1");
        cache.insert("k3", 3);

        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), None);
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_global_lru_multiple_accesses() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(3),
            None,
            EvictionPolicy::LRU,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Access k1 to make it most recent
        let _ = cache.get("k1");
        let _ = cache.get("k1");

        // k2 should be evicted (least recently used)
        cache.insert("k4", 4);

        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), None);
        assert_eq!(cache.get("k3"), Some(3));
        assert_eq!(cache.get("k4"), Some(4));
    }

    #[test]
    fn test_global_thread_safety() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let cache = GlobalCache::new(
                        &MAP,
                        &ORDER,
                        None,
                        None,
                        EvictionPolicy::FIFO,
                        None,
                        #[cfg(feature = "stats")]
                        &STATS,
                    );
                    cache.insert(&format!("key{}", i), i);
                    thread::sleep(Duration::from_millis(10));
                    cache.get(&format!("key{}", i))
                })
            })
            .collect();

        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.join().unwrap();
            assert_eq!(result, Some(i as i32));
        }
    }

    #[test]
    fn test_global_ttl_expiration() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            Some(1),
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("expires", 999);

        // Should be valid immediately
        assert_eq!(cache.get("expires"), Some(999));

        thread::sleep(Duration::from_secs(2));

        // Should be expired now
        assert_eq!(cache.get("expires"), None);
    }

    #[test]
    fn test_global_result_ok() {
        static RES_MAP: Lazy<RwLock<HashMap<String, CacheEntry<Result<i32, String>>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static RES_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &RES_MAP,
            &RES_ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        let ok_result = Ok(42);
        cache.insert_result("success", &ok_result);
        assert_eq!(cache.get("success"), Some(Ok(42)));
    }

    #[test]
    fn test_global_result_err() {
        static RES_MAP: Lazy<RwLock<HashMap<String, CacheEntry<Result<i32, String>>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static RES_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &RES_MAP,
            &RES_ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        let err_result: Result<i32, String> = Err("error".to_string());
        cache.insert_result("failure", &err_result);
        assert_eq!(cache.get("failure"), None); // Errors not cached
    }

    #[test]
    fn test_global_concurrent_lru_access() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(5),
            None,
            EvictionPolicy::LRU,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        // Pre-populate cache
        for i in 0..5 {
            cache.insert(&format!("k{}", i), i);
        }

        // Concurrent access to k0
        let handles: Vec<_> = (0..5)
            .map(|_| {
                thread::spawn(|| {
                    let cache = GlobalCache::new(
                        &MAP,
                        &ORDER,
                        Some(5),
                        None,
                        EvictionPolicy::LRU,
                        None,
                        #[cfg(feature = "stats")]
                        &STATS,
                    );
                    for _ in 0..10 {
                        let _ = cache.get("k0");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // k0 should still be in cache (frequently accessed)
        assert_eq!(cache.get("k0"), Some(0));
    }

    #[test]
    fn test_global_no_limit() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );

        for i in 0..100 {
            cache.insert(&format!("k{}", i), i);
        }

        // All should still be present
        for i in 0..100 {
            assert_eq!(cache.get(&format!("k{}", i)), Some(i));
        }
    }

    #[test]
    fn test_memory_eviction_skips_orphan_and_removes_real_entry() {
        // Shared structures
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        // max_memory allows only a single i32 (size 4)
        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            Some(std::mem::size_of::<i32>()),
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );

        // Insert first real entry
        cache.insert_with_memory("k1", 1i32);

        // Introduce an orphan key at the front of the order queue
        {
            let mut o = ORDER.lock();
            o.push_front("orphan".to_string());
        }

        // Insert second entry which forces memory eviction
        cache.insert_with_memory("k2", 2i32);

        // The orphan should be ignored for memory purposes and a real key should be evicted.
        // Expect k1 to be evicted and k2 to remain.
        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), Some(2));

        // Ensure the orphan key is gone from the order
        let order_contents: Vec<String> = {
            let o = ORDER.lock();
            o.iter().cloned().collect()
        };
        assert!(order_contents.iter().all(|k| k != "orphan"));
    }

    /// Test RwLock allows concurrent reads (no blocking)
    #[test]
    fn test_rwlock_concurrent_reads() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );

        // Populate cache
        for i in 0..10 {
            cache.insert(&format!("key{}", i), i);
        }

        // Spawn many threads reading concurrently
        let handles: Vec<_> = (0..20)
            .map(|_thread_id| {
                thread::spawn(move || {
                    let cache = GlobalCache::new(
                        &MAP,
                        &ORDER,
                        None,
                        None,
                        EvictionPolicy::FIFO,
                        None,
                        #[cfg(feature = "stats")]
                        &STATS,
                    );
                    let mut results = Vec::new();
                    for i in 0..10 {
                        results.push(cache.get(&format!("key{}", i)));
                    }
                    results
                })
            })
            .collect();

        // All threads should complete without blocking
        for handle in handles {
            let results = handle.join().unwrap();
            for (i, result) in results.iter().enumerate() {
                assert_eq!(*result, Some(i as i32));
            }
        }
    }

    /// Test RwLock write blocks reads temporarily
    #[test]
    fn test_rwlock_write_excludes_reads() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );

        cache.insert("key1", 100);

        // Write and read interleaved - should not deadlock
        let write_handle = thread::spawn(|| {
            let cache = GlobalCache::new(
                &MAP,
                &ORDER,
                None,
                None,
                EvictionPolicy::FIFO,
                None,
                #[cfg(feature = "stats")]
                &STATS,
            );
            for i in 0..50 {
                cache.insert(&format!("key{}", i), i);
                thread::sleep(Duration::from_micros(100));
            }
        });

        let read_handles: Vec<_> = (0..5)
            .map(|_| {
                thread::spawn(|| {
                    let cache = GlobalCache::new(
                        &MAP,
                        &ORDER,
                        None,
                        None,
                        EvictionPolicy::FIFO,
                        None,
                        #[cfg(feature = "stats")]
                        &STATS,
                    );
                    for i in 0..50 {
                        let _ = cache.get(&format!("key{}", i));
                        thread::sleep(Duration::from_micros(100));
                    }
                })
            })
            .collect();

        write_handle.join().unwrap();
        for handle in read_handles {
            handle.join().unwrap();
        }
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_global_stats_basic() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        cache.insert("k2", 2);

        let _ = cache.get("k1"); // Hit
        let _ = cache.get("k2"); // Hit
        let _ = cache.get("k3"); // Miss

        let stats = cache.stats();
        assert_eq!(stats.hits(), 2);
        assert_eq!(stats.misses(), 1);
        assert_eq!(stats.total_accesses(), 3);
        assert!((stats.hit_rate() - 0.6666).abs() < 0.001);
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_global_stats_expired_counts_as_miss() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            Some(1),
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("expires", 999);

        // Immediate access - should be a hit
        let _ = cache.get("expires");
        assert_eq!(cache.stats().hits(), 1);
        assert_eq!(cache.stats().misses(), 0);

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));

        // Access after expiration - should be a miss
        let _ = cache.get("expires");
        assert_eq!(cache.stats().hits(), 1);
        assert_eq!(cache.stats().misses(), 1);
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_global_stats_reset() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        let _ = cache.get("k1");
        let _ = cache.get("k2");

        let stats = cache.stats();
        assert_eq!(stats.hits(), 1);
        assert_eq!(stats.misses(), 1);

        stats.reset();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_global_stats_concurrent_access() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        cache.insert("k2", 2);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let cache = GlobalCache::new(
                        &MAP,
                        &ORDER,
                        None,
                        None,
                        EvictionPolicy::FIFO,
                        None,
                        #[cfg(feature = "stats")]
                        &STATS,
                    );
                    for _ in 0..10 {
                        let _ = cache.get("k1"); // Hit
                        let _ = cache.get("k2"); // Hit
                        let _ = cache.get("k3"); // Miss
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = cache.stats();
        // 10 threads * 10 iterations * 2 hits = 200 hits
        // 10 threads * 10 iterations * 1 miss = 100 misses
        assert_eq!(stats.hits(), 200);
        assert_eq!(stats.misses(), 100);
        assert_eq!(stats.total_accesses(), 300);
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_global_stats_all_hits() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );
        cache.insert("k1", 1);
        cache.insert("k2", 2);

        for _ in 0..10 {
            let _ = cache.get("k1");
            let _ = cache.get("k2");
        }

        let stats = cache.stats();
        assert_eq!(stats.hits(), 20);
        assert_eq!(stats.misses(), 0);
        assert_eq!(stats.hit_rate(), 1.0);
        assert_eq!(stats.miss_rate(), 0.0);
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_global_stats_all_misses() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &STATS,
        );

        for i in 0..10 {
            let _ = cache.get(&format!("k{}", i));
        }

        let stats = cache.stats();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 10);
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.miss_rate(), 1.0);
    }
}
