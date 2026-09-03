//! `#[derive(ComponentVariant)]` — cva-style matchers for variant/size enums.
//!
//! Hand-written `*Variant`/`*Size` enums are the most duplicated boilerplate in
//! `gpui-ui-kit`. This derive generates the standard matchers from the enum
//! definition so every component exposes the same introspectable surface:
//!
//! - `Enum::all()` — every variant, in declaration order (powers docs tables,
//!   showcase matrices, and prop-controls).
//! - `variant.as_str()` — stable snake_case identifier, overridable per variant
//!   with `#[variant(name = "...")]`.
//! - `variant.is_default_variant()` — whether the variant carries `#[default]`.
//! - `Display` / `FromStr` — round-trip through the same identifiers.
//!
//! # Example
//!
//! ```ignore
//! use gpui_ui_kit_macros::ComponentVariant;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentVariant)]
//! pub enum ButtonVariant {
//!     #[default]
//!     Primary,
//!     Secondary,
//!     #[variant(name = "danger")]
//!     Destructive,
//! }
//!
//! assert_eq!(ButtonVariant::all().len(), 3);
//! assert_eq!(ButtonVariant::Destructive.as_str(), "danger");
//! assert!("primary".parse::<ButtonVariant>().is_ok());
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Lit, Meta};

/// Convert `CamelCase` to `snake_case` without extra dependencies.
fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, ch) in name.char_indices() {
        if ch.is_ascii_uppercase() {
            if index != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Derive cva-style matchers for a fieldless enum.
///
/// Only unit variants are supported; any variant with fields is a compile
/// error. Generic enums are rejected because the generated `ALL` table needs
/// a concrete type.
pub(crate) fn derive_component_variant_impl(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(error) => return error.to_compile_error(),
    };
    let name = &input.ident;

    if !input.generics.params.is_empty() {
        return syn::Error::new(
            input.generics.span(),
            "ComponentVariant does not support generic enums; use a concrete variant enum",
        )
        .to_compile_error();
    }

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return syn::Error::new(
                input.span(),
                "ComponentVariant only supports fieldless enums",
            )
            .to_compile_error();
        }
    };

    if variants.is_empty() {
        return syn::Error::new(
            input.span(),
            "ComponentVariant requires at least one variant",
        )
        .to_compile_error();
    }

    let mut errors = Vec::new();
    let mut idents = Vec::with_capacity(variants.len());
    let mut str_names = Vec::with_capacity(variants.len());
    let mut is_default = Vec::with_capacity(variants.len());
    let mut seen_default = None::<syn::Ident>;

    for variant in variants {
        if !matches!(variant.fields, syn::Fields::Unit) {
            errors.push(syn::Error::new(
                variant.span(),
                format!(
                    "ComponentVariant only supports unit variants; `{}` has fields",
                    variant.ident
                ),
            ));
            continue;
        }

        let mut str_name = to_snake_case(&variant.ident.to_string());
        for attr in &variant.attrs {
            if !attr.path().is_ident("variant") {
                continue;
            }
            let nested = match attr.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) {
                Ok(nested) => nested,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            for meta in nested {
                match meta {
                    Meta::NameValue(nv) if nv.path.is_ident("name") => {
                        match &nv.value {
                            syn::Expr::Lit(lit) => match &lit.lit {
                                Lit::Str(value) => str_name = value.value(),
                                _ => errors.push(syn::Error::new(
                                    nv.value.span(),
                                    "`name` in #[variant(...)] must be a string literal",
                                )),
                            },
                            _ => errors.push(syn::Error::new(
                                nv.value.span(),
                                "`name` in #[variant(...)] must be a string literal",
                            )),
                        }
                    }
                    other => errors.push(syn::Error::new(
                        other.span(),
                        "Unknown variant attribute; expected `name = \"...\"`",
                    )),
                }
            }
        }

        let default_attr = variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"));
        if default_attr
            && let Some(previous) = seen_default.replace(variant.ident.clone())
        {
            errors.push(syn::Error::new(
                variant.ident.span(),
                format!("Multiple #[default] variants; `{previous}` is already the default"),
            ));
        }

        idents.push(variant.ident.clone());
        str_names.push(str_name);
        is_default.push(default_attr);
    }

    if !errors.is_empty() {
        return super::derive::combined_compile_error(errors);
    }

    let all_count = idents.len();
    let expanded = quote! {
        #[automatically_derived]
        impl #name {
            /// Every variant in declaration order.
            pub const ALL: &'static [Self] = &[#(Self::#idents),*];

            /// Every variant in declaration order.
            pub fn all() -> &'static [Self] {
                Self::ALL
            }

            /// Number of variants.
            pub fn variant_count() -> usize {
                #all_count
            }

            /// Stable snake_case identifier for docs, tests, and prop-controls.
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(Self::#idents => #str_names),*
                }
            }

            /// Whether this variant carries `#[default]`.
            pub fn is_default_variant(&self) -> bool {
                match self {
                    #(Self::#idents => #is_default),*
                }
            }
        }

        #[automatically_derived]
        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        #[automatically_derived]
        impl ::core::str::FromStr for #name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    #(#str_names => Ok(Self::#idents),)*
                    _ => Err(format!("unknown {} variant: {value}", stringify!(#name))),
                }
            }
        }
    };

    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant_derive(input: &str) -> String {
        derive_component_variant_impl(input.parse().unwrap()).to_string()
    }

    #[test]
    fn generates_all_as_str_display_from_str() {
        let out = variant_derive(
            r#"
            pub enum ButtonVariant {
                #[default]
                Primary,
                Secondary,
                #[variant(name = "danger")]
                Destructive,
            }
            "#,
        );
        assert!(out.contains("pub const ALL"));
        assert!(out.contains("pub fn all"));
        assert!(out.contains("pub fn as_str"));
        assert!(out.contains("pub fn is_default_variant"));
        assert!(out.contains("impl :: core :: fmt :: Display for ButtonVariant"));
        assert!(out.contains("impl :: core :: str :: FromStr for ButtonVariant"));
        assert!(out.contains("\"danger\""));
        assert!(out.contains("\"primary\""));
        assert!(out.contains("\"secondary\""));
    }

    #[test]
    fn rejects_non_enum() {
        let out = variant_derive("pub struct NotAnEnum { pub x: u8 }");
        assert!(out.contains("compile_error !"));
        assert!(out.contains("only supports fieldless enums"));
    }

    #[test]
    fn rejects_variants_with_fields() {
        let out = variant_derive(
            r#"
            pub enum WithFields {
                Unit,
                Tuple(u8),
            }
            "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("only supports unit variants"));
    }

    #[test]
    fn rejects_empty_and_generic_enums() {
        let empty = variant_derive("pub enum Empty {}");
        assert!(empty.contains("compile_error !"));
        assert!(empty.contains("at least one variant"));

        let generic = variant_derive(
            r#"
            pub enum Generic<T> {
                A,
                B,
            }
            "#,
        );
        assert!(generic.contains("compile_error !"));
        assert!(generic.contains("does not support generic enums"));
    }

    #[test]
    fn rejects_duplicate_defaults_and_unknown_keys() {
        let out = variant_derive(
            r#"
            pub enum Bad {
                #[default]
                A,
                #[default]
                B,
                #[variant(label = "x")]
                C,
            }
            "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("Multiple #[default] variants"));
        assert!(out.contains("Unknown variant attribute"));
    }

    #[test]
    fn rejects_non_string_name() {
        let out = variant_derive(
            r#"
            pub enum Bad {
                #[variant(name = 1)]
                A,
            }
            "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("must be a string literal"));
    }
}
