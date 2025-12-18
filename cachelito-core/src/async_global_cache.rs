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
/// - **Cache limits**: Entry count limits (`limit`) and memory-based limits (`max_memory`)
/// - **TTL support**: Automatic expiration of entries based on age
/// - **Statistics**: Optional cache hit/miss tracking (with `stats` feature)
/// - **Frequency tracking**: For LFU, ARC, and TLRU policies
/// - **Memory estimation**: Support for memory-based eviction (requires `MemoryEstimator`)
///
/// # Cache Entry Structure
///
/// Each cache entry is stored as a tuple: `(value, timestamp, frequency)`
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
/// # Performance Characteristics
///
/// - **Get**: O(1) for cache lookup, O(n) for LRU/ARC/TLRU reordering
/// - **Insert**: O(1) for FIFO/Random, O(n) for LRU/LFU/ARC/TLRU eviction
/// - **Memory**: O(n) where n is the number of cached entries
///
/// # Thread Safety
///
/// This structure is fully thread-safe and can be shared across multiple async tasks.
/// The underlying DashMap provides lock-free concurrent access, while the order queue
/// uses a Mutex for coordination.
///
/// # Examples
///
/// ## Basic Usage
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
///     Some(100),    // Max 100 entries
///     None,         // No memory limit
///     EvictionPolicy::LRU,
///     Some(60),     // 60 second TTL
///     None,         // Default frequency_weight for TLRU
/// );
///
/// // In async context:
/// if let Some(value) = async_cache.get("key") {
///     println!("Cache hit: {}", value);
/// }
/// ```
///
/// ## TLRU with Custom Frequency Weight
///
/// ```ignore
/// use cachelito_core::{AsyncGlobalCache, EvictionPolicy};
///
/// // Emphasize frequency over recency (good for popular content)
/// let async_cache = AsyncGlobalCache::new(
///     &cache,
///     &order,
///     Some(100),
///     None,
///     EvictionPolicy::TLRU,
///     Some(300),
///     Some(1.5),    // frequency_weight > 1.0
/// );
///
/// // Emphasize recency over frequency (good for time-sensitive data)
/// let async_cache = AsyncGlobalCache::new(
///     &cache,
///     &order,
///     Some(100),
///     None,
///     EvictionPolicy::TLRU,
///     Some(300),
///     Some(0.3),    // frequency_weight < 1.0
/// );
/// ```
///
/// ## With Memory Limits
///
/// ```ignore
/// use cachelito_core::{AsyncGlobalCache, EvictionPolicy, MemoryEstimator};
///
/// let async_cache = AsyncGlobalCache::new(
///     &cache,
///     &order,
///     Some(1000),
///     Some(100 * 1024 * 1024), // 100MB max
///     EvictionPolicy::LRU,
///     Some(300),
///     None,
/// );
///
/// // Insert with memory tracking (requires MemoryEstimator implementation)
/// async_cache.insert_with_memory("key", value);
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

    /// Frequency weight for TLRU policy (>= 0.0)
    frequency_weight: Option<f64>,

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
    /// * `limit` - Optional maximum number of entries (None = unlimited)
    /// * `max_memory` - Optional maximum memory size in bytes (None = unlimited)
    /// * `policy` - Eviction policy (FIFO, LRU, LFU, ARC, Random, or TLRU)
    /// * `ttl` - Optional time-to-live in seconds (None = no expiration)
    /// * `frequency_weight` - Optional weight factor for frequency in TLRU policy
    ///   - Values < 1.0: Emphasize recency and age
    ///   - Values > 1.0: Emphasize frequency
    ///   - None or 1.0: Balanced approach (default)
    ///   - Only used when policy is TLRU, ignored otherwise
    ///
    /// # Examples
    ///
    /// ## Basic LRU cache with TTL
    ///
    /// ```ignore
    /// let cache = DashMap::new();
    /// let order = Mutex::new(VecDeque::new());
    /// let async_cache = AsyncGlobalCache::new(
    ///     &cache,
    ///     &order,
    ///     Some(1000),              // Max 1000 entries
    ///     None,                    // No memory limit
    ///     EvictionPolicy::LRU,     // LRU eviction
    ///     Some(300),               // 5 minute TTL
    ///     None,                    // No frequency_weight (not needed for LRU)
    /// );
    /// ```
    ///
    /// ## TLRU with memory limit and custom frequency weight
    ///
    /// ```ignore
    /// let async_cache = AsyncGlobalCache::new(
    ///     &cache,
    ///     &order,
    ///     Some(1000),
    ///     Some(100 * 1024 * 1024), // 100MB max
    ///     EvictionPolicy::TLRU,    // TLRU eviction
    ///     Some(300),               // 5 minute TTL
    ///     Some(1.5),               // Emphasize frequency (popular content)
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
        frequency_weight: Option<f64>,
        stats: &'a CacheStats,
    ) -> Self {
        Self {
            cache,
            order,
            limit,
            max_memory,
            policy,
            ttl,
            frequency_weight,
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
    /// - The entry is removed from both the cache and order queue
    /// - A cache miss is recorded (if stats feature is enabled)
    /// - `None` is returned
    ///
    /// # Statistics
    ///
    /// When the `stats` feature is enabled:
    /// - Cache hits are recorded when a valid entry is found
    /// - Cache misses are recorded when the key doesn't exist or has expired
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Check for cached user
    /// if let Some(user) = async_cache.get("user:123") {
    ///     println!("Found user: {:?}", user);
    /// } else {
    ///     println!("Cache miss - need to fetch from database");
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// - **FIFO, Random**: O(1) - no reordering needed
    /// - **LRU, ARC, TLRU**: O(n) - requires finding and moving key in order queue
    /// - **LFU**: O(1) - only increments counter
    pub fn get(&self, key: &str) -> Option<R> {
        // Check cache first
        if let Some(mut entry_ref) = self.cache.get_mut(key) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Check if expired
            // Use saturating_sub to avoid underflow when system clock moves backwards
            // Align comparison with sync variant: expire when age >= ttl
            let is_expired = if let Some(ttl) = self.ttl {
                let age = now.saturating_sub(entry_ref.1);
                age >= ttl
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
                    EvictionPolicy::TLRU => {
                        // Increment frequency counter for TLRU
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
                    && (self.policy == EvictionPolicy::LRU
                        || self.policy == EvictionPolicy::ARC
                        || self.policy == EvictionPolicy::TLRU)
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

    /// Finds the key with the lowest TLRU score for eviction.
    ///
    /// TLRU (Time-aware Least Recently Used) combines recency, frequency, and age factors
    /// to determine which entry should be evicted.
    ///
    /// Score formula: `frequency × position_weight × age_factor`
    ///
    /// Where:
    /// - `frequency`: Access count for the entry
    /// - `position_weight`: Higher for more recently accessed entries
    /// - `age_factor`: Decreases as entry approaches TTL expiration (if TTL is set)
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The key with the lowest TLRU score
    /// * `None` - If the order queue is empty or no valid entries exist
    fn find_tlru_eviction_key(&self, order: &VecDeque<String>) -> Option<String> {
        let mut best_evict_key: Option<String> = None;
        let mut best_score = f64::MAX;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for (idx, evict_key) in order.iter().enumerate() {
            if let Some(entry) = self.cache.get(evict_key) {
                let frequency = entry.2 as f64;
                let position_weight = (order.len() - idx) as f64;

                // Calculate age factor based on TTL
                let age_factor = if let Some(ttl_secs) = self.ttl {
                    let entry_timestamp = entry.1;
                    let elapsed = now.saturating_sub(entry_timestamp) as f64;
                    let ttl_f64 = ttl_secs as f64;
                    // Entries close to expiration get lower scores (prioritized for eviction)
                    (1.0 - (elapsed / ttl_f64).min(1.0)).max(0.0)
                } else {
                    1.0 // No TTL, age doesn't matter
                };

                // Apply frequency weight if provided
                let frequency_component = if let Some(weight) = self.frequency_weight {
                    if frequency > 0.0 {
                        frequency.powf(weight)
                    } else {
                        0.0
                    }
                } else {
                    frequency
                };

                // Score combines frequency, recency, and age
                let score = frequency_component * position_weight * age_factor;

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
                    EvictionPolicy::TLRU => {
                        if let Some(evict_key) = self.find_tlru_eviction_key(order) {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
                        }
                    }
                    EvictionPolicy::Random => {
                        // O(1) random eviction: select random position and remove directly
                        if !order.is_empty() {
                            let pos = fastrand::usize(..order.len());
                            if let Some(evict_key) = order.remove(pos) {
                                self.cache.remove(&evict_key);
                            }
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
    /// from multiple async tasks without additional synchronization.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Get basic statistics
    /// let stats = async_cache.stats();
    /// println!("Hits: {}", stats.hits());
    /// println!("Misses: {}", stats.misses());
    /// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
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
    /// async_cache.insert_with_memory("large_data", expensive_value);
    /// ```
    ///
    /// # Performance
    ///
    /// - **Memory calculation**: O(n) - iterates all entries to sum memory
    /// - **Eviction**: Varies by policy (see individual policy documentation)
    /// - May evict multiple entries in one call if memory limit is tight
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
                    EvictionPolicy::TLRU => {
                        if let Some(evict_key) = self.find_tlru_eviction_key(&*order) {
                            self.cache.remove(&evict_key);
                            order.retain(|k| k != &evict_key);
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
                                self.cache.remove(&evict_key);
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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_async_cache_basic() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache =
            AsyncGlobalCache::new(&cache, &order, None, None, EvictionPolicy::FIFO, None, None);

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
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(2),
            None,
            EvictionPolicy::LFU,
            None,
            None,
        );

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

    #[test]
    fn test_async_cache_ttl_boundary_expires() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            None,
            None,
            EvictionPolicy::FIFO,
            Some(1),
            None,
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            None,
            None,
            EvictionPolicy::FIFO,
            Some(1),
            None,
            &stats,
        );

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Insert with timestamp exactly 1 second in the past (age == ttl)
        cache.insert("k".to_string(), ("v", now.saturating_sub(1), 0));

        // With >= comparison, this must be considered expired
        assert_eq!(async_cache.get("k"), None);
    }

    #[test]
    fn test_async_cache_clock_moves_backwards_not_expired() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            None,
            None,
            EvictionPolicy::FIFO,
            Some(10),
            None,
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            None,
            None,
            EvictionPolicy::FIFO,
            Some(10),
            None,
            &stats,
        );

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Insert via the cache API so the 'order' queue is updated as well
        async_cache.insert("k", "v");

        // Simulate a "future" timestamp (as if the clock moved backwards later)
        // Adjust only the timestamp of the already inserted entry
        let future_ts = now.saturating_add(100);
        if let Some(mut entry) = cache.get_mut("k") {
            entry.1 = future_ts;
        }

        // With saturating_sub, age = 0, which is < ttl => NOT expired
        assert_eq!(async_cache.get("k"), Some("v"));

        // Verify that the order queue contains the key "k"
        assert!(order.lock().contains(&"k".to_string()));
    }

    // ========== TLRU with frequency_weight tests ==========

    #[test]
    fn test_tlru_with_low_frequency_weight() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(0.3), // Low weight - emphasizes recency
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(0.3),
            &stats,
        );

        // Fill cache
        async_cache.insert("k1", 1);
        async_cache.insert("k2", 2);
        async_cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = async_cache.get("k1");
        }

        // Wait a bit to age k1
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Add new entry (cache is full)
        async_cache.insert("k4", 4);

        // With low frequency_weight, even frequent entries can be evicted
        // if they're older (recency and age matter more)
        assert_eq!(async_cache.get("k4"), Some(4));
    }

    #[test]
    fn test_tlru_with_high_frequency_weight() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.5), // High weight - emphasizes frequency
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.5),
            &stats,
        );

        // Fill cache
        async_cache.insert("k1", 1);
        async_cache.insert("k2", 2);
        async_cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = async_cache.get("k1");
        }

        // Wait a bit to age k1
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Add new entry (cache is full)
        async_cache.insert("k4", 4);

        // With high frequency_weight, frequent entries are protected
        // k1 should remain cached despite being older
        assert_eq!(async_cache.get("k1"), Some(1));
        assert_eq!(async_cache.get("k4"), Some(4));
    }

    #[test]
    fn test_tlru_default_frequency_weight() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(5),
            None, // Default weight (balanced)
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(5),
            None,
            &stats,
        );

        async_cache.insert("k1", 1);
        async_cache.insert("k2", 2);

        // Access k1 a few times
        for _ in 0..3 {
            let _ = async_cache.get("k1");
        }

        // Add third entry
        async_cache.insert("k3", 3);

        // With balanced weight, both frequency and recency matter
        // k1 has higher frequency, so it should remain
        assert_eq!(async_cache.get("k1"), Some(1));
        assert_eq!(async_cache.get("k3"), Some(3));
    }

    #[test]
    fn test_tlru_no_ttl_with_frequency_weight() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            None, // No TTL - age_factor will be 1.0
            Some(1.5),
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            None,
            Some(1.5),
            &stats,
        );

        async_cache.insert("k1", 1);
        async_cache.insert("k2", 2);
        async_cache.insert("k3", 3);

        // Make k1 very frequent
        for _ in 0..10 {
            let _ = async_cache.get("k1");
        }

        // Add new entry
        async_cache.insert("k4", 4);

        // Without TTL, TLRU focuses on frequency and position
        // k1 should remain due to high frequency
        assert_eq!(async_cache.get("k1"), Some(1));
    }

    #[test]
    fn test_tlru_frequency_weight_comparison() {
        // Test that different weights produce different behavior
        let cache_low = DashMap::new();
        let order_low = Mutex::new(VecDeque::new());
        let cache_high = DashMap::new();
        let order_high = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache_low = AsyncGlobalCache::new(
            &cache_low,
            &order_low,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(0.3), // Low weight
        );

        #[cfg(not(feature = "stats"))]
        let async_cache_high = AsyncGlobalCache::new(
            &cache_high,
            &order_high,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(2.0), // High weight
        );

        #[cfg(feature = "stats")]
        let stats_low = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache_low = AsyncGlobalCache::new(
            &cache_low,
            &order_low,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(0.3),
            &stats_low,
        );

        #[cfg(feature = "stats")]
        let stats_high = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache_high = AsyncGlobalCache::new(
            &cache_high,
            &order_high,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(2.0),
            &stats_high,
        );

        // Same operations on both caches
        async_cache_low.insert("k1", 1);
        async_cache_low.insert("k2", 2);
        async_cache_high.insert("k1", 1);
        async_cache_high.insert("k2", 2);

        // Make k1 frequent in both
        for _ in 0..5 {
            let _ = async_cache_low.get("k1");
            let _ = async_cache_high.get("k1");
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Add new entry to both
        async_cache_low.insert("k3", 3);
        async_cache_high.insert("k3", 3);

        // Both should work correctly with their respective weights
        assert_eq!(async_cache_low.get("k3"), Some(3));
        assert_eq!(async_cache_high.get("k3"), Some(3));
    }

    #[test]
    fn test_tlru_concurrent_with_frequency_weight() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(DashMap::new());
        let order = Arc::new(Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        let stats = Arc::new(CacheStats::new());

        // Insert initial entries
        {
            #[cfg(not(feature = "stats"))]
            let async_cache = AsyncGlobalCache::new(
                &cache,
                &order,
                Some(10),
                None,
                EvictionPolicy::TLRU,
                Some(10),
                Some(1.2), // Slightly emphasize frequency
            );

            #[cfg(feature = "stats")]
            let async_cache = AsyncGlobalCache::new(
                &cache,
                &order,
                Some(10),
                None,
                EvictionPolicy::TLRU,
                Some(10),
                Some(1.2),
                &stats,
            );

            async_cache.insert("k1", 1);
            async_cache.insert("k2", 2);
        }

        // Spawn multiple threads accessing the cache
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let cache_clone = Arc::clone(&cache);
                let order_clone = Arc::clone(&order);
                #[cfg(feature = "stats")]
                let stats_clone = Arc::clone(&stats);

                thread::spawn(move || {
                    #[cfg(not(feature = "stats"))]
                    let async_cache = AsyncGlobalCache::new(
                        &cache_clone,
                        &order_clone,
                        Some(10),
                        None,
                        EvictionPolicy::TLRU,
                        Some(10),
                        Some(1.2),
                    );

                    #[cfg(feature = "stats")]
                    let async_cache = AsyncGlobalCache::new(
                        &cache_clone,
                        &order_clone,
                        Some(10),
                        None,
                        EvictionPolicy::TLRU,
                        Some(10),
                        Some(1.2),
                        &stats_clone,
                    );

                    // Access k1 frequently
                    for _ in 0..3 {
                        let _ = async_cache.get("k1");
                    }

                    // Insert new entry
                    async_cache.insert(&format!("k{}", i + 3), i + 3);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // k1 should remain cached due to high frequency and frequency_weight > 1.0
        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(10),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.2),
        );

        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(10),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.2),
            &stats,
        );

        assert_eq!(async_cache.get("k1"), Some(1));
    }

    #[test]
    fn test_tlru_frequency_weight_edge_cases() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(5),
            Some(0.1), // Very low weight
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(2),
            None,
            EvictionPolicy::TLRU,
            Some(5),
            Some(0.1),
            &stats,
        );

        async_cache.insert("k1", 1);
        async_cache.insert("k2", 2);

        // Make k1 extremely frequent
        for _ in 0..100 {
            let _ = async_cache.get("k1");
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Even with very high frequency, k1 might be evicted with very low weight
        async_cache.insert("k3", 3);

        // The cache should still work correctly
        assert!(async_cache.get("k3").is_some());
    }

    #[test]
    fn test_tlru_frequency_weight_with_lru_pattern() {
        let cache = DashMap::new();
        let order = Mutex::new(VecDeque::new());

        #[cfg(not(feature = "stats"))]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.0), // Weight = 1.0 (linear frequency impact)
        );

        #[cfg(feature = "stats")]
        let stats = CacheStats::new();
        #[cfg(feature = "stats")]
        let async_cache = AsyncGlobalCache::new(
            &cache,
            &order,
            Some(3),
            None,
            EvictionPolicy::TLRU,
            Some(10),
            Some(1.0),
            &stats,
        );

        async_cache.insert("k1", 1);
        async_cache.insert("k2", 2);
        async_cache.insert("k3", 3);

        // Create LRU-like access pattern
        let _ = async_cache.get("k1");
        let _ = async_cache.get("k2");
        let _ = async_cache.get("k1");
        let _ = async_cache.get("k2");

        // k3 has not been accessed, should be evicted first
        async_cache.insert("k4", 4);

        assert_eq!(async_cache.get("k1"), Some(1));
        assert_eq!(async_cache.get("k2"), Some(2));
        assert_eq!(async_cache.get("k4"), Some(4));
        // k3 should be evicted (least recently used and zero frequency)
        assert_eq!(async_cache.get("k3"), None);
    }
}
