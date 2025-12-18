use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

// Import shared utilities from cachelito-macro-utils
use cachelito_macro_utils::{generate_key_expr, parse_async_attributes, AsyncCacheAttributes};

/// Parse macro attributes from the attribute token stream
fn parse_attributes(attr: TokenStream) -> AsyncCacheAttributes {
    let attr_stream: TokenStream2 = attr.into();
    match parse_async_attributes(attr_stream) {
        Ok(attrs) => attrs,
        Err(err) => {
            // Return default attributes with the error embedded
            // This will cause a compile error with a helpful message
            panic!("Failed to parse attributes: {}", err);
        }
    }
}
/// Generate the appropriate insert call based on max_memory configuration
fn generate_insert_call(max_memory_expr: &TokenStream2) -> TokenStream2 {
    let max_memory_str = max_memory_expr.to_string();
    let has_max_memory = !max_memory_str.contains("None");

    if has_max_memory {
        quote! { __cache.insert_with_memory(&__key, __result.clone()); }
    } else {
        quote! { __cache.insert(&__key, __result.clone()); }
    }
}

/// Generate common cache lookup and execution logic
fn generate_cache_logic_block(
    key_expr: &TokenStream2,
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    stats_ident: &syn::Ident,
    limit_expr: &TokenStream2,
    max_memory_expr: &TokenStream2,
    policy_expr: &TokenStream2,
    ttl_expr: &TokenStream2,
    frequency_weight_expr: &TokenStream2,
    invalidation_check: &TokenStream2,
    block: &syn::Block,
    cache_insert: &TokenStream2,
) -> TokenStream2 {
    quote! {
        // Generate cache key
        let __key = #key_expr;

        // Create AsyncGlobalCache wrapper
        let __cache = cachelito_core::AsyncGlobalCache::new(
            &*#cache_ident,
            &*#order_ident,
            #limit_expr,
            #max_memory_expr,
            #policy_expr,
            #ttl_expr,
            #frequency_weight_expr,
            &*#stats_ident,
        );

        // Try to get from cache
        if let Some(__cached) = __cache.get(&__key) {
            #invalidation_check
        }

        // Execute original async function (cache miss or expired)
        let __result = (async #block).await;

        // Cache the result (conditional based on cache_if predicate or default behavior)
        #cache_insert

        __result
    }
}

/// A procedural macro that adds automatic async memoization to async functions and methods.
///
/// This macro transforms an async function into a cached version that stores results
/// in a global DashMap based on the function arguments. Subsequent calls with the same
/// arguments will return the cached result instead of re-executing the function body.
///
/// # Requirements
///
/// - **Function must be async**: The function must be declared with `async fn`
/// - **Arguments**: Must implement `Debug` for key generation
/// - **Return type**: Must implement `Clone` for cache storage and retrieval
/// - **Function purity**: For correct behavior, the function should be pure
///   (same inputs always produce same outputs with no side effects)
///
/// # Macro Parameters
///
/// - `limit` (optional): Maximum number of entries in the cache. When the limit is reached,
///   entries are evicted according to the specified policy. Default: unlimited.
/// - `policy` (optional): Eviction policy to use when the cache is full. Options:
///   - `"fifo"` - First In, First Out
///   - `"lru"` - Least Recently Used (default)
///   - `"lfu"` - Least Frequently Used
///   - `"arc"` - Adaptive Replacement Cache
///   - `"random"` - Random Replacement
///   - `"tlru"` - Time-aware Least Recently Used (combines recency, frequency, and age)
/// - `ttl` (optional): Time-to-live in seconds. Entries older than this will be
///   automatically removed when accessed. Default: None (no expiration).
/// - `frequency_weight` (optional): Weight factor for frequency in TLRU policy.
///   Controls the balance between recency and frequency in eviction decisions.
///   - Values < 1.0: Emphasize recency and age over frequency (good for time-sensitive data)
///   - Value = 1.0 (or omitted): Balanced approach (default TLRU behavior)
///   - Values > 1.0: Emphasize frequency over recency (good for popular content)
///   - Formula: `score = frequency^weight × position × age_factor`
///   - Only applicable when `policy = "tlru"`. Ignored for other policies.
///   - Example: `frequency_weight = 1.5` makes frequently accessed entries more resistant to eviction
/// - `name` (optional): Custom identifier for the cache. Default: the function name.
/// - `max_memory` (optional): Maximum memory usage (e.g., "100MB", "1GB"). Requires
///   the return type to implement `MemoryEstimator`.
/// - `tags` (optional): Array of tags for group invalidation. Example: `["user_data", "profile"]`
/// - `events` (optional): Array of events that trigger invalidation. Example: `["user_updated"]`
/// - `dependencies` (optional): Array of cache dependencies. Example: `["get_user"]`
/// - `invalidate_on` (optional): Function that checks if a cached entry should be invalidated.
///   Signature: `fn(key: &String, value: &T) -> bool`. Return `true` to invalidate.
/// - `cache_if` (optional): Function that determines if a result should be cached.
///   Signature: `fn(key: &String, value: &T) -> bool`. Return `true` to cache the result.
///   When not specified, all results are cached (default behavior).
///
/// # Cache Behavior
///
/// - **Global scope**: Cache is ALWAYS shared across all tasks and threads (no thread-local option)
/// - **Regular async functions**: All results are cached
/// - **Result-returning async functions**: Only `Ok` values are cached, `Err` values are not
/// - **Thread-safe**: Uses lock-free concurrent hash map (DashMap)
/// - **Eviction**: When limit is reached, entries are removed according to the policy
/// - **Expiration**: When TTL is set, expired entries are removed on access
///
/// # Examples
///
/// ## Basic Async Function Caching
///
/// ```ignore
/// use cachelito_async::cache_async;
///
/// #[cache_async]
/// async fn fetch_user(id: u64) -> User {
///     // Simulates async API call
///     tokio::time::sleep(Duration::from_secs(1)).await;
///     User { id, name: format!("User {}", id) }
/// }
/// ```
///
/// ## Cache with Invalidation
///
/// ```ignore
/// use cachelito_async::cache_async;
/// use cachelito_core::{invalidate_by_tag, invalidate_by_event};
///
/// #[cache_async(
///     limit = 100,
///     policy = "lru",
///     tags = ["user_data"],
///     events = ["user_updated"]
/// )]
/// async fn get_user_profile(user_id: u64) -> UserProfile {
///     // Fetch from database
///     fetch_profile_from_db(user_id).await
/// }
///
/// // Later, invalidate all caches with the "user_data" tag
/// invalidate_by_tag("user_data");
///
/// // Or invalidate by event
/// invalidate_by_event("user_updated");
/// ```
///
/// ## TLRU with Custom Frequency Weight
///
/// ```ignore
/// use cachelito_async::cache_async;
///
/// // Low frequency_weight (0.3) - emphasizes recency and age
/// // Good for time-sensitive data where freshness matters more than popularity
/// #[cache_async(
///     policy = "tlru",
///     limit = 100,
///     ttl = 300,
///     frequency_weight = 0.3
/// )]
/// async fn fetch_realtime_data(source: String) -> Data {
///     // Fetch time-sensitive data
///     api_client.fetch(source).await
/// }
///
/// // High frequency_weight (1.5) - emphasizes access frequency
/// // Good for popular content that should stay cached despite age
/// #[cache_async(
///     policy = "tlru",
///     limit = 100,
///     ttl = 300,
///     frequency_weight = 1.5
/// )]
/// async fn fetch_popular_content(id: u64) -> Content {
///     // Frequently accessed entries remain cached longer
///     database.fetch_content(id).await
/// }
///
/// // Default behavior (balanced) - omit frequency_weight
/// #[cache_async(policy = "tlru", limit = 100, ttl = 300)]
/// async fn fetch_balanced(key: String) -> Value {
///     // Balanced approach between recency and frequency
///     expensive_operation(key).await
/// }
/// ```
///
/// # Performance Considerations
///
/// - **Lock-free**: Uses DashMap for concurrent access without blocking
/// - **Cache key generation**: Uses `Debug` formatting for keys
/// - **Memory usage**: Controlled by the `limit` parameter
/// - **Async overhead**: Minimal, no `.await` needed for cache operations
///
#[proc_macro_attribute]
pub fn cache_async(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attrs = parse_attributes(attr);

    // Extract function components
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let fn_name = &sig.ident;
    let fn_name_string = fn_name.to_string();
    let fn_name_str = attrs.custom_name.as_ref().unwrap_or(&fn_name_string);

    // Extract return type
    let ret_type = match &sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };

    // Collect function arguments for key generation
    let mut has_self = false;
    let mut arg_pats = Vec::new();

    for arg in &sig.inputs {
        match arg {
            syn::FnArg::Receiver(_) => {
                has_self = true;
            }
            syn::FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                arg_pats.push(quote! { #pat });
            }
        }
    }

    // Generate identifiers for the cache components
    let cache_ident = syn::Ident::new(
        &format!("__CACHE_{}", fn_name.to_string().to_uppercase()),
        fn_name.span(),
    );
    let order_ident = syn::Ident::new(
        &format!("__ORDER_{}", fn_name.to_string().to_uppercase()),
        fn_name.span(),
    );
    let stats_ident = syn::Ident::new(
        &format!("__STATS_{}", fn_name.to_string().to_uppercase()),
        fn_name.span(),
    );

    // Generate cache key expression
    let key_expr = generate_key_expr(has_self, &arg_pats);

    // Detect Result type and extract inner type if needed
    let (is_result, _cache_value_type) = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        if s.starts_with("Result<") || s.starts_with("std::result::Result<") {
            // Extract the Ok type from Result<T, E>
            // For simplicity, we'll use the full return type and let the compiler infer
            // But we need to specify that the cache stores the inner T, not Result<T, E>
            (true, ret_type.clone())
        } else {
            (false, ret_type.clone())
        }
    };

    let limit_expr = &attrs.limit;
    let policy_str = &attrs.policy;
    let ttl_expr = &attrs.ttl;
    let max_memory_expr = &attrs.max_memory;
    let frequency_weight_expr = &attrs.frequency_weight;

    // Convert policy string to EvictionPolicy
    let policy_expr = quote! {
        cachelito_core::EvictionPolicy::from(#policy_str)
    };

    // Generate invalidation check expression
    let invalidation_check = if let Some(pred_fn) = &attrs.invalidate_on {
        quote! {
            // Validate cached value with invalidation check
            // If function returns true, entry is stale - don't use it, re-execute
            if !#pred_fn(&__key, &__cached) {
                // Check function returned false, entry is valid
                return __cached;
            }
            // If function returned true, fall through to re-execute
        }
    } else {
        quote! {
            return __cached;
        }
    };

    // Generate cache logic based on Result or regular return
    let cache_logic = {
        let insert_call = generate_insert_call(max_memory_expr);

        // Generate conditional caching logic
        let cache_insert = if let Some(pred_fn) = &attrs.cache_if {
            quote! {
                // Only cache if the cache_if predicate returns true
                if #pred_fn(&__key, &__result) {
                    #insert_call
                }
            }
        } else if is_result {
            // Default behavior for Result types: only cache Ok values
            quote! {
                if __result.is_ok() {
                    #insert_call
                }
            }
        } else {
            // Default behavior for regular types: always cache
            insert_call
        };

        // Use the common helper function to generate the cache logic block
        generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            limit_expr,
            max_memory_expr,
            &policy_expr,
            ttl_expr,
            frequency_weight_expr,
            &invalidation_check,
            block,
            &cache_insert,
        )
    };

    // Generate invalidation registration code
    let invalidation_registration = if !attrs.tags.is_empty()
        || !attrs.events.is_empty()
        || !attrs.dependencies.is_empty()
    {
        let tags = &attrs.tags;
        let events = &attrs.events;
        let deps = &attrs.dependencies;

        quote! {
            // Register invalidation metadata and callback (happens once on first access)
            static INVALIDATION_REGISTERED: once_cell::sync::OnceCell<()> = once_cell::sync::OnceCell::new();
            INVALIDATION_REGISTERED.get_or_init(|| {
                let metadata = cachelito_core::InvalidationMetadata::new(
                    vec![#(#tags.to_string()),*],
                    vec![#(#events.to_string()),*],
                    vec![#(#deps.to_string()),*],
                );
                cachelito_core::InvalidationRegistry::global().register(#fn_name_str, metadata);

                // Register invalidation callback to clear the async cache
                cachelito_core::InvalidationRegistry::global().register_callback(
                    #fn_name_str,
                    move || {
                        #cache_ident.clear();
                        #order_ident.lock().clear();
                    }
                );
            });
        }
    } else {
        quote! {}
    };

    // Always register invalidation callback for async caches
    let invalidation_callback_registration = quote! {
        // Register invalidation check callback
        static INVALIDATION_CHECK_REGISTERED: once_cell::sync::OnceCell<()> = once_cell::sync::OnceCell::new();
        INVALIDATION_CHECK_REGISTERED.get_or_init(|| {
            cachelito_core::InvalidationRegistry::global().register_invalidation_callback(
                #fn_name_str,
                move |invalidation_check: &dyn Fn(&str) -> bool| {
                    // Collect keys to remove based on invalidation check function
                    let keys_to_remove: Vec<String> = #cache_ident
                        .iter()
                        .filter(|entry| invalidation_check(entry.key().as_str()))
                        .map(|entry| entry.key().clone())
                        .collect();

                    // Remove matched keys
                    let mut order_write = #order_ident.lock();
                    for key in &keys_to_remove {
                        #cache_ident.remove(key);
                        if let Some(pos) = order_write.iter().position(|k| k == key) {
                            order_write.remove(pos);
                        }
                    }
                }
            );
        });
    };

    // Generate final expanded code
    let expanded = quote! {
        #vis #sig {
            use std::collections::VecDeque;

            // DashMap stores: (value, timestamp, frequency)
            static #cache_ident: once_cell::sync::Lazy<dashmap::DashMap<String, (#ret_type, u64, u64)>> =
                once_cell::sync::Lazy::new(|| dashmap::DashMap::new());
            static #order_ident: once_cell::sync::Lazy<parking_lot::Mutex<VecDeque<String>>> =
                once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(VecDeque::new()));
            static #stats_ident: once_cell::sync::Lazy<cachelito_core::CacheStats> =
                once_cell::sync::Lazy::new(|| cachelito_core::CacheStats::new());

            // Register stats in the registry (happens once on first access)
            static STATS_REGISTERED: once_cell::sync::OnceCell<()> = once_cell::sync::OnceCell::new();
            STATS_REGISTERED.get_or_init(|| {
                cachelito_core::stats_registry::register(#fn_name_str, &#stats_ident);
            });

            #invalidation_registration
            #invalidation_callback_registration

            #cache_logic
        }
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_generate_insert_call_without_max_memory() {
        let max_memory_expr = quote! { None };
        let result = generate_insert_call(&max_memory_expr);
        let result_str = result.to_string();

        assert!(result_str.contains("__cache") && result_str.contains("insert"));
        assert!(!result_str.contains("insert_with_memory"));
    }

    #[test]
    fn test_generate_insert_call_with_max_memory() {
        let max_memory_expr = quote! { Some(1024 * 1024) };
        let result = generate_insert_call(&max_memory_expr);
        let result_str = result.to_string();

        assert!(result_str.contains("insert_with_memory"));
    }

    #[test]
    fn test_generate_cache_logic_block_structure() {
        let key_expr = quote! { format!("{:?}", arg1) };
        let cache_ident = syn::Ident::new("__CACHE_TEST", proc_macro2::Span::call_site());
        let order_ident = syn::Ident::new("__ORDER_TEST", proc_macro2::Span::call_site());
        let stats_ident = syn::Ident::new("__STATS_TEST", proc_macro2::Span::call_site());
        let limit_expr = quote! { Some(100) };
        let max_memory_expr = quote! { None };
        let policy_expr = quote! { cachelito_core::EvictionPolicy::LRU };
        let ttl_expr = quote! { None };
        let invalidation_check = quote! { return __cached; };
        let block: syn::Block = syn::parse2(quote! { { 42 } }).unwrap();
        let cache_insert = quote! { __cache.insert(&__key, __result.clone()); };

        let result = generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            &limit_expr,
            &max_memory_expr,
            &policy_expr,
            &ttl_expr,
            &quote! { Option::<f64>::None },
            &invalidation_check,
            &block,
            &cache_insert,
        );

        let result_str = result.to_string();

        // Verify key components are present
        assert!(result_str.contains("let __key"));
        assert!(result_str.contains("AsyncGlobalCache") && result_str.contains("new"));
        assert!(result_str.contains("__cache") && result_str.contains("get"));
        assert!(result_str.contains("let __result"));
        assert!(result_str.contains("__CACHE_TEST"));
        assert!(result_str.contains("__ORDER_TEST"));
        assert!(result_str.contains("__STATS_TEST"));
    }

    #[test]
    fn test_generate_cache_logic_includes_all_parameters() {
        let key_expr = quote! { format!("{:?}", x) };
        let cache_ident = syn::Ident::new("__CACHE_FN", proc_macro2::Span::call_site());
        let order_ident = syn::Ident::new("__ORDER_FN", proc_macro2::Span::call_site());
        let stats_ident = syn::Ident::new("__STATS_FN", proc_macro2::Span::call_site());
        let limit_expr = quote! { Some(50) };
        let max_memory_expr = quote! { Some(1024) };
        let policy_expr = quote! { cachelito_core::EvictionPolicy::FIFO };
        let ttl_expr = quote! { Some(60) };
        let invalidation_check = quote! { if !check_fn(&__key, &__cached) { return __cached; } };
        let block: syn::Block = syn::parse2(quote! { { expensive_computation() } }).unwrap();
        let cache_insert = quote! {
            if should_cache(&__key, &__result) {
                __cache.insert(&__key, __result.clone());
            }
        };

        let result = generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            &limit_expr,
            &max_memory_expr,
            &policy_expr,
            &ttl_expr,
            &quote! { Option::<f64>::None },
            &invalidation_check,
            &block,
            &cache_insert,
        );

        let result_str = result.to_string();

        // Verify all parameters are used
        assert!(result_str.contains("Some (50)"));
        assert!(result_str.contains("Some (1024)"));
        assert!(result_str.contains("Some (60)"));
        assert!(result_str.contains("check_fn"));
        assert!(result_str.contains("should_cache"));
        assert!(result_str.contains("expensive_computation"));
    }

    #[test]
    fn test_insert_call_format() {
        let max_memory_none = quote! { None };
        let result_none = generate_insert_call(&max_memory_none);

        // Should call insert method
        assert_eq!(
            result_none.to_string(),
            "__cache . insert (& __key , __result . clone ()) ;"
        );

        let max_memory_some = quote! { Some(2048) };
        let result_some = generate_insert_call(&max_memory_some);

        // Should call insert_with_memory method
        assert_eq!(
            result_some.to_string(),
            "__cache . insert_with_memory (& __key , __result . clone ()) ;"
        );
    }

    #[test]
    fn test_cache_logic_block_contains_invalidation_check() {
        let key_expr = quote! { key };
        let cache_ident = syn::Ident::new("CACHE", proc_macro2::Span::call_site());
        let order_ident = syn::Ident::new("ORDER", proc_macro2::Span::call_site());
        let stats_ident = syn::Ident::new("STATS", proc_macro2::Span::call_site());
        let limit_expr = quote! { None };
        let max_memory_expr = quote! { None };
        let policy_expr = quote! { cachelito_core::EvictionPolicy::LRU };
        let ttl_expr = quote! { None };

        // Test with custom invalidation check
        let custom_invalidation = quote! {
            if my_custom_check(&__key, &__cached) {
                return __cached;
            }
        };
        let block: syn::Block = syn::parse2(quote! { { compute() } }).unwrap();
        let cache_insert = quote! { __cache.insert(&__key, __result.clone()); };

        let result = generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            &limit_expr,
            &max_memory_expr,
            &policy_expr,
            &ttl_expr,
            &quote! { Option::<f64>::None },
            &custom_invalidation,
            &block,
            &cache_insert,
        );

        let result_str = result.to_string();
        assert!(result_str.contains("my_custom_check"));
    }

    #[test]
    fn test_cache_logic_block_contains_cache_insert() {
        let key_expr = quote! { key };
        let cache_ident = syn::Ident::new("CACHE", proc_macro2::Span::call_site());
        let order_ident = syn::Ident::new("ORDER", proc_macro2::Span::call_site());
        let stats_ident = syn::Ident::new("STATS", proc_macro2::Span::call_site());
        let limit_expr = quote! { None };
        let max_memory_expr = quote! { None };
        let policy_expr = quote! { cachelito_core::EvictionPolicy::LRU };
        let ttl_expr = quote! { None };
        let invalidation_check = quote! { return __cached; };
        let block: syn::Block = syn::parse2(quote! { { compute() } }).unwrap();

        // Test with conditional cache insert
        let conditional_insert = quote! {
            if predicate(&__key, &__result) {
                __cache.insert(&__key, __result.clone());
            }
        };

        let result = generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            &limit_expr,
            &max_memory_expr,
            &policy_expr,
            &ttl_expr,
            &quote! { Option::<f64>::None },
            &invalidation_check,
            &block,
            &conditional_insert,
        );

        let result_str = result.to_string();
        assert!(result_str.contains("predicate"));
        assert!(result_str.contains("if"));
    }

    #[test]
    fn test_generate_insert_call_detects_none_correctly() {
        // Test various representations of None
        let none_variants = vec![quote! { None }, quote! { ::std::option::Option::None }];

        for none_expr in none_variants {
            let result = generate_insert_call(&none_expr);
            let result_str = result.to_string();
            assert!(
                result_str.contains("insert") && !result_str.contains("insert_with_memory"),
                "Failed for None variant: {}",
                none_expr
            );
        }
    }

    #[test]
    fn test_generate_insert_call_detects_some_correctly() {
        // Test various representations of Some
        let some_variants = vec![
            quote! { Some(100) },
            quote! { Some(1024 * 1024) },
            quote! { Some(MAX_SIZE) },
        ];

        for some_expr in some_variants {
            let result = generate_insert_call(&some_expr);
            let result_str = result.to_string();
            assert!(
                result_str.contains("insert_with_memory"),
                "Failed for Some variant: {}",
                some_expr
            );
        }
    }

    #[test]
    fn test_cache_logic_block_async_execution() {
        let key_expr = quote! { format!("{:?}", id) };
        let cache_ident = syn::Ident::new("CACHE", proc_macro2::Span::call_site());
        let order_ident = syn::Ident::new("ORDER", proc_macro2::Span::call_site());
        let stats_ident = syn::Ident::new("STATS", proc_macro2::Span::call_site());
        let limit_expr = quote! { None };
        let max_memory_expr = quote! { None };
        let policy_expr = quote! { cachelito_core::EvictionPolicy::LRU };
        let ttl_expr = quote! { None };
        let invalidation_check = quote! { return __cached; };
        let block: syn::Block = syn::parse2(quote! {
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
                42
            }
        })
        .unwrap();
        let cache_insert = quote! { __cache.insert(&__key, __result.clone()); };

        let result = generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            &limit_expr,
            &max_memory_expr,
            &policy_expr,
            &ttl_expr,
            &quote! { Option::<f64>::None },
            &invalidation_check,
            &block,
            &cache_insert,
        );

        let result_str = result.to_string();

        // Verify async execution structure
        assert!(result_str.contains("(async"));
        assert!(result_str.contains(") . await"));
        assert!(result_str.contains("tokio :: time :: sleep"));
        assert!(result_str.contains("tokio :: time :: sleep"));
    }

    #[test]
    fn test_cache_logic_block_contains_async_global_cache_initialization() {
        let key_expr = quote! { key };
        let cache_ident = syn::Ident::new("TEST_CACHE", proc_macro2::Span::call_site());
        let order_ident = syn::Ident::new("TEST_ORDER", proc_macro2::Span::call_site());
        let stats_ident = syn::Ident::new("TEST_STATS", proc_macro2::Span::call_site());
        let limit_expr = quote! { Some(200) };
        let max_memory_expr = quote! { Some(4096) };
        let policy_expr = quote! { cachelito_core::EvictionPolicy::ARC };
        let ttl_expr = quote! { Some(120) };
        let invalidation_check = quote! { return __cached; };
        let block: syn::Block = syn::parse2(quote! { { value } }).unwrap();
        let cache_insert = quote! { __cache.insert_with_memory(&__key, __result.clone()); };

        let result = generate_cache_logic_block(
            &key_expr,
            &cache_ident,
            &order_ident,
            &stats_ident,
            &limit_expr,
            &max_memory_expr,
            &policy_expr,
            &ttl_expr,
            &quote! { Option::<f64>::None },
            &invalidation_check,
            &block,
            &cache_insert,
        );

        let result_str = result.to_string();

        // Verify AsyncGlobalCache initialization with all parameters
        assert!(result_str.contains("AsyncGlobalCache :: new"));
        assert!(result_str.contains("& * TEST_CACHE"));
        assert!(result_str.contains("& * TEST_ORDER"));
        assert!(result_str.contains("Some (200)"));
        assert!(result_str.contains("Some (4096)"));
        assert!(result_str.contains("EvictionPolicy :: ARC"));
        assert!(result_str.contains("Some (120)"));
        assert!(result_str.contains("& * TEST_STATS"));
    }
}
