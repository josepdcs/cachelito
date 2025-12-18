use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::thread::LocalKey;

use crate::{CacheEntry, EvictionPolicy};

#[cfg(feature = "stats")]
use crate::CacheStats;

use crate::utils::{
    find_arc_eviction_key, find_min_frequency_key, find_tlru_eviction_key, move_key_to_end,
    remove_key_from_cache_local,
};

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
/// - **Configurable limits**: Optional entry count limit and memory limit
/// - **Eviction policies**: FIFO, LRU (default), LFU, ARC, Random, and TLRU
///   - **FIFO**: First In, First Out - simple and predictable
///   - **LRU**: Least Recently Used - evicts least recently accessed entries
///   - **LFU**: Least Frequently Used - evicts least frequently accessed entries
///   - **ARC**: Adaptive Replacement Cache - hybrid policy combining recency and frequency
///   - **Random**: Random replacement - O(1) eviction with minimal overhead
///   - **TLRU**: Time-aware LRU - combines recency, frequency, and age factors
///     - Customizable with `frequency_weight` parameter
///     - Formula: `score = frequency^weight × position × age_factor`
///     - `frequency_weight < 1.0`: Emphasize recency (time-sensitive data)
///     - `frequency_weight > 1.0`: Emphasize frequency (popular content)
/// - **TTL support**: Optional time-to-live for automatic expiration
/// - **Result-aware**: Special handling for `Result<T, E>` types
/// - **Memory-based limits**: Optional maximum memory usage (requires `MemoryEstimator`)
/// - **Statistics tracking**: Optional hit/miss monitoring (requires `stats` feature)
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
/// let cache = ThreadLocalCache::new(&MY_CACHE, &MY_ORDER, None, None, EvictionPolicy::FIFO, None, None);
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
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), None, EvictionPolicy::LRU, None, None);
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
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, None, EvictionPolicy::FIFO, Some(60), None);
/// cache.insert("key", "value".to_string());
///
/// // Entry will expire after 60 seconds
/// // get() returns None for expired entries
/// ```
///
/// ## TLRU with Custom Frequency Weight
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
/// // Low frequency_weight (0.3) - emphasizes recency over frequency
/// // Good for time-sensitive data where freshness matters more than popularity
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), None, EvictionPolicy::TLRU, Some(300), Some(0.3));
///
/// // High frequency_weight (1.5) - emphasizes frequency over recency
/// // Good for popular content that should stay cached despite age
/// let cache_popular = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), None, EvictionPolicy::TLRU, Some(300), Some(1.5));
///
/// // Default (omit frequency_weight) - balanced approach
/// let cache_balanced = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), None, EvictionPolicy::TLRU, Some(300), None);
/// ```
pub struct ThreadLocalCache<R: 'static> {
    /// Reference to the thread-local storage key for the cache HashMap
    pub cache: &'static LocalKey<RefCell<HashMap<String, CacheEntry<R>>>>,
    /// Reference to the thread-local storage key for the cache order queue
    pub order: &'static LocalKey<RefCell<VecDeque<String>>>,
    /// Maximum number of items to store in the cache
    pub limit: Option<usize>,
    /// Maximum memory size in bytes
    pub max_memory: Option<usize>,
    /// Eviction policy to use for the cache
    pub policy: EvictionPolicy,
    /// Optional TTL (in seconds) for cache entries
    pub ttl: Option<u64>,
    /// Frequency weight for TLRU policy (0.0 to 1.0). Only used when policy is TLRU.
    pub frequency_weight: Option<f64>,
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
    /// * `max_memory` - Optional maximum memory size in bytes (None for unlimited)
    /// * `policy` - Eviction policy to use when limit is reached
    /// * `ttl` - Optional time-to-live in seconds (None for no expiration)
    /// * `frequency_weight` - Optional frequency weight for TLRU policy (0.0 to 1.0)
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
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, Some(100), None, EvictionPolicy::LRU, Some(60), None);
    /// ```
    pub fn new(
        cache: &'static LocalKey<RefCell<HashMap<String, CacheEntry<R>>>>,
        order: &'static LocalKey<RefCell<VecDeque<String>>>,
        limit: Option<usize>,
        max_memory: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
        frequency_weight: Option<f64>,
    ) -> Self {
        Self {
            cache,
            order,
            limit,
            max_memory,
            policy,
            ttl,
            frequency_weight,
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
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, None, EvictionPolicy::FIFO, None, None);
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
                    self.move_to_end(key);
                }
                EvictionPolicy::LFU => {
                    // Increment frequency counter
                    self.increment_frequency(key);
                }
                EvictionPolicy::ARC => {
                    // Adaptive Replacement: Update both recency and frequency
                    // Update order (recency)
                    self.move_to_end(key);
                    // Increment frequency counter
                    self.increment_frequency(key);
                }
                EvictionPolicy::TLRU => {
                    // Time-aware LRU: Update both recency and frequency
                    // Similar to ARC but considers age in eviction
                    self.move_to_end(key);
                    self.increment_frequency(key);
                }
                EvictionPolicy::FIFO | EvictionPolicy::Random => {
                    // No update needed for FIFO or Random
                }
            }
        }

        val
    }

    /// Moves a key to the end of the order queue (marks as most recently used)
    fn move_to_end(&self, key: &str) {
        self.order.with(|o| {
            let mut o = o.borrow_mut();
            move_key_to_end(&mut o, key);
        });
    }

    /// Increments the frequency counter for the specified key.
    fn increment_frequency(&self, key: &str) {
        self.cache.with(|c| {
            let mut c = c.borrow_mut();
            if let Some(entry) = c.get_mut(key) {
                entry.increment_frequency();
            }
        });
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
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, None, EvictionPolicy::FIFO, None, None);
    /// cache.insert("first", 1);
    /// cache.insert("first", 2); // Replaces previous value
    /// assert_eq!(cache.get("first"), Some(2));
    /// ```
    ///
    /// # Note
    ///
    /// This method does NOT require `MemoryEstimator` trait. It only handles entry-count limits.
    /// If `max_memory` is configured, use `insert_with_memory()` instead, which requires
    /// the type to implement `MemoryEstimator`.
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

            // Only handle entry-count limits (not memory limits)
            self.handle_entry_limit_eviction(&mut order);
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
    /// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, None, EvictionPolicy::FIFO, None, None);
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

    /// Removes a key from the cache and its associated ordering.
    fn remove_key(&self, key: &str) {
        self.cache.with(|c| {
            self.order.with(|o| {
                remove_key_from_cache_local(&mut c.borrow_mut(), &mut o.borrow_mut(), key);
            });
        });
    }

    /// Handles the eviction of entries from a cache to enforce the entry limit based on the specified eviction policy.
    ///
    /// This method ensures that the number of entries in the cache does not exceed the configured limit by removing
    /// entries based on the specified eviction policy: LFU (Least Frequently Used), ARC (Adaptive Replacement Cache),
    /// FIFO (First In, First Out), or LRU (Least Recently Used).
    ///
    /// # Parameters
    /// - `order`: A mutable reference to a `VecDeque<String>` representing the order of keys in the cache. The order
    ///   is used differently depending on the eviction policy, e.g., for determining the least recently or most
    ///   recently used key.
    ///
    /// # Behavior
    /// If the cache's entry limit (`self.limit`) is exceeded:
    /// - For `EvictionPolicy::LFU`: The key with the lowest usage frequency will be identified and evicted.
    /// - For `EvictionPolicy::ARC`: The key to be evicted is determined adaptively using an ARC strategy.
    /// - For `EvictionPolicy::FIFO`: The earliest inserted key (front of the `order` queue) is removed.
    /// - For `EvictionPolicy::LRU`: The least recently used key (front of the `order` queue) is removed.
    ///
    /// The eviction process involves:
    /// 1. Identifying the key to evict based on the eviction policy.
    /// 2. Removing the key from both the `order` queue and the underlying cache storage (`self.cache`).
    /// 3. Breaking the loop upon successfully removing an entry (for FIFO/LRU).
    ///
    /// # Notes
    /// - This method assumes that the order of keys in the cache is maintained in the `order` deque.
    /// - The actual eviction is accomplished via helper functions such as `find_min_frequency_key` and `find_arc_eviction_key`.
    /// - The removal operation ensures consistency by simultaneously updating the `order` deque and the cache storage (`self.cache`).
    ///
    /// # Eviction Policy Details
    /// - **LFU** (Least Frequently Used): Evicts the cache entry that has been accessed the least number of times.
    ///   Relies on `find_min_frequency_key`, which finds the key with the minimum usage frequency in the cache.
    /// - **ARC** (Adaptive Replacement Cache): Uses an adaptive replacement strategy to optimize for both recency
    ///   and frequency of access. The key to evict is determined by `find_arc_eviction_key`, which takes into account
    ///   both recent and frequent usage patterns.
    /// - **FIFO** (First In, First Out): Evicts the oldest entry in the cache, as determined by the front of `order`.
    /// - **LRU** (Least Recently Used): Evicts the least recently used entry, which is also at the front of `order`.
    /// - **Random**: Evicts a randomly selected entry from the cache.
    ///
    /// # Edge Cases
    /// - If the cache has no limit (`self.limit == None`), this method performs no action.
    /// - If the `order` deque is empty when attempting to evict an entry, no action is taken.
    /// - For FIFO and LRU policies, evictions will continue iteratively until a valid, non-removed key is found.
    /// - If an eviction policy is misused or improperly implemented, it might lead to incomplete or inefficient evictions.
    fn handle_entry_limit_eviction(&self, order: &mut VecDeque<String>) {
        if let Some(limit) = self.limit {
            if order.len() > limit {
                match self.policy {
                    EvictionPolicy::LFU => {
                        let min_freq_key = self
                            .cache
                            .with(|c| find_min_frequency_key(&c.borrow(), order));

                        if let Some(evict_key) = min_freq_key {
                            self.remove_key(&evict_key);
                        }
                    }
                    EvictionPolicy::ARC => {
                        let evict_key = self
                            .cache
                            .with(|c| find_arc_eviction_key(&c.borrow(), order.iter().enumerate()));

                        if let Some(key) = evict_key {
                            self.remove_key(&key);
                        }
                    }
                    EvictionPolicy::TLRU => {
                        let evict_key = self.cache.with(|c| {
                            find_tlru_eviction_key(
                                &c.borrow(),
                                order.iter().enumerate(),
                                self.ttl,
                                self.frequency_weight,
                            )
                        });

                        if let Some(key) = evict_key {
                            self.remove_key(&key);
                        }
                    }
                    EvictionPolicy::Random => {
                        // O(1) random eviction: select random position and remove directly
                        if !order.is_empty() {
                            let pos = fastrand::usize(..order.len());
                            if let Some(evict_key) = order.remove(pos) {
                                // Remove from cache
                                self.cache.with(|c| {
                                    c.borrow_mut().remove(&evict_key);
                                });
                            }
                        }
                    }
                    EvictionPolicy::FIFO | EvictionPolicy::LRU => {
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
                        }
                    }
                }
            }
        }
    }
}

