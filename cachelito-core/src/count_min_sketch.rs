use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Count-Min Sketch - Probabilistic data structure for frequency estimation.
///
/// This implementation provides approximate frequency counting with configurable
/// accuracy and memory efficiency. It's used in W-TinyLFU for admission control.
///
/// # Algorithm
///
/// The Count-Min Sketch uses multiple hash functions and maintains a 2D array of counters.
/// When incrementing or querying a key:
/// 1. Hash the key with each hash function to get `depth` positions
/// 2. For increment: increment all counters at those positions
/// 3. For query: return the minimum value among all positions
///
/// # Memory Usage
///
/// Total memory: `width × depth × size_of::<u32>()` bytes
///
/// # Examples
///
/// ```
/// use cachelito_core::CountMinSketch;
///
/// let mut sketch = CountMinSketch::new(2048, 4);
/// sketch.increment(&"key1");
/// sketch.increment(&"key1");
/// sketch.increment(&"key2");
///
/// assert_eq!(sketch.estimate(&"key1"), 2);
/// assert_eq!(sketch.estimate(&"key2"), 1);
/// assert_eq!(sketch.estimate(&"key3"), 0);
/// ```
#[derive(Clone, Debug)]
pub struct CountMinSketch {
    /// 2D array of counters: [depth][width]
    counters: Vec<Vec<u32>>,
    /// Number of hash functions (rows)
    depth: usize,
    /// Size of each counter array (columns)
    width: usize,
    /// Total increments (for decay)
    total_count: u64,
}

impl CountMinSketch {
    /// Creates a new Count-Min Sketch with the specified dimensions.
    ///
    /// # Parameters
    ///
    /// * `width` - Number of buckets per hash function (columns). Larger values reduce collision probability.
    /// * `depth` - Number of hash functions (rows). Typical values: 4-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// // Create sketch with 2048 buckets and 4 hash functions
    /// let sketch = CountMinSketch::new(2048, 4);
    /// ```
    pub fn new(width: usize, depth: usize) -> Self {
        Self {
            counters: vec![vec![0u32; width]; depth],
            depth,
            width,
            total_count: 0,
        }
    }

    /// Increments the frequency counter for the given key.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to increment
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(1024, 4);
    /// sketch.increment(&"my_key");
    /// assert_eq!(sketch.estimate(&"my_key"), 1);
    /// ```
    pub fn increment<K: Hash>(&mut self, key: &K) {
        for i in 0..self.depth {
            let index = self.hash(key, i);
            self.counters[i][index] = self.counters[i][index].saturating_add(1);
        }
        self.total_count = self.total_count.saturating_add(1);
    }

    /// Estimates the frequency of the given key.
    ///
    /// Returns the minimum count across all hash functions, which is the
    /// conservative estimate (guarantees we never underestimate).
    ///
    /// # Parameters
    ///
    /// * `key` - The key to query
    ///
    /// # Returns
    ///
    /// The estimated frequency (minimum of all counters)
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(1024, 4);
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    ///
    /// assert_eq!(sketch.estimate(&"key1"), 3);
    /// ```
    pub fn estimate<K: Hash>(&self, key: &K) -> u32 {
        let mut min_count = u32::MAX;
        for i in 0..self.depth {
            let index = self.hash(key, i);
            min_count = min_count.min(self.counters[i][index]);
        }
        min_count
    }

    /// Decays all counters by dividing them by 2.
    ///
    /// This is called periodically to:
    /// - Prevent counter saturation
    /// - Give more weight to recent accesses
    /// - Adapt to changing access patterns
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(1024, 4);
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    ///
    /// assert_eq!(sketch.estimate(&"key1"), 4);
    ///
    /// sketch.decay();
    /// assert_eq!(sketch.estimate(&"key1"), 2);
    ///
    /// sketch.decay();
    /// assert_eq!(sketch.estimate(&"key1"), 1);
    /// ```
    pub fn decay(&mut self) {
        for row in &mut self.counters {
            for counter in row.iter_mut() {
                *counter /= 2;
            }
        }
        self.total_count /= 2;
    }

    /// Resets all counters to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(1024, 4);
    /// sketch.increment(&"key1");
    /// assert_eq!(sketch.estimate(&"key1"), 1);
    ///
    /// sketch.reset();
    /// assert_eq!(sketch.estimate(&"key1"), 0);
    /// ```
    pub fn reset(&mut self) {
        for row in &mut self.counters {
            for counter in row.iter_mut() {
                *counter = 0;
            }
        }
        self.total_count = 0;
    }

