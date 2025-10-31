use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, ReturnType};

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
pub fn cache(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let sig = &input.sig;
    let ident = &sig.ident;
    let block = &input.block;

    let ret_type = match &sig.output {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
    };

    let mut arg_names = Vec::new();
    let mut has_self = false;

    for arg in sig.inputs.iter() {
        match arg {
            FnArg::Receiver(_) => has_self = true,
            FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                arg_names.push(quote! { #pat });
            }
        }
    }

    let cache_ident = format_ident!("_CACHE_{}_MAP", ident.to_string().to_uppercase());

    // Build the key expression using `CacheableKey`
    let key_expr = if has_self {
        quote! {
            {
                let mut key_parts = Vec::new();
                key_parts.push(self.to_cache_key());
                #( key_parts.push( (#arg_names).to_cache_key() ); )*
                key_parts.join("|")
            }
        }
    } else {
        quote! {
            {
                let mut key_parts = Vec::new();
                #( key_parts.push( (#arg_names).to_cache_key() ); )*
                key_parts.join("|")
            }
        }
    };

    let is_result = quote!(#ret_type)
        .to_string()
        .replace(' ', "")
        .starts_with("Result<")
        || quote!(#ret_type)
            .to_string()
            .replace(' ', "")
            .starts_with("std::result::Result<");

    let expanded = if is_result {
        quote! {
            #vis #sig {
                use cachelito_core::{ThreadLocalCache, CacheableKey};

                thread_local! {
                    static #cache_ident: ::std::cell::RefCell<::std::collections::HashMap<String, #ret_type>> =
                        ::std::cell::RefCell::new(::std::collections::HashMap::new());
                }

                let __key = #key_expr;

                let __cache = ThreadLocalCache::new(&#cache_ident);

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
                use cachelito_core::{ThreadLocalCache, CacheableKey};

                thread_local! {
                    static #cache_ident: ::std::cell::RefCell<::std::collections::HashMap<String, #ret_type>> =
                        ::std::cell::RefCell::new(::std::collections::HashMap::new());
                }

                let __key = #key_expr;

                let __cache = ThreadLocalCache::new(&#cache_ident);

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
