#[cfg(feature = "stats")]
use crate::CacheStats;
use crate::EvictionPolicy;
use dashmap::DashMap;
use parking_lot::lock_api::MutexGuard;
use parking_lot::{Mutex, RawMutex};
use std::collections::VecDeque;

/// A thread-safe async global cache with configurable eviction policies and TTL support.
///
/// This cache is designed specifically for async/await contexts and uses lock-free
/// concurrent data structures (DashMap) for optimal performance under high concurrency.
///
/// # Type Parameters
///
/// * `R` - The type of values stored in the cache. Must implement `Clone`.
///
/// # Features
///
/// - **Lock-free reads/writes**: Uses DashMap for concurrent access without blocking
/// - **Eviction policies**: FIFO, LRU, and LFU
/// - **TTL support**: Automatic expiration of entries
/// - **Statistics**: Optional cache hit/miss tracking (with `stats` feature)
/// - **Frequency tracking**: For LFU policy
///
/// # Cache Entry Structure
///
/// Each cache entry is stored as a tuple: `(value, timestamp, frequency)`
/// - `value`: The cached value of type R
/// - `timestamp`: Unix timestamp when the entry was created (for TTL)
/// - `frequency`: Access counter for LFU policy
///
/// # Examples
///
/// ```ignore
/// use cachelito_core::{AsyncGlobalCache, EvictionPolicy};
/// use dashmap::DashMap;
/// use parking_lot::Mutex;
/// use std::collections::VecDeque;
///
/// let cache = DashMap::new();
/// let order = Mutex::new(VecDeque::new());
/// let async_cache = AsyncGlobalCache::new(
///     &cache,
///     &order,
///     Some(100),
///     EvictionPolicy::LRU,
///     Some(60),
/// );
///
/// // In async context:
/// if let Some(value) = async_cache.get("key") {
///     println!("Cache hit: {}", value);
/// }
/// ```
pub struct AsyncGlobalCache<'a, R: Clone> {
    /// The underlying DashMap storing cache entries
    /// Structure: key -> (value, timestamp, frequency)
    cache: &'a DashMap<String, (R, u64, u64)>,

    /// Order queue for FIFO/LRU eviction tracking
    order: &'a Mutex<VecDeque<String>>,

    /// Maximum number of entries (None = unlimited)
    limit: Option<usize>,

    /// Maximum memory size in bytes (None = unlimited)
    max_memory: Option<usize>,

    /// Eviction policy to use
    policy: EvictionPolicy,

    /// Time-to-live in seconds (None = no expiration)
    ttl: Option<u64>,

    /// Cache statistics (when stats feature is enabled)
    #[cfg(feature = "stats")]
    stats: &'a CacheStats,
}

impl<'a, R: Clone> AsyncGlobalCache<'a, R> {
    /// Creates a new `AsyncGlobalCache`.
    ///
    /// # Arguments
    ///
    /// * `cache` - Reference to the DashMap storing cache entries
    /// * `order` - Reference to the Mutex-protected eviction order queue
    /// * `limit` - Optional maximum number of entries
    /// * `max_memory` - Optional maximum memory size in bytes
    /// * `policy` - Eviction policy (FIFO, LRU, or LFU)
    /// * `ttl` - Optional time-to-live in seconds
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let cache = DashMap::new();
    /// let order = Mutex::new(VecDeque::new());
    /// let async_cache = AsyncGlobalCache::new(
    ///     &cache,
    ///     &order,
    ///     Some(1000),
    ///     Some(100 * 1024 * 1024), // 100MB
    ///     EvictionPolicy::LRU,
    ///     Some(300),
    /// );
    /// ```
    #[cfg(not(feature = "stats"))]
    pub fn new(
        cache: &'a DashMap<String, (R, u64, u64)>,
        order: &'a Mutex<VecDeque<String>>,
        limit: Option<usize>,
        max_memory: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
    ) -> Self {
        Self {
            cache,
            order,
            limit,
            max_memory,
            policy,
            ttl,
        }
    }

    /// Creates a new `AsyncGlobalCache` with statistics support.
    ///
    /// This version is available when the `stats` feature is enabled.
    #[cfg(feature = "stats")]
    pub fn new(
        cache: &'a DashMap<String, (R, u64, u64)>,
        order: &'a Mutex<VecDeque<String>>,
        limit: Option<usize>,
        max_memory: Option<usize>,
        policy: EvictionPolicy,
        ttl: Option<u64>,
        stats: &'a CacheStats,
    ) -> Self {
        Self {
            cache,
            order,
            limit,
            max_memory,
            policy,
            ttl,
            stats,
        }
    }

