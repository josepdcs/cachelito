use std::cmp::PartialEq;

/// Represents the policy used for evicting elements from a cache when it reaches its limit.
///
/// Eviction policies determine which cached entry should be removed when the cache is full
/// and a new entry needs to be added.
///
/// # Variants
///
/// * `FIFO` - **First In, First Out** eviction policy
///   - Elements are evicted in the order they were added
///   - The oldest inserted element is removed first
///   - Accessing a cached value does NOT change its position
///   - Simple and predictable behavior
///   - O(1) eviction performance
///
/// * `LRU` - **Least Recently Used** eviction policy (default)
///   - Elements are evicted based on when they were last accessed
///   - The least recently accessed element is removed first
///   - Accessing a cached value moves it to the "most recent" position
///   - Better for workloads with temporal locality
///   - O(n) overhead on cache hits for reordering
///
/// * `LFU` - **Least Frequently Used** eviction policy
///   - Elements are evicted based on access frequency
///   - The least frequently accessed element is removed first
///   - Each cache hit increments the frequency counter
///   - Better for workloads where popular items should stay cached
///   - O(n) overhead on eviction to find minimum frequency
///
/// * `ARC` - **Adaptive Replacement Cache (Hybrid LRU/LFU)** eviction policy
///   - Hybrid policy that combines recency (LRU) and frequency (LFU) using a scoring function
///   - Uses a single order queue with a score: `frequency × position_weight`
///   - Not a full implementation of the classic ARC algorithm (no T1/T2/B1/B2 lists or self-tuning parameter)
///   - Provides a balance between LRU and LFU for mixed workloads
///   - O(n) operations for some cache operations due to scoring and reordering
///
///
/// # Examples
///
/// ```
/// use cachelito_core::EvictionPolicy;
///
/// // Creating policies
/// let fifo = EvictionPolicy::FIFO;
/// let lru = EvictionPolicy::LRU;
/// let lfu = EvictionPolicy::LFU;
/// let arc = EvictionPolicy::ARC;
///
/// // Using default (LRU)
/// let default_policy = EvictionPolicy::default();
/// assert_eq!(default_policy, EvictionPolicy::LRU);
///
/// // Converting from string
/// let policy: EvictionPolicy = "lru".into();
/// assert_eq!(policy, EvictionPolicy::LRU);
/// ```
///
/// # Performance Characteristics
///
/// | Policy | Eviction | Cache Hit | Cache Miss | Use Case |
/// |--------|----------|-----------|------------|----------|
/// | FIFO   | O(1)     | O(1)      | O(1)       | Simple, predictable caching |
/// | LRU    | O(1)     | O(n)      | O(1)       | Workloads with temporal locality |
/// | LFU    | O(n)     | O(1)      | O(1)       | Workloads with frequency patterns |
/// | ARC    | O(n)     | O(n)      | O(1)       | Mixed workloads, self-tuning |
///
/// # Derives
///
/// This enum derives the following traits:
///
/// * `Clone` - Enables the creation of a duplicate `EvictionPolicy` value
/// * `Copy` - Allows `EvictionPolicy` values to be duplicated by simple assignment
/// * `Debug` - Provides a human-readable string representation for debugging
/// * `PartialEq` - Enables equality comparison between policies
#[derive(Clone, Copy, Debug)]
pub enum EvictionPolicy {
    FIFO,
    LRU,
    LFU,
    ARC,
}

impl EvictionPolicy {
    /// Returns the default eviction policy (LRU).
    ///
    /// LRU is chosen as the default because:
    /// - Good balance between simplicity and effectiveness
    /// - Works well for most caching scenarios with temporal locality
    /// - Commonly expected behavior for caches
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::EvictionPolicy;
    ///
    /// let default = EvictionPolicy::default();
    /// assert_eq!(default, EvictionPolicy::LRU);
    /// ```
    pub const fn default() -> Self {
        EvictionPolicy::LRU
    }

    /// Returns true if the given string is a valid eviction policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::EvictionPolicy;
    ///
    /// assert!(EvictionPolicy::is_valid("fifo"));
    /// assert!(EvictionPolicy::is_valid("LRU"));
    /// assert!(!EvictionPolicy::is_valid("random"));
    /// assert!(EvictionPolicy::is_valid("lfu"));
    /// assert!(EvictionPolicy::is_valid("arc"));
    /// ```
    pub fn is_valid(p: &str) -> bool {
        matches!(p.to_lowercase().as_str(), "fifo" | "lru" | "lfu" | "arc")
    }
}

/// Converts a string slice to an `EvictionPolicy`.
///
/// The conversion is case-insensitive and defaults to LRU for unrecognized values.
///
/// # Supported Values
///
/// - `"fifo"` or `"FIFO"` → `EvictionPolicy::FIFO`
/// - `"lru"` or `"LRU"` → `EvictionPolicy::LRU`
/// - `"lfu"` or `"LFU"` → `EvictionPolicy::LFU`
/// - `"arc"` or `"ARC"` → `EvictionPolicy::ARC`
/// - Any other value → `EvictionPolicy::LRU` (default)
///
/// # Examples
///
/// ```
/// use cachelito_core::EvictionPolicy;
///
/// let fifo: EvictionPolicy = "fifo".into();
/// assert_eq!(fifo, EvictionPolicy::FIFO);
///
/// let lru: EvictionPolicy = "LRU".into();
/// assert_eq!(lru, EvictionPolicy::LRU);
///
/// let lfu: EvictionPolicy = "lfu".into();
/// assert_eq!(lfu, EvictionPolicy::LFU);
///
/// let arc: EvictionPolicy = "arc".into();
/// assert_eq!(arc, EvictionPolicy::ARC);
///
/// let unknown: EvictionPolicy = "random".into();
/// assert_eq!(unknown, EvictionPolicy::LRU); // defaults to LRU
/// ```
impl From<&str> for EvictionPolicy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fifo" => EvictionPolicy::FIFO,
            "lfu" => EvictionPolicy::LFU,
            "arc" => EvictionPolicy::ARC,
            _ => EvictionPolicy::LRU,
        }
    }
}

impl PartialEq for EvictionPolicy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EvictionPolicy::FIFO, EvictionPolicy::FIFO) => true,
            (EvictionPolicy::LRU, EvictionPolicy::LRU) => true,
            (EvictionPolicy::LFU, EvictionPolicy::LFU) => true,
            (EvictionPolicy::ARC, EvictionPolicy::ARC) => true,
            _ => false,
        }
    }
}
