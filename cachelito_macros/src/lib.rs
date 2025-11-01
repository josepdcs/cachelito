use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Expr, FnArg, ItemFn, MetaNameValue, ReturnType, Token};

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
/// # Cache Behavior
///
/// - **Regular functions**: All results are cached
/// - **Result-returning functions**: Only `Ok` values are cached, `Err` values are not
/// - **Thread-local storage**: Each thread maintains its own independent cache
/// - **Methods**: Works with `self`, `&self`, and `&mut self` parameters
///
/// # Examples
///
/// ## Basic Function Caching
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
/// ## Method Caching
///
/// ```ignore
/// use cachelito::cache;
///
/// #[derive(Debug, Clone)]
/// struct Calculator;
///
/// impl Calculator {
///     #[cache]
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
/// #[cache]
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
/// - **Thread-local storage**: Each thread has its own cache
/// - **Memory usage**: The cache grows unbounded
///
#[proc_macro_attribute]
pub fn cache(attr: TokenStream, item: TokenStream) -> TokenStream {
    use proc_macro2::TokenStream as TokenStream2;
    use quote::{format_ident, quote};
    use syn::{
        parse_macro_input, punctuated::Punctuated, Expr, FnArg, ItemFn, MetaNameValue, ReturnType,
        Token,
    };

    // Parse attributes: e.g. #[cache(limit = 100, policy = "lru")]
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let parsed_args = parser.parse(attr).unwrap_or_default();

    let mut limit_expr = quote! { None };
    let mut policy_expr = quote! { cachelito_core::EvictionPolicy::FIFO };

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
                    ref lit => {
                        return quote! {
                            compile_error!(
                                concat!(
                                    "Invalid literal for `limit`: expected integer, got: ",
                                    stringify!(#lit)
                                )
                            );
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
                            other => {
                                return quote! {
                                    compile_error!(concat!("Invalid policy: ", #other));
                                }
                                .into();
                            }
                        };
                        policy_expr = pol;
                    }
                    lit => {
                        return quote! {
                            compile_error!(
                                concat!(
                                    "Invalid literal for `policy`: expected string, got: ",
                                    stringify!(#lit)
                                )
                            );
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
        }
    }

    // --- parse function ---
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let sig = &input.sig;
    let ident = &sig.ident;
    let block = &input.block;

    // Return type
    let ret_type = match &sig.output {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
    };

    // Collect argument patterns (no types) + detect self
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

    // Thread-local identifiers
    let cache_ident = format_ident!("_CACHE_{}_MAP", ident.to_string().to_uppercase());
    let order_ident = format_ident!("_CACHE_{}_ORDER", ident.to_string().to_uppercase());

    // Build cache key expression
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
    } else {
        if arg_pats.is_empty() {
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
        }
    };

    // Detect if return type is Result<...>
    let is_result = {
        let s = quote!(#ret_type).to_string().replace(' ', "");
        s.starts_with("Result<") || s.starts_with("std::result::Result<")
    };

    // --- Generate expanded function ---
    let expanded = if is_result {
        quote! {
            #vis #sig {
                use ::std::collections::{HashMap, VecDeque};
                use ::std::cell::RefCell;
                use ::cachelito_core::{ThreadLocalCache, CacheableKey};

                thread_local! {
                    static #cache_ident: RefCell<HashMap<String, #ret_type>> = RefCell::new(HashMap::new());
                    static #order_ident: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
                }

                let __key = #key_expr;
                let __cache = ThreadLocalCache::<#ret_type>::new(
                    &#cache_ident,
                    &#order_ident,
                    #limit_expr,
                    #policy_expr
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
                    static #cache_ident: RefCell<HashMap<String, #ret_type>> = RefCell::new(HashMap::new());
                    static #order_ident: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
                }

                let __key = #key_expr;
                let __cache = ThreadLocalCache::<#ret_type>::new(
                    &#cache_ident,
                    &#order_ident,
                    #limit_expr,
                    #policy_expr
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
