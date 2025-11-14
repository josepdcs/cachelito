#[cfg(feature = "stats")]
use crate::CacheStats;
use crate::EvictionPolicy;
use dashmap::DashMap;
use parking_lot::Mutex;
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
    ///     EvictionPolicy::LRU,
    ///     Some(300),
    /// );
    /// ```
    #[cfg(not(feature = "stats"))]
    pub fn new(
        cache: &'a DashMap<String, (R, u64, u64)>,
        order: &'a Mutex<VecDeque<String>>,
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
        policy: EvictionPolicy,
        ttl: Option<u64>,
        stats: &'a CacheStats,
    ) -> Self {
        Self {
            cache,
            order,
            limit,
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
                    EvictionPolicy::LRU => {
                        // LRU update happens after releasing the entry lock
                    }
                    EvictionPolicy::FIFO => {
                        // No update needed
                    }
                }

                drop(entry_ref);

                // Record cache hit
                #[cfg(feature = "stats")]
                self.stats.record_hit();

                // Update LRU order on cache hit (after releasing DashMap lock)
                if self.limit.is_some() && self.policy == EvictionPolicy::LRU {
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
    ///
    /// # Thread Safety
    ///
    /// This method uses locks to ensure consistency between the cache and
    /// the order queue. The order lock is held during eviction and insertion
    /// to prevent race conditions.
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

        // Handle limit and update order - acquire lock first to ensure atomicity
        if let Some(limit) = self.limit {
            let mut order = self.order.lock();

            // Check if another task already inserted this key while we were computing
            if self.cache.contains_key(key) {
                // Key already exists, just update the order if LRU
                if self.policy == EvictionPolicy::LRU {
                    order.retain(|k| k != key);
                    order.push_back(key.to_string());
                }
                // Don't insert again
                return;
            }

            // Check limit after acquiring lock to prevent race condition
            if self.cache.len() >= limit {
                match self.policy {
                    EvictionPolicy::LFU => {
                        // Find and evict the entry with minimum frequency
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

                        if let Some(evict_key) = min_freq_key {
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

            // Add the new entry to the order queue
            order.push_back(key.to_string());

            // Insert into cache with frequency initialized to 0
            self.cache.insert(key.to_string(), (value, timestamp, 0));
        } else {
            // No limit - just insert with frequency 0
            self.cache.insert(key.to_string(), (value, timestamp, 0));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_cache_basic() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(&cache, &order, None, EvictionPolicy::FIFO, None);

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache =
            AsyncGlobalCache::new(&cache, &order, None, EvictionPolicy::FIFO, None, &stats);

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
        let async_cache = AsyncGlobalCache::new(&cache, &order, Some(2), EvictionPolicy::LFU, None);

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache =
            AsyncGlobalCache::new(&cache, &order, Some(2), EvictionPolicy::LFU, None, &stats);

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
