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
/// - `ttl` (optional): Time-to-live in seconds. Entries older than this will be
///   automatically removed when accessed. Default: None (no expiration).
/// - `name` (optional): Custom identifier for the cache. Default: the function name.
/// - `max_memory` (optional): Maximum memory usage (e.g., "100MB", "1GB"). Requires
///   the return type to implement `MemoryEstimator`.
/// - `tags` (optional): Array of tags for group invalidation. Example: `["user_data", "profile"]`
/// - `events` (optional): Array of events that trigger invalidation. Example: `["user_updated"]`
/// - `dependencies` (optional): Array of cache dependencies. Example: `["get_user"]`
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
/// use cachelito_core::invalidate_by_tag;
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
    let policy_str = &attrs.policy;
    let ttl_expr = &attrs.ttl;
    let max_memory_expr = &attrs.max_memory;

    // Convert policy string to EvictionPolicy
    let policy_expr = quote! {
        cachelito_core::EvictionPolicy::from(#policy_str)
    };

    // Generate cache logic based on Result or regular return
    let cache_logic = if is_result {
        // Check if we need memory-aware insertion
        // max_memory is a TokenStream2, check if it represents None
        let max_memory_str = max_memory_expr.to_string();

        let insert_call = if !max_memory_str.contains("None") {
            quote! { __cache.insert_with_memory(&__key, __ok_value.clone()); }
        } else {
            quote! { __cache.insert(&__key, __ok_value.clone()); }
        };

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
                &*#stats_ident,
            );

            // Try to get from cache
            if let Some(__cached) = __cache.get(&__key) {
                return __cached;
            }

            // Execute original async function (cache miss or expired)
            let __result = (async #block).await;

            // Only cache Ok values
            if let Ok(ref __ok_value) = __result {
                #insert_call
            }

            __result
        }
    } else {
        // Check if we need memory-aware insertion
        // max_memory is a TokenStream2, check if it represents None
        let max_memory_str = max_memory_expr.to_string();

        let insert_call = if !max_memory_str.contains("None") {
            quote! { __cache.insert_with_memory(&__key, __result.clone()); }
        } else {
            quote! { __cache.insert(&__key, __result.clone()); }
        };

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
                &*#stats_ident,
            );

            // Try to get from cache
            if let Some(__cached) = __cache.get(&__key) {
                return __cached;
            }

            // Execute original async function (cache miss or expired)
            let __result = (async #block).await;

            // Cache the result
            #insert_call

            __result
        }
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

            #cache_logic
        }
    };

    TokenStream::from(expanded)
}
