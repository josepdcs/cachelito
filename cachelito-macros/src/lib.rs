use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, ReturnType};

// Import shared utilities
use cachelito_macro_utils::{
    generate_key_expr_with_cacheable_key, parse_sync_attributes, SyncCacheAttributes,
};

/// Parse macro attributes from the attribute token stream
fn parse_attributes(attr: TokenStream) -> SyncCacheAttributes {
    let attr_stream: TokenStream2 = attr.into();
    match parse_sync_attributes(attr_stream) {
        Ok(attrs) => attrs,
        Err(err) => {
            // Return default attributes with the error embedded
            // This will cause a compile error with a helpful message
            panic!("Failed to parse attributes: {}", err);
        }
    }
}

/// Generate the appropriate insert call based on memory configuration and result type
fn generate_insert_call(has_max_memory: bool, is_result: bool) -> TokenStream2 {
    if has_max_memory {
        // Use memory-aware insert methods when max_memory is configured
        if is_result {
            quote! { __cache.insert_result_with_memory(&__key, &__result); }
        } else {
            quote! { __cache.insert_with_memory(&__key, __result.clone()); }
        }
    } else {
        // Use regular insert methods when max_memory is None
        if is_result {
            quote! { __cache.insert_result(&__key, &__result); }
        } else {
            quote! { __cache.insert(&__key, __result.clone()); }
        }
    }
}

/// Generate the thread-local cache branch
fn generate_thread_local_branch(
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    ret_type: &TokenStream2,
    limit_expr: &TokenStream2,
    max_memory_expr: &TokenStream2,
    policy_expr: &TokenStream2,
    ttl_expr: &TokenStream2,
    frequency_weight_expr: &TokenStream2,
    key_expr: &TokenStream2,
    block: &syn::Block,
    is_result: bool,
    invalidate_on: &Option<syn::Path>,
    cache_if: &Option<syn::Path>,
) -> TokenStream2 {
    // Check if max_memory is None by comparing the token stream
    let has_max_memory = has_max_memory(max_memory_expr);

    let invalidation_check = generate_invalidation_check(invalidate_on);
    let cache_condition = generate_cache_condition(cache_if, has_max_memory, is_result);

    quote! {
        thread_local! {
            static #cache_ident: RefCell<std::collections::HashMap<String, CacheEntry<#ret_type>>> = RefCell::new(std::collections::HashMap::new());
            static #order_ident: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
        }

        let __cache = ThreadLocalCache::<#ret_type>::new(
            &#cache_ident,
            &#order_ident,
            #limit_expr,
            #max_memory_expr,
            #policy_expr,
            #ttl_expr,
            #frequency_weight_expr
        );

        let __key = #key_expr;

        if let Some(cached) = __cache.get(&__key) {
            #invalidation_check
        }

        let __result = (|| #block)();
        #cache_condition
        __result
    }
}
/// Check if max_memory is None by comparing the token stream
fn has_max_memory(max_memory_expr: &TokenStream2) -> bool {
    let max_memory_str = max_memory_expr.to_string();
    let has_max_memory = !max_memory_str.contains("None");
    has_max_memory
}

/// Generate invalidation check code if an invalidate_on function is specified
fn generate_invalidation_check(invalidate_on: &Option<syn::Path>) -> TokenStream2 {
    if let Some(pred_fn) = invalidate_on {
        quote! {
            // Validate cached value with invalidate_on function
            // If function returns true, entry is stale - don't use it, re-execute
            if !#pred_fn(&__key, &cached) {
                // Function returned false, entry is valid
                return cached;
            }
            // If function returned true, entry is stale/invalid - fall through to re-execute and refresh cache
        }
    } else {
        quote! {
            return cached;
        }
    }
}

/// Generate cache condition check code if a cache_if function is specified
fn generate_cache_condition(
    cache_if: &Option<syn::Path>,
    has_max_memory: bool,
    is_result: bool,
) -> TokenStream2 {
    let insert_call = generate_insert_call(has_max_memory, is_result);

    if let Some(pred_fn) = cache_if {
        quote! {
            // Check if result should be cached using cache_if function
            // Only cache if function returns true
            if #pred_fn(&__key, &__result) {
                #insert_call
            }
        }
    } else {
        // Always cache if no predicate is specified (default behavior)
        insert_call
    }
}

