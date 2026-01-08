/// Configuration for W-TinyLFU (Windowed TinyLFU) cache policy.
///
/// W-TinyLFU divides the cache into two segments:
/// - **Window segment (W)**: Captures recent accesses (FIFO-like behavior)
/// - **Protected segment**: Main cache using frequency-based eviction
///
/// The policy uses a Count-Min Sketch for approximate frequency counting
/// and an admission policy to decide whether new entries should replace victims.
///
/// # Examples
///
/// ```
/// use cachelito_core::WTinyLFUConfig;
///
/// // Default configuration
/// let config = WTinyLFUConfig::default();
///
/// // Custom configuration
/// let custom = WTinyLFUConfig {
///     window_ratio: 0.15,
///     sketch_width: 4096,
///     sketch_depth: 4,
///     decay_interval: 20000,
/// };
/// ```
#[derive(Clone, Copy, Debug)]
pub struct WTinyLFUConfig {
    /// Ratio of cache capacity allocated to the window segment.
    ///
    /// - Default: 0.20 (20% window, 80% protected)
    /// - Range: 0.01 to 0.99
    /// - Lower values: More emphasis on frequency
    /// - Higher values: More emphasis on recency
    pub window_ratio: f64,

    /// Number of buckets in the Count-Min Sketch (width).
    ///
    /// - Default: 2048
    /// - Larger values: Better accuracy, more memory
    /// - Typical range: 1024 to 8192
    pub sketch_width: usize,

    /// Number of hash functions in the Count-Min Sketch (depth).
    ///
    /// - Default: 4
    /// - Larger values: Better accuracy, slower queries
    /// - Typical range: 3 to 8
    pub sketch_depth: usize,

    /// Number of insertions before triggering counter decay.
    ///
    /// - Default: 10000
    /// - Decay divides all counters by 2
    /// - Prevents saturation and adapts to changing patterns
    pub decay_interval: u64,
}

impl WTinyLFUConfig {
    /// Creates a new W-TinyLFU configuration with custom parameters.
    ///
    /// # Parameters
    ///
    /// * `window_ratio` - Proportion of cache for window segment (0.01 to 0.99)
    /// * `sketch_width` - Number of buckets in Count-Min Sketch
    /// * `sketch_depth` - Number of hash functions in Count-Min Sketch
    /// * `decay_interval` - Insertions before counter decay
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::WTinyLFUConfig;
    ///
    /// let config = WTinyLFUConfig::new(0.15, 4096, 4, 20000);
    /// assert_eq!(config.window_ratio, 0.15);
    /// ```
    pub fn new(
        window_ratio: f64,
        sketch_width: usize,
        sketch_depth: usize,
        decay_interval: u64,
    ) -> Self {
        Self {
            window_ratio: window_ratio.clamp(0.01, 0.99),
            sketch_width,
            sketch_depth,
            decay_interval,
        }
    }

    /// Calculates the window segment size for a given cache capacity.
    ///
    /// # Parameters
    ///
    /// * `total_capacity` - Total cache capacity
    ///
    /// # Returns
    ///
    /// Number of entries allocated to the window segment (at least 1)
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::WTinyLFUConfig;
    ///
    /// let config = WTinyLFUConfig::default();
    /// assert_eq!(config.window_size(100), 20); // 20% of 100
    /// assert_eq!(config.window_size(5), 1);    // At least 1
    /// ```
    pub fn window_size(&self, total_capacity: usize) -> usize {
        ((total_capacity as f64 * self.window_ratio).round() as usize).max(1)
    }

    /// Calculates the protected segment size for a given cache capacity.
    ///
    /// # Parameters
    ///
    /// * `total_capacity` - Total cache capacity
    ///
    /// # Returns
    ///
    /// Number of entries allocated to the protected segment
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::WTinyLFUConfig;
    ///
    /// let config = WTinyLFUConfig::default();
    /// assert_eq!(config.protected_size(100), 80); // 80% of 100
    /// ```
    pub fn protected_size(&self, total_capacity: usize) -> usize {
        total_capacity.saturating_sub(self.window_size(total_capacity))
    }
}

impl Default for WTinyLFUConfig {
    /// Returns the default W-TinyLFU configuration.
    ///
    /// - `window_ratio`: 0.20 (20% window)
    /// - `sketch_width`: 2048 buckets
    /// - `sketch_depth`: 4 hash functions
    /// - `decay_interval`: 10000 insertions
    ///
    /// # Examples
    ///
    /// ```
    /// use cachelito_core::WTinyLFUConfig;
    ///
    /// let config = WTinyLFUConfig::default();
    /// assert_eq!(config.window_ratio, 0.20);
    /// assert_eq!(config.sketch_width, 2048);
    /// assert_eq!(config.sketch_depth, 4);
    /// assert_eq!(config.decay_interval, 10000);
    /// ```
    fn default() -> Self {
        Self {
            window_ratio: 0.20,
            sketch_width: 2048,
            sketch_depth: 4,
            decay_interval: 10000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WTinyLFUConfig::default();
        assert_eq!(config.window_ratio, 0.20);
        assert_eq!(config.sketch_width, 2048);
        assert_eq!(config.sketch_depth, 4);
        assert_eq!(config.decay_interval, 10000);
    }

    #[test]
    fn test_custom_config() {
        let config = WTinyLFUConfig::new(0.15, 4096, 8, 20000);
        assert_eq!(config.window_ratio, 0.15);
        assert_eq!(config.sketch_width, 4096);
        assert_eq!(config.sketch_depth, 8);
        assert_eq!(config.decay_interval, 20000);
    }

    #[test]
    fn test_window_ratio_clamping() {
        let too_low = WTinyLFUConfig::new(0.0, 2048, 4, 10000);
        assert_eq!(too_low.window_ratio, 0.01);

        let too_high = WTinyLFUConfig::new(1.5, 2048, 4, 10000);
        assert_eq!(too_high.window_ratio, 0.99);
    }

    #[test]
    fn test_window_size_calculation() {
        let config = WTinyLFUConfig::default(); // 20% window

        assert_eq!(config.window_size(100), 20);
        assert_eq!(config.window_size(50), 10);
        assert_eq!(config.window_size(10), 2);
        assert_eq!(config.window_size(5), 1); // At least 1
        assert_eq!(config.window_size(1), 1);
    }

    #[test]
    fn test_protected_size_calculation() {
        let config = WTinyLFUConfig::default(); // 20% window

        assert_eq!(config.protected_size(100), 80);
        assert_eq!(config.protected_size(50), 40);
        assert_eq!(config.protected_size(10), 8);
        assert_eq!(config.protected_size(5), 4);
    }

    #[test]
    fn test_custom_window_ratio() {
        let config = WTinyLFUConfig::new(0.10, 2048, 4, 10000); // 10% window

        assert_eq!(config.window_size(100), 10);
        assert_eq!(config.protected_size(100), 90);

        let large_window = WTinyLFUConfig::new(0.50, 2048, 4, 10000); // 50% window
        assert_eq!(large_window.window_size(100), 50);
        assert_eq!(large_window.protected_size(100), 50);
    }

    #[test]
    fn test_total_equals_capacity() {
        let config = WTinyLFUConfig::default();

        for capacity in [10, 50, 100, 1000, 10000] {
            let window = config.window_size(capacity);
            let protected = config.protected_size(capacity);
            assert_eq!(window + protected, capacity);
        }
    }
}
