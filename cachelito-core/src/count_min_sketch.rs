//! # Count-Min Sketch
//!
//! A probabilistic data structure for estimating frequencies with low memory overhead.
//! Used by W-TinyLFU policy for approximate frequency counting.
//!
//! ## Algorithm
//!
//! - Uses `depth` hash functions and `width` counters per row
//! - Increment: hash key with each function, increment corresponding counters
//! - Estimate: return minimum of all counters for that key
//! - Decay: periodically halve all counters to prevent saturation
//!
//! ## Complexity
//!
//! - Space: O(width × depth)
//! - Increment: O(depth)
//! - Estimate: O(depth)

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Count-Min Sketch for approximate frequency counting
#[derive(Clone)]
pub struct CountMinSketch {
    /// 2D array of counters: depth rows × width columns
    counters: Vec<Vec<u32>>,
    /// Number of hash functions (rows)
    depth: usize,
    /// Number of counters per hash function (columns)
    width: usize,
    /// Total number of items added (for decay logic)
    total_count: u64,
}

impl CountMinSketch {
    /// Creates a new Count-Min Sketch
    ///
    /// # Arguments
    ///
    /// * `width` - Number of counters per row (affects accuracy)
    /// * `depth` - Number of rows/hash functions (affects collision probability)
    ///
    /// # Recommended values
    ///
    /// - width: 2048-8192 (higher = more accurate)
    /// - depth: 4-8 (diminishing returns after 4)
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let sketch = CountMinSketch::new(2048, 4);
    /// ```
    pub fn new(width: usize, depth: usize) -> Self {
        assert!(width > 0, "Width must be greater than 0");
        assert!(depth > 0, "Depth must be greater than 0");

        Self {
            counters: vec![vec![0; width]; depth],
            depth,
            width,
            total_count: 0,
        }
    }

    /// Increments the frequency estimate for a key
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(2048, 4);
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// assert!(sketch.estimate(&"key1") >= 2);
    /// ```
    pub fn increment<K: Hash>(&mut self, key: &K) {
        for i in 0..self.depth {
            let hash = self.hash(key, i);
            let idx = (hash % self.width as u64) as usize;

            // Prevent overflow by capping at u32::MAX
            if self.counters[i][idx] < u32::MAX {
                self.counters[i][idx] += 1;
            }
        }

        self.total_count += 1;
    }

    /// Estimates the frequency of a key
    ///
    /// Returns the minimum counter value across all hash functions.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(2048, 4);
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// assert_eq!(sketch.estimate(&"key1"), 3);
    /// ```
    pub fn estimate<K: Hash>(&self, key: &K) -> u32 {
        let mut min = u32::MAX;

        for i in 0..self.depth {
            let hash = self.hash(key, i);
            let idx = (hash % self.width as u64) as usize;
            min = min.min(self.counters[i][idx]);
        }

        min
    }

    /// Decays all counters by halving them
    ///
    /// This prevents counter saturation and allows the sketch to adapt
    /// to changing access patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(2048, 4);
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// sketch.increment(&"key1");
    /// assert_eq!(sketch.estimate(&"key1"), 4);
    ///
    /// sketch.decay();
    /// assert_eq!(sketch.estimate(&"key1"), 2);
    /// ```
    pub fn decay(&mut self) {
        for row in &mut self.counters {
            for counter in row {
                *counter /= 2;
            }
        }
        self.total_count /= 2;
    }

    /// Returns the total count of increments
    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    /// Resets all counters to zero
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::CountMinSketch;
    ///
    /// let mut sketch = CountMinSketch::new(2048, 4);
    /// sketch.increment(&"key1");
    /// sketch.reset();
    /// assert_eq!(sketch.estimate(&"key1"), 0);
    /// ```
    pub fn reset(&mut self) {
        for row in &mut self.counters {
            for counter in row {
                *counter = 0;
            }
        }
        self.total_count = 0;
    }

    /// Hash function with seed for different rows
    fn hash<K: Hash>(&self, key: &K, seed: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sketch() {
        let sketch = CountMinSketch::new(100, 4);
        assert_eq!(sketch.width, 100);
        assert_eq!(sketch.depth, 4);
        assert_eq!(sketch.total_count, 0);
    }

    #[test]
    fn test_increment_and_estimate() {
        let mut sketch = CountMinSketch::new(1000, 4);

        sketch.increment(&"key1");
        assert_eq!(sketch.estimate(&"key1"), 1);

        sketch.increment(&"key1");
        sketch.increment(&"key1");
        assert_eq!(sketch.estimate(&"key1"), 3);
    }

    #[test]
    fn test_multiple_keys() {
        let mut sketch = CountMinSketch::new(1000, 4);

        sketch.increment(&"key1");
        sketch.increment(&"key1");
        sketch.increment(&"key2");

        assert_eq!(sketch.estimate(&"key1"), 2);
        assert_eq!(sketch.estimate(&"key2"), 1);
        assert_eq!(sketch.estimate(&"key3"), 0);
    }

    #[test]
    fn test_decay() {
        let mut sketch = CountMinSketch::new(1000, 4);

        for _ in 0..10 {
            sketch.increment(&"key1");
        }
        assert_eq!(sketch.estimate(&"key1"), 10);

        sketch.decay();
        assert_eq!(sketch.estimate(&"key1"), 5);

        sketch.decay();
        assert_eq!(sketch.estimate(&"key1"), 2);
    }

    #[test]
    fn test_reset() {
        let mut sketch = CountMinSketch::new(1000, 4);

        sketch.increment(&"key1");
        sketch.increment(&"key2");
        assert_eq!(sketch.total_count(), 2);

        sketch.reset();
        assert_eq!(sketch.estimate(&"key1"), 0);
        assert_eq!(sketch.estimate(&"key2"), 0);
        assert_eq!(sketch.total_count(), 0);
    }

    #[test]
    fn test_total_count() {
        let mut sketch = CountMinSketch::new(1000, 4);

        sketch.increment(&"key1");
        sketch.increment(&"key1");
        sketch.increment(&"key2");

        assert_eq!(sketch.total_count(), 3);
    }

    #[test]
    fn test_over_estimation() {
        // With small width, we expect over-estimation due to collisions
        let mut sketch = CountMinSketch::new(10, 4);

        // Add many different keys
        for i in 0..100 {
            sketch.increment(&format!("key{}", i));
        }

        // Each key should have count >= 1 (may be higher due to collisions)
        for i in 0..100 {
            assert!(sketch.estimate(&format!("key{}", i)) >= 1);
        }
    }

    #[test]
    #[should_panic(expected = "Width must be greater than 0")]
    fn test_invalid_width() {
        CountMinSketch::new(0, 4);
    }

    #[test]
    #[should_panic(expected = "Depth must be greater than 0")]
    fn test_invalid_depth() {
        CountMinSketch::new(100, 0);
    }
}
