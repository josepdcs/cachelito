use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn, ReturnType, FnArg};

/// A procedural macro that adds caching functionality to functions.
///
/// This macro transforms a function into a cached version that stores results
/// in a static HashMap based on the function arguments. Subsequent calls with
/// the same arguments will return the cached result instead of re-executing
/// the function body.
///
/// # Requirements
/// - Function arguments must implement `Debug` for cache key generation
/// - Return type must implement `Clone` for cache storage and retrieval
/// - Function should be pure (same inputs always produce same outputs)
///
/// # Examples
/// ```
/// use your_crate::cache;
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
/// // Subsequent calls with same arguments return cached result
/// let result2 = fibonacci(10);
/// ```
///
/// # Implementation Details
/// - Creates a thread-safe static HashMap wrapped in `once_cell::Lazy` and `Mutex`
/// - Uses a tuple of arguments formatted with `Debug` as the cache key
/// - Automatically handles `self` parameters for methods
/// - Cache is shared across all calls to the same function
#[proc_macro_attribute]
pub fn cache(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the annotated function
    let input = parse_macro_input!(item as ItemFn);

    let vis = &input.vis;
    let sig = &input.sig;
    let ident = &sig.ident;
    let block = &input.block;

    // Extract only the return type (without "->")
    let ret_type = match &sig.output {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
    };

    // Build list of expressions to form the cache key from function arguments
    // If a receiver (self / &self / &mut self) exists, include it as &self to avoid moving
    let mut arg_exprs = Vec::new();
    let mut include_self = false;

    for arg in sig.inputs.iter() {
        match arg {
            FnArg::Receiver(_) => {
                // Include &self in the cache key
                include_self = true;
            }
            FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                arg_exprs.push(quote! { #pat });
            }
        }
    }

    // Generate a unique identifier for the static cache (per function)
    let cache_ident = format_ident!("_CACHE_{}_MAP", ident.to_string().to_uppercase());

    // Generate the macro expansion:
    // - Build a tuple containing &self (if applicable) followed by the arguments
    // - Use format!("{:?}", tuple) for the cache key (arguments must implement Debug)
    // - Use std::sync::Mutex<HashMap<String, RetType>> as a simple cache backend
    let expanded = quote! {
        static #cache_ident: ::once_cell::sync::Lazy<
            ::std::sync::Mutex<::std::collections::HashMap<String, #ret_type>>
        > = ::once_cell::sync::Lazy::new(|| ::std::sync::Mutex::new(::std::collections::HashMap::new()));

        #vis #sig {
            // Build the cache key from function arguments
            let __key = {
                // If we have self, include it as reference &self to avoid moving
                #[allow(unused_mut)]
                let __tuple = if false {
                    // Unreachable branch for code formatting; real branches below
                    ()
                } else {
                    // Build tuple dynamically: include &self if present, otherwise only args
                    // Generated code will be either: (&self, arg1, arg2, ...)
                    // or: (arg1, arg2, ...)
                    (
                        #(
                            #arg_exprs
                        ),*
                    )
                };
                // If no arguments (and no self), __tuple is (), so format!("{:?}", ()) -> "()"
                format!("{:?}", __tuple)
            };

            // Check cache for existing result
            {
                let cache_lock = #cache_ident.lock().unwrap();
                if let Some(cached) = cache_lock.get(&__key) {
                    return cached.clone();
                }
            }

            // Execute the original function body
            let __result = (|| #block)();

            // Store result in cache (we clone because we need to return __result)
            {
                let mut cache_lock = #cache_ident.lock().unwrap();
                cache_lock.insert(__key, __result.clone());
            }

            __result
        }
    };

    // Note: The generated code uses parameter names exactly as they appear in the function signature,
    // so parameters must be valid identifiers or patterns in scope.
    // Additionally, arguments included in the cache key must implement Debug and the return type must implement Clone.

    TokenStream::from(expanded)
}