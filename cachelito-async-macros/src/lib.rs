use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, FnArg, ItemFn, MetaNameValue, ReturnType, Token,
};

// Import shared utilities from cachelito-macro-utils
use cachelito_macro_utils::{
    generate_key_expr, parse_limit_attribute, parse_name_attribute, parse_policy_attribute,
    parse_ttl_attribute,
};

/// Parsed macro attributes
struct CacheAttributes {
    limit: TokenStream2,
    policy: TokenStream2,
    ttl: TokenStream2,
    custom_name: Option<String>,
}

impl Default for CacheAttributes {
    fn default() -> Self {
        Self {
            limit: quote! { Option::<usize>::None },
            policy: quote! { "fifo" },
            ttl: quote! { Option::<u64>::None },
            custom_name: None,
        }
    }
}

/// Parse macro attributes from the attribute token stream
fn parse_attributes(attr: TokenStream) -> CacheAttributes {
    use syn::parse::Parser;

    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let parsed_args = parser.parse(attr).unwrap_or_default();
    let mut attrs = CacheAttributes::default();

    for nv in parsed_args {
        if nv.path.is_ident("limit") {
            attrs.limit = parse_limit_attribute(&nv);
        } else if nv.path.is_ident("policy") {
            attrs.policy = parse_policy_attribute(&nv);
        } else if nv.path.is_ident("ttl") {
            attrs.ttl = parse_ttl_attribute(&nv);
        } else if nv.path.is_ident("name") {
            attrs.custom_name = parse_name_attribute(&nv);
        }
    }

    attrs
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
///   - `"fifo"` - First In, First Out (default)
///   - `"lru"` - Least Recently Used
/// - `ttl` (optional): Time-to-live in seconds. Entries older than this will be
///   automatically removed when accessed. Default: None (no expiration).
/// - `name` (optional): Custom identifier for the cache. Default: the function name.
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
///
/// // First call fetches and caches
/// let user1 = fetch_user(42).await;
/// // Second call returns cached result (instant)
/// let user2 = fetch_user(42).await;
/// ```
///
/// ## Cache with Limit and LRU Policy
///
/// ```ignore
/// use cachelito_async::cache_async;
///
/// #[cache_async(limit = 100, policy = "lru")]
/// async fn expensive_async_computation(x: i32) -> i32 {
///     tokio::time::sleep(Duration::from_millis(100)).await;
///     x * x
/// }
/// ```
///
/// ## Cache with TTL (Time To Live)
///
/// ```ignore
/// use cachelito_async::cache_async;
///
/// #[cache_async(ttl = 60)]
/// async fn fetch_data(endpoint: &str) -> Result<Data, Error> {
///     // Cache expires after 60 seconds
///     make_http_request(endpoint).await
/// }
/// ```
///
/// ## Combining All Features
///
/// ```ignore
/// use cachelito_async::cache_async;
///
/// #[cache_async(limit = 50, policy = "lru", ttl = 300, name = "api_v1")]
/// async fn api_call(endpoint: String) -> Result<Response, Error> {
///     // - Max 50 entries
///     // - LRU eviction
///     // - 5 minute TTL
///     // - Only Ok values cached
///     make_request(&endpoint).await
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
    // Parse macro attributes
    let attrs = parse_attributes(attr);

    // Parse function
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let sig = &input.sig;
    let ident = &sig.ident;
    let block = &input.block;

    // Verify that function is async
    if sig.asyncness.is_none() {
        return quote! {
            compile_error!("cache_async can only be used with async functions");
        }
        .into();
    }

    // Extract return type (strip Future wrapper if present)
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
    let cache_ident = format_ident!("ASYNC_CACHE_{}", ident.to_string().to_uppercase());
    let order_ident = format_ident!("ASYNC_ORDER_{}", ident.to_string().to_uppercase());
    let stats_ident = format_ident!("ASYNC_STATS_{}", ident.to_string().to_uppercase());

    // Determine cache name (custom or function name)
    let fn_name_string = ident.to_string();
    let fn_name_str = attrs.custom_name.as_ref().unwrap_or(&fn_name_string);

    // Generate cache key expression
    let key_expr = generate_key_expr(has_self, &arg_pats);

    // Detect Result type
    let is_result = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        s.starts_with("Result<") || s.starts_with("std::result::Result<")
    };

    let limit_expr = &attrs.limit;
    let policy_expr = &attrs.policy;
    let ttl_expr = &attrs.ttl;

    // Generate cache insert based on Result or regular return
    let cache_logic = if is_result {
        quote! {
            let __key = #key_expr;

            // Check cache first - return early if valid cached value exists
            if let Some(__entry_ref) = #cache_ident.get(&__key) {
                let __now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let __is_expired = if let Some(__ttl) = #ttl_expr {
                    __now - __entry_ref.1 > __ttl
                } else {
                    false
                };

                if !__is_expired {
                    let __cached_value = __entry_ref.0.clone();
                    drop(__entry_ref);

                    // Record cache hit
                    #stats_ident.record_hit();

                    // Update LRU order on cache hit
                    if let Some(__limit) = #limit_expr {
                        if #policy_expr == "lru" {
                            let mut __order = #order_ident.lock();
                            __order.retain(|k| k != &__key);
                            __order.push_back(__key.clone());
                        }
                    }

                    return __cached_value;
                }

                // Expired - remove and continue to execute
                drop(__entry_ref);
                #cache_ident.remove(&__key);
            }

            // Record cache miss
            #stats_ident.record_miss();

            // Execute original async function (cache miss or expired)
            let __result = (async #block).await;

            // Only cache Ok values
            if let Ok(ref __ok_value) = __result {
                let __timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Handle limit
                if let Some(__limit) = #limit_expr {
                    if #cache_ident.len() >= __limit && !#cache_ident.contains_key(&__key) {
                        // Evict based on policy - keep trying until we find a valid entry
                        let mut __order = #order_ident.lock();
                        while let Some(__evict_key) = __order.pop_front() {
                            if #cache_ident.contains_key(&__evict_key) {
                                #cache_ident.remove(&__evict_key);
                                break;
                            }
                            // Key doesn't exist in cache (already removed), try next one
                        }
                    }

                    // Update order
                    let mut __order = #order_ident.lock();
                    if #policy_expr == "lru" {
                        // Remove and re-add to mark as recently used
                        __order.retain(|k| k != &__key);
                    }
                    __order.push_back(__key.clone());
                }

                #cache_ident.insert(__key, (__result.clone(), __timestamp));
            }

            __result
        }
    } else {
        quote! {
            let __key = #key_expr;

            // Check cache first - return early if valid cached value exists
            if let Some(__entry_ref) = #cache_ident.get(&__key) {
                let __now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let __is_expired = if let Some(__ttl) = #ttl_expr {
                    __now - __entry_ref.1 > __ttl
                } else {
                    false
                };

                if !__is_expired {
                    let __cached_value = __entry_ref.0.clone();
                    drop(__entry_ref);

                    // Record cache hit
                    #stats_ident.record_hit();

                    // Update LRU order on cache hit
                    if let Some(__limit) = #limit_expr {
                        if #policy_expr == "lru" {
                            let mut __order = #order_ident.lock();
                            __order.retain(|k| k != &__key);
                            __order.push_back(__key.clone());
                        }
                    }

                    return __cached_value;
                }

                // Expired - remove and continue to execute
                drop(__entry_ref);
                #cache_ident.remove(&__key);
            }

            // Record cache miss
            #stats_ident.record_miss();

            // Execute original async function (cache miss or expired)
            let __result = (async #block).await;

            let __timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Handle limit
            if let Some(__limit) = #limit_expr {
                if #cache_ident.len() >= __limit && !#cache_ident.contains_key(&__key) {
                    // Evict based on policy - keep trying until we find a valid entry
                    let mut __order = #order_ident.lock();
                    while let Some(__evict_key) = __order.pop_front() {
                        if #cache_ident.contains_key(&__evict_key) {
                            #cache_ident.remove(&__evict_key);
                            break;
                        }
                        // Key doesn't exist in cache (already removed), try next one
                    }
                }

                // Update order
                let mut __order = #order_ident.lock();
                if #policy_expr == "lru" {
                    // Remove and re-add to mark as recently used
                    __order.retain(|k| k != &__key);
                }
                __order.push_back(__key.clone());
            }

            #cache_ident.insert(__key, (__result.clone(), __timestamp));
            __result
        }
    };

    // Generate final expanded code
    let expanded = quote! {
        #vis #sig {
            use std::collections::VecDeque;

            static #cache_ident: once_cell::sync::Lazy<dashmap::DashMap<String, (#ret_type, u64)>> =
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

            #cache_logic
        }
    };

    TokenStream::from(expanded)
}
