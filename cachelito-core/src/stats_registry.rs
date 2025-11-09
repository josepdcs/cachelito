use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;

use crate::CacheStats;

/// Global registry for cache statistics.
///
/// This registry maintains statistics for all cached functions, indexed by name.
/// It allows querying statistics programmatically without needing to access
/// individual cache instances.
///
/// # Thread Safety
///
/// This registry is thread-safe and can be accessed from multiple threads concurrently.
///
/// # Examples
///
/// ```
/// use cachelito_core::stats_registry;
///
/// // Get stats for a specific cached function
/// if let Some(stats) = stats_registry::get("my_function") {
///     println!("Hits: {}", stats.hits());
///     println!("Misses: {}", stats.misses());
/// }
///
/// // List all cached functions
/// let all_names = stats_registry::list();
/// for name in all_names {
///     println!("Function: {}", name);
/// }
/// ```
static STATS_REGISTRY: Lazy<RwLock<HashMap<String, &'static Lazy<CacheStats>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a cache's statistics under a given name.
///
/// This is called automatically by the `#[cache]` macro when the `stats` feature is enabled.
///
/// # Parameters
///
/// * `name` - The name to register the statistics under (typically the function name)
/// * `stats` - A static reference to the Lazy<CacheStats> for this cache
///
/// # Examples
///
/// ```ignore
/// use cachelito_core::stats_registry;
///
/// static MY_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
/// stats_registry::register("my_function", &MY_STATS);
/// ```
pub fn register(name: &str, stats: &'static Lazy<CacheStats>) {
    let mut registry = STATS_REGISTRY.write();
    registry.insert(name.to_string(), stats);
}

/// Get statistics for a cached function by name.
///
/// Returns a cloned snapshot of the statistics at the time of the call.
///
/// # Parameters
///
/// * `name` - The name of the cached function
///
/// # Returns
///
/// * `Some(CacheStats)` - The statistics if the function is registered
/// * `None` - If no function with that name is registered
///
/// # Examples
///
/// ```
/// use cachelito_core::stats_registry;
///
/// if let Some(stats) = stats_registry::get("my_function") {
///     println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
/// } else {
///     println!("Function not found");
/// }
/// ```
pub fn get(name: &str) -> Option<CacheStats> {
    let registry = STATS_REGISTRY.read();
    registry.get(name).map(|stats| (**stats).clone())
}

/// Get a reference to the statistics for a cached function by name.
///
/// This provides direct access to the statistics without cloning.
///
/// # Parameters
///
/// * `name` - The name of the cached function
///
/// # Returns
///
/// * `Some(&CacheStats)` - A reference to the statistics if the function is registered
/// * `None` - If no function with that name is registered
///
/// # Examples
///
/// ```
/// use cachelito_core::stats_registry;
///
/// if let Some(stats) = stats_registry::get_ref("my_function") {
///     println!("Total accesses: {}", stats.total_accesses());
/// }
/// ```
pub fn get_ref(name: &str) -> Option<&'static CacheStats> {
    let registry = STATS_REGISTRY.read();
    registry.get(name).map(|stats| &***stats)
}

/// List all registered cached function names.
///
/// # Returns
///
/// A vector of all registered function names.
///
/// # Examples
///
/// ```
/// use cachelito_core::stats_registry;
///
/// let functions = stats_registry::list();
/// println!("Registered functions: {:?}", functions);
/// ```
pub fn list() -> Vec<String> {
    let registry = STATS_REGISTRY.read();
    registry.keys().cloned().collect()
}

/// Clear all registered statistics.
///
/// This removes all entries from the registry but does not reset the statistics themselves.
/// Useful for testing or reinitializing the registry.
///
/// # Examples
///
/// ```
/// use cachelito_core::stats_registry;
///
/// stats_registry::clear();
/// assert!(stats_registry::list().is_empty());
/// ```
pub fn clear() {
    let mut registry = STATS_REGISTRY.write();
    registry.clear();
}

/// Reset statistics for a specific function.
///
/// This resets the hit/miss counters for the specified function to zero.
///
/// # Parameters
///
/// * `name` - The name of the cached function
///
/// # Returns
///
/// * `true` - If the function was found and reset
/// * `false` - If no function with that name is registered
///
/// # Examples
///
/// ```
/// use cachelito_core::stats_registry;
///
/// if stats_registry::reset("my_function") {
///     println!("Statistics reset successfully");
/// } else {
///     println!("Function not found");
/// }
/// ```
pub fn reset(name: &str) -> bool {
    let registry = STATS_REGISTRY.read();
    if let Some(stats) = registry.get(name) {
        stats.reset();
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        static TEST_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        register("test_fn", &TEST_STATS);

        let stats = get("test_fn");
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
    }

    #[test]
    fn test_get_ref() {
        static TEST_STATS2: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        register("test_fn2", &TEST_STATS2);
        TEST_STATS2.record_hit();
        TEST_STATS2.record_miss();

        let stats = get_ref("test_fn2");
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.hits(), 1);
        assert_eq!(stats.misses(), 1);
    }

    #[test]
    fn test_list() {
        clear(); // Clear any previous registrations

        static TEST_STATS3: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
        static TEST_STATS4: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        register("fn1", &TEST_STATS3);
        register("fn2", &TEST_STATS4);

        let names = list();
        assert!(names.contains(&"fn1".to_string()));
        assert!(names.contains(&"fn2".to_string()));
    }

    #[test]
    fn test_reset() {
        static TEST_STATS5: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        register("test_fn5", &TEST_STATS5);
        TEST_STATS5.record_hit();
        TEST_STATS5.record_hit();

        assert_eq!(TEST_STATS5.hits(), 2);

        assert!(reset("test_fn5"));
        assert_eq!(TEST_STATS5.hits(), 0);

        assert!(!reset("nonexistent"));
    }

    #[test]
    fn test_clear() {
        static TEST_STATS6: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

        register("test_fn6", &TEST_STATS6);
        assert!(!list().is_empty());

        clear();
        assert!(list().is_empty());
    }
}
