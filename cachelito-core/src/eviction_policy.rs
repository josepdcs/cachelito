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
/// * `LRU` - **Least Recently Used** eviction policy
///   - Elements are evicted based on when they were last accessed
///   - The least recently accessed element is removed first
///   - Accessing a cached value moves it to the "most recent" position
///   - Better for workloads with temporal locality
///   - O(n) overhead on cache hits for reordering
///
/// # Examples
///
/// ```
/// use cachelito_core::EvictionPolicy;
///
/// // Creating policies
/// let fifo = EvictionPolicy::FIFO;
/// let lru = EvictionPolicy::LRU;
///
/// // Using default (FIFO)
/// let default_policy = EvictionPolicy::default();
/// assert_eq!(default_policy, EvictionPolicy::FIFO);
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
}

impl EvictionPolicy {
    /// Returns the default eviction policy (FIFO).
    ///
    /// FIFO is chosen as the default because:
    /// - Simple and predictable behavior
    /// - Lower overhead (O(1) for all operations)
    /// - No additional bookkeeping on cache hits
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::EvictionPolicy;
    ///
    /// let default = EvictionPolicy::default();
    /// assert_eq!(default, EvictionPolicy::FIFO);
    /// ```
    pub const fn default() -> Self {
        EvictionPolicy::FIFO
    }
}

/// Converts a string slice to an `EvictionPolicy`.
///
/// The conversion is case-insensitive and defaults to FIFO for unrecognized values.
///
/// # Supported Values
///
/// - `"fifo"` or `"FIFO"` → `EvictionPolicy::FIFO`
/// - `"lru"` or `"LRU"` → `EvictionPolicy::LRU`
/// - Any other value → `EvictionPolicy::FIFO` (default)
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
/// let unknown: EvictionPolicy = "random".into();
/// assert_eq!(unknown, EvictionPolicy::FIFO); // defaults to FIFO
/// ```
impl From<&str> for EvictionPolicy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lru" => EvictionPolicy::LRU,
            _ => EvictionPolicy::FIFO,
        }
    }
}

impl PartialEq for EvictionPolicy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EvictionPolicy::FIFO, EvictionPolicy::FIFO) => true,
            (EvictionPolicy::LRU, EvictionPolicy::LRU) => true,
            _ => false,
        }
    }
}
