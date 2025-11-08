use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parser;

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
///   - `"thread"` - Thread-local storage (default, no synchronization overhead)
///   - `"global"` - Global storage shared across all threads (uses Mutex)
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
/// // Thread-local cache (default) - each thread has its own cache
/// #[cache(limit = 100)]
/// fn thread_local_computation(x: i32) -> i32 {
///     x * x
/// }
///
/// // Global cache - shared across all threads
/// #[cache(limit = 100, scope = "global")]
/// fn global_computation(x: i32) -> i32 {
///     // Uses Mutex for thread-safe access
///     x * x
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
/// # Version History
///
/// ## Version 0.4.0 (Current)
/// - Added `scope` parameter for global cache across threads
/// - Global cache support with Mutex synchronization
/// - Enhanced documentation with global scope examples
///
/// ## Version 0.3.0
/// - Added `ttl` parameter for time-to-live expiration
/// - Automatic removal of expired entries
/// - Enhanced documentation with TTL examples
///
/// ## Version 0.2.0
/// - Added `limit` parameter for cache size control
/// - Added `policy` parameter with FIFO and LRU support
/// - Enhanced documentation with examples
///
/// ## Version 0.1.0
/// - Initial release with basic caching functionality
///
#[proc_macro_attribute]
pub fn cache(attr: TokenStream, item: TokenStream) -> TokenStream {
    use proc_macro2::TokenStream as TokenStream2;
    use quote::{format_ident, quote};
    use syn::{
        parse_macro_input, punctuated::Punctuated, Expr, FnArg, ItemFn, MetaNameValue, ReturnType,
        Token,
    };

    // Parse attributes: limit, policy, ttl (seconds), scope ("thread"|"global")
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let parsed_args = parser.parse(attr).unwrap_or_default();

    let mut limit_expr = quote! { None };
    let mut policy_expr = quote! { cachelito_core::EvictionPolicy::FIFO };
    let mut ttl_expr = quote! { None };
    let mut scope_expr = quote! { cachelito_core::CacheScope::ThreadLocal };

    for nv in parsed_args {
        if nv.path.is_ident("limit") {
            match nv.value {
                Expr::Lit(ref expr_lit) => match expr_lit.lit {
                    syn::Lit::Int(ref lit_int) => {
                        let val = lit_int
                            .base10_parse::<usize>()
                            .expect("limit must be a positive integer");
                        limit_expr = quote! { Some(#val) };
                    }
                    _ => {
                        return quote! {
                            compile_error!("Invalid literal for `limit`: expected integer");
                        }
                        .into();
                    }
                },
                _ => {
                    return quote! {
                        compile_error!("Invalid syntax for `limit`: expected `limit = <integer>`");
                    }
                    .into();
                }
            }
        } else if nv.path.is_ident("policy") {
            match nv.value {
                Expr::Lit(ref expr_lit) => match &expr_lit.lit {
                    syn::Lit::Str(s) => {
                        let pol = match s.value().to_lowercase().as_str() {
                            "fifo" => quote! { cachelito_core::EvictionPolicy::FIFO },
                            "lru" => quote! { cachelito_core::EvictionPolicy::LRU },
                            _ => {
                                return quote! {
                                    compile_error!("Invalid policy: expected \"fifo\" or \"lru\"");
                                }
                                .into();
                            }
                        };
                        policy_expr = pol;
                    }
                    _ => {
                        return quote! {
                            compile_error!("Invalid literal for `policy`: expected string");
                        }
                        .into();
                    }
                },
                _ => {
                    return quote! {
                        compile_error!("Invalid syntax for `policy`: expected `policy = \"fifo\"|\"lru\"`");
                    }
                        .into();
                }
            }
        } else if nv.path.is_ident("ttl") {
            match nv.value {
                Expr::Lit(ref expr_lit) => match expr_lit.lit {
                    syn::Lit::Int(ref lit_int) => {
                        let val = lit_int
                            .base10_parse::<u64>()
                            .expect("ttl must be a positive integer (seconds)");
                        ttl_expr = quote! { Some(#val) };
                    }
                    _ => {
                        return quote! {
                            compile_error!("Invalid literal for `ttl`: expected integer (seconds)");
                        }
                        .into();
                    }
                },
                _ => {
                    return quote! {
                        compile_error!("Invalid syntax for `ttl`: expected `ttl = <integer>`");
                    }
                    .into();
                }
            }
        } else if nv.path.is_ident("scope") {
            match nv.value {
                Expr::Lit(ref expr_lit) => match &expr_lit.lit {
                    syn::Lit::Str(s) => {
                        let val = s.value().to_lowercase();
                        match val.as_str() {
                            "thread" => {
                                scope_expr = quote! { cachelito_core::CacheScope::ThreadLocal };
                            }
                            "global" => {
                                scope_expr = quote! { cachelito_core::CacheScope::Global };
                            }
                            other => {
                                return quote! {
                                    compile_error!(concat!("Invalid scope: ", #other));
                                }
                                .into();
                            }
                        }
                    }
                    lit => {
                        return quote! {
                            compile_error!(concat!("Invalid literal for `scope`: ", stringify!(#lit)));
                        }
                            .into();
                    }
                },
                _ => {
                    return quote! {
                        compile_error!("Invalid syntax for `scope`");
                    }
                    .into();
                }
            }
        }
    }

    // === parse function ===
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let sig = &input.sig;
    let ident = &sig.ident;
    let block = &input.block;

    let ret_type = match &sig.output {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
    };

    // arguments, self detection
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

    // idents for per-function storages
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

    let key_expr: TokenStream2 = if has_self {
        if arg_pats.is_empty() {
            quote! {{
                use cachelito_core::CacheableKey;
                self.to_cache_key()
            }}
        } else {
            quote! {{
                use cachelito_core::CacheableKey;
                let mut __key_parts = Vec::new();
                __key_parts.push(self.to_cache_key());
                #(
                    __key_parts.push((#arg_pats).to_cache_key());
                )*
                __key_parts.join("|")
            }}
        }
    } else if arg_pats.is_empty() {
        quote! {{ String::new() }}
    } else {
        quote! {{
            use cachelito_core::CacheableKey;
            let mut __key_parts = Vec::new();
            #(
                __key_parts.push((#arg_pats).to_cache_key());
            )*
            __key_parts.join("|")
        }}
    };

    // detect Result<...>
    let is_result = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        s.starts_with("Result<") || s.starts_with("std::result::Result<")
    };

    // For global scope we will generate `static` Lazy Mutex maps; for thread we use thread_local!
    // We use cachelito_core::CacheEntry<#ret_type> as stored value in both cases.

    // Generate stats accessor function name
    let stats_fn_name = format_ident!("{}_stats", ident);

    let expanded = if is_result {
        quote! {
            #vis #sig {
                use ::std::sync::LazyLock;
                use ::std::collections::VecDeque;
                use ::std::cell::RefCell;
                use ::cachelito_core::{CacheEntry, CacheScope, ThreadLocalCache, GlobalCache, CacheableKey};

                // choose storage depending on scope
                let __scope = #scope_expr;

                if __scope == cachelito_core::CacheScope::ThreadLocal {
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
                        // cached is Result<T,E>
                        return cached;
                    }

                    let __result = (|| #block)();
                    __cache.insert_result(&__key, &__result);
                    __result
                } else {
                    static #cache_ident: once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<String, CacheEntry<#ret_type>>>> = once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
                    static #order_ident: once_cell::sync::Lazy<parking_lot::Mutex<VecDeque<String>>> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(VecDeque::new()));

                    #[cfg(feature = "stats")]
                    static #stats_ident: once_cell::sync::Lazy<cachelito_core::CacheStats> = once_cell::sync::Lazy::new(|| cachelito_core::CacheStats::new());

                    let __cache = GlobalCache::<#ret_type>::new(
                        &#cache_ident,
                        &#order_ident,
                        #limit_expr,
                        #policy_expr,
                        #ttl_expr,
                        #[cfg(feature = "stats")]
                        &#stats_ident,
                    );

                    let __key = #key_expr;

                    if let Some(cached) = __cache.get(&__key) {
                        return cached;
                    }

                    let __result = (|| #block)();
                    __cache.insert_result(&__key, &__result);
                    __result
                }
            }

            /// Returns a reference to the cache statistics (only available with `stats` feature).
            #[cfg(feature = "stats")]
            #[allow(dead_code)]
            pub fn #stats_fn_name() -> cachelito_core::CacheStats {
                use ::cachelito_core::CacheScope;

                let __scope = #scope_expr;

                if __scope == cachelito_core::CacheScope::ThreadLocal {
                    // For thread-local, we need to access the cache instance and clone stats
                    use ::std::collections::VecDeque;
                    use ::std::cell::RefCell;
                    use ::cachelito_core::{CacheEntry, ThreadLocalCache};

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

                    __cache.stats().clone()
                } else {
                    // For global, clone the Lazy static
                    static #cache_ident: once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<String, cachelito_core::CacheEntry<#ret_type>>>> = once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
                    static #order_ident: once_cell::sync::Lazy<parking_lot::Mutex<std::collections::VecDeque<String>>> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(std::collections::VecDeque::new()));
                    static #stats_ident: once_cell::sync::Lazy<cachelito_core::CacheStats> = once_cell::sync::Lazy::new(|| cachelito_core::CacheStats::new());

                    (*#stats_ident).clone()
                }
            }
        }
    } else {
        quote! {
            #vis #sig {
                use ::std::collections::VecDeque;
                use ::std::cell::RefCell;
                use ::cachelito_core::{CacheEntry, CacheScope, ThreadLocalCache, GlobalCache, CacheableKey};

                let __scope = #scope_expr;

                if __scope == cachelito_core::CacheScope::ThreadLocal {
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
                    __cache.insert(&__key, __result.clone());
                    __result
                } else {
                    static #cache_ident: once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<String, CacheEntry<#ret_type>>>> = once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
                    static #order_ident: once_cell::sync::Lazy<parking_lot::Mutex<VecDeque<String>>> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(VecDeque::new()));

                    #[cfg(feature = "stats")]
                    static #stats_ident: once_cell::sync::Lazy<cachelito_core::CacheStats> = once_cell::sync::Lazy::new(|| cachelito_core::CacheStats::new());

                    let __cache = GlobalCache::<#ret_type>::new(
                        &#cache_ident,
                        &#order_ident,
                        #limit_expr,
                        #policy_expr,
                        #ttl_expr,
                        #[cfg(feature = "stats")]
                        &#stats_ident,
                    );

                    let __key = #key_expr;
                    if let Some(cached) = __cache.get(&__key) {
                        return cached;
                    }

                    let __result = (|| #block)();
                    __cache.insert(&__key, __result.clone());
                    __result
                }
            }

            /// Returns a reference to the cache statistics (only available with `stats` feature).
            #[cfg(feature = "stats")]
            #[allow(dead_code)]
            pub fn #stats_fn_name() -> cachelito_core::CacheStats {
                use ::cachelito_core::CacheScope;

                let __scope = #scope_expr;

                if __scope == cachelito_core::CacheScope::ThreadLocal {
                    // For thread-local, we need to access the cache instance and clone stats
                    use ::std::collections::VecDeque;
                    use ::std::cell::RefCell;
                    use ::cachelito_core::{CacheEntry, ThreadLocalCache};

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

                    __cache.stats().clone()
                } else {
                    // For global, clone the Lazy static
                    static #cache_ident: once_cell::sync::Lazy<parking_lot::RwLock<std::collections::HashMap<String, cachelito_core::CacheEntry<#ret_type>>>> = once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
                    static #order_ident: once_cell::sync::Lazy<parking_lot::Mutex<std::collections::VecDeque<String>>> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(std::collections::VecDeque::new()));
                    static #stats_ident: once_cell::sync::Lazy<cachelito_core::CacheStats> = once_cell::sync::Lazy::new(|| cachelito_core::CacheStats::new());

                    (*#stats_ident).clone()
                }
            }
        }
    };

    TokenStream::from(expanded)
}