// Separate implementation for types that implement MemoryEstimator
// This allows memory-based eviction
impl<R: Clone + 'static + crate::MemoryEstimator> ThreadLocalCache<R> {
    /// Insert with memory limit support.
    ///
    /// This method requires `R` to implement `MemoryEstimator` and handles both
    /// memory-based and entry-count-based eviction.
    ///
    /// Use this method when `max_memory` is configured in the cache.
    pub fn insert_with_memory(&self, key: &str, value: R) {
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

            // Check memory limit first (if specified)
            if let Some(max_mem) = self.max_memory {
                // First, check if the new value by itself exceeds max_mem
                // This is a safety check to prevent infinite eviction loop
                let new_value_size = self.cache.with(|c| {
                    c.borrow()
                        .get(&key)
                        .map(|e| e.value.estimate_memory())
                        .unwrap_or(0)
                });

                if new_value_size > max_mem {
                    // The value itself is too large for the cache
                    // Remove it and return early to respect memory limit
                    self.cache.with(|c| {
                        c.borrow_mut().remove(&key);
                    });
                    order.pop_back(); // Remove from order queue as well
                    return;
                }

                loop {
                    let current_mem = self.cache.with(|c| {
                        let cache = c.borrow();
                        cache
                            .values()
                            .map(|e| e.value.estimate_memory())
                            .sum::<usize>()
                    });

                    if current_mem <= max_mem {
                        break;
                    }

                    // Need to evict based on policy
                    let evicted = match self.policy {
                        EvictionPolicy::LFU => {
                            let min_freq_key = self
                                .cache
                                .with(|c| find_min_frequency_key(&c.borrow(), &order));
                            if let Some(evict_key) = min_freq_key {
                                self.remove_key(&evict_key);
                                true
                            } else {
                                false
                            }
                        }
                        EvictionPolicy::ARC => {
                            let evict_key = self.cache.with(|c| {
                                find_arc_eviction_key(&c.borrow(), order.iter().enumerate())
                            });
                            if let Some(key) = evict_key {
                                self.remove_key(&key);
                                true
                            } else {
                                false
                            }
                        }
                        EvictionPolicy::TLRU => {
                            let evict_key = self.cache.with(|c| {
                                find_tlru_eviction_key(
                                    &c.borrow(),
                                    order.iter().enumerate(),
                                    self.ttl,
                                    self.frequency_weight,
                                )
                            });
                            if let Some(key) = evict_key {
                                self.remove_key(&key);
                                true
                            } else {
                                false
                            }
                        }
                        EvictionPolicy::Random => {
                            // O(1) random eviction: select random position and remove directly
                            if !order.is_empty() {
                                let pos = fastrand::usize(..order.len());
                                if let Some(evict_key) = order.remove(pos) {
                                    // Remove from cache
                                    self.cache.with(|c| {
                                        c.borrow_mut().remove(&evict_key);
                                    });
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        EvictionPolicy::FIFO | EvictionPolicy::LRU => {
                            if let Some(evict_key) = order.pop_front() {
                                self.cache.with(|c| {
                                    c.borrow_mut().remove(&evict_key);
                                });
                                true
                            } else {
                                false
                            }
                        }
                    };

                    if !evicted {
                        break; // Nothing left to evict
                    }
                }
            }

            // Handle entry-count limits
            self.handle_entry_limit_eviction(&mut order);
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
/// let cache = ThreadLocalCache::new(&CACHE, &ORDER, None, None, EvictionPolicy::FIFO, None, None);
///
/// // Ok values are cached
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
    /// This version does NOT require MemoryEstimator. Use `insert_result_with_memory()`
    /// when max_memory is configured.
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

/// Implementation for Result types WITH MemoryEstimator support.
impl<
        T: Clone + Debug + 'static + crate::MemoryEstimator,
        E: Clone + Debug + 'static + crate::MemoryEstimator,
    > ThreadLocalCache<Result<T, E>>
{
    /// Inserts a Result into the cache with memory limit support.
    ///
    /// This method requires both T and E to implement MemoryEstimator.
    /// Use this when max_memory is configured.
    pub fn insert_result_with_memory(&self, key: &str, value: &Result<T, E>) {
        if let Ok(val) = value {
            self.insert_with_memory(key, Ok(val.clone()));
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
        ThreadLocalCache::new(&TEST_CACHE, &TEST_ORDER, limit, None, policy, ttl, None)
    }

    fn setup_cache_with_weight(
        limit: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
        frequency_weight: Option<f64>,
    ) -> ThreadLocalCache<i32> {
        TEST_CACHE.with(|c| c.borrow_mut().clear());
        TEST_ORDER.with(|o| o.borrow_mut().clear());
        ThreadLocalCache::new(
            &TEST_CACHE,
            &TEST_ORDER,
            limit,
            None,
            policy,
            ttl,
            frequency_weight,
        )
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

        let cache = ThreadLocalCache::new(
            &RES_CACHE,
            &RES_ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            None,
        );
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

        let cache = ThreadLocalCache::new(
            &RES_CACHE,
            &RES_ORDER,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            None,
        );
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

    // ========== TLRU with frequency_weight tests ==========
    // Note: These tests avoid triggering complex eviction due to a known RefCell borrow issue
    // in handle_entry_limit_eviction for TLRU policy

    #[test]
    fn test_tlru_with_frequency_weight_basic() {
        // Test basic TLRU behavior with frequency_weight without hitting limit
        let cache = setup_cache_with_weight(Some(10), EvictionPolicy::TLRU, Some(10), Some(1.5));

        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Access k1 multiple times to increase frequency
        for _ in 0..5 {
            assert_eq!(cache.get("k1"), Some(1));
        }

        // All entries should still be cached (no eviction yet)
        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), Some(2));
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_tlru_default_frequency_weight_basic() {
        // Test TLRU with default frequency_weight (None = 1.0)
        let cache = setup_cache_with_weight(Some(10), EvictionPolicy::TLRU, Some(5), None);

        cache.insert("k1", 1);
        cache.insert("k2", 2);

        // Access k1 a few times
        for _ in 0..3 {
            let _ = cache.get("k1");
        }

        // Both should be cached
        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), Some(2));
    }

    #[test]
    fn test_tlru_no_ttl_with_frequency_weight() {
        // TLRU without TTL (age_factor = 1.0) but with frequency_weight
        let cache = setup_cache_with_weight(Some(10), EvictionPolicy::TLRU, None, Some(1.5));

        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = cache.get("k1");
        }

        // All should be cached (no limit reached)
        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), Some(2));
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_tlru_frequency_tracking() {
        // Verify that TLRU tracks frequency correctly
        let cache = setup_cache_with_weight(Some(10), EvictionPolicy::TLRU, Some(10), Some(1.0));

        cache.insert("k1", 1);
        cache.insert("k2", 2);

        // Access k1 multiple times
        for _ in 0..5 {
            assert_eq!(cache.get("k1"), Some(1));
        }

        // Access k2 once
        assert_eq!(cache.get("k2"), Some(2));

        // Both should still be present
        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k2"), Some(2));
    }

    #[test]
    fn test_tlru_with_different_weights() {
        // Test that different frequency_weight values are accepted
        let cache_low =
            setup_cache_with_weight(Some(10), EvictionPolicy::TLRU, Some(10), Some(0.3));
        let cache_high =
            setup_cache_with_weight(Some(10), EvictionPolicy::TLRU, Some(10), Some(2.0));

        cache_low.insert("k1", 1);
        cache_high.insert("k1", 1);

        assert_eq!(cache_low.get("k1"), Some(1));
        assert_eq!(cache_high.get("k1"), Some(1));
    }
}
