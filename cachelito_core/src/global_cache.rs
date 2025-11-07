use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::sync::Mutex;

use crate::{CacheEntry, EvictionPolicy};

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
/// This cache uses `Mutex` to protect internal state, making it safe to use across
/// multiple threads. However, this adds synchronization overhead compared to
/// thread-local caches.
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
/// use std::sync::Mutex;
/// use std::collections::{HashMap, VecDeque};
///
/// static CACHE_MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
///     Lazy::new(|| Mutex::new(HashMap::new()));
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
    pub map: &'static Lazy<Mutex<HashMap<String, CacheEntry<R>>>>,
    pub order: &'static Lazy<Mutex<VecDeque<String>>>,
    pub limit: Option<usize>,
    pub policy: EvictionPolicy,
    pub ttl: Option<u64>,
}

impl<R: Clone + 'static> GlobalCache<R> {
    /// Creates a new global cache instance.
    ///
    /// # Parameters
    ///
    /// * `map` - Static reference to a mutex-protected HashMap for storing cache entries
    /// * `order` - Static reference to a mutex-protected VecDeque for tracking entry order
    /// * `limit` - Optional maximum number of entries (None for unlimited)
    /// * `policy` - Eviction policy to use when limit is reached
    /// * `ttl` - Optional time-to-live in seconds for cache entries
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
    ///     EvictionPolicy::FIFO,
    ///     Some(300), // 5 minutes TTL
    /// );
    /// ```
    pub fn new(
        map: &'static Lazy<Mutex<HashMap<String, CacheEntry<R>>>>,
        order: &'static Lazy<Mutex<VecDeque<String>>>,
        limit: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
    ) -> Self {
        Self {
            map,
            order,
            limit,
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

        // Acquire lock and check entry
        if let Ok(m) = self.map.lock() {
            if let Some(entry) = m.get(key) {
                if entry.is_expired(self.ttl) {
                    expired = true;
                } else {
                    result = Some(entry.value.clone());
                }
            }
        }

        if expired {
            self.remove_key(key);
            return None;
        }

        // Update LRU order after releasing map lock
        if result.is_some() && self.policy == EvictionPolicy::LRU {
            if let Ok(mut o) = self.order.lock() {
                if let Some(pos) = o.iter().position(|k| k == key) {
                    o.remove(pos);
                    o.push_back(key.to_string());
                }
            }
        }

        result
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
    pub fn insert(&self, key: &str, value: R) {
        let key_s = key.to_string();
        let entry = CacheEntry::new(value);

        if let Ok(mut m) = self.map.lock() {
            m.insert(key_s.clone(), entry);
        }

        if let Ok(mut o) = self.order.lock() {
            if let Some(pos) = o.iter().position(|k| *k == key_s) {
                o.remove(pos);
            }
            o.push_back(key_s.clone());

            if let Some(limit) = self.limit {
                if o.len() > limit {
                    if let Some(evict_key) = o.pop_front() {
                        if let Ok(mut m) = self.map.lock() {
                            m.remove(&evict_key);
                        }
                    }
                }
            }
        }
    }

    /// Removes an entry from the cache by key.
    ///
    /// This method removes the entry from both the map and the order queue.
    /// It acquires locks on both data structures to ensure consistency.
    ///
    /// # Parameters
    ///
    /// * `key` - The cache key to remove
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and will not panic if locks cannot be acquired.
    /// If a lock acquisition fails, the operation is silently skipped.
    fn remove_key(&self, key: &str) {
        if let Ok(mut m) = self.map.lock() {
            m.remove(key);
        }
        if let Ok(mut o) = self.order.lock() {
            if let Some(pos) = o.iter().position(|k| k == key) {
                o.remove(pos);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_global_basic_insert_get() {
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, None, EvictionPolicy::FIFO, None);
        cache.insert("key1", 100);
        assert_eq!(cache.get("key1"), Some(100));
    }

    #[test]
    fn test_global_missing_key() {
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, None, EvictionPolicy::FIFO, None);
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_global_update_existing() {
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, None, EvictionPolicy::FIFO, None);
        cache.insert("key", 1);
        cache.insert("key", 2);
        assert_eq!(cache.get("key"), Some(2));
    }

    #[test]
    fn test_global_fifo_eviction() {
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, Some(2), EvictionPolicy::FIFO, None);
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), Some(2));
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_global_lru_eviction() {
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, Some(2), EvictionPolicy::LRU, None);
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
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, Some(3), EvictionPolicy::LRU, None);
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
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let cache = GlobalCache::new(&MAP, &ORDER, None, EvictionPolicy::FIFO, None);
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
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, None, EvictionPolicy::FIFO, Some(1));
        cache.insert("expires", 999);

        // Should be valid immediately
        assert_eq!(cache.get("expires"), Some(999));

        thread::sleep(Duration::from_secs(2));

        // Should be expired now
        assert_eq!(cache.get("expires"), None);
    }

    #[test]
    fn test_global_result_ok() {
        static RES_MAP: Lazy<Mutex<HashMap<String, CacheEntry<Result<i32, String>>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static RES_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&RES_MAP, &RES_ORDER, None, EvictionPolicy::FIFO, None);
        let ok_result = Ok(42);
        cache.insert_result("success", &ok_result);
        assert_eq!(cache.get("success"), Some(Ok(42)));
    }

    #[test]
    fn test_global_result_err() {
        static RES_MAP: Lazy<Mutex<HashMap<String, CacheEntry<Result<i32, String>>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static RES_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&RES_MAP, &RES_ORDER, None, EvictionPolicy::FIFO, None);
        let err_result: Result<i32, String> = Err("error".to_string());
        cache.insert_result("failure", &err_result);
        assert_eq!(cache.get("failure"), None); // Errors not cached
    }

    #[test]
    fn test_global_concurrent_lru_access() {
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, Some(5), EvictionPolicy::LRU, None);

        // Pre-populate cache
        for i in 0..5 {
            cache.insert(&format!("k{}", i), i);
        }

        // Concurrent access to k0
        let handles: Vec<_> = (0..5)
            .map(|_| {
                thread::spawn(|| {
                    let cache = GlobalCache::new(&MAP, &ORDER, Some(5), EvictionPolicy::LRU, None);
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
        static MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        let cache = GlobalCache::new(&MAP, &ORDER, None, EvictionPolicy::FIFO, None);

        for i in 0..100 {
            cache.insert(&format!("k{}", i), i);
        }

        // All should still be present
        for i in 0..100 {
            assert_eq!(cache.get(&format!("k{}", i)), Some(i));
        }
    }
}
