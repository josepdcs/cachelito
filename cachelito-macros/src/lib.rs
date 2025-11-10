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

/// Generate the thread-local cache branch
fn generate_thread_local_branch(
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    ret_type: &TokenStream2,
    limit_expr: &TokenStream2,
    policy_expr: &TokenStream2,
    ttl_expr: &TokenStream2,
    key_expr: &TokenStream2,
    block: &syn::Block,
    is_result: bool,
) -> TokenStream2 {
    let insert_call = if is_result {
        quote! { __cache.insert_result(&__key, &__result); }
    } else {
        quote! { __cache.insert(&__key, __result.clone()); }
    };

    quote! {
        thread_local! {
            static #cache_ident: RefCell<std::collections::HashMap<String, CacheEntry<#ret_type>>> = RefCell::new(std::collections::HashMap::new());
            static #order_ident: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
        }

        let __cache = ThreadLocalCache::<#ret_type>::new(
            &#cache_ident,
            &#order_ident,
            #limit_expr,
            #policy_expr,
            #ttl_expr
        );

        let __key = #key_expr;

        if let Some(cached) = __cache.get(&__key) {
            return cached;
        }

        let __result = (|| #block)();
        #insert_call
        __result
    }
}

/// Generate the global cache branch
fn generate_global_branch(
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    stats_ident: &syn::Ident,
    ret_type: &TokenStream2,
    limit_expr: &TokenStream2,
    policy_expr: &TokenStream2,
    ttl_expr: &TokenStream2,
    key_expr: &TokenStream2,
    block: &syn::Block,
    fn_name_str: &str,
    is_result: bool,
) -> TokenStream2 {
    let insert_call = if is_result {
        quote! { __cache.insert_result(&__key, &__result); }
    } else {
        quote! { __cache.insert(&__key, __result.clone()); }
    };

    quote! {
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

        #[cfg(feature = "stats")]
        let __cache = GlobalCache::<#ret_type>::new(
            &#cache_ident,
            &#order_ident,
            #limit_expr,
            #policy_expr,
            #ttl_expr,
            &#stats_ident,
        );
        #[cfg(not(feature = "stats"))]
        let __cache = GlobalCache::<#ret_type>::new(
            &#cache_ident,
            &#order_ident,
            #limit_expr,
            #policy_expr,
            #ttl_expr,
        );

        let __key = #key_expr;
        if let Some(cached) = __cache.get(&__key) {
            return cached;
        }

        let __result = (|| #block)();
        #insert_call
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
/// - `policy` (optional): Eviction policy to use when the cache is full. Options:
///   - `"fifo"` - First In, First Out (default)
///   - `"lru"` - Least Recently Used
/// - `ttl` (optional): Time-to-live in seconds. Entries older than this will be
///   automatically removed when accessed. Default: None (no expiration).
/// - `scope` (optional): Cache scope - where the cache is stored. Options:
///   - `"global"` - Global storage shared across all threads (default, uses RwLock)
///   - `"thread"` - Thread-local storage (no synchronization overhead)
/// - `name` (optional): Custom identifier for the cache in the statistics registry.
///   Default: the function name. Useful when you want a more descriptive name or
///   when caching multiple versions of a function. Only relevant with `stats` feature.
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
/// # Performance Considerations
///
/// - **Cache key generation**: Uses `CacheableKey::to_cache_key()` method
/// - **Thread-local storage** (default): Each thread has its own cache (no locks needed)
/// - **Global storage**: With `scope = "global"`, uses Mutex for synchronization (adds overhead)
/// - **Memory usage**: Controlled by the `limit` parameter
/// - **FIFO overhead**: O(1) for all operations
/// - **LRU overhead**: O(n) for cache hits (reordering), O(1) for misses and evictions
/// - **TTL overhead**: O(1) expiration check on each get()
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
    let fn_name_str = attrs.custom_name.unwrap_or_else(|| ident.to_string());

    // Generate thread-local and global cache branches
    let thread_local_branch = generate_thread_local_branch(
        &cache_ident,
        &order_ident,
        &ret_type,
        &attrs.limit,
        &attrs.policy,
        &attrs.ttl,
        &key_expr,
        block,
        is_result,
    );

    let global_branch = generate_global_branch(
        &cache_ident,
        &order_ident,
        &stats_ident,
        &ret_type,
        &attrs.limit,
        &attrs.policy,
        &attrs.ttl,
        &key_expr,
        block,
        &fn_name_str,
        is_result,
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
