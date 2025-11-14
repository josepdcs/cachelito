use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::thread::LocalKey;

use crate::{CacheEntry, EvictionPolicy};

#[cfg(feature = "stats")]
use crate::CacheStats;

/// Core cache abstraction that stores values in a thread-local HashMap with configurable limits.
///
/// This cache is designed to work with static thread-local maps declared using
/// the `thread_local!` macro. Each thread maintains its own independent cache,
/// ensuring thread safety without the need for locks.
///
/// # Type Parameters
///
/// * `R` - The type of values stored in the cache. Must be `'static` to satisfy
///   thread-local storage requirements and `Clone` for retrieval.
///
/// # Features
///
/// - **Thread-local storage**: Each thread has its own cache instance
/// - **Configurable limits**: Optional maximum cache size
/// - **Eviction policies**: FIFO or LRU eviction when limit is reached
/// - **TTL support**: Optional time-to-live for automatic expiration
/// - **Result-aware**: Special handling for `Result<T, E>` types
///
/// # Thread Safety
///
/// The cache is thread-safe by design - each thread has its own independent copy
/// of the cache data. This means:
/// - No locks or synchronization needed
/// - No contention between threads
/// - Cache entries are not shared across threads
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use std::cell::RefCell;
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
///
/// thread_local! {
///     static MY_CACHE: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
///     static MY_ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
/// }
///
/// let cache = ThreadLocalCache::new(&MY_CACHE, &MY_ORDER, None, EvictionPolicy::FIFO, None);
/// cache.insert("answer", 42);
/// assert_eq!(cache.get("answer"), Some(42));
/// ```
///
/// ## With Cache Limit and LRU Policy
///
/// ```
/// use std::cell::RefCell;
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
///
/// thread_local! {
///     static CACHE: RefCell<HashMap<String, CacheEntry<String>>> = RefCell::new(HashMap::new());
///     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
/// }
///
/// // Cache with limit of 100 entries using LRU eviction
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), EvictionPolicy::LRU, None);
/// cache.insert("key1", "value1".to_string());
/// cache.insert("key2", "value2".to_string());
///
/// // Accessing key1 moves it to the end (most recently used)
/// let _ = cache.get("key1");
/// ```
///
/// ## With TTL (Time To Live)
///
/// ```
/// use std::cell::RefCell;
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
///
/// thread_local! {
///     static CACHE: RefCell<HashMap<String, CacheEntry<String>>> = RefCell::new(HashMap::new());
///     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
/// }
///
/// // Cache with 60 second TTL
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, EvictionPolicy::FIFO, Some(60));
/// cache.insert("key", "value".to_string());
///
/// // Entry will expire after 60 seconds
/// // get() returns None for expired entries
/// ```
pub struct ThreadLocalCache<R: 'static> {
    /// Reference to the thread-local storage key for the cache HashMap
    pub cache: &'static LocalKey<RefCell<HashMap<String, CacheEntry<R>>>>,
    /// Reference to the thread-local storage key for the cache order queue
    pub order: &'static LocalKey<RefCell<VecDeque<String>>>,
    /// Maximum number of items to store in the cache
    pub limit: Option<usize>,
    /// Eviction policy to use for the cache
    pub policy: EvictionPolicy,
    /// Optional TTL (in seconds) for cache entries
    pub ttl: Option<u64>,
    /// Cache statistics (when stats feature is enabled)
    #[cfg(feature = "stats")]
    pub stats: CacheStats,
}

impl<R: Clone + 'static> ThreadLocalCache<R> {
    /// Creates a new `ThreadLocalCache` wrapper around thread-local storage keys.
    ///
    /// # Arguments
    ///
    /// * `cache` - A static reference to a `LocalKey` that stores the cache HashMap
    /// * `order` - A static reference to a `LocalKey` that stores the eviction order queue
    /// * `limit` - Optional maximum number of entries (None for unlimited)
    /// * `policy` - Eviction policy to use when limit is reached
    /// * `ttl` - Optional time-to-live in seconds (None for no expiration)
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::collections::{HashMap, VecDeque};
    /// use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
    ///
    /// thread_local! {
    ///     static CACHE: RefCell<HashMap<String, CacheEntry<String>>> = RefCell::new(HashMap::new());
    ///     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    /// }
    ///
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), EvictionPolicy::LRU, Some(60));
    /// ```
    pub fn new(
        cache: &'static LocalKey<RefCell<HashMap<String, CacheEntry<R>>>>,
        order: &'static LocalKey<RefCell<VecDeque<String>>>,
        limit: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
    ) -> Self {
        Self {
            cache,
            order,
            limit,
            policy,
            ttl,
            #[cfg(feature = "stats")]
            stats: CacheStats::new(),
        }
    }

    /// Retrieves a value from the cache by key.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to look up
    ///
    /// # Returns
    ///
    /// * `Some(value)` if the key exists in the cache and is not expired
    /// * `None` if the key is not found or has expired
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::collections::{HashMap, VecDeque};
    /// # use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
    /// # thread_local! {
    /// #     static CACHE: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
    /// #     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    /// # }
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, EvictionPolicy::FIFO, None);
    /// cache.insert("key", 100);
    /// assert_eq!(cache.get("key"), Some(100));
    /// assert_eq!(cache.get("missing"), None);
    /// ```
    pub fn get(&self, key: &str) -> Option<R> {
        let mut expired = false;

        let val = self.cache.with(|c| {
            let c = c.borrow();
            if let Some(entry) = c.get(key) {
                if entry.is_expired(self.ttl) {
                    expired = true;
                    return None;
                }
                Some(entry.value.clone())
            } else {
                None
            }
        });

        // If expired, remove key from cache and return None
        if expired {
            self.remove_key(key);
            #[cfg(feature = "stats")]
            self.stats.record_miss();
            return None;
        }

        // Record stats
        #[cfg(feature = "stats")]
        {
            if val.is_some() {
                self.stats.record_hit();
            } else {
                self.stats.record_miss();
            }
        }

        // Update access patterns based on policy
        if val.is_some() {
            match self.policy {
                EvictionPolicy::LRU => {
                    // Move key to end of order queue (most recently used)
                    self.order.with(|o| {
                        let mut o = o.borrow_mut();
                        if let Some(pos) = o.iter().position(|k| k == key) {
                            o.remove(pos);
                            o.push_back(key.to_string());
                        }
                    });
                }
                EvictionPolicy::LFU => {
                    // Increment frequency counter
                    self.cache.with(|c| {
                        let mut c = c.borrow_mut();
                        if let Some(entry) = c.get_mut(key) {
                            entry.increment_frequency();
                        }
                    });
                }
                EvictionPolicy::FIFO => {
                    // No update needed for FIFO
                }
            }
        }

        val
    }

    /// Inserts a value into the cache with the specified key.
    ///
    /// If a value already exists for this key, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `value` - The value to store
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::collections::{HashMap, VecDeque};
    /// # use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
    /// # thread_local! {
    /// #     static CACHE: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
    /// #     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    /// # }
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, EvictionPolicy::FIFO, None);
    /// cache.insert("first", 1);
    /// cache.insert("first", 2); // Replaces previous value
    /// assert_eq!(cache.get("first"), Some(2));
    /// ```
    pub fn insert(&self, key: &str, value: R) {
        let key = key.to_string();
        let entry = CacheEntry::new(value);

        self.cache.with(|c| {
            c.borrow_mut().insert(key.clone(), entry);
        });

        self.order.with(|o| {
            let mut order = o.borrow_mut();
            if let Some(pos) = order.iter().position(|k| *k == key) {
                order.remove(pos);
            }
            order.push_back(key.clone());

            if let Some(limit) = self.limit {
                if order.len() > limit {
                    match self.policy {
                        EvictionPolicy::LFU => {
                            // Find and evict the entry with the minimum frequency
                            let mut min_freq_key: Option<String> = None;
                            let mut min_freq = u64::MAX;

                            self.cache.with(|c| {
                                let cache = c.borrow();
                                for evict_key in order.iter() {
                                    if let Some(entry) = cache.get(evict_key) {
                                        if entry.frequency < min_freq {
                                            min_freq = entry.frequency;
                                            min_freq_key = Some(evict_key.clone());
                                        }
                                    }
                                }
                            });

                            if let Some(evict_key) = min_freq_key {
                                self.cache.with(|c| {
                                    c.borrow_mut().remove(&evict_key);
                                });
                                if let Some(pos) = order.iter().position(|k| *k == evict_key) {
                                    order.remove(pos);
                                }
                            }
                        }
                        EvictionPolicy::FIFO | EvictionPolicy::LRU => {
                            // Keep trying to evict until we find a valid entry or queue is empty
                            while let Some(evict_key) = order.pop_front() {
                                let mut removed = false;
                                self.cache.with(|c| {
                                    let mut cache = c.borrow_mut();
                                    if cache.contains_key(&evict_key) {
                                        cache.remove(&evict_key);
                                        removed = true;
                                    }
                                });
                                if removed {
                                    break;
                                }
                                // Key doesn't exist in cache (already removed), try next one
                            }
                        }
                    }
                }
            }
        });
    }

    /// Returns a reference to the cache statistics.
    ///
    /// This method is only available when the `stats` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "stats")]
    /// # {
    /// # use std::cell::RefCell;
    /// # use std::collections::{HashMap, VecDeque};
    /// # use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
    /// # thread_local! {
    /// #     static CACHE: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
    /// #     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    /// # }
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, EvictionPolicy::FIFO, None);
    /// cache.insert("key1", 100);
    /// let _ = cache.get("key1");
    /// let _ = cache.get("key2");
    ///
    /// let stats = cache.stats();
    /// assert_eq!(stats.hits(), 1);
    /// assert_eq!(stats.misses(), 1);
    /// # }
    /// ```
    #[cfg(feature = "stats")]
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    fn remove_key(&self, key: &str) {
        self.cache.with(|c| {
            c.borrow_mut().remove(key);
        });
        self.order.with(|o| {
            let mut o = o.borrow_mut();
            if let Some(pos) = o.iter().position(|k| k == key) {
                o.remove(pos);
            }
        });
    }
}

/// Specialized implementation for caching `Result<T, E>` return types.
///
/// This implementation provides a method to cache only successful (`Ok`) results,
/// which is useful for functions that may fail - you typically don't want to cache
/// errors, as retrying the operation might succeed later.
///
/// # Type Parameters
///
/// * `T` - The success type (inner type of `Ok`)
/// * `E` - The error type (inner type of `Err`)
///
/// # Examples
///
/// ```
/// # use std::cell::RefCell;
/// # use std::collections::{HashMap, VecDeque};
/// # use cachelito_core::{ThreadLocalCache, EvictionPolicy, CacheEntry};
/// # thread_local! {
/// #     static CACHE: RefCell<HashMap<String, CacheEntry<Result<i32, String>>>> = RefCell::new(HashMap::new());
/// #     static ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
/// # }
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, EvictionPolicy::FIFO, None);
///
/// // Only Ok values are cached
/// cache.insert_result("success", &Ok(42));
/// assert_eq!(cache.get("success"), Some(Ok(42)));
///
/// // Err values are NOT cached
/// cache.insert_result("failure", &Err("error".to_string()));
/// assert_eq!(cache.get("failure"), None);
/// ```
impl<T: Clone + Debug + 'static, E: Clone + Debug + 'static> ThreadLocalCache<Result<T, E>> {
    /// Inserts a `Result` into the cache, but only if it's an `Ok` value.
    ///
    /// This method is specifically designed for caching functions that return
    /// `Result<T, E>`. It intelligently ignores `Err` values, as errors typically
    /// should not be cached (the operation might succeed on retry).
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `value` - The `Result` to potentially cache
    ///
    /// # Behavior
    ///
    /// * If `value` is `Ok(v)`, stores `Ok(v.clone())` in the cache
    /// * If `value` is `Err(_)`, does nothing (error is not cached)
    pub fn insert_result(&self, key: &str, value: &Result<T, E>) {
        if let Ok(val) = value {
            self.insert(key, Ok(val.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static TEST_CACHE: RefCell<HashMap<String, CacheEntry<i32>>> = RefCell::new(HashMap::new());
        static TEST_ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
    }

    fn setup_cache(
        limit: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
    ) -> ThreadLocalCache<i32> {
        TEST_CACHE.with(|c| c.borrow_mut().clear());
        TEST_ORDER.with(|o| o.borrow_mut().clear());
        ThreadLocalCache::new(&TEST_CACHE, &TEST_ORDER, limit, policy, ttl)
    }

    #[test]
    fn test_basic_insert_get() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
        cache.insert("key1", 42);
        assert_eq!(cache.get("key1"), Some(42));
    }

    #[test]
    fn test_missing_key() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn test_update_existing_key() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
        cache.insert("key", 1);
        cache.insert("key", 2);
        assert_eq!(cache.get("key"), Some(2));
    }

    #[test]
    fn test_fifo_eviction() {
        let cache = setup_cache(Some(2), EvictionPolicy::FIFO, None);
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3); // Evicts k1

        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), Some(2));
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_lru_eviction() {
        let cache = setup_cache(Some(2), EvictionPolicy::LRU, None);
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        let _ = cache.get("k1"); // Access k1, making it recently used
        cache.insert("k3", 3); // Should evict k2 (least recently used)

        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), None);
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_lru_access_updates_order() {
        let cache = setup_cache(Some(3), EvictionPolicy::LRU, None);
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Access k1 multiple times
        let _ = cache.get("k1");
        let _ = cache.get("k1");

        // k2 is now LRU, should be evicted
        cache.insert("k4", 4);

        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), None);
        assert_eq!(cache.get("k3"), Some(3));
        assert_eq!(cache.get("k4"), Some(4));
    }

    #[test]
    fn test_result_caching_ok() {
        thread_local! {
            static RES_CACHE: RefCell<HashMap<String, CacheEntry<Result<i32, String>>>> = RefCell::new(HashMap::new());
            static RES_ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
        }

        let cache = ThreadLocalCache::new(&RES_CACHE, &RES_ORDER, None, EvictionPolicy::FIFO, None);
        let ok_result = Ok(100);
        cache.insert_result("success", &ok_result);
        assert_eq!(cache.get("success"), Some(Ok(100)));
    }

    #[test]
    fn test_result_caching_err() {
        thread_local! {
            static RES_CACHE: RefCell<HashMap<String, CacheEntry<Result<i32, String>>>> = RefCell::new(HashMap::new());
            static RES_ORDER: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
        }

        let cache = ThreadLocalCache::new(&RES_CACHE, &RES_ORDER, None, EvictionPolicy::FIFO, None);
        let err_result: Result<i32, String> = Err("error".to_string());
        cache.insert_result("failure", &err_result);
        assert_eq!(cache.get("failure"), None); // Errors not cached
    }

    #[test]
    fn test_ttl_expiration() {
        use std::thread;
        use std::time::Duration;

        let cache = setup_cache(None, EvictionPolicy::FIFO, Some(1));
        cache.insert("expires", 999);

        // Should still be valid immediately
        assert_eq!(cache.get("expires"), Some(999));

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));

        // Should be expired now
        assert_eq!(cache.get("expires"), None);
    }

    #[test]
    fn test_no_limit() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
        for i in 0..1000 {
            cache.insert(&format!("key{}", i), i);
        }

        // All entries should still be present
        for i in 0..1000 {
            assert_eq!(cache.get(&format!("key{}", i)), Some(i));
        }
    }

    #[test]
    #[cfg(feature = "stats")]
    fn test_stats_basic() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
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
    fn test_stats_expired_counts_as_miss() {
        use std::thread;
        use std::time::Duration;

        let cache = setup_cache(None, EvictionPolicy::FIFO, Some(1));
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
    fn test_stats_reset() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
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
    fn test_stats_all_hits() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);
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
    fn test_stats_all_misses() {
        let cache = setup_cache(None, EvictionPolicy::FIFO, None);

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
