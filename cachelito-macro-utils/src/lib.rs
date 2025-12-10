//! Shared utilities for cachelito procedural macros
//!
//! This crate provides common parsing and code generation utilities
//! used by both `cachelito-macros` and `cachelito-async-macros`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{punctuated::Punctuated, Expr, MetaNameValue, Token};

/// List of supported eviction policies
static POLICIES: &[&str] = &["fifo", "lru", "lfu", "arc", "random"];

pub fn policies_str_with_separator(separator: &str) -> String {
    POLICIES
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(separator)
}

/// Parsed macro attributes for async caching
pub struct AsyncCacheAttributes {
    pub limit: TokenStream2,
    pub policy: TokenStream2,
    pub ttl: TokenStream2,
    pub custom_name: Option<String>,
    pub max_memory: TokenStream2,
    pub tags: Vec<String>,
    pub events: Vec<String>,
    pub dependencies: Vec<String>,
    pub invalidate_on: Option<syn::Path>,
    pub cache_if: Option<syn::Path>,
}

impl Default for AsyncCacheAttributes {
    fn default() -> Self {
        Self {
            limit: quote! { Option::<usize>::None },
            policy: quote! { "fifo" },
            ttl: quote! { Option::<u64>::None },
            custom_name: None,
            max_memory: quote! { Option::<usize>::None },
            tags: Vec::new(),
            events: Vec::new(),
            dependencies: Vec::new(),
            invalidate_on: None,
            cache_if: None,
        }
    }
}

/// Parsed macro attributes for sync caching
pub struct SyncCacheAttributes {
    pub limit: TokenStream2,
    pub policy: TokenStream2,
    pub ttl: TokenStream2,
    pub scope: TokenStream2,
    pub custom_name: Option<String>,
    pub max_memory: TokenStream2,
    pub tags: Vec<String>,
    pub events: Vec<String>,
    pub dependencies: Vec<String>,
    pub invalidate_on: Option<syn::Path>,
    pub cache_if: Option<syn::Path>,
}

impl Default for SyncCacheAttributes {
    fn default() -> Self {
        Self {
            limit: quote! { None },
            policy: quote! { cachelito_core::EvictionPolicy::FIFO },
            ttl: quote! { None },
            scope: quote! { cachelito_core::CacheScope::Global },
            custom_name: None,
            max_memory: quote! { None },
            tags: Vec::new(),
            events: Vec::new(),
            dependencies: Vec::new(),
            invalidate_on: None,
            cache_if: None,
        }
    }
}

/// Parse the `limit` attribute
pub fn parse_limit_attribute(nv: &MetaNameValue) -> TokenStream2 {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Int(lit_int) => match lit_int.base10_parse::<usize>() {
                Ok(val) => quote! { Some(#val) },
                Err(_) => quote! { compile_error!("limit must be a valid positive integer") },
            },
            _ => quote! { compile_error!("Invalid literal for `limit`: expected integer") },
        },
        _ => quote! { compile_error!("Invalid syntax for `limit`: expected `limit = <integer>`") },
    }
}

/// Parse the `policy` attribute and return the string value
pub fn parse_policy_attribute(nv: &MetaNameValue) -> Result<String, TokenStream2> {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(s) => {
                let val = s.value();
                // Validate the policy value
                if POLICIES.contains(&val.as_str()) {
                    Ok(val)
                } else {
                    let policies = policies_str_with_separator(", ");
                    let err_msg = format!("Invalid policy: expected one of {}", policies);
                    Err(quote! { compile_error!(#err_msg) })
                }
            }
            _ => Err(quote! { compile_error!("Invalid literal for `policy`: expected string") }),
        },
        _ => {
            let policies = policies_str_with_separator("|");
            let err_msg = format!(
                "Invalid syntax for `policy`: expected `policy = \"{}\"`",
                policies
            );
            Err(quote! {
                compile_error!(#err_msg)
            })
        }
    }
}

/// Parse the `ttl` attribute
pub fn parse_ttl_attribute(nv: &MetaNameValue) -> TokenStream2 {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Int(lit_int) => {
                let val = lit_int
                    .base10_parse::<u64>()
                    .expect("ttl must be a positive integer (seconds)");
                quote! { Some(#val) }
            }
            _ => quote! { compile_error!("Invalid literal for `ttl`: expected integer (seconds)") },
        },
        _ => quote! { compile_error!("Invalid syntax for `ttl`: expected `ttl = <integer>`") },
    }
}

/// Parse the `name` attribute
pub fn parse_name_attribute(nv: &MetaNameValue) -> Option<String> {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(s) => Some(s.value()),
            _ => None,
        },
        _ => None,
    }
}

