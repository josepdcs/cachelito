//! # W-TinyLFU Utilities
//!
//! Utility functions for the W-TinyLFU (Windowed TinyLFU) cache eviction policy.
//!
//! ## Algorithm Overview
//!
//! W-TinyLFU uses:
//! - **Window segment (W)**: A small percentage of cache capacity for new entries (recency-based)
//! - **Protected segment**: Main cache using LFU-like eviction
//! - **Count-Min Sketch**: Approximate frequency counter for admission decisions
//! - **Admission policy**: New entries admitted only if they have higher frequency than victims
//!
//! ## Components
//!
//! - Window eviction: FIFO/LRU for the window segment
//! - Protected eviction: LFU for the main segment
//! - Admission gate: Compares frequency of candidate vs victim using Count-Min Sketch

use crate::CountMinSketch;

/// Configuration for W-TinyLFU policy
#[derive(Clone, Debug)]
pub struct WTinyLFUConfig {
    /// Total cache capacity
    pub capacity: usize,
    /// Ratio of window segment (0.0-1.0, typically 0.01-0.20)
    pub window_ratio: f64,
    /// Width of Count-Min Sketch (number of counters per row)
    pub sketch_width: usize,
    /// Depth of Count-Min Sketch (number of hash functions/rows)
    pub sketch_depth: usize,
    /// Decay interval: reset sketch after this many accesses
    pub decay_interval: u64,
}

impl Default for WTinyLFUConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            window_ratio: 0.01, // 1% window
            sketch_width: 2048,
            sketch_depth: 4,
            decay_interval: 10_000,
        }
    }
}

impl WTinyLFUConfig {
    /// Creates a new configuration with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    /// Returns the size of the window segment
    pub fn window_size(&self) -> usize {
        ((self.capacity as f64 * self.window_ratio).ceil() as usize).max(1)
    }

    /// Returns the size of the protected segment
    pub fn protected_size(&self) -> usize {
        self.capacity.saturating_sub(self.window_size())
    }

    /// Sets the window ratio (percentage of cache for window segment)
    pub fn with_window_ratio(mut self, ratio: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&ratio),
            "Window ratio must be between 0.0 and 1.0"
        );
        self.window_ratio = ratio;
        self
    }

    /// Sets the sketch dimensions
    pub fn with_sketch(mut self, width: usize, depth: usize) -> Self {
        assert!(width > 0, "Sketch width must be greater than 0");
        assert!(depth > 0, "Sketch depth must be greater than 0");
        self.sketch_width = width;
        self.sketch_depth = depth;
        self
    }

    /// Sets the decay interval
    pub fn with_decay_interval(mut self, interval: u64) -> Self {
        self.decay_interval = interval;
        self
    }
}

/// Determines which segment a key should belong to based on W-TinyLFU policy
///
/// Returns `true` if the key should be in the window segment, `false` for protected segment
pub fn should_be_in_window<K>(
    key: &K,
    window_keys: &[K],
    protected_keys: &[K],
    config: &WTinyLFUConfig,
) -> bool
where
    K: PartialEq,
{
    // If key is already in window, keep it there
    if window_keys.contains(key) {
        return true;
    }

    // If key is in protected, keep it there
    if protected_keys.contains(key) {
        return false;
    }

    // New keys go to window
    window_keys.len() < config.window_size()
}

/// Selects a victim from the window segment (FIFO)
///
/// Returns the key to evict from the window (the oldest one)
pub fn select_window_victim<K: Clone>(window_keys: &[K]) -> Option<K> {
    window_keys.first().cloned()
}

/// Selects a victim from the protected segment (LFU)
///
/// Returns the key with the lowest frequency
pub fn select_protected_victim<K, F>(protected_keys: &[K], get_frequency: F) -> Option<K>
where
    K: Clone,
    F: Fn(&K) -> u32,
{
    protected_keys
        .iter()
        .min_by_key(|k| get_frequency(k))
        .cloned()
}

/// Admission policy: decides if a candidate should replace a victim
///
/// Returns `true` if the candidate has higher estimated frequency than the victim
pub fn should_admit<K: std::hash::Hash>(
    candidate: &K,
    victim: &K,
    sketch: &CountMinSketch,
) -> bool {
    let candidate_freq = sketch.estimate(candidate);
    let victim_freq = sketch.estimate(victim);

    candidate_freq > victim_freq
}