    /// Attempts to retrieve a value from the cache.
    ///
    /// This method checks if the key exists, validates TTL expiration,
    /// updates access patterns based on the eviction policy, and records statistics.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to look up
    ///
    /// # Returns
    ///
    /// * `Some(R)` - The cached value if found and not expired
    /// * `None` - If the key doesn't exist or has expired
    ///
    /// # Behavior by Policy
    ///
    /// - **FIFO**: No updates on cache hit
    /// - **LRU**: Moves the key to the end of the order queue (most recently used)
    /// - **LFU**: Increments the frequency counter
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(user) = async_cache.get("user:123") {
    ///     println!("Found user: {:?}", user);
    /// } else {
    ///     println!("Cache miss");
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<R> {
        // Check cache first
        if let Some(mut entry_ref) = self.cache.get_mut(key) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Check if expired
            let is_expired = if let Some(ttl) = self.ttl {
                now - entry_ref.1 > ttl
            } else {
                false
            };

            if !is_expired {
                let cached_value = entry_ref.0.clone();

                // Update access patterns based on policy
                match self.policy {
                    EvictionPolicy::LFU => {
                        // Increment frequency counter
                        entry_ref.2 = entry_ref.2.saturating_add(1);
                    }
                    EvictionPolicy::ARC => {
                        // Increment frequency counter for ARC
                        entry_ref.2 = entry_ref.2.saturating_add(1);
                        // LRU update happens after releasing the entry lock
                    }
                    EvictionPolicy::LRU => {
                        // LRU update happens after releasing the entry lock
                    }
                    EvictionPolicy::FIFO | EvictionPolicy::Random => {
                        // No update needed
                    }
                }

                drop(entry_ref);

                // Record cache hit
                #[cfg(feature = "stats")]
                self.stats.record_hit();

                // Update LRU order on cache hit (after releasing DashMap lock)
                if self.limit.is_some()
                    && (self.policy == EvictionPolicy::LRU || self.policy == EvictionPolicy::ARC)
                {
                    if self.cache.contains_key(key) {
                        let mut order = self.order.lock();
                        // Double-check after acquiring lock
                        if self.cache.contains_key(key) {
                            order.retain(|k| k != key);
                            order.push_back(key.to_string());
                        }
                    }
                }

                return Some(cached_value);
            }

            // Expired - remove and continue
            drop(entry_ref);
            self.cache.remove(key);

            // Also remove from order queue to prevent orphaned keys
            let mut order = self.order.lock();
            order.retain(|k| k != key);
        }

        // Record cache miss
        #[cfg(feature = "stats")]
        self.stats.record_miss();

        None
    }

    /// Inserts a value into the cache.
    ///
    /// This method handles cache limit enforcement and eviction according to
    /// the configured policy. If the cache is full, it evicts an entry before
    /// inserting the new one.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `value` - The value to cache
    ///
    /// # Eviction Behavior
    ///
    /// - **FIFO**: Evicts the oldest inserted entry (front of queue)
    /// - **LRU**: Evicts the least recently used entry (front of queue)
    /// - **LFU**: Evicts the entry with the lowest frequency counter
    /// - **ARC**: Evicts based on a hybrid score of frequency and recency
    ///
    /// # Thread Safety
    ///
    /// This method uses locks to ensure consistency between the cache and
    /// the order queue. The order lock is held during eviction and insertion
    /// to prevent race conditions.
    ///
    /// # Note
    ///
    /// This method does NOT require `MemoryEstimator` trait. It only handles entry-count limits.
    /// If `max_memory` is configured, use `insert_with_memory()` instead, which requires
    /// the type to implement `MemoryEstimator`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Insert a new value
    /// async_cache.insert("user:123", user_data);
    ///
    /// // Update existing value
    /// async_cache.insert("user:123", updated_user_data);
    /// ```
    pub fn insert(&self, key: &str, value: R) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut order = self.order.lock();

        // Check if another task already inserted this key while we were computing
        if self.is_already_key_inserted(key, &mut order) {
            return;
        }

        // Handle entry-count limits
        self.handle_entry_limit_eviction(&mut order);

        // Add the new entry to the order queue
        order.push_back(key.to_string());

