use crate::{CacheEntry, EvictionPolicy};
use once_cell::sync::Lazy;
use parking_lot::lock_api::MutexGuard;
use parking_lot::{Mutex, RawMutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

use crate::utils::{
    find_arc_eviction_key, find_min_frequency_key, find_tlru_eviction_key, move_key_to_end,
    remove_key_from_global_cache,
};
#[cfg(feature = "stats")]
use crate::CacheStats;

/// A thread-safe global cache that can be shared across multiple threads.
///
/// Unlike `ThreadLocalCache` which uses thread-local storage, `GlobalCache` stores
/// cached values in global static variables protected by locks, allowing cache
/// sharing across all threads in the application.
///
/// # Type Parameters
///
/// * `R` - The return type to be cached. Must be `'static` to be stored in global state.
///
/// # Features
///
/// - **Thread-safe sharing**: Multiple threads access the same cache through RwLock/Mutex
/// - **Eviction policies**: FIFO, LRU, LFU, ARC, Random, and TLRU
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
/// - **Cache limits**: Entry count limits (`limit`) and memory-based limits (`max_memory`)
/// - **TTL support**: Automatic expiration of entries based on age
/// - **Statistics**: Optional cache hit/miss tracking (with `stats` feature)
/// - **Frequency tracking**: For LFU, ARC, and TLRU policies
/// - **Memory estimation**: Support for memory-based eviction (requires `MemoryEstimator`)
///
/// # Cache Entry Structure
///
/// Cache entries are stored as `CacheEntry<R>` which contains:
/// - `value`: The cached value of type R
/// - `timestamp`: Unix timestamp when the entry was created (for TTL and TLRU age factor)
/// - `frequency`: Access counter for LFU, ARC, and TLRU policies
///
/// # Eviction Behavior
///
/// When the cache reaches its limit (entry count or memory), entries are evicted according
/// to the configured policy:
///
/// - **FIFO**: Oldest entry (first in order queue) is evicted
/// - **LRU**: Least recently accessed entry (first in order queue) is evicted
/// - **LFU**: Entry with lowest frequency counter is evicted
/// - **ARC**: Entry with lowest score (frequency × position_weight) is evicted
/// - **Random**: Randomly selected entry is evicted
/// - **TLRU**: Entry with lowest score (frequency^weight × position × age_factor) is evicted
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
/// # Performance Characteristics
///
/// - **Get**: O(1) for cache lookup, O(n) for LRU/ARC/TLRU reordering
/// - **Insert**: O(1) for FIFO/Random, O(n) for LRU/LFU/ARC/TLRU eviction
/// - **Memory**: O(n) where n is the number of cached entries
/// - **Synchronization**: Lock acquisition overhead on every operation
///
/// # Performance Considerations
///
/// - **Synchronization overhead**: Each cache operation requires acquiring locks
/// - **Lock contention**: High concurrent access may cause threads to wait
/// - **Read optimization**: RwLock allows concurrent reads (no blocking for cache hits)
/// - **Write bottleneck**: Only one thread can modify cache at a time
/// - **Shared benefits**: All threads benefit from cached results
/// - **Best for**: Expensive computations where sharing outweighs synchronization cost
///
/// # Examples
///
/// ## Basic Usage
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
///     Some(100),         // Max 100 entries
///     None,              // No memory limit
///     EvictionPolicy::LRU,
///     Some(60),          // 60 second TTL
///     None,              // Default frequency_weight
/// );
///
/// // All threads can access the same cache
/// cache.insert("key1", 42);
/// assert_eq!(cache.get("key1"), Some(42));
/// ```
///
/// ## TLRU with Custom Frequency Weight
///
/// ```ignore
/// use cachelito_core::{GlobalCache, EvictionPolicy};
///
/// // Emphasize frequency over recency (good for popular content)
/// let cache = GlobalCache::new(
///     &CACHE_MAP,
///     &CACHE_ORDER,
///     Some(100),
///     None,
///     EvictionPolicy::TLRU,
///     Some(300),
///     Some(1.5),         // frequency_weight > 1.0
/// );
///
/// // Emphasize recency over frequency (good for time-sensitive data)
/// let cache = GlobalCache::new(
///     &CACHE_MAP,
///     &CACHE_ORDER,
///     Some(100),
///     None,
///     EvictionPolicy::TLRU,
///     Some(300),
///     Some(0.3),         // frequency_weight < 1.0
/// );
/// ```
///
/// ## With Memory Limits
///
/// ```ignore
/// use cachelito_core::{GlobalCache, EvictionPolicy, MemoryEstimator};
///
/// let cache = GlobalCache::new(
///     &CACHE_MAP,
///     &CACHE_ORDER,
///     Some(1000),
///     Some(100 * 1024 * 1024), // 100MB max
///     EvictionPolicy::LRU,
///     Some(300),
///     None,
/// );
///
/// // Insert with memory tracking (requires MemoryEstimator implementation)
/// cache.insert_with_memory("key", value);
/// ```
pub struct GlobalCache<R: 'static> {
    pub map: &'static Lazy<RwLock<HashMap<String, CacheEntry<R>>>>,
    pub order: &'static Lazy<Mutex<VecDeque<String>>>,
    pub limit: Option<usize>,
    pub max_memory: Option<usize>,
    pub policy: EvictionPolicy,
    pub ttl: Option<u64>,
    pub frequency_weight: Option<f64>,
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
    /// * `policy` - Eviction policy (FIFO, LRU, LFU, ARC, Random, or TLRU)
    /// * `ttl` - Optional time-to-live in seconds for cache entries (None for no expiration)
    /// * `frequency_weight` - Optional weight factor for frequency in TLRU policy
    ///   - Values < 1.0: Emphasize recency and age
    ///   - Values > 1.0: Emphasize frequency
    ///   - None or 1.0: Balanced approach (default)
    ///   - Only used when policy is TLRU, ignored otherwise
    /// * `stats` - Static reference to CacheStats for tracking hit/miss statistics (stats feature only)
    ///
    /// # Returns
    ///
    /// A new `GlobalCache` instance configured with the provided parameters.
    ///
    /// # Examples
    ///
    /// ## Basic LRU cache with TTL
    ///
    /// ```ignore
    /// let cache = GlobalCache::new(
    ///     &CACHE_MAP,
    ///     &CACHE_ORDER,
    ///     Some(1000),              // Max 1000 entries
    ///     None,                    // No memory limit
    ///     EvictionPolicy::LRU,     // LRU eviction
    ///     Some(300),               // 5 minute TTL
    ///     None,                    // No frequency_weight (not needed for LRU)
    ///     #[cfg(feature = "stats")]
    ///     &CACHE_STATS,
    /// );
    /// ```
    ///
    /// ## TLRU with memory limit and custom frequency weight
    ///
    /// ```ignore
    /// let cache = GlobalCache::new(
    ///     &CACHE_MAP,
    ///     &CACHE_ORDER,
    ///     Some(1000),
    ///     Some(100 * 1024 * 1024), // 100MB max
    ///     EvictionPolicy::TLRU,    // TLRU eviction
    ///     Some(300),               // 5 minute TTL
    ///     Some(1.5),               // Emphasize frequency (popular content)
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
        frequency_weight: Option<f64>,
        stats: &'static Lazy<CacheStats>,
    ) -> Self {
        Self {
            map,
            order,
            limit,
            max_memory,
            policy,
            ttl,
            frequency_weight,
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
        frequency_weight: Option<f64>,
    ) -> Self {
        Self {
            map,
            order,
            limit,
            max_memory,
            policy,
            ttl,
            frequency_weight,
        }
    }

    /// Retrieves a cached value by key.
    ///
    /// This method attempts to retrieve a cached value, checking for expiration
    /// and updating access patterns based on the eviction policy.
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
    /// # Behavior by Policy
    ///
    /// - **FIFO**: No updates on cache hit (order remains unchanged)
    /// - **LRU**: Moves the key to the end of the order queue (most recently used)
    /// - **LFU**: Increments the frequency counter for the entry
    /// - **ARC**: Increments frequency counter and updates position in order queue
    /// - **Random**: No updates on cache hit
    /// - **TLRU**: Increments frequency counter and updates position in order queue
    ///
    /// # TTL Expiration
    ///
    /// If a TTL is configured and the entry has expired:
    /// - The entry is removed from both the cache map and order queue
    /// - A cache miss is recorded (if stats feature is enabled)
    /// - `None` is returned
    ///
    /// # Statistics
    ///
    /// When the `stats` feature is enabled:
    /// - Cache hits are recorded when a valid entry is found
    /// - Cache misses are recorded when the key doesn't exist or has expired
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and uses a multi-phase locking strategy:
    /// 1. **Read lock** for initial lookup (allows concurrent reads)
    /// 2. **Mutex + Write lock** for expired entry removal (if needed)
    /// 3. **Mutex lock** for order queue updates (for LRU/ARC/TLRU)
    ///
    /// Multiple threads can safely call this method concurrently. Read-heavy
    /// workloads benefit from RwLock's concurrent read capability.
    ///
    /// # Performance
    ///
    /// - **FIFO, Random**: O(1) - no reordering needed
    /// - **LRU, ARC, TLRU**: O(n) - requires finding and moving key in order queue
    /// - **LFU**: O(1) - only increments counter
    /// - **Lock overhead**: Read lock for lookup + potential write lock for updates
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Insert and retrieve
    /// cache.insert("user:123", user_data);
    /// assert_eq!(cache.get("user:123"), Some(user_data));
    ///
    /// // Non-existent key
    /// assert_eq!(cache.get("user:999"), None);
    ///
    /// // Expired entry (with TTL)
    /// cache.insert("temp", data);
    /// std::thread::sleep(Duration::from_secs(61)); // Wait for TTL expiration
    /// assert_eq!(cache.get("temp"), None);
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
                EvictionPolicy::TLRU => {
                    // Time-aware LRU: Update both recency and frequency
                    // Similar to ARC but considers age in eviction
                    move_key_to_end(&mut self.order.lock(), key);
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
    /// When the cache limit is reached, entries are evicted according to the policy:
    /// - **FIFO**: Evicts oldest inserted entry (front of queue)
    /// - **LRU**: Evicts least recently used entry (front of queue, updated by `get()`)
    /// - **LFU**: Evicts entry with lowest frequency counter
    /// - **ARC**: Evicts based on hybrid score (frequency × position_weight)
    /// - **Random**: Evicts randomly selected entry
    /// - **TLRU**: Evicts based on TLRU score (frequency^weight × position × age_factor)
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
                    EvictionPolicy::TLRU => {
                        let mut map_write = self.map.write();
                        if let Some(evict_key) = find_tlru_eviction_key(
                            &map_write,
                            o.iter().enumerate(),
                            self.ttl,
                            self.frequency_weight,
                        ) {
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
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `value` - The value to cache (must implement `MemoryEstimator`)
    ///
    /// # Memory Management
    ///
    /// The method calculates the memory footprint of all cached entries and evicts
    /// entries as needed to stay within the `max_memory` limit. Eviction follows
    /// the configured policy.
    ///
    /// # Safety Check
    ///
    /// If the value to be inserted is larger than `max_memory`, the insertion is
    /// skipped entirely to avoid infinite eviction loops. This ensures the cache
    /// respects the memory limit even if individual values are very large.
    ///
    /// # Eviction Behavior by Policy
    ///
    /// When memory limit is exceeded:
    /// - **FIFO/LRU**: Evicts from front of order queue
    /// - **LFU**: Evicts entry with lowest frequency
    /// - **ARC**: Evicts based on hybrid score (frequency × position_weight)
    /// - **TLRU**: Evicts based on TLRU score (frequency^weight × position × age_factor)
    /// - **Random**: Evicts randomly selected entry
    ///
    /// The eviction loop continues until there's enough memory for the new value.
    ///
    /// # Entry Count Limit
    ///
    /// After satisfying memory constraints, this method also checks the entry count
    /// limit (if configured) and evicts additional entries if needed.
    ///
    /// # Thread Safety
    ///
    /// This method uses write locks to ensure consistency between the map and
    /// order queue during eviction and insertion.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use cachelito_core::MemoryEstimator;
    ///
    /// // Type must implement MemoryEstimator
    /// impl MemoryEstimator for MyLargeStruct {
    ///     fn estimate_memory(&self) -> usize {
    ///         std::mem::size_of::<Self>() + self.data.capacity()
    ///     }
    /// }
    ///
    /// // Insert with automatic memory-based eviction
    /// cache.insert_with_memory("large_data", expensive_value);
    /// ```
    ///
    /// # Performance
    ///
    /// - **Memory calculation**: O(n) - iterates all entries to sum memory
    /// - **Eviction**: Varies by policy (see individual policy documentation)
    /// - May evict multiple entries in one call if memory limit is tight
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
                    EvictionPolicy::TLRU => {
                        let mut map_write = self.map.write();
                        if let Some(evict_key) = find_tlru_eviction_key(
                            &map_write,
                            o.iter().enumerate(),
                            self.ttl,
                            self.frequency_weight,
                        ) {
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
    /// # Available Metrics
    ///
    /// The returned CacheStats provides:
    /// - **hits()**: Number of successful cache lookups
    /// - **misses()**: Number of cache misses (key not found or expired)
    /// - **hit_rate()**: Ratio of hits to total accesses (0.0 to 1.0)
    /// - **total_accesses()**: Total number of get operations
    ///
    /// # Thread Safety
    ///
    /// Statistics use atomic counters (`AtomicU64`) and can be safely accessed
    /// from multiple threads without additional synchronization.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Get basic statistics
    /// let stats = cache.stats();
    /// println!("Hits: {}", stats.hits());
    /// println!("Misses: {}", stats.misses());
    /// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
    /// println!("Total accesses: {}", stats.total_accesses());
    ///
    /// // Monitor cache performance
    /// let total = stats.total_accesses();
    /// if total > 1000 && stats.hit_rate() < 0.5 {
    ///     println!("Warning: Low cache hit rate");
    /// }
    /// ```
    ///
    /// # See Also
    ///
    /// - [`CacheStats`] - The statistics structure
    /// - [`crate::stats_registry::get()`] - Access stats by cache name
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
            None,
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
            None,
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

    // ========== TLRU with frequency_weight tests ==========

    #[test]
    fn test_tlru_with_low_frequency_weight() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        // Low frequency_weight (0.3) - emphasizes recency over frequency
        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(0.3), // Low weight
            #[cfg(feature = "stats")]
            &STATS,
        );

        // Fill cache
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = cache.get("k1");
        }

        // Wait a bit to age k1
        thread::sleep(Duration::from_millis(100));

        // Add new entry (cache is full)
        cache.insert("k4", 4);

        // With low frequency_weight, even frequent entries can be evicted
        // if they're older (recency and age matter more)
        assert_eq!(cache.get("k4"), Some(4));
    }

    #[test]
    fn test_tlru_with_high_frequency_weight() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        // High frequency_weight (1.5) - emphasizes frequency over recency
        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.5), // High weight
            #[cfg(feature = "stats")]
            &STATS,
        );

        // Fill cache
        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = cache.get("k1");
        }

        // Wait a bit to age k1
        thread::sleep(Duration::from_millis(100));

        // Add new entry (cache is full)
        cache.insert("k4", 4);

        // With high frequency_weight, frequent entries are protected
        // k1 should remain cached despite being older
        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k4"), Some(4));
    }

    #[test]
    fn test_tlru_default_frequency_weight() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        // Default frequency_weight (None = 1.0) - balanced approach
        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(5),
            None, // Default weight
            #[cfg(feature = "stats")]
            &STATS,
        );

        cache.insert("k1", 1);
        cache.insert("k2", 2);

        // Access k1 a few times
        for _ in 0..3 {
            let _ = cache.get("k1");
        }

        // Add third entry
        cache.insert("k3", 3);

        // With balanced weight, both frequency and recency matter
        // k1 has higher frequency, so it should remain
        assert_eq!(cache.get("k1"), Some(1));
        assert_eq!(cache.get("k3"), Some(3));
    }

    #[test]
    fn test_tlru_frequency_weight_comparison() {
        // Test that different weights produce different behavior
        static MAP_LOW: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER_LOW: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        static MAP_HIGH: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER_HIGH: Lazy<Mutex<VecDeque<String>>> =
            Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS_LOW: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
        #[cfg(feature = "stats")]
        static STATS_HIGH: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        let cache_low = GlobalCache::new(
            &MAP_LOW,
            &ORDER_LOW,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(0.3), // Low weight
            #[cfg(feature = "stats")]
            &STATS_LOW,
        );

        let cache_high = GlobalCache::new(
            &MAP_HIGH,
            &ORDER_HIGH,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(2.0), // High weight
            #[cfg(feature = "stats")]
            &STATS_HIGH,
        );

        // Same operations on both caches
        cache_low.insert("k1", 1);
        cache_low.insert("k2", 2);
        cache_high.insert("k1", 1);
        cache_high.insert("k2", 2);

        // Make k1 frequent in both
        for _ in 0..5 {
            let _ = cache_low.get("k1");
            let _ = cache_high.get("k1");
        }

        thread::sleep(Duration::from_millis(50));

        // Add new entry to both
        cache_low.insert("k3", 3);
        cache_high.insert("k3", 3);

        // Both should work correctly with their respective weights
        assert_eq!(cache_low.get("k3"), Some(3));
        assert_eq!(cache_high.get("k3"), Some(3));
    }

    #[test]
    fn test_tlru_no_ttl_with_frequency_weight() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        // TLRU without TTL (behaves like ARC but with frequency_weight)
        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            None, // No TTL - age_factor will be 1.0
            Some(1.5),
            #[cfg(feature = "stats")]
            &STATS,
        );

        cache.insert("k1", 1);
        cache.insert("k2", 2);
        cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = cache.get("k1");
        }

        // Add new entry
        cache.insert("k4", 4);

        // Without TTL, TLRU focuses on frequency and position
        // k1 should remain due to high frequency
        assert_eq!(cache.get("k1"), Some(1));
    }

    #[test]
    fn test_tlru_concurrent_with_frequency_weight() {
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
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.2), // Slightly emphasize frequency
            #[cfg(feature = "stats")]
            &STATS,
        );

        // Insert initial entries
        cache.insert("k1", 1);
        cache.insert("k2", 2);

        // Spawn multiple threads accessing the cache
        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let cache = GlobalCache::new(
                        &MAP,
                        &ORDER,
                        Some(5),
                        None,
                        EvictionPolicy::TLRU,
                        Some(10),
                        Some(1.2),
                        #[cfg(feature = "stats")]
                        &STATS,
                    );

                    // Access k1 frequently
                    for _ in 0..3 {
                        let _ = cache.get("k1");
                    }

                    // Insert new entry
                    cache.insert(&format!("k{}", i + 3), i + 3);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // k1 should remain cached due to high frequency and frequency_weight > 1.0
        assert_eq!(cache.get("k1"), Some(1));
    }

    #[test]
    fn test_tlru_frequency_weight_edge_cases() {
        static MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
            Lazy::new(|| RwLock::new(HashMap::new()));
        static ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        // Test with very low weight (close to 0)
        let cache = GlobalCache::new(
            &MAP,
            &ORDER,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(5),
            Some(0.1), // Very low weight
            #[cfg(feature = "stats")]
            &STATS,
        );

        cache.insert("k1", 1);
        cache.insert("k2", 2);

        // Make k1 extremely frequent
        for _ in 0..100 {
            let _ = cache.get("k1");
        }

        thread::sleep(Duration::from_millis(50));

        // Even with very high frequency, k1 might be evicted with very low weight
        cache.insert("k3", 3);

        // The cache should still work correctly
        assert!(cache.get("k3").is_some());
    }
}
