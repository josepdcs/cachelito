use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, ReturnType};

/// Procedural macro that caches function results in a thread-local map.
///
/// - Automatically builds cache keys using the `CacheableKey` trait.
/// - For types without custom keys, you can `impl DefaultCacheableKey`.
/// - Works with functions returning `Result<T, E>` (only caches `Ok` results).
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