        // Insert into cache with frequency initialized to 0
        self.cache.insert(key.to_string(), (value, timestamp, 0));
    }

    /// Checks if a key is already present in the cache and updates its position in the eviction order
    /// if the eviction policy is Least Recently Used (LRU) or Adaptive Replacement Cache (ARC).
    ///
    /// # Parameters
    /// - `key`: A reference to the key being checked as a `&str`.
    /// - `order`: A mutable reference to a locked `VecDeque<String>` wrapped in a `MutexGuard`.
    ///    This represents the ordered list of keys, used to determine eviction order.
    ///
    /// # Returns
    /// - `true` if the key is already present in the cache and was processed for eviction policy.
    /// - `false` if the key was not found in the cache.
    ///
    /// # Behavior
    /// 1. If the key exists in the cache:
    ///    - If the eviction policy is `LRU` or `ARC`, the key's position in the eviction list (`order`)
    ///      is updated to reflect that it was recently accessed by removing the old position and appending
    ///      the key to the back of the `VecDeque`.
    ///    - The function returns `true`, indicating the key is already in the cache.
    /// 2. If the key does not exist in the cache:
    ///    - The function returns `false`, allowing the caller to handle the key insertion.
    ///
    /// # Eviction Policies
    /// - `LRU` (Least Recently Used): Keys recently accessed should stay in the cache,
    ///   and their access order is updated.
    /// - `ARC` (Adaptive Replacement Cache): Performs similarly to LRU but may enhance
    ///   replacement policies in specific cases.
    fn is_already_key_inserted(
        &self,
        key: &str,
        order: &mut MutexGuard<RawMutex, VecDeque<String>>,
    ) -> bool {
        if self.cache.contains_key(key) {
            // Key already exists, just update the order if LRU or ARC
            if self.policy == EvictionPolicy::LRU || self.policy == EvictionPolicy::ARC {
                order.retain(|k| k != key);
                order.push_back(key.to_string());
            }
            // Don't insert again
            return true;
        }
        false
    }

    /// Finds the key with minimum frequency for LFU eviction.
    ///
    /// # Parameters
    ///
    /// * `order` - The order queue to search
    ///
    /// # Returns
    ///
    /// * `Option<String>` - The key with minimum frequency, or None if not found
    fn find_min_frequency_key(&self, order: &VecDeque<String>) -> Option<String> {
        let mut min_freq_key: Option<String> = None;
        let mut min_freq = u64::MAX;

        for evict_key in order.iter() {
            if let Some(entry) = self.cache.get(evict_key) {
                if entry.2 < min_freq {
                    min_freq = entry.2;
                    min_freq_key = Some(evict_key.clone());
                }
            }
        }

        min_freq_key
    }

    /// Finds the key to evict using ARC (Adaptive Replacement Cache) policy.
    ///
    /// ARC uses a hybrid score combining frequency and recency.
    /// Score = frequency * position_weight (higher position = more recent)
    ///
    /// # Parameters
    ///
    /// * `order` - The order queue to search
    ///
    /// # Returns
    ///
    /// * `Option<String>` - The key with lowest score, or None if not found
    fn find_arc_eviction_key(&self, order: &VecDeque<String>) -> Option<String> {
        let mut best_evict_key: Option<String> = None;
        let mut best_score = f64::MAX;

        for (idx, evict_key) in order.iter().enumerate() {
            if let Some(entry) = self.cache.get(evict_key) {
                let frequency = entry.2 as f64;
                let position_weight = (order.len() - idx) as f64;
                let score = frequency * position_weight;

                if score < best_score {
                    best_score = score;
                    best_evict_key = Some(evict_key.clone());
                }
            }
        }

        best_evict_key
    }

    /// Handles the eviction of entries from the cache to enforce the entry limit based on the eviction policy.
    ///
    /// This method ensures that the number of entries in the cache does not exceed the configured limit by removing
    /// entries based on the specified eviction policy.
    ///
    /// # Parameters
    ///
    /// * `order` - A mutable reference to the order queue
    ///
    /// # Behavior
    ///
    /// If the cache's entry limit is exceeded:
    /// - **LFU**: Evicts the entry with the lowest frequency counter
    /// - **ARC**: Evicts based on a hybrid score of frequency and recency
    /// - **FIFO/LRU**: Evicts from the front of the queue
    fn handle_entry_limit_eviction(&self, order: &mut VecDeque<String>) {
        if let Some(limit) = self.limit {
            if self.cache.len() >= limit {
                match self.policy {
                    EvictionPolicy::LFU => {
                        if let Some(evict_key) = self.find_min_frequency_key(order) {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                        }
                    }
                    EvictionPolicy::ARC => {
                        if let Some(evict_key) = self.find_arc_eviction_key(order) {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                        }
                    }
                    EvictionPolicy::Random => {
                        if let Some(evict_key) =
                            crate::utils::select_random_eviction_key(order.iter())
                        {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                        }
                    }
                    EvictionPolicy::FIFO | EvictionPolicy::LRU => {
                        // FIFO and LRU: evict from front of queue
                        while let Some(evict_key) = order.pop_front() {
                            if self.cache.contains_key(&evict_key) {
                                self.cache.remove(&evict_key);
                                break;
                            }
                            // Key doesn't exist in cache (already removed), try next one
                        }
                    }
                }
            }
        }
    }

    /// Returns a reference to the cache statistics.
    ///
    /// This method is only available when the `stats` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let stats = async_cache.stats();
    /// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
    /// ```
    #[cfg(feature = "stats")]
    pub fn stats(&self) -> &CacheStats {
        self.stats
    }
}

