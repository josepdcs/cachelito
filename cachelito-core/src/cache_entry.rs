use std::time::Instant;

/// Internal wrapper that tracks when a value was inserted into the cache.
/// Used for TTL expiration support.
///
/// This structure is used internally to support TTL (Time To Live) expiration.
/// Each cached value is wrapped in a `CacheEntry` which records the insertion
/// timestamp using `Instant::now()`.
///
/// # Type Parameters
///
/// * `R` - The type of the cached value
///
/// # Fields
///
/// * `value` - The actual cached value
/// * `inserted_at` - The `Instant` when this entry was created
/// * `frequency` - The number of times this entry has been accessed (for LFU policy)
///
/// # Examples
///
/// ```
/// use cachelito_core::CacheEntry;
///
/// let entry = CacheEntry::new(42);
/// assert_eq!(entry.value, 42);
/// assert_eq!(entry.frequency, 0);
///
/// // Check if expired (TTL of 60 seconds)
/// assert!(!entry.is_expired(Some(60)));
/// ```
#[derive(Clone)]
pub struct CacheEntry<R> {
    pub value: R,
    pub inserted_at: Instant,
    pub frequency: u64,
}

impl<R> CacheEntry<R> {
    /// Creates a new cache entry with the current timestamp.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to cache
    ///
    /// # Returns
    ///
    /// A new `CacheEntry` with `inserted_at` set to `Instant::now()` and `frequency` set to 0
    pub fn new(value: R) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
            frequency: 0,
        }
    }

    /// Returns true if the entry has expired based on the provided TTL.
    ///
    /// # Arguments
    ///
    /// * `ttl` - Optional time-to-live in seconds. `None` means no expiration.
    ///
    /// # Returns
    ///
    /// * `true` if the entry age exceeds the TTL
    /// * `false` if TTL is `None` or the entry is still valid
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheEntry;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let entry = CacheEntry::new("data");
    ///
    /// // Fresh entry is not expired
    /// assert!(!entry.is_expired(Some(1)));
    ///
    /// // Wait 2 seconds
    /// thread::sleep(Duration::from_secs(2));
    ///
    /// // Now it's expired (TTL was 1 second)
    /// assert!(entry.is_expired(Some(1)));
    ///
    /// // No TTL means never expires
    /// assert!(!entry.is_expired(None));
    /// ```
    pub fn is_expired(&self, ttl: Option<u64>) -> bool {
        if let Some(ttl_secs) = ttl {
            self.inserted_at.elapsed().as_secs() >= ttl_secs
        } else {
            false
        }
    }

    /// Increments the access frequency counter.
    ///
    /// This method is used by the LFU (Least Frequently Used) eviction policy
    /// to track how many times an entry has been accessed.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheEntry;
    ///
    /// let mut entry = CacheEntry::new(42);
    /// assert_eq!(entry.frequency, 0);
    ///
    /// entry.increment_frequency();
    /// assert_eq!(entry.frequency, 1);
    ///
    /// entry.increment_frequency();
    /// assert_eq!(entry.frequency, 2);
    /// ```
    pub fn increment_frequency(&mut self) {
        self.frequency = self.frequency.saturating_add(1);
    }
}

// Implement MemoryEstimator for CacheEntry
use crate::MemoryEstimator;

impl<R: MemoryEstimator> MemoryEstimator for CacheEntry<R> {
    fn estimate_memory(&self) -> usize {
        // Base struct size (stack-allocated data: Instant + u64)
        let base = std::mem::size_of::<Self>();

        // Size of the value (including heap allocations)
        let value_size = self.value.estimate_memory();

        // Note: We subtract size_of_val(&self.value) to avoid double-counting
        // the stack-allocated part which is already included in `base`
        base + value_size.saturating_sub(std::mem::size_of_val(&self.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_entry_not_expired() {
        let entry = CacheEntry::new(42);
        assert_eq!(entry.value, 42);
        assert!(!entry.is_expired(Some(10)));
    }

    #[test]
    fn test_entry_expiration() {
        let entry = CacheEntry::new("data");
        thread::sleep(Duration::from_secs(2));
        assert!(entry.is_expired(Some(1)));
        assert!(!entry.is_expired(Some(3)));
    }

    #[test]
    fn test_no_ttl_never_expires() {
        let entry = CacheEntry::new(100);
        thread::sleep(Duration::from_millis(100));
        assert!(!entry.is_expired(None));
    }

    #[test]
    fn test_memory_estimation_primitive() {
        let entry = CacheEntry::new(42i32);
        let estimated = entry.estimate_memory();

        // Should be approximately: size_of::<CacheEntry<i32>>()
        // (Instant + u64 + i32)
        assert!(estimated >= std::mem::size_of::<CacheEntry<i32>>());
    }

    #[test]
    fn test_memory_estimation_string() {
        let s = String::from("Hello, World!");
        let entry = CacheEntry::new(s.clone());
        let estimated = entry.estimate_memory();

        // Should include struct overhead + string capacity
        let expected_min = std::mem::size_of::<CacheEntry<String>>() + s.capacity();
        assert!(estimated >= expected_min);
    }

    #[test]
    fn test_memory_estimation_vec() {
        let v = vec![1, 2, 3, 4, 5];
        let entry = CacheEntry::new(v.clone());
        let estimated = entry.estimate_memory();

        // Should include struct overhead + vec capacity
        let expected_min =
            std::mem::size_of::<CacheEntry<Vec<i32>>>() + v.capacity() * std::mem::size_of::<i32>();
        assert!(estimated >= expected_min);
    }
}