/// Parse the `max_memory` attribute
/// Supports formats like: "100MB", "1GB", "500KB", or raw numbers
pub fn parse_max_memory_attribute(nv: &MetaNameValue) -> TokenStream2 {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(s) => {
                let val_str = s.value();
                let val_str = val_str.to_uppercase();

                // Parse memory size with units
                let bytes = if val_str.ends_with("GB") {
                    let num_str = val_str.trim_end_matches("GB");
                    match num_str.parse::<usize>() {
                        Ok(n) => n * 1024 * 1024 * 1024,
                        Err(_) => {
                            return quote! { compile_error!("Invalid number format for max_memory") }
                        }
                    }
                } else if val_str.ends_with("MB") {
                    let num_str = val_str.trim_end_matches("MB");
                    match num_str.parse::<usize>() {
                        Ok(n) => n * 1024 * 1024,
                        Err(_) => {
                            return quote! { compile_error!("Invalid number format for max_memory") }
                        }
                    }
                } else if val_str.ends_with("KB") {
                    let num_str = val_str.trim_end_matches("KB");
                    match num_str.parse::<usize>() {
                        Ok(n) => n * 1024,
                        Err(_) => {
                            return quote! { compile_error!("Invalid number format for max_memory") }
                        }
                    }
                } else {
                    // Try to parse as raw number (bytes)
                    match val_str.parse::<usize>() {
                        Ok(n) => n,
                        Err(_) => {
                            return quote! { compile_error!("Invalid format for max_memory: expected \"100MB\", \"1GB\", \"500KB\", or number") }
                        }
                    }
                };

                quote! { Some(#bytes) }
            }
            syn::Lit::Int(lit_int) => {
                let val = lit_int
                    .base10_parse::<usize>()
                    .expect("max_memory must be a positive integer (bytes)");
                quote! { Some(#val) }
            }
            _ => {
                quote! { compile_error!("Invalid literal for `max_memory`: expected string (\"100MB\") or integer") }
            }
        },
        _ => {
            quote! { compile_error!("Invalid syntax for `max_memory`: expected `max_memory = \"100MB\"`") }
        }
    }
}

/// Parse the `scope` attribute and return the string value
pub fn parse_scope_attribute(nv: &MetaNameValue) -> Result<String, TokenStream2> {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(s) => {
                let val = s.value();
                // Validate the scope value
                if val == "global" || val == "thread" {
                    Ok(val)
                } else {
                    Err(
                        quote! { compile_error!("Invalid scope: expected \"global\" or \"thread\"") },
                    )
                }
            }
            _ => Err(quote! { compile_error!("Invalid literal for `scope`: expected string") }),
        },
        _ => Err(
            quote! { compile_error!("Invalid syntax for `scope`: expected `scope = \"global\"|\"thread\"`") },
        ),
    }
}

