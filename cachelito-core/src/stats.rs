use std::sync::atomic::{AtomicU64, Ordering};

/// Cache statistics for monitoring hit/miss rates and performance.
///
/// This structure tracks cache access patterns using atomic operations for
/// thread-safe statistics collection with minimal overhead.
///
/// # Thread Safety
///
/// All operations are thread-safe using atomic operations with `Relaxed` ordering,
/// which provides the best performance while still maintaining consistency.
///
/// # Examples
///
/// ```
/// use cachelito_core::CacheStats;
///
/// let stats = CacheStats::new();
///
/// // Simulate cache operations
/// stats.record_hit();
/// stats.record_hit();
/// stats.record_miss();
///
/// assert_eq!(stats.hits(), 2);
/// assert_eq!(stats.misses(), 1);
/// assert_eq!(stats.total_accesses(), 3);
/// assert!((stats.hit_rate() - 0.6666).abs() < 0.001);
/// ```
#[derive(Debug)]
pub struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
}

impl CacheStats {
    /// Creates a new `CacheStats` instance with zero counters.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// assert_eq!(stats.hits(), 0);
    /// assert_eq!(stats.misses(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Records a cache hit (successful lookup).
    ///
    /// This method is called internally when a cache lookup finds a valid entry.
    /// Uses atomic operations for thread-safety with minimal overhead.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_hit();
    /// assert_eq!(stats.hits(), 1);
    /// ```
    #[inline]
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss (failed lookup).
    ///
    /// This method is called internally when a cache lookup doesn't find an entry
    /// or finds an expired entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_miss();
    /// assert_eq!(stats.misses(), 1);
    /// ```
    #[inline]
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the total number of cache hits.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_hit();
    /// stats.record_hit();
    /// assert_eq!(stats.hits(), 2);
    /// ```
    #[inline]
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Returns the total number of cache misses.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_miss();
    /// stats.record_miss();
    /// stats.record_miss();
    /// assert_eq!(stats.misses(), 3);
    /// ```
    #[inline]
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Returns the total number of cache accesses (hits + misses).
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_hit();
    /// stats.record_miss();
    /// stats.record_hit();
    /// assert_eq!(stats.total_accesses(), 3);
    /// ```
    #[inline]
    pub fn total_accesses(&self) -> u64 {
        self.hits() + self.misses()
    }

    /// Calculates and returns the cache hit rate as a fraction (0.0 to 1.0).
    ///
    /// The hit rate is the ratio of successful lookups to total lookups.
    /// Returns 0.0 if there have been no accesses.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_hit();
    /// stats.record_hit();
    /// stats.record_miss();
    ///
    /// // 2 hits out of 3 total = 0.6666...
    /// assert!((stats.hit_rate() - 0.6666).abs() < 0.001);
    /// ```
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_accesses();
        if total == 0 {
            0.0
        } else {
            self.hits() as f64 / total as f64
        }
    }

    /// Calculates and returns the cache miss rate as a fraction (0.0 to 1.0).
    ///
    /// The miss rate is the ratio of failed lookups to total lookups.
    /// Returns 0.0 if there have been no accesses.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_hit();
    /// stats.record_miss();
    /// stats.record_miss();
    ///
    /// // 2 misses out of 3 total = 0.6666...
    /// assert!((stats.miss_rate() - 0.6666).abs() < 0.001);
    /// ```
    #[inline]
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// Resets all statistics counters to zero.
    ///
    /// This can be useful for measuring statistics over specific time periods
    /// or after configuration changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// stats.record_hit();
    /// stats.record_miss();
    /// assert_eq!(stats.total_accesses(), 2);
    ///
    /// stats.reset();
    /// assert_eq!(stats.total_accesses(), 0);
    /// assert_eq!(stats.hits(), 0);
    /// assert_eq!(stats.misses(), 0);
    /// ```
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CacheStats {
    fn clone(&self) -> Self {
        Self {
            hits: AtomicU64::new(self.hits()),
            misses: AtomicU64::new(self.misses()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stats() {
        let stats = CacheStats::new();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
        assert_eq!(stats.total_accesses(), 0);
    }

    #[test]
    fn test_record_hit() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        assert_eq!(stats.hits(), 2);
        assert_eq!(stats.misses(), 0);
    }

    #[test]
    fn test_record_miss() {
        let stats = CacheStats::new();
        stats.record_miss();
        stats.record_miss();
        stats.record_miss();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 3);
    }

    #[test]
    fn test_total_accesses() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        assert_eq!(stats.total_accesses(), 3);
    }

    #[test]
    fn test_hit_rate() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        assert!((stats.hit_rate() - 0.6666).abs() < 0.001);
    }

    #[test]
    fn test_miss_rate() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_miss();
        stats.record_miss();
        assert!((stats.miss_rate() - 0.6666).abs() < 0.001);
    }

    #[test]
    fn test_hit_rate_no_accesses() {
        let stats = CacheStats::new();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.miss_rate(), 1.0);
    }

    #[test]
    fn test_reset() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        assert_eq!(stats.total_accesses(), 3);

        stats.reset();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
        assert_eq!(stats.total_accesses(), 0);
    }

    #[test]
    fn test_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
    }

    #[test]
    fn test_clone() {
        let stats = CacheStats::new();
        stats.record_hit();
        stats.record_miss();

        let cloned = stats.clone();
        assert_eq!(cloned.hits(), stats.hits());
        assert_eq!(cloned.misses(), stats.misses());

        // Ensure they're independent
        stats.record_hit();
        assert_eq!(stats.hits(), 2);
        assert_eq!(cloned.hits(), 1);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let stats = Arc::new(CacheStats::new());
        let mut handles = vec![];

        // Spawn 10 threads that each record 100 hits and 50 misses
        for _ in 0..10 {
            let stats_clone = Arc::clone(&stats);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    stats_clone.record_hit();
                }
                for _ in 0..50 {
                    stats_clone.record_miss();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to finish
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify totals: 10 threads * 100 hits = 1000, 10 threads * 50 misses = 500
        assert_eq!(stats.hits(), 1000);
        assert_eq!(stats.misses(), 500);
        assert_eq!(stats.total_accesses(), 1500);
        assert!((stats.hit_rate() - 0.6666).abs() < 0.001);
    }
}