/// Promotes a key from window to protected segment
///
/// This happens when a key in the window segment gets accessed frequently
pub fn promote_to_protected<K: Clone + PartialEq>(
    key: &K,
    window_keys: &mut Vec<K>,
    protected_keys: &mut Vec<K>,
) {
    if let Some(pos) = window_keys.iter().position(|k| k == key) {
        window_keys.remove(pos);
        protected_keys.push(key.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_default() {
        let config = WTinyLFUConfig::default();
        assert_eq!(config.capacity, 100);
        assert_eq!(config.window_ratio, 0.01);
        assert_eq!(config.sketch_width, 2048);
        assert_eq!(config.sketch_depth, 4);
        assert_eq!(config.decay_interval, 10_000);
    }

    #[test]
    fn test_config_window_size() {
        let config = WTinyLFUConfig::new(100).with_window_ratio(0.2);
        assert_eq!(config.window_size(), 20);
        assert_eq!(config.protected_size(), 80);
    }

    #[test]
    fn test_config_small_window() {
        let config = WTinyLFUConfig::new(100).with_window_ratio(0.001);
        // Minimum window size is 1
        assert_eq!(config.window_size(), 1);
        assert_eq!(config.protected_size(), 99);
    }

    #[test]
    fn test_should_be_in_window_new_key() {
        let config = WTinyLFUConfig::new(10).with_window_ratio(0.2);
        let window_keys: Vec<String> = vec![];
        let protected_keys: Vec<String> = vec![];

        assert!(should_be_in_window(
            &"new_key".to_string(),
            &window_keys,
            &protected_keys,
            &config
        ));
    }

    #[test]
    fn test_should_be_in_window_existing_window() {
        let config = WTinyLFUConfig::new(10).with_window_ratio(0.2);
        let window_keys: Vec<String> = vec!["key1".to_string()];
        let protected_keys: Vec<String> = vec![];

        assert!(should_be_in_window(
            &"key1".to_string(),
            &window_keys,
            &protected_keys,
            &config
        ));
    }

    #[test]
    fn test_should_be_in_window_existing_protected() {
        let config = WTinyLFUConfig::new(10).with_window_ratio(0.2);
        let window_keys: Vec<String> = vec![];
        let protected_keys: Vec<String> = vec!["key1".to_string()];

        assert!(!should_be_in_window(
            &"key1".to_string(),
            &window_keys,
            &protected_keys,
            &config
        ));
    }

    #[test]
    fn test_select_window_victim() {
        let window_keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        let victim = select_window_victim(&window_keys);
        assert_eq!(victim, Some("key1".to_string()));
    }

    #[test]
    fn test_select_window_victim_empty() {
        let window_keys: Vec<String> = vec![];
        let victim = select_window_victim(&window_keys);
        assert_eq!(victim, None);
    }

    #[test]
    fn test_select_protected_victim() {
        let protected_keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];

        let frequencies = [("key1", 5), ("key2", 2), ("key3", 10)]
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<HashMap<_, _>>();

        let victim =
            select_protected_victim(&protected_keys, |k| *frequencies.get(k).unwrap_or(&0));

        assert_eq!(victim, Some("key2".to_string()));
    }

    #[test]
    fn test_should_admit() {
        let mut sketch = CountMinSketch::new(1000, 4);

        // Simulate candidate being accessed more frequently
        for _ in 0..10 {
            sketch.increment(&"candidate");
        }
        for _ in 0..3 {
            sketch.increment(&"victim");
        }

        assert!(should_admit(&"candidate", &"victim", &sketch));
        assert!(!should_admit(&"victim", &"candidate", &sketch));
    }

    #[test]
    fn test_promote_to_protected() {
        let mut window = vec!["key1".to_string(), "key2".to_string()];
        let mut protected = vec!["key3".to_string()];

        promote_to_protected(&"key1".to_string(), &mut window, &mut protected);

        assert_eq!(window, vec!["key2".to_string()]);
        assert_eq!(protected, vec!["key3".to_string(), "key1".to_string()]);
    }

    #[test]
    #[should_panic(expected = "Window ratio must be between 0.0 and 1.0")]
    fn test_invalid_window_ratio() {
        WTinyLFUConfig::new(100).with_window_ratio(1.5);
    }

    #[test]
    #[should_panic(expected = "Sketch width must be greater than 0")]
    fn test_invalid_sketch_width() {
        WTinyLFUConfig::new(100).with_sketch(0, 4);
    }
}