    /// Returns the total number of increments performed.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(1024, 4);
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key2");
    /// sketch.increment(&"key1");
    ///
    /// assert_eq!(sketch.total_count(), 3);
    /// ```
    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    /// Hash function that generates a bucket index for a given key and hash function index.
    ///
    /// Uses DefaultHasher with different seeds for each hash function.
    fn hash<K: Hash>(&self, key: &K, hash_index: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        // Use hash_index as a seed to generate different hash functions
        hash_index.hash(&mut hasher);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_increment_and_estimate() {
        let mut sketch = CountMinSketch::new(1024, 4);

        sketch.increment(&"key1");
        assert_eq!(sketch.estimate(&"key1"), 1);

        sketch.increment(&"key1");
        assert_eq!(sketch.estimate(&"key1"), 2);

        sketch.increment(&"key2");
        assert_eq!(sketch.estimate(&"key2"), 1);
        assert_eq!(sketch.estimate(&"key1"), 2);
    }

    #[test]
    fn test_estimate_nonexistent_key() {
        let sketch = CountMinSketch::new(1024, 4);
        assert_eq!(sketch.estimate(&"nonexistent"), 0);
    }

    #[test]
    fn test_decay() {
        let mut sketch = CountMinSketch::new(1024, 4);

        sketch.increment(&"key1");
        sketch.increment(&"key1");
        sketch.increment(&"key1");
        sketch.increment(&"key1");

        assert_eq!(sketch.estimate(&"key1"), 4);

        sketch.decay();
        assert_eq!(sketch.estimate(&"key1"), 2);

        sketch.decay();
        assert_eq!(sketch.estimate(&"key1"), 1);

        sketch.decay();
        assert_eq!(sketch.estimate(&"key1"), 0);
    }

    #[test]
    fn test_reset() {
        let mut sketch = CountMinSketch::new(1024, 4);

        sketch.increment(&"key1");
        sketch.increment(&"key2");
        sketch.increment(&"key3");

        assert_eq!(sketch.total_count(), 3);
        assert!(sketch.estimate(&"key1") > 0);

        sketch.reset();

        assert_eq!(sketch.total_count(), 0);
        assert_eq!(sketch.estimate(&"key1"), 0);
        assert_eq!(sketch.estimate(&"key2"), 0);
        assert_eq!(sketch.estimate(&"key3"), 0);
    }

    #[test]
    fn test_total_count() {
        let mut sketch = CountMinSketch::new(1024, 4);

        assert_eq!(sketch.total_count(), 0);

        sketch.increment(&"key1");
        assert_eq!(sketch.total_count(), 1);

        sketch.increment(&"key2");
        sketch.increment(&"key1");
        assert_eq!(sketch.total_count(), 3);

        sketch.decay();
        assert_eq!(sketch.total_count(), 1); // 3 / 2 = 1
    }

    #[test]
    fn test_multiple_keys() {
        let mut sketch = CountMinSketch::new(2048, 4);

        for i in 0..100 {
            sketch.increment(&format!("key{}", i));
        }

        for i in 0..100 {
            assert_eq!(sketch.estimate(&format!("key{}", i)), 1);
        }

        // Add more to specific keys
        for _ in 0..10 {
            sketch.increment(&"key5");
        }

        assert_eq!(sketch.estimate(&"key5"), 11);
        assert_eq!(sketch.estimate(&"key10"), 1);
    }

    #[test]
    fn test_saturation() {
        let mut sketch = CountMinSketch::new(1024, 4);

        // Manually set counters to near saturation point to test saturating_add behavior
        // We need to set the counters at the positions where "key1" hashes to
        let test_key = "key1";
        for i in 0..sketch.depth {
            let index = sketch.hash(&test_key, i);
            sketch.counters[i][index] = u32::MAX - 1;
        }

        // Increment a few more times - should saturate at u32::MAX, not overflow
        sketch.increment(&test_key);
        sketch.increment(&test_key);
        sketch.increment(&test_key);

        let estimate = sketch.estimate(&test_key);
        assert_eq!(estimate, u32::MAX);
    }

    #[test]
    fn test_different_types() {
        let mut sketch = CountMinSketch::new(1024, 4);

        sketch.increment(&42u32);
        sketch.increment(&"string_key");
        sketch.increment(&(1, 2, 3));

        assert_eq!(sketch.estimate(&42u32), 1);
        assert_eq!(sketch.estimate(&"string_key"), 1);
        assert_eq!(sketch.estimate(&(1, 2, 3)), 1);
    }
}
