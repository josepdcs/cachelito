/// Integration tests for custom cache names feature

#[cfg(feature = "stats")]
#[cfg(test)]
mod tests {
    use cachelito::cache;

    // Define functions outside tests to ensure they share the same statics
    #[cache(scope = "global", name = "custom_test_cache")]
    fn test_function_with_custom_name(x: i32) -> i32 {
        x * 2
    }

    #[cache(scope = "global")]
    fn test_function_default_name(x: i32) -> i32 {
        x * 3
    }

    #[test]
    fn test_custom_name_registration() {
        // Call functions to trigger registration
        // Use unique values to avoid cache pollution
        test_function_with_custom_name(9001);
        test_function_default_name(9001);

        // Check that both are registered
        let registered = cachelito::stats_registry::list();
        assert!(
            registered.contains(&"custom_test_cache".to_string()),
            "Custom name should be registered"
        );
        assert!(
            registered.contains(&"test_function_default_name".to_string()),
            "Default name should be registered"
        );
    }

    #[test]
    fn test_custom_name_statistics() {
        // Reset stats for this cache
        cachelito::stats_registry::reset("custom_test_cache");

        // Use unique values to avoid cache pollution from other tests
        // The cache itself is NOT cleared by reset(), only the stats
        let unique_val1 = 10001;
        let unique_val2 = 10002;

        // Make some calls
        test_function_with_custom_name(unique_val1);
        test_function_with_custom_name(unique_val1); // Hit
        test_function_with_custom_name(unique_val2);
        test_function_with_custom_name(unique_val1); // Hit

        // Check statistics using custom name
        let stats = cachelito::stats_registry::get("custom_test_cache");
        assert!(
            stats.is_some(),
            "Stats should be available for custom_test_cache"
        );

        let stats = stats.unwrap();
        assert_eq!(stats.hits(), 2, "Should have 2 hits");
        assert_eq!(stats.misses(), 2, "Should have 2 misses");
        assert_eq!(stats.total_accesses(), 4, "Should have 4 total accesses");
    }

    #[test]
    fn test_default_name_statistics() {
        // Reset stats for this cache
        cachelito::stats_registry::reset("test_function_default_name");

        // Use unique values to avoid conflicts with other tests
        test_function_default_name(1005);
        test_function_default_name(1005); // Hit
        test_function_default_name(1010);

        // Check statistics using function name
        let stats = cachelito::stats_registry::get("test_function_default_name");
        assert!(
            stats.is_some(),
            "Stats should be available for test_function_default_name"
        );

        let stats = stats.unwrap();
        assert_eq!(stats.hits(), 1, "Should have 1 hit");
        assert_eq!(stats.misses(), 2, "Should have 2 misses");
        assert_eq!(stats.total_accesses(), 3, "Should have 3 total accesses");
    }

    #[test]
    fn test_multiple_custom_names() {
        // Define functions inline for this specific test
        mod inner {
            use cachelito::cache;

            #[cache(scope = "global", name = "cache_a")]
            pub fn func_a(x: i32) -> i32 {
                x + 1
            }

            #[cache(scope = "global", name = "cache_b")]
            pub fn func_b(x: i32) -> i32 {
                x + 2
            }
        }

        // Reset stats
        cachelito::stats_registry::reset("cache_a");
        cachelito::stats_registry::reset("cache_b");

        // Call both functions
        inner::func_a(1);
        inner::func_a(1);
        inner::func_b(1);
        inner::func_b(1);
        inner::func_b(2);

        // Check both caches are registered separately
        let stats_a =
            cachelito::stats_registry::get("cache_a").expect("cache_a should be registered");
        let stats_b =
            cachelito::stats_registry::get("cache_b").expect("cache_b should be registered");

        assert_eq!(stats_a.hits(), 1, "cache_a should have 1 hit");
        assert_eq!(stats_a.misses(), 1, "cache_a should have 1 miss");

        assert_eq!(stats_b.hits(), 1, "cache_b should have 1 hit");
        assert_eq!(stats_b.misses(), 2, "cache_b should have 2 misses");
    }
}
