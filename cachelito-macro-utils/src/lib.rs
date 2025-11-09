//! Shared utilities for cachelito procedural macros
//!
//! This crate provides common parsing and code generation utilities
//! used by both `cachelito-macros` and `cachelito-async-macros`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Expr, MetaNameValue};

/// Parsed macro attributes
pub struct CacheAttributes {
    pub limit: TokenStream2,
    pub policy: TokenStream2,
    pub ttl: TokenStream2,
    pub custom_name: Option<String>,
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

/// Parse the `policy` attribute
pub fn parse_policy_attribute(nv: &MetaNameValue) -> TokenStream2 {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(s) => {
                let val = s.value();
                quote! { #val }
            }
            _ => quote! { compile_error!("Invalid literal for `policy`: expected string") },
        },
        _ => {
            quote! { compile_error!("Invalid syntax for `policy`: expected `policy = \"fifo\"|\"lru\"`") }
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

/// Parse the `scope` attribute (only for sync macros)
pub fn parse_scope_attribute(nv: &MetaNameValue) -> TokenStream2 {
    match &nv.value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(s) => {
                let val = s.value();
                quote! { #val }
            }
            _ => quote! { compile_error!("Invalid literal for `scope`: expected string") },
        },
        _ => {
            quote! { compile_error!("Invalid syntax for `scope`: expected `scope = \"global\"|\"thread\"`") }
        }
    }
}

/// Generate cache key expression based on function arguments
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