/// Generate the global cache branch
fn generate_global_branch(
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    stats_ident: &syn::Ident,
    ret_type: &TokenStream2,
    limit_expr: &TokenStream2,
    max_memory_expr: &TokenStream2,
    policy_expr: &TokenStream2,
    ttl_expr: &TokenStream2,
    frequency_weight_expr: &TokenStream2,
    key_expr: &TokenStream2,
    block: &syn::Block,
    fn_name_str: &str,
    is_result: bool,
    attrs: &SyncCacheAttributes,
) -> TokenStream2 {
    // ...existing code...
    let has_max_memory = has_max_memory(max_memory_expr);

    let invalidation_check = generate_invalidation_check(&attrs.invalidate_on);
    let cache_condition = generate_cache_condition(&attrs.cache_if, has_max_memory, is_result);

    // ...existing code...

    let invalidation_registration = if !attrs.tags.is_empty()
        || !attrs.events.is_empty()
        || !attrs.dependencies.is_empty()
    {
        // ...existing code...
        let tags = &attrs.tags;
        let events = &attrs.events;
        let deps = &attrs.dependencies;

        quote! {
            // Register invalidation metadata
            {
                use std::sync::Once;
                static INVALIDATION_REGISTER_ONCE: Once = Once::new();
                INVALIDATION_REGISTER_ONCE.call_once(|| {
                    let metadata = cachelito_core::InvalidationMetadata::new(
                        vec![#(#tags.to_string()),*],
                        vec![#(#events.to_string()),*],
                        vec![#(#deps.to_string()),*],
                    );
                    cachelito_core::InvalidationRegistry::global().register(#fn_name_str, metadata);

                    // Register invalidation callback
                    cachelito_core::InvalidationRegistry::global().register_callback(
                        #fn_name_str,
                        move || {
                            #cache_ident.write().clear();
                            #order_ident.lock().clear();
                        }
                    );
                });
            }
        }
    } else {
        quote! {}
    };

    // ...existing code...
    let invalidation_callback_registration = quote! {
        // Register callback for runtime invalidation checks
        {
            use std::sync::Once;
            static INVALIDATION_CALLBACK_REGISTER_ONCE: Once = Once::new();
            INVALIDATION_CALLBACK_REGISTER_ONCE.call_once(|| {
                cachelito_core::InvalidationRegistry::global().register_invalidation_callback(
                    #fn_name_str,
                    move |check_fn: &dyn Fn(&str) -> bool| {
                        let mut map_write = #cache_ident.write();
                        let mut order_write = #order_ident.lock();

                        // Collect keys to remove based on check function
                        let keys_to_remove: Vec<String> = map_write
                            .keys()
                            .filter(|k| check_fn(k.as_str()))
                            .cloned()
                            .collect();

                        // Remove matched keys
                        for key in &keys_to_remove {
                            map_write.remove(key);
                            if let Some(pos) = order_write.iter().position(|k| k == key) {
                                order_write.remove(pos);
                            }
                        }
                    }
                );
            });
        }
    };

    quote! {
        // ...existing code...
        static #cache_ident: once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<String, CacheEntry<#ret_type>>>> =
            once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
        static #order_ident: once_cell::sync::Lazy<parking_lot::Mutex<VecDeque<String>>> =
            once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(VecDeque::new()));

        #[cfg(feature = "stats")]
        static #stats_ident: once_cell::sync::Lazy<cachelito_core::CacheStats> =
            once_cell::sync::Lazy::new(|| cachelito_core::CacheStats::new());

        #[cfg(feature = "stats")]
        {
            use std::sync::Once;
            static REGISTER_ONCE: Once = Once::new();
            REGISTER_ONCE.call_once(|| {
                cachelito_core::stats_registry::register(#fn_name_str, &#stats_ident);
            });
        }

        #invalidation_registration
        #invalidation_callback_registration

        #[cfg(feature = "stats")]
        let __cache = GlobalCache::<#ret_type>::new(
            &#cache_ident,
            &#order_ident,
            #limit_expr,
            #max_memory_expr,
            #policy_expr,
            #ttl_expr,
            #frequency_weight_expr,
            &#stats_ident,
        );
        #[cfg(not(feature = "stats"))]
        let __cache = GlobalCache::<#ret_type>::new(
            &#cache_ident,
            &#order_ident,
            #limit_expr,
            #max_memory_expr,
            #policy_expr,
            #ttl_expr,
            #frequency_weight_expr,
        );

        let __key = #key_expr;
        if let Some(cached) = __cache.get(&__key) {
            #invalidation_check
        }

        let __result = (|| #block)();
        #cache_condition
        __result
    }
}

/// A procedural macro that adds automatic memoization to functions and methods.
///
/// This macro transforms a function into a cached version that stores results
/// in a thread-local HashMap based on the function arguments. Subsequent calls
/// with the same arguments will return the cached result instead of re-executing
/// the function body.
///
/// # Requirements
///
/// - **Arguments**: Must implement `CacheableKey` (or `DefaultCacheableKey` + `Debug`)
/// - **Return type**: Must implement `Clone` for cache storage and retrieval
/// - **Function purity**: For correct behavior, the function should be pure
///   (same inputs always produce same outputs with no side effects)
///
/// # Macro Parameters
///
/// - `limit` (optional): Maximum number of entries in the cache. When the limit is reached,
///   entries are evicted according to the specified policy. Default: unlimited.
/// - `max_memory` (optional): Maximum memory size for the cache (e.g., `"100MB"`, `"1GB"`).
///   When specified, entries are evicted based on memory usage. Requires implementing
///   `MemoryEstimator` trait for cached types. Default: None (no memory limit).
/// - `policy` (optional): Eviction policy to use when the cache is full. Options:
///   - `"fifo"` - First In, First Out (default)
///   - `"lru"` - Least Recently Used
///   - `"lfu"` - Least Frequently Used
///   - `"arc"` - Adaptive Replacement Cache (hybrid LRU/LFU)
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
/// - `scope` (optional): Cache scope - where the cache is stored. Options:
///   - `"global"` - Global storage shared across all threads (default, uses RwLock)
///   - `"thread"` - Thread-local storage (no synchronization overhead)
/// - `name` (optional): Custom identifier for the cache in the statistics registry.
///   Default: the function name. Useful when you want a more descriptive name or
///   when caching multiple versions of a function. Only relevant with `stats` feature.
/// - `tags` (optional): Array of tags for invalidation grouping (e.g., `tags = ["user_data", "profile"]`).
///   Enables tag-based cache invalidation. Only relevant with `scope = "global"`.
/// - `events` (optional): Array of event names that trigger invalidation (e.g., `events = ["user_updated"]`).
///   Enables event-driven cache invalidation. Only relevant with `scope = "global"`.
/// - `dependencies` (optional): Array of cache names this cache depends on (e.g., `dependencies = ["get_user"]`).
///   When dependencies are invalidated, this cache is also invalidated. Only relevant with `scope = "global"`.
/// - `invalidate_on` (optional): Function that checks if a cached entry should be invalidated.
///   Signature: `fn(key: &String, value: &T) -> bool`. Return `true` to invalidate.
///   The check runs on every cache access. Example: `invalidate_on = is_stale`.
/// - `cache_if` (optional): Function that determines if a result should be cached.
///   Signature: `fn(key: &String, value: &T) -> bool`. Return `true` to cache the result.
///   The check runs after computing the result but before caching it. Example: `cache_if = should_cache`.
///   When not specified, all results are cached (default behavior).
///
/// # Cache Behavior
///
/// - **Regular functions**: All results are cached
/// - **Result-returning functions**: Only `Ok` values are cached, `Err` values are not
/// - **Thread-local storage** (default): Each thread maintains its own independent cache
/// - **Global storage**: With `scope = "global"`, cache is shared across all threads
/// - **Methods**: Works with `self`, `&self`, and `&mut self` parameters
/// - **Eviction**: When limit is reached, entries are removed according to the policy
/// - **Expiration**: When TTL is set, expired entries are removed on access
///
/// # Examples
///
/// ## Basic Function Caching (Unlimited)
///
/// ```ignore
/// use cachelito::cache;
///
/// #[cache]
/// fn fibonacci(n: u32) -> u64 {
///     if n <= 1 {
///         return n as u64;
///     }
///     fibonacci(n - 1) + fibonacci(n - 2)
/// }
///
/// // First call computes and caches the result
/// let result1 = fibonacci(10);
/// // Subsequent calls return cached result (instant)
/// let result2 = fibonacci(10);
/// ```
///
/// ## Cache with Limit and FIFO Policy (Default)
///
/// ```ignore
/// use cachelito::cache;
///
/// #[cache(limit = 100)]
/// fn expensive_computation(x: i32) -> i32 {
///     // Cache will hold at most 100 entries
///     // Oldest entries are evicted first (FIFO)
///     x * x
/// }
/// ```
///
/// ## Cache with Limit and LRU Policy
///
/// ```ignore
/// use cachelito::cache;
///
/// #[cache(limit = 100, policy = "lru")]
/// fn expensive_computation(x: i32) -> i32 {
///     // Cache will hold at most 100 entries
///     // Least recently used entries are evicted first
///     x * x
/// }
/// ```
///
/// ## Cache with TTL (Time To Live)
///
/// ```ignore
/// use cachelito::cache;
///
/// #[cache(ttl = 60)]
/// fn fetch_user_data(user_id: u32) -> UserData {
///     // Cache expires after 60 seconds
///     // Expired entries are automatically removed
///     fetch_from_database(user_id)
/// }
/// ```
///
/// ## Combining All Features
///
/// ```ignore
/// use cachelito::cache;
///
/// #[cache(limit = 50, policy = "lru", ttl = 300)]
/// fn api_call(endpoint: &str) -> Result<Response, Error> {
///     // - Max 50 entries
///     // - LRU eviction
///     // - 5 minute TTL
///     // - Only Ok values cached
///     make_http_request(endpoint)
/// }
/// ```
///
/// ## Method Caching
///
/// ```ignore
/// use cachelito::cache;
///
/// #[derive(Debug, Clone)]
/// struct Calculator;
///
/// impl Calculator {
///     #[cache(limit = 50, policy = "lru", ttl = 60)]
///     fn compute(&self, x: f64, y: f64) -> f64 {
///         x.powf(y)
///     }
/// }
/// ```
///
/// ## Result Type Caching (Errors NOT Cached)
///
/// ```ignore
/// use cachelito::cache;
///
/// #[cache(limit = 10, ttl = 30)]
/// fn divide(a: i32, b: i32) -> Result<i32, String> {
///     if b == 0 {
///         Err("Division by zero".to_string())
///     } else {
///         Ok(a / b)
///     }
/// }
/// ```
///
/// ## Global Scope Cache (Shared Across Threads)
///
/// ```ignore
/// use cachelito::cache;
///
/// // Global cache (default) - shared across all threads
/// #[cache(limit = 100)]
/// fn global_computation(x: i32) -> i32 {
///     // Cache IS shared across all threads
///     // Uses RwLock for thread-safe access
///     x * x
/// }
///
/// // Thread-local cache - each thread has its own cache
/// #[cache(limit = 100, scope = "thread")]
/// fn thread_local_computation(x: i32) -> i32 {
///     // Cache is NOT shared across threads
///     x * x
/// }
/// ```
///
/// ## Custom Cache Name for Statistics
///
/// ```ignore
/// use cachelito::cache;
///
/// // Use a custom name for the cache in the statistics registry
/// #[cache(scope = "global", name = "user_api_v1")]
/// fn fetch_user(id: u32) -> User {
///     // The cache will be registered as "user_api_v1" instead of "fetch_user"
///     api_call(id)
/// }
///
/// #[cache(scope = "global", name = "user_api_v2")]
/// fn fetch_user_v2(id: u32) -> UserV2 {
///     // Different cache with its own statistics
///     new_api_call(id)
/// }
///
/// // Access statistics using the custom name
/// #[cfg(feature = "stats")]
/// {
///     if let Some(stats) = cachelito::stats_registry::get("user_api_v1") {
///         println!("V1 hit rate: {:.2}%", stats.hit_rate() * 100.0);
///     }
///     if let Some(stats) = cachelito::stats_registry::get("user_api_v2") {
///         println!("V2 hit rate: {:.2}%", stats.hit_rate() * 100.0);
///     }
/// }
/// ```
///
/// ## TLRU with Custom Frequency Weight
///
/// ```ignore
/// use cachelito::cache;
///
/// // Low frequency_weight (0.3) - emphasizes recency and age
/// // Good for time-sensitive data where freshness matters more than popularity
/// #[cache(
///     policy = "tlru",
///     limit = 100,
///     ttl = 300,
///     frequency_weight = 0.3
/// )]
/// fn fetch_realtime_data(source: String) -> Data {
///     // Fetch time-sensitive data
///     // Recent entries are preferred even if less frequently accessed
///     api_client.fetch(source)
/// }
///
/// // High frequency_weight (1.5) - emphasizes access frequency
/// // Good for popular content that should stay cached despite age
/// #[cache(
///     policy = "tlru",
///     limit = 100,
///     ttl = 300,
///     frequency_weight = 1.5
/// )]
/// fn fetch_popular_content(id: u64) -> Content {
///     // Frequently accessed entries remain cached longer
///     // Popular content is protected from eviction
///     database.fetch_content(id)
/// }
///
/// // Default behavior (balanced) - omit frequency_weight
/// #[cache(policy = "tlru", limit = 100, ttl = 300)]
/// fn fetch_balanced(key: String) -> Value {
///     // Balanced approach between recency and frequency
///     // Neither recency nor frequency dominates eviction decisions
///     expensive_operation(key)
/// }
/// ```
///
/// # Performance Considerations
///
/// - **Cache key generation**: Uses `CacheableKey::to_cache_key()` method
/// - **Thread-local storage**: Each thread has its own cache (no locks needed)
/// - **Global storage**: With `scope = "global"`, uses `parking_lot::RwLock` for concurrent reads
/// - **Memory usage**: Controlled by `limit` and/or `max_memory` parameters
/// - **FIFO overhead**: O(1) for all operations
/// - **LRU overhead**: O(n) for cache hits (reordering), O(1) for misses and evictions
/// - **LFU overhead**: O(n) for eviction (finding minimum frequency)
/// - **ARC overhead**: O(n) for cache operations (scoring and reordering)
/// - **Random overhead**: O(1) for eviction selection
/// - **TTL overhead**: O(1) expiration check on each get()
/// - **Memory estimation**: O(1) if `MemoryEstimator` is implemented efficiently
///
#[proc_macro_attribute]
pub fn cache(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse macro attributes
    let attrs = parse_attributes(attr);

    // Parse function
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let sig = &input.sig;
    let ident = &sig.ident;
    let block = &input.block;

    // Extract return type
    let ret_type = match &sig.output {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
    };

    // Parse arguments and detect self
    let mut arg_pats = Vec::new();
    let mut has_self = false;
    for arg in sig.inputs.iter() {
        match arg {
            FnArg::Receiver(_) => has_self = true,
            FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                arg_pats.push(quote! { #pat });
            }
        }
    }

    // Generate unique identifiers for static storage
    let cache_ident = format_ident!(
        "GLOBAL_OR_THREAD_CACHE_{}",
        ident.to_string().to_uppercase()
    );
    let order_ident = format_ident!(
        "GLOBAL_OR_THREAD_ORDER_{}",
        ident.to_string().to_uppercase()
    );
    let stats_ident = format_ident!(
        "GLOBAL_OR_THREAD_STATS_{}",
        ident.to_string().to_uppercase()
    );

    // Generate cache key expression
    let key_expr = generate_key_expr_with_cacheable_key(has_self, &arg_pats);

    // Detect Result type
    let is_result = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        s.starts_with("Result<") || s.starts_with("std::result::Result<")
    };

    // Use custom name if provided, otherwise use function name
    let fn_name_str = attrs
        .custom_name
        .clone()
        .unwrap_or_else(|| ident.to_string());

    // Generate thread-local and global cache branches
    let thread_local_branch = generate_thread_local_branch(
        &cache_ident,
        &order_ident,
        &ret_type,
        &attrs.limit,
        &attrs.max_memory,
        &attrs.policy,
        &attrs.ttl,
        &attrs.frequency_weight,
        &key_expr,
        block,
        is_result,
        &attrs.invalidate_on,
        &attrs.cache_if,
    );

    let global_branch = generate_global_branch(
        &cache_ident,
        &order_ident,
        &stats_ident,
        &ret_type,
        &attrs.limit,
        &attrs.max_memory,
        &attrs.policy,
        &attrs.ttl,
        &attrs.frequency_weight,
        &key_expr,
        block,
        &fn_name_str,
        is_result,
        &attrs,
    );

    // Generate final expanded code
    let scope_expr = &attrs.scope;
    let expanded = quote! {
        #vis #sig {
            use ::std::collections::VecDeque;
            use ::std::cell::RefCell;
            use ::cachelito_core::{CacheEntry, CacheScope, ThreadLocalCache, GlobalCache, CacheableKey};

            let __scope = #scope_expr;

            if __scope == cachelito_core::CacheScope::ThreadLocal {
                #thread_local_branch
            } else {
                #global_branch
            }

        }
    };

    TokenStream::from(expanded)
}
