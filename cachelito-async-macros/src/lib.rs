use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, ItemFn, MetaNameValue, Token};

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

/// Generate the cache hit logic (check and return cached value if valid)
fn generate_cache_hit_logic(
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    stats_ident: &syn::Ident,
    limit_expr: &TokenStream2,
    policy_expr: &TokenStream2,
    ttl_expr: &TokenStream2,
) -> TokenStream2 {
    quote! {
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
                // Verify key still exists to avoid orphaned keys in the order queue
                if let Some(__limit) = #limit_expr {
                    if #policy_expr == "lru" && #cache_ident.contains_key(&__key) {
                        let mut __order = #order_ident.lock();
                        // Double-check after acquiring lock
                        if #cache_ident.contains_key(&__key) {
                            __order.retain(|k| k != &__key);
                            __order.push_back(__key.clone());
                        }
                    }
                }

                return __cached_value;
            }

            // Expired - remove and continue to execute
            drop(__entry_ref);
            #cache_ident.remove(&__key);

            // Also remove from order queue to prevent orphaned keys
            let mut __order = #order_ident.lock();
            __order.retain(|k| k != &__key);
        }

        // Record cache miss
        #stats_ident.record_miss();
    }
}

/// Generate the cache insert logic (evict if needed, update order, insert)
fn generate_cache_insert_logic(
    cache_ident: &syn::Ident,
    order_ident: &syn::Ident,
    limit_expr: &TokenStream2,
    policy_expr: &TokenStream2,
) -> TokenStream2 {
    quote! {
        let __timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Handle limit and update order - acquire lock first to ensure atomicity
        if let Some(__limit) = #limit_expr {
            let mut __order = #order_ident.lock();

            // Check if another task already inserted this key while we were computing
            if #cache_ident.contains_key(&__key) {
                // Key already exists, just update the order if LRU
                if #policy_expr == "lru" {
                    __order.retain(|k| k != &__key);
                    __order.push_back(__key.clone());
                }
                // Don't insert again, return the computed result
                drop(__order);
                return __result;
            }

            // Check limit after acquiring lock to prevent race condition
            if #cache_ident.len() >= __limit {
                // Keep trying until we find a valid entry to evict
                while let Some(__evict_key) = __order.pop_front() {
                    if #cache_ident.contains_key(&__evict_key) {
                        #cache_ident.remove(&__evict_key);
                        break;
                    }
                    // Key doesn't exist in cache (already removed), try next one
                }
            }

            // Update order for the new entry
            if #policy_expr == "lru" {
                // Remove and re-add to mark as recently used
                __order.retain(|k| k != &__key);
            }
            __order.push_back(__key.clone());

            // Insert while still holding lock to ensure atomicity
            drop(__order);
            #cache_ident.insert(__key, (__result.clone(), __timestamp));
        } else {
            // No limit, just insert
            #cache_ident.insert(__key, (__result.clone(), __timestamp));
        }
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

    // Detect Result type
    let is_result = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        s.starts_with("Result<") || s.starts_with("std::result::Result<")
    };

    let limit_expr = &attrs.limit;
    let policy_expr = &attrs.policy;
    let ttl_expr = &attrs.ttl;

    // Generate cache hit logic (shared between Result and non-Result)
    let cache_hit_logic = generate_cache_hit_logic(
        &cache_ident,
        &order_ident,
        &stats_ident,
        limit_expr,
        policy_expr,
        ttl_expr,
    );

    // Generate cache insert logic (shared between Result and non-Result)
    let cache_insert_logic =
        generate_cache_insert_logic(&cache_ident, &order_ident, limit_expr, policy_expr);

    // Generate cache logic based on Result or regular return
    let cache_logic = if is_result {
        quote! {
            let __key = #key_expr;

            #cache_hit_logic

            // Execute original async function (cache miss or expired)
            let __result = (async #block).await;

            // Only cache Ok values
            if let Ok(_) = __result {
                #cache_insert_logic
            }

            __result
        }
    } else {
        quote! {
            let __key = #key_expr;

            #cache_hit_logic

            // Execute original async function (cache miss or expired)
            let __result = (async #block).await;

            #cache_insert_logic

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
