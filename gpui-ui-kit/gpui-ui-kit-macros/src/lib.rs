//! Proc macros for gpui-ui-kit
//!
//! Provides derive macros to reduce boilerplate in component theme definitions.
//! The primary macro is [`ComponentTheme`] which generates `Default` and `From<&Theme>`
//! implementations for theme structs, reducing repetitive boilerplate code.
//!
//! # Quick Start
//!
//! ```ignore
//! use gpui_ui_kit_macros::ComponentTheme;
//!
//! #[derive(Debug, Clone, ComponentTheme)]
//! pub struct MyComponentTheme {
//!     #[theme(default = 0x007acc, from = accent)]
//!     pub primary_color: Rgba,
//!
//!     #[theme(default = 0xffffff, from = text_primary)]
//!     pub text_color: Rgba,
//! }
//! ```
//!
//! This generates:
//! - `impl Default for MyComponentTheme` using the hex `default` values
//! - `impl From<&Theme> for MyComponentTheme` mapping from global theme fields
//!
//! # Crate Features
//!
//! This is a proc-macro crate. It must be used alongside the main `gpui-ui-kit`
//! crate which re-exports the macro as `ComponentTheme`.

use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Meta, Token, parse_macro_input};

/// Derive macro for component themes.
///
/// Generates `Default` and `From<&Theme>` implementations for theme structs,
/// allowing components to have fallback colors while also automatically adapting
/// to the global theme.
///
/// # Requirements
///
/// - Only works on structs with named fields
/// - Every field must have a `#[theme(...)]` attribute
/// - Each field needs both a default value and a mapping from Theme
///
/// # Attribute Reference
///
/// ## For Color Fields (Rgba)
///
/// | Attribute | Description | Example |
/// |-----------|-------------|---------|
/// | `default = 0xRRGGBB` | RGB hex color for Default impl | `default = 0x007acc` |
/// | `default = 0xRRGGBBAA` | RGBA hex color (with alpha) | `default = 0x007acc80` |
/// | `from = field_name` | Direct mapping from Theme field | `from = accent` |
/// | `from_expr = "expr"` | Custom expression (uses `theme` variable) | `from_expr = "with_alpha(theme.accent, 0.2)"` |
///
/// ## For Numeric Fields (f32, etc.)
///
/// | Attribute | Description | Example |
/// |-----------|-------------|---------|
/// | `default_f32 = value` | f32 literal for Default impl | `default_f32 = 0.5` |
/// | `from_expr = "value"` | Expression for From impl | `from_expr = "0.5"` |
///
/// ## For Other Types (Option, nested themes, etc.)
///
/// | Attribute | Description | Example |
/// |-----------|-------------|---------|
/// | `default_expr = "expr"` | Arbitrary expression for Default | `default_expr = "None"` |
/// | `from_expr = "expr"` | Arbitrary expression for From | `from_expr = "Some(theme.accent)"` |
///
/// # Available Theme Fields
///
/// The global `Theme` struct provides these fields for mapping:
///
/// **Backgrounds:** `background`, `surface`, `surface_hover`, `muted`, `transparent`, `overlay_bg`
///
/// **Text:** `text_primary`, `text_secondary`, `text_muted`
///
/// **Accent:** `accent`, `accent_hover`, `accent_muted`
///
/// **Semantic:** `success`, `warning`, `error`, `info`
///
/// **Border:** `border`, `border_hover`
///
/// # Examples
///
/// ## Basic Color Theme
///
/// ```ignore
/// #[derive(Debug, Clone, ComponentTheme)]
/// pub struct ButtonTheme {
///     #[theme(default = 0x007acc, from = accent)]
///     pub background: Rgba,
///
///     #[theme(default = 0xffffff, from = text_primary)]
///     pub text: Rgba,
///
///     #[theme(default = 0x3a3a3a, from = border)]
///     pub border: Rgba,
/// }
/// ```
///
/// ## With Custom Expressions
///
/// ```ignore
/// use crate::color_tokens::with_alpha;
///
/// #[derive(Debug, Clone, ComponentTheme)]
/// pub struct TooltipTheme {
///     #[theme(default = 0x2a2a2aff, from = surface)]
///     pub background: Rgba,
///
///     // Use with_alpha helper for transparency
///     #[theme(default = 0x007acc33, from_expr = "with_alpha(theme.accent, 0.2)")]
///     pub highlight: Rgba,
///
///     // Derived from another theme field
///     #[theme(default = 0x888888, from_expr = "darken(theme.text_secondary, 0.1)")]
///     pub shadow: Rgba,
/// }
/// ```
///
/// ## With Non-Color Fields
///
/// ```ignore
/// #[derive(Debug, Clone, ComponentTheme)]
/// pub struct FadeTheme {
///     #[theme(default = 0xffffff, from = text_primary)]
///     pub color: Rgba,
///
///     #[theme(default_f32 = 0.5, from_expr = "0.5")]
///     pub disabled_opacity: f32,
///
///     #[theme(default_expr = "None", from_expr = "None")]
///     pub optional_accent: Option<Rgba>,
/// }
/// ```
///
/// # Generated Code
///
/// For a theme struct `MyTheme`, this macro generates:
///
/// ```ignore
/// impl Default for MyTheme {
///     fn default() -> Self {
///         Self {
///             // Fields initialized with default values
///         }
///     }
/// }
///
/// impl From<&crate::theme::Theme> for MyTheme {
///     fn from(theme: &crate::theme::Theme) -> Self {
///         Self {
///             // Fields mapped from global theme
///         }
///     }
/// }
/// ```
///
/// # Common Patterns
///
/// ## Creating a theme from global state
///
/// ```ignore
/// fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
///     let global_theme = cx.theme();
///     let button_theme = ButtonTheme::from(&global_theme);
///     // or use the default
///     let default_theme = ButtonTheme::default();
/// }
/// ```
///
/// ## Customizing specific fields
///
/// ```ignore
/// let mut theme = ButtonTheme::from(&cx.theme());
/// theme.background = rgb(0xff0000); // Override just the background
/// ```
///
/// # Compile Errors
///
/// The macro will panic at compile time if:
/// - A field is missing the `#[theme(...)]` attribute
/// - A field is missing `default`, `default_f32`, or `default_expr`
/// - A field is missing `from` or `from_expr`
/// - An expression in `from_expr` or `default_expr` fails to parse
#[proc_macro_derive(ComponentTheme, attributes(theme))]
pub fn derive_component_theme(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("ComponentTheme only supports structs with named fields"),
        },
        _ => panic!("ComponentTheme only supports structs"),
    };

    let mut default_fields = Vec::new();
    let mut from_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();

        // Find the #[theme(...)] attribute
        let theme_attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("theme"));

        let Some(attr) = theme_attr else {
            panic!("Field `{}` is missing #[theme(...)] attribute", field_name);
        };

        let mut default_value: Option<u32> = None;
        let mut default_f32: Option<f64> = None;
        let mut default_expr_str: Option<String> = None;
        let mut from_field: Option<syn::Ident> = None;
        let mut from_expr: Option<String> = None;

        // Parse the attribute arguments
        let nested = attr
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .expect("Failed to parse theme attribute");

        for meta in nested {
            match meta {
                Meta::NameValue(nv) => {
                    let ident = nv.path.get_ident().expect("Expected identifier");
                    match ident.to_string().as_str() {
                        "default" => {
                            if let Expr::Lit(lit) = &nv.value
                                && let Lit::Int(int_lit) = &lit.lit
                            {
                                default_value = Some(int_lit.base10_parse().unwrap());
                            }
                        }
                        "default_f32" => {
                            if let Expr::Lit(lit) = &nv.value {
                                match &lit.lit {
                                    Lit::Float(f) => {
                                        default_f32 = Some(f.base10_parse().unwrap());
                                    }
                                    Lit::Int(i) => {
                                        // Allow integers like 0 or 1
                                        default_f32 = Some(i.base10_parse::<i64>().unwrap() as f64);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "default_expr" => {
                            if let Expr::Lit(lit) = &nv.value
                                && let Lit::Str(s) = &lit.lit
                            {
                                default_expr_str = Some(s.value());
                            }
                        }
                        "from" => {
                            if let Expr::Path(path) = &nv.value {
                                from_field = path.path.get_ident().cloned();
                            }
                        }
                        "from_expr" => {
                            if let Expr::Lit(lit) = &nv.value
                                && let Lit::Str(s) = &lit.lit
                            {
                                from_expr = Some(s.value());
                            }
                        }
                        _ => panic!("Unknown theme attribute: {}", ident),
                    }
                }
                _ => panic!("Expected name = value in theme attribute"),
            }
        }

        // Generate Default field based on type
        if let Some(expr_str) = default_expr_str {
            // Arbitrary expression (for Option types, nested themes, etc.)
            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap_or_else(|_| {
                panic!("Failed to parse default_expr for field `{}`", field_name)
            });
            default_fields.push(quote! {
                #field_name: #expr
            });
        } else if let Some(f32_val) = default_f32 {
            // f32 field
            default_fields.push(quote! {
                #field_name: #f32_val as f32
            });
        } else if let Some(default_val) = default_value {
            // Check if it's RGB (6 hex digits) or RGBA (8 hex digits)
            let default_expr = if default_val > 0xFFFFFF {
                // RGBA - use rgba()
                quote! { gpui::rgba(#default_val) }
            } else {
                // RGB - use rgb()
                quote! { gpui::rgb(#default_val) }
            };

            default_fields.push(quote! {
                #field_name: #default_expr
            });
        } else {
            panic!(
                "Field `{}` is missing `default`, `default_f32`, or `default_expr` in #[theme(...)]",
                field_name
            );
        }

        // Generate From<&Theme> field
        if let Some(expr_str) = from_expr {
            let expr: syn::Expr = syn::parse_str(&expr_str)
                .unwrap_or_else(|_| panic!("Failed to parse from_expr for field `{}`", field_name));
            from_fields.push(quote! {
                #field_name: #expr
            });
        } else if let Some(from) = from_field {
            from_fields.push(quote! {
                #field_name: theme.#from
            });
        } else {
            panic!(
                "Field `{}` needs either `from` or `from_expr` in #[theme(...)]",
                field_name
            );
        }
    }

    let expanded = quote! {
        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#default_fields),*
                }
            }
        }

        impl From<&crate::theme::Theme> for #name {
            fn from(theme: &crate::theme::Theme) -> Self {
                Self {
                    #(#from_fields),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
