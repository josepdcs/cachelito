/// Integration tests for custom cache names feature

#[cfg(feature = "stats")]
#[cfg(test)]
mod tests {
    use cachelito::cache;
    use serial_test::serial;

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
    #[serial]
    fn test_custom_name_registration() {
        // Call functions to trigger registration with unique values per test
        // Use a base value that's unlikely to conflict (test hash-based)
        let base = 900100;
        test_function_with_custom_name(base);
        test_function_default_name(base);

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
    #[serial]
    fn test_custom_name_statistics() {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Reset stats for this cache
        cachelito::stats_registry::reset("custom_test_cache");

        // Use truly unique values based on current timestamp to avoid any cache pollution
        // Use microseconds and modulo to get smaller values that won't overflow when multiplied by 2
        let timestamp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            % 1_000_000_000) as i32;

        let unique_val1 = timestamp;
        let unique_val2 = timestamp.wrapping_add(1);

        // Make some calls with unique values
        test_function_with_custom_name(unique_val1); // Miss
        test_function_with_custom_name(unique_val1); // Hit
        test_function_with_custom_name(unique_val2); // Miss
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
    #[serial]
    fn test_default_name_statistics() {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Reset stats for this cache
        cachelito::stats_registry::reset("test_function_default_name");

        // Use truly unique values based on current timestamp
        // Use microseconds and modulo to get smaller values that won't overflow when multiplied by 3
        let timestamp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            % 700_000_000) as i32;

        let unique_val1 = timestamp;
        let unique_val2 = timestamp.wrapping_add(10);

        // Make calls with unique values
        test_function_default_name(unique_val1); // Miss
        test_function_default_name(unique_val1); // Hit
        test_function_default_name(unique_val2); // Miss

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
    #[serial]
    fn test_multiple_custom_names() {
        use std::time::{SystemTime, UNIX_EPOCH};

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

        // Use unique values based on timestamp
        // Use microseconds and modulo to get smaller values
        let timestamp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            % 1_000_000_000) as i32;

        let val1 = timestamp;
        let val2 = timestamp.wrapping_add(100);

        // Call both functions
        inner::func_a(val1); // Miss
        inner::func_a(val1); // Hit
        inner::func_b(val1); // Miss
        inner::func_b(val1); // Hit
        inner::func_b(val2); // Miss

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