// Separate implementation for types that implement MemoryEstimator
// This allows memory-based eviction
impl<'a, R: Clone + crate::MemoryEstimator> AsyncGlobalCache<'a, R> {
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
    /// * `value` - The value to cache
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // For types that implement MemoryEstimator
    /// async_cache.insert_with_memory("large_data", expensive_value);
    /// ```
    pub fn insert_with_memory(&self, key: &str, value: R) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut order = self.order.lock();

        // Check if another task already inserted this key while we were computing
        if self.is_already_key_inserted(key, &mut order) {
            return;
        }

        // Check memory limit first (if specified)
        if let Some(max_mem) = self.max_memory {
            let value_size = value.estimate_memory();

            // Safety check: if the value itself is larger than max_mem,
            // we need to handle it to avoid infinite loop
            if value_size > max_mem {
                // Value is too large to fit in cache even when empty
                // We have two options:
                // 1. Don't cache it at all (skip insertion)
                // 2. Clear all entries and cache it anyway
                // We choose option 1 to respect the memory limit
                return;
            }

            loop {
                let current_mem: usize = self
                    .cache
                    .iter()
                    .map(|entry| entry.value().0.estimate_memory())
                    .sum();

                if current_mem + value_size <= max_mem {
                    break;
                }

                // Need to evict based on policy
                let evicted = match self.policy {
                    EvictionPolicy::LFU => {
                        if let Some(evict_key) = self.find_min_frequency_key(&*order) {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                            true
                        } else {
                            false
                        }
                    }
                    EvictionPolicy::ARC => {
                        if let Some(evict_key) = self.find_arc_eviction_key(&*order) {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                            true
                        } else {
                            false
                        }
                    }
                    EvictionPolicy::Random => {
                        if let Some(evict_key) =
                            crate::utils::select_random_eviction_key(order.iter())
                        {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                            true
                        } else {
                            false
                        }
                    }
                    EvictionPolicy::FIFO | EvictionPolicy::LRU => {
                        if let Some(evict_key) = order.pop_front() {
                            self.cache.remove(&evict_key);
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

        // Handle entry-count limits (reuse the same method)
        self.handle_entry_limit_eviction(&mut order);

        // Add the new entry to the order queue
        order.push_back(key.to_string());

        // Insert into cache with frequency initialized to 0
        self.cache.insert(key.to_string(), (value, timestamp, 0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_cache_basic() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache =
            AsyncGlobalCache::new(&cache, &order, None, None, EvictionPolicy::FIFO, None);

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            None,
            None,
            EvictionPolicy::FIFO,
            None,
            &stats,
        );

        // Test insert and get
        async_cache.insert("key1", "value1");
        assert_eq!(async_cache.get("key1"), Some("value1"));
        assert_eq!(async_cache.get("key2"), None);
    }

    #[test]
    fn test_async_cache_lfu_eviction() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache =
            AsyncGlobalCache::new(&cache, &order, Some(2), None, EvictionPolicy::LFU, None);

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(2),
            None,
            EvictionPolicy::LFU,
            None,
            &stats,
        );

        // Insert two entries
        async_cache.insert("key1", "value1");
        async_cache.insert("key2", "value2");

        // Access key1 multiple times to increase frequency
        for _ in 0..5 {
            async_cache.get("key1");
        }

        // Insert key3 - should evict key2 (lower frequency)
        async_cache.insert("key3", "value3");

        // key1 should still be cached (high frequency)
        assert_eq!(async_cache.get("key1"), Some("value1"));
        // key2 should be evicted
        assert_eq!(async_cache.get("key2"), None);
        // key3 should be cached
        assert_eq!(async_cache.get("key3"), Some("value3"));
    }
}
