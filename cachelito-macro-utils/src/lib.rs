//! Shared utilities for cachelito procedural macros
//!
//! This crate provides common parsing and code generation utilities
//! used by both `cachelito-macros` and `cachelito-async-macros`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{punctuated::Punctuated, Expr, MetaNameValue, Token};

/// Parsed macro attributes for async caching
pub struct AsyncCacheAttributes {
    pub limit: TokenStream2,
    pub policy: TokenStream2,
    pub ttl: TokenStream2,
    pub custom_name: Option<String>,
}

impl Default for AsyncCacheAttributes {
    fn default() -> Self {
        Self {
            limit: quote! { Option::<usize>::None },
            policy: quote! { "fifo" },
            ttl: quote! { Option::<u64>::None },
            custom_name: None,
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
}

impl Default for SyncCacheAttributes {
    fn default() -> Self {
        Self {
            limit: quote! { None },
            policy: quote! { cachelito_core::EvictionPolicy::FIFO },
            ttl: quote! { None },
            scope: quote! { cachelito_core::CacheScope::Global },
            custom_name: None,
        }
    }
}

/// Parse the `limit` attribute
pub fn parse_limit_attribute(nv: &MetaNameValue) -> TokenStream2 {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Int(lit_int) => {
                let val = lit_int
                    .base10_parse::<usize>()
                    .expect("limit must be a positive integer");
                quote! { Some(#val) }
            }
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
                if val == "fifo" || val == "lru" || val == "lfu" {
                    Ok(val)
                } else {
                    Err(
                        quote! { compile_error!("Invalid policy: expected \"fifo\", \"lru\", or \"lfu\"") },
                    )
                }
            }
            _ => Err(quote! { compile_error!("Invalid literal for `policy`: expected string") }),
        },
        _ => Err(
            quote! { compile_error!("Invalid syntax for `policy`: expected `policy = \"fifo\"|\"lru\"|\"lfu\"`") },
        ),
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
        } else if nv.path.is_ident("name") {
            attrs.custom_name = parse_name_attribute(&nv);
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
                    } else {
                        return Err(
                            quote! { compile_error!("Invalid policy: expected \"fifo\", \"lru\", or \"lfu\"") },
                        );
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
        } else if nv.path.is_ident("name") {
            attrs.custom_name = parse_name_attribute(&nv);
        }
    }

    Ok(attrs)
}
