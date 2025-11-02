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
///
/// # Cache Behavior
///
/// - **Regular functions**: All results are cached
/// - **Result-returning functions**: Only `Ok` values are cached, `Err` values are not
/// - **Thread-local storage**: Each thread maintains its own independent cache
/// - **Methods**: Works with `self`, `&self`, and `&mut self` parameters
/// - **Eviction**: When limit is reached, entries are removed according to the policy
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
/// ## Method Caching
///
/// ```ignore
/// use cachelito::cache;
///
/// #[derive(Debug, Clone)]
/// struct Calculator;
///
/// impl Calculator {
///     #[cache(limit = 50, policy = "lru")]
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
/// #[cache(limit = 10)]
/// fn divide(a: i32, b: i32) -> Result<i32, String> {
///     if b == 0 {
///         Err("Division by zero".to_string())
///     } else {
///         Ok(a / b)
///     }
/// }
/// ```
///
/// # Performance Considerations
///
/// - **Cache key generation**: Uses `CacheableKey::to_cache_key()` method
/// - **Thread-local storage**: Each thread has its own cache (no locks needed)
/// - **Memory usage**: Controlled by the `limit` parameter
/// - **FIFO overhead**: O(1) for all operations
/// - **LRU overhead**: O(n) for cache hits (reordering), O(1) for misses and evictions
///
/// # Version History
///
/// ## Version 0.2.0 (Current)
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

    // Parse attributes: #[cache(limit = 100, policy = "lru", ttl = 60)]
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let parsed_args = parser.parse(attr).unwrap_or_default();

    let mut limit_expr = quote! { None };
    let mut policy_expr = quote! { cachelito_core::EvictionPolicy::FIFO };
    let mut ttl_expr = quote! { None };

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
                        ttl_expr = quote! { Some(std::time::Duration::from_secs(#val)) };
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

    let cache_ident = format_ident!("_CACHE_{}_MAP", ident.to_string().to_uppercase());
    let order_ident = format_ident!("_CACHE_{}_ORDER", ident.to_string().to_uppercase());

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

    let is_result = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        s.starts_with("Result<") || s.starts_with("std::result::Result<")
    };

    // === Expanded function ===
    let expanded = if is_result {
        quote! {
            #vis #sig {
                use ::std::collections::{HashMap, VecDeque};
                use ::std::cell::RefCell;
                use ::cachelito_core::{ThreadLocalCache, CacheableKey};

                thread_local! {
                    static #cache_ident: RefCell<HashMap<String, cachelito_core::CacheEntry<#ret_type>>> = RefCell::new(HashMap::new());
                    static #order_ident: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
                }

                let __key = #key_expr;
                let __cache = ThreadLocalCache::<#ret_type>::new(
                    &#cache_ident,
                    &#order_ident,
                    #limit_expr,
                    #policy_expr,
                    #ttl_expr
                );

                if let Some(cached) = __cache.get(&__key) {
                    if let Ok(val) = cached.clone() {
                        return Ok(val);
                    }
                }

                let __result = (|| #block)();
                __cache.insert_result(&__key, &__result);
                __result
            }
        }
    } else {
        quote! {
            #vis #sig {
                use ::std::collections::{HashMap, VecDeque};
                use ::std::cell::RefCell;
                use ::cachelito_core::{ThreadLocalCache, CacheableKey};

                thread_local! {
                    static #cache_ident: RefCell<HashMap<String, cachelito_core::CacheEntry<#ret_type>>> = RefCell::new(HashMap::new());
                    static #order_ident: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
                }

                let __key = #key_expr;
                let __cache = ThreadLocalCache::<#ret_type>::new(
                    &#cache_ident,
                    &#order_ident,
                    #limit_expr,
                    #policy_expr,
                    #ttl_expr
                );

                if let Some(cached) = __cache.get(&__key) {
                    return cached;
                }

                let __result = (|| #block)();
                __cache.insert(&__key, __result.clone());
                __result
            }
        }
    };

    TokenStream::from(expanded)
}