/// Generate cache key expression based on function arguments (for async macros using format!)
pub fn generate_key_expr(has_self: bool, arg_pats: &[TokenStream2]) -> TokenStream2 {
    if has_self {
        if arg_pats.is_empty() {
            quote! {{
                format!("{:?}", self)
            }}
        } else {
            quote! {{
                let mut __key_parts = Vec::new();
                __key_parts.push(format!("{:?}", self));
                #(
                    __key_parts.push(format!("{:?}", #arg_pats));
                )*
                __key_parts.join("|")
            }}
        }
    } else if arg_pats.is_empty() {
        quote! {{ String::new() }}
    } else {
        quote! {{
            let mut __key_parts = Vec::new();
            #(
                __key_parts.push(format!("{:?}", #arg_pats));
            )*
            __key_parts.join("|")
        }}
    }
}

/// Generate cache key expression using CacheableKey trait (for sync macros)
pub fn generate_key_expr_with_cacheable_key(
    has_self: bool,
    arg_pats: &[TokenStream2],
) -> TokenStream2 {
    if has_self {
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
    }
}

/// Parse array of strings from attribute (for tags, events, dependencies)
///
/// Supports formats like:
/// - `tags = ["tag1", "tag2"]`
/// - `tags = ["single"]`
/// - `events = ["event1"]`
pub fn parse_string_array_attribute(nv: &MetaNameValue) -> Result<Vec<String>, TokenStream2> {
    match &nv.value {
        Expr::Array(array) => {
            let mut strings = Vec::new();
            for elem in &array.elems {
                match elem {
                    Expr::Lit(expr_lit) => match &expr_lit.lit {
                        syn::Lit::Str(s) => {
                            strings.push(s.value());
                        }
                        _ => {
                            return Err(
                                quote! { compile_error!("Array elements must be string literals") },
                            );
                        }
                    },
                    _ => {
                        return Err(
                            quote! { compile_error!("Array elements must be string literals") },
                        );
                    }
                }
            }
            Ok(strings)
        }
        _ => Err(quote! { compile_error!("Expected array of strings like [\"tag1\", \"tag2\"]") }),
    }
}

/// Parse the `invalidate_on` attribute
/// Expects a function path like `invalidate_on = is_stale` or `invalidate_on = my_module::is_stale`
pub fn parse_invalidate_on_attribute(nv: &MetaNameValue) -> Result<syn::Path, TokenStream2> {
    match &nv.value {
        Expr::Path(expr_path) => Ok(expr_path.path.clone()),
        _ => Err(
            quote! { compile_error!("Invalid syntax for `invalidate_on`: expected `invalidate_on = function_name`") },
        ),
    }
}

/// Parse the `cache_if` attribute
/// Expects a function path like `cache_if = should_cache` or `cache_if = my_module::should_cache`
pub fn parse_cache_if_attribute(nv: &MetaNameValue) -> Result<syn::Path, TokenStream2> {
    match &nv.value {
        Expr::Path(expr_path) => Ok(expr_path.path.clone()),
        _ => Err(
            quote! { compile_error!("Invalid syntax for `cache_if`: expected `cache_if = function_name`") },
        ),
    }
}

/// Parse common attributes shared between async and sync caches
/// Returns true if the attribute was recognized and processed
fn parse_common_attribute(
    nv: &MetaNameValue,
    custom_name: &mut Option<String>,
    max_memory: &mut TokenStream2,
    tags: &mut Vec<String>,
    events: &mut Vec<String>,
    dependencies: &mut Vec<String>,
    invalidate_on: &mut Option<syn::Path>,
    cache_if: &mut Option<syn::Path>,
) -> Result<bool, TokenStream2> {
    if nv.path.is_ident("name") {
        *custom_name = parse_name_attribute(nv);
        Ok(true)
    } else if nv.path.is_ident("max_memory") {
        *max_memory = parse_max_memory_attribute(nv);
        Ok(true)
    } else if nv.path.is_ident("tags") {
        *tags = parse_string_array_attribute(nv)?;
        Ok(true)
    } else if nv.path.is_ident("events") {
        *events = parse_string_array_attribute(nv)?;
        Ok(true)
    } else if nv.path.is_ident("dependencies") {
        *dependencies = parse_string_array_attribute(nv)?;
        Ok(true)
    } else if nv.path.is_ident("invalidate_on") {
        *invalidate_on = Some(parse_invalidate_on_attribute(nv)?);
        Ok(true)
    } else if nv.path.is_ident("cache_if") {
        *cache_if = Some(parse_cache_if_attribute(nv)?);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse async cache attributes from a token stream
pub fn parse_async_attributes(attr: TokenStream2) -> Result<AsyncCacheAttributes, TokenStream2> {
    use syn::parse::Parser;

    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let parsed_args = parser.parse2(attr).map_err(|e| {
        let msg = format!("Failed to parse attributes: {}", e);
        quote! { compile_error!(#msg) }
    })?;

    let mut attrs = AsyncCacheAttributes::default();

    for nv in parsed_args {
        if nv.path.is_ident("limit") {
            attrs.limit = parse_limit_attribute(&nv);
        } else if nv.path.is_ident("policy") {
            match parse_policy_attribute(&nv) {
                Ok(policy_str) => attrs.policy = quote! { #policy_str },
                Err(err) => return Err(err),
            }
        } else if nv.path.is_ident("ttl") {
            attrs.ttl = parse_ttl_attribute(&nv);
        } else {
            // Try to parse as common attribute
            if !parse_common_attribute(
                &nv,
                &mut attrs.custom_name,
                &mut attrs.max_memory,
                &mut attrs.tags,
                &mut attrs.events,
                &mut attrs.dependencies,
                &mut attrs.invalidate_on,
                &mut attrs.cache_if,
            )? {
                // Unknown attribute - generate compile error
                let attr_name = nv
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let err_msg = format!(
                    "Unknown attribute: `{}`. Valid attributes are: limit, policy, ttl, name, max_memory, tags, events, dependencies, invalidate_on, cache_if",
                    attr_name
                );
                return Err(quote! { compile_error!(#err_msg) });
            }
        }
    }

    Ok(attrs)
}

/// Parse sync cache attributes from a token stream
pub fn parse_sync_attributes(attr: TokenStream2) -> Result<SyncCacheAttributes, TokenStream2> {
    use syn::parse::Parser;

    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let parsed_args = parser.parse2(attr).map_err(|e| {
        let msg = format!("Failed to parse attributes: {}", e);
        quote! { compile_error!(#msg) }
    })?;

    let mut attrs = SyncCacheAttributes::default();

    for nv in parsed_args {
        if nv.path.is_ident("limit") {
            attrs.limit = parse_limit_attribute(&nv);
        } else if nv.path.is_ident("policy") {
            match parse_policy_attribute(&nv) {
                Ok(policy_str) => {
                    attrs.policy = if policy_str == "fifo" {
                        quote! { cachelito_core::EvictionPolicy::FIFO }
                    } else if policy_str == "lru" {
                        quote! { cachelito_core::EvictionPolicy::LRU }
                    } else if policy_str == "lfu" {
                        quote! { cachelito_core::EvictionPolicy::LFU }
                    } else if policy_str == "arc" {
                        quote! { cachelito_core::EvictionPolicy::ARC }
                    } else if policy_str == "random" {
                        quote! { cachelito_core::EvictionPolicy::Random }
                    } else {
                        let policies = policies_str_with_separator(", ");
                        let err_msg = format!("Invalid policy: expected one of {}", policies);
                        return Err(quote! { compile_error!(#err_msg) });
                    };
                }
                Err(err) => return Err(err),
            }
        } else if nv.path.is_ident("ttl") {
            attrs.ttl = parse_ttl_attribute(&nv);
        } else if nv.path.is_ident("scope") {
            match parse_scope_attribute(&nv) {
                Ok(scope_str) => {
                    attrs.scope = if scope_str == "thread" {
                        quote! { cachelito_core::CacheScope::ThreadLocal }
                    } else if scope_str == "global" {
                        quote! { cachelito_core::CacheScope::Global }
                    } else {
                        return Err(
                            quote! { compile_error!("Invalid scope: expected \"global\" or \"thread\"") },
                        );
                    };
                }
                Err(err) => return Err(err),
            }
        } else {
            // Try to parse as common attribute
            if !parse_common_attribute(
                &nv,
                &mut attrs.custom_name,
                &mut attrs.max_memory,
                &mut attrs.tags,
                &mut attrs.events,
                &mut attrs.dependencies,
                &mut attrs.invalidate_on,
                &mut attrs.cache_if,
            )? {
                // Unknown attribute - generate compile error
                let attr_name = nv
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let err_msg = format!(
                    "Unknown attribute: `{}`. Valid attributes are: limit, policy, ttl, scope, name, max_memory, tags, events, dependencies, invalidate_on, cache_if",
                    attr_name
                );
                return Err(quote! { compile_error!(#err_msg) });
            }
        }
    }

    Ok(attrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn test_policies_str_with_separator() {
        let result = policies_str_with_separator(", ");
        assert_eq!(result, "\"fifo\", \"lru\", \"lfu\", \"arc\", \"random\"");

        let result = policies_str_with_separator("|");
        assert_eq!(result, "\"fifo\"|\"lru\"|\"lfu\"|\"arc\"|\"random\"");
    }

    #[test]
    fn test_parse_limit_attribute_valid() {
        let nv: MetaNameValue = parse_quote! { limit = 100 };
        let result = parse_limit_attribute(&nv);
        assert_eq!(result.to_string(), "Some (100usize)");
    }

    #[test]
    fn test_parse_policy_attribute_valid() {
        let nv: MetaNameValue = parse_quote! { policy = "fifo" };
        let result = parse_policy_attribute(&nv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fifo");

        let nv: MetaNameValue = parse_quote! { policy = "lru" };
        let result = parse_policy_attribute(&nv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "lru");
    }

    #[test]
    fn test_parse_max_memory_attribute() {
        // Test MB format
        let nv: MetaNameValue = parse_quote! { max_memory = "100MB" };
        let result = parse_max_memory_attribute(&nv);
        let expected = 100 * 1024 * 1024;
        assert_eq!(result.to_string(), format!("Some ({}usize)", expected));

        // Test GB format
        let nv: MetaNameValue = parse_quote! { max_memory = "1GB" };
        let result = parse_max_memory_attribute(&nv);
        let expected = 1024 * 1024 * 1024;
        assert_eq!(result.to_string(), format!("Some ({}usize)", expected));

        // Test KB format
        let nv: MetaNameValue = parse_quote! { max_memory = "500KB" };
        let result = parse_max_memory_attribute(&nv);
        let expected = 500 * 1024;
        assert_eq!(result.to_string(), format!("Some ({}usize)", expected));

        // Test raw number
        let nv: MetaNameValue = parse_quote! { max_memory = 1024 };
        let result = parse_max_memory_attribute(&nv);
        assert_eq!(result.to_string(), "Some (1024usize)");

        // Test raw number as string
        let nv: MetaNameValue = parse_quote! { max_memory = "2048" };
        let result = parse_max_memory_attribute(&nv);
        assert_eq!(result.to_string(), "Some (2048usize)");
    }

    #[test]
    fn test_parse_policy_attribute_invalid() {
        let nv: MetaNameValue = parse_quote! { policy = "invalid" };
        let result = parse_policy_attribute(&nv);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ttl_attribute_valid() {
        let nv: MetaNameValue = parse_quote! { ttl = 60 };
        let result = parse_ttl_attribute(&nv);
        assert_eq!(result.to_string(), "Some (60u64)");
    }

    #[test]
    fn test_parse_name_attribute() {
        let nv: MetaNameValue = parse_quote! { name = "my_cache" };
        let result = parse_name_attribute(&nv);
        assert_eq!(result, Some("my_cache".to_string()));
    }

    #[test]
    fn test_parse_scope_attribute_valid() {
        let nv: MetaNameValue = parse_quote! { scope = "global" };
        let result = parse_scope_attribute(&nv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "global");

        let nv: MetaNameValue = parse_quote! { scope = "thread" };
        let result = parse_scope_attribute(&nv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "thread");
    }

    #[test]
    fn test_parse_scope_attribute_invalid() {
        let nv: MetaNameValue = parse_quote! { scope = "invalid" };
        let result = parse_scope_attribute(&nv);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_key_expr_no_self_no_args() {
        let result = generate_key_expr(false, &[]);
        assert_eq!(result.to_string(), "{ String :: new () }");
    }

    #[test]
    fn test_generate_key_expr_with_self_no_args() {
        let result = generate_key_expr(true, &[]);
        let expected = quote! {{ format!("{:?}", self) }};
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_key_expr_with_args() {
        let args = vec![quote! { arg1 }, quote! { arg2 }];
        let result = generate_key_expr(false, &args);
        assert!(result.to_string().contains("__key_parts"));
    }

    #[test]
    fn test_parse_async_attributes_defaults() {
        let attrs = parse_async_attributes(quote! {}).unwrap();
        assert_eq!(attrs.limit.to_string(), "Option :: < usize > :: None");
        assert_eq!(attrs.policy.to_string(), "\"fifo\"");
        assert_eq!(attrs.ttl.to_string(), "Option :: < u64 > :: None");
        assert_eq!(attrs.custom_name, None);
    }

    #[test]
    fn test_parse_async_attributes_complete() {
        let attrs = parse_async_attributes(quote! {
            limit = 50,
            policy = "lru",
            ttl = 120,
            name = "test_cache"
        })
        .unwrap();

        assert_eq!(attrs.limit.to_string(), "Some (50usize)");
        assert_eq!(attrs.policy.to_string(), "\"lru\"");
        assert_eq!(attrs.ttl.to_string(), "Some (120u64)");
        assert_eq!(attrs.custom_name, Some("test_cache".to_string()));
    }

    #[test]
    fn test_parse_sync_attributes_defaults() {
        let attrs = parse_sync_attributes(quote! {}).unwrap();
        assert_eq!(attrs.limit.to_string(), "None");
        assert_eq!(
            attrs.policy.to_string(),
            "cachelito_core :: EvictionPolicy :: FIFO"
        );
        assert_eq!(
            attrs.scope.to_string(),
            "cachelito_core :: CacheScope :: Global"
        );
    }

    #[test]
    fn test_parse_sync_attributes_complete() {
        let attrs = parse_sync_attributes(quote! {
            limit = 100,
            policy = "arc",
            ttl = 300,
            scope = "thread",
            name = "sync_cache"
        })
        .unwrap();

        assert_eq!(attrs.limit.to_string(), "Some (100usize)");
        assert_eq!(
            attrs.policy.to_string(),
            "cachelito_core :: EvictionPolicy :: ARC"
        );
        assert_eq!(
            attrs.scope.to_string(),
            "cachelito_core :: CacheScope :: ThreadLocal"
        );
        assert_eq!(attrs.custom_name, Some("sync_cache".to_string()));
    }

    #[test]
    fn test_parse_common_attribute_name() {
        let nv: MetaNameValue = parse_quote! { name = "test_cache" };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(custom_name, Some("test_cache".to_string()));
    }

    #[test]
    fn test_parse_common_attribute_max_memory() {
        let nv: MetaNameValue = parse_quote! { max_memory = "100MB" };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        let expected = 100 * 1024 * 1024;
        assert_eq!(max_memory.to_string(), format!("Some ({}usize)", expected));
    }

    #[test]
    fn test_parse_common_attribute_tags() {
        let nv: MetaNameValue = parse_quote! { tags = ["tag1", "tag2"] };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(tags, vec!["tag1".to_string(), "tag2".to_string()]);
    }

    #[test]
    fn test_parse_common_attribute_events() {
        let nv: MetaNameValue = parse_quote! { events = ["event1"] };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(events, vec!["event1".to_string()]);
    }

    #[test]
    fn test_parse_common_attribute_dependencies() {
        let nv: MetaNameValue = parse_quote! { dependencies = ["dep1", "dep2"] };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(dependencies, vec!["dep1".to_string(), "dep2".to_string()]);
    }

    #[test]
    fn test_parse_common_attribute_unknown() {
        let nv: MetaNameValue = parse_quote! { unknown = "value" };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // Not recognized
    }

    #[test]
    fn test_parse_common_attribute_invalidate_on() {
        let nv: MetaNameValue = parse_quote! { invalidate_on = is_stale };
        let mut custom_name = None;
        let mut max_memory = quote! { None };
        let mut tags = Vec::new();
        let mut events = Vec::new();
        let mut dependencies = Vec::new();
        let mut invalidate_on = None;
        let mut cache_if = None;

        let result = parse_common_attribute(
            &nv,
            &mut custom_name,
            &mut max_memory,
            &mut tags,
            &mut events,
            &mut dependencies,
            &mut invalidate_on,
            &mut cache_if,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert!(invalidate_on.is_some());
        assert_eq!(
            invalidate_on
                .unwrap()
                .segments
                .first()
                .unwrap()
                .ident
                .to_string(),
            "is_stale"
        );
    }

    #[test]
    fn test_parse_string_array_attribute() {
        let nv: MetaNameValue = parse_quote! { tags = ["tag1", "tag2", "tag3"] };
        let result = parse_string_array_attribute(&nv);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()]
        );
    }

    #[test]
    fn test_parse_async_attributes_unknown_attribute() {
        let result = parse_async_attributes(quote! {
            limit = 100,
            unknown_attr = "value"
        });

        assert!(result.is_err());
        if let Err(err) = result {
            let err_str = err.to_string();
            assert!(err_str.contains("Unknown attribute"));
            assert!(err_str.contains("unknown_attr"));
        }
    }

    #[test]
    fn test_parse_sync_attributes_unknown_attribute() {
        let result = parse_sync_attributes(quote! {
            limit = 100,
            typo_tag = ["tag1"]
        });

        assert!(result.is_err());
        if let Err(err) = result {
            let err_str = err.to_string();
            assert!(err_str.contains("Unknown attribute"));
            assert!(err_str.contains("typo_tag"));
        }
    }

    #[test]
    fn test_parse_async_attributes_typo_in_tags() {
        // Test common typo: "tag" instead of "tags"
        let result = parse_async_attributes(quote! {
            tag = ["user_data"]
        });

        assert!(result.is_err());
        if let Err(err) = result {
            let err_str = err.to_string();
            assert!(err_str.contains("Unknown attribute"));
            assert!(err_str.contains("tag"));
        }
    }

    #[test]
    fn test_parse_sync_attributes_typo_in_events() {
        // Test common typo: "event" instead of "events"
        let result = parse_sync_attributes(quote! {
            event = ["user_updated"]
        });

        assert!(result.is_err());
        if let Err(err) = result {
            let err_str = err.to_string();
            assert!(err_str.contains("Unknown attribute"));
            assert!(err_str.contains("event"));
        }
    }
}
