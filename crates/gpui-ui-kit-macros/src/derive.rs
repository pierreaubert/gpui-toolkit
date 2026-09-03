use super::builder_field::BuilderField;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Expr, Fields, Lit, LitInt, Meta, Token};

pub(crate) fn combined_compile_error(errors: Vec<syn::Error>) -> TokenStream {
    let mut iter = errors.into_iter();
    let mut combined = iter.next().expect("expected at least one macro error");
    for error in iter {
        combined.combine(error);
    }
    combined.to_compile_error()
}

fn parse_string_attribute(
    attr: &syn::Attribute,
    attr_name: &str,
) -> Result<Option<(String, proc_macro2::Span)>, syn::Error> {
    if !attr.path().is_ident(attr_name) {
        return Ok(None);
    }

    let Meta::NameValue(nv) = &attr.meta else {
        return Err(syn::Error::new(
            attr.span(),
            format!("`{attr_name}` must use `#[{attr_name} = \"path\"]` syntax"),
        ));
    };

    let Expr::Lit(lit) = &nv.value else {
        return Err(syn::Error::new(
            nv.value.span(),
            format!("`{attr_name}` must be a string literal"),
        ));
    };

    let Lit::Str(value) = &lit.lit else {
        return Err(syn::Error::new(
            lit.lit.span(),
            format!("`{attr_name}` must be a string literal"),
        ));
    };

    Ok(Some((value.value(), value.span())))
}

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
/// # Struct-Level Attributes
///
/// | Attribute | Description | Default |
/// |-----------|-------------|---------|
/// | `theme_path` | Path to the global Theme type | `crate::theme::Theme` |
/// | `gpui_path`  | Path to the gpui crate | `gpui` |
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
/// **Text:** `text_primary`, `text_secondary`, `text_muted`, `text_on_accent`, `icon_on_accent`
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
/// The macro emits a compile error if:
/// - A field is missing the `#[theme(...)]` attribute
/// - A field is missing `default`, `default_f32`, or `default_expr`
/// - A field is missing `from` or `from_expr`
/// - An expression in `from_expr` or `default_expr` fails to parse
/// - A numeric literal is out of range for the expected type
// Scan a `from_expr` string for `theme.<field>` references. The coverage
// gate in `THEME_SOURCES` uses this so fields consumed indirectly — for
// example via `from_expr = "with_alpha(theme.accent, 0.2)"` — are still
// attributed to the component theme.
fn theme_refs_in_expr(expr: &str) -> Vec<String> {
    let chars: Vec<char> = expr.chars().collect();
    let mut refs = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let rest: String = chars[index..].iter().collect();
        if let Some(stripped) = rest.strip_prefix("theme.") {
            let end: usize = stripped
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .map(|ch| ch.len_utf8())
                .sum();
            if end > 0 {
                refs.push(stripped[..end].to_string());
                index += "theme.".len() + end;
            } else {
                index += chars[index].len_utf8();
            }
        } else {
            index += chars[index].len_utf8();
        }
    }
    refs
}

pub(crate) fn derive_component_theme_impl(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(e) => return e.to_compile_error(),
    };
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new(
                    data.fields.span(),
                    "ComponentTheme only supports structs with named fields",
                )
                .to_compile_error();
            }
        },
        _ => {
            return syn::Error::new(input.span(), "ComponentTheme only supports structs")
                .to_compile_error();
        }
    };

    // Parse struct-level attributes for theme_path and gpui_path
    let mut theme_path_str = "crate::theme::Theme".to_string();
    let mut theme_path_span = input.ident.span();
    let mut gpui_path_str = "gpui".to_string();
    let mut gpui_path_span = input.ident.span();
    let mut errors = Vec::new();

    for attr in &input.attrs {
        match parse_string_attribute(attr, "theme_path") {
            Ok(Some((value, span))) => {
                theme_path_str = value;
                theme_path_span = span;
            }
            Ok(None) => {}
            Err(error) => errors.push(error),
        }

        match parse_string_attribute(attr, "gpui_path") {
            Ok(Some((value, span))) => {
                gpui_path_str = value;
                gpui_path_span = span;
            }
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }

    let theme_path: syn::Type = match syn::parse_str(&theme_path_str) {
        Ok(t) => t,
        Err(e) => {
            errors.push(syn::Error::new(
                theme_path_span,
                format!("Invalid theme_path: {e}"),
            ));
            syn::parse_quote!(crate::theme::Theme)
        }
    };

    let gpui_path: syn::Path = match syn::parse_str(&gpui_path_str) {
        Ok(p) => p,
        Err(e) => {
            errors.push(syn::Error::new(
                gpui_path_span,
                format!("Invalid gpui_path: {e}"),
            ));
            syn::parse_quote!(gpui)
        }
    };

    if !errors.is_empty() {
        return combined_compile_error(errors);
    }

    let field_count = fields.len();
    let mut default_fields = Vec::with_capacity(field_count);
    let mut from_fields = Vec::with_capacity(field_count);
    let mut theme_sources: Vec<String> = Vec::with_capacity(field_count);
    let mut errors: Vec<syn::Error> = Vec::with_capacity(field_count);

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_span = field.ident.span();

        // Find the #[theme(...)] attribute
        let mut theme_attrs = field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("theme"));

        let Some(attr) = theme_attrs.next() else {
            errors.push(syn::Error::new(
                field_span,
                format!("Field `{field_name}` is missing #[theme(...)] attribute"),
            ));
            continue;
        };
        if let Some(duplicate) = theme_attrs.next() {
            errors.push(syn::Error::new(
                duplicate.span(),
                format!("Field `{field_name}` has multiple #[theme(...)] attributes"),
            ));
            continue;
        }

        let mut default_value: Option<u32> = None;
        let mut default_int_lit: Option<LitInt> = None;
        let mut default_f32: Option<f64> = None;
        let mut default_expr_str: Option<String> = None;
        let mut from_field: Option<syn::Ident> = None;
        let mut from_expr: Option<String> = None;
        let mut seen_attribute_keys = HashSet::new();

        // Parse the attribute arguments
        let nested = match attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
            Ok(n) => n,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        for meta in nested {
            match meta {
                Meta::NameValue(nv) => {
                    let ident = match nv.path.get_ident() {
                        Some(i) => i,
                        None => {
                            errors.push(syn::Error::new(nv.path.span(), "Expected identifier"));
                            continue;
                        }
                    };
                    if !seen_attribute_keys.insert(ident.to_string()) {
                        errors.push(syn::Error::new(
                            ident.span(),
                            format!("Duplicate theme attribute: {ident}"),
                        ));
                        continue;
                    }
                    if ident == "default" {
                        if let Expr::Lit(lit) = &nv.value
                            && let Lit::Int(int_lit) = &lit.lit
                        {
                            match int_lit.base10_parse::<u32>() {
                                Ok(v) => {
                                    default_value = Some(v);
                                    default_int_lit = Some(int_lit.clone());
                                }
                                Err(e) => {
                                    errors.push(syn::Error::new(
                                        int_lit.span(),
                                        format!(
                                            "Unable to parse default value for field `{field_name}`: {e}"
                                        ),
                                    ));
                                }
                            }
                        } else {
                            errors.push(syn::Error::new(
                                nv.value.span(),
                                format!(
                                    "`default` for field `{field_name}` must be an integer literal such as 0x007acc or 0x007accff"
                                ),
                            ));
                        }
                    } else if ident == "default_f32" {
                        if let Expr::Lit(lit) = &nv.value {
                            match &lit.lit {
                                Lit::Float(f) => match f.base10_parse::<f64>() {
                                    Ok(v) => default_f32 = Some(v),
                                    Err(e) => {
                                        errors.push(syn::Error::new(
                                                f.span(),
                                                format!(
                                                    "Unable to parse default_f32 value for field `{field_name}`: {e}"
                                                ),
                                            ));
                                    }
                                },
                                Lit::Int(i) => {
                                    // Allow integers like 0 or 1
                                    match i.base10_parse::<i64>() {
                                        Ok(v) => default_f32 = Some(v as f64),
                                        Err(e) => {
                                            errors.push(syn::Error::new(
                                                i.span(),
                                                format!(
                                                    "Unable to parse default_f32 value for field `{field_name}`: {e}"
                                                ),
                                            ));
                                        }
                                    }
                                }
                                _ => errors.push(syn::Error::new(
                                    lit.lit.span(),
                                    format!(
                                        "`default_f32` for field `{field_name}` must be an integer or float literal"
                                    ),
                                )),
                            }
                        } else {
                            errors.push(syn::Error::new(
                                nv.value.span(),
                                format!(
                                    "`default_f32` for field `{field_name}` must be an integer or float literal"
                                ),
                            ));
                        }
                    } else if ident == "default_expr" {
                        if let Expr::Lit(lit) = &nv.value
                            && let Lit::Str(s) = &lit.lit
                        {
                            default_expr_str = Some(s.value());
                        } else {
                            errors.push(syn::Error::new(
                                nv.value.span(),
                                format!(
                                    "`default_expr` for field `{field_name}` must be a string literal containing a Rust expression"
                                ),
                            ));
                        }
                    } else if ident == "from" {
                        if let Expr::Path(path) = &nv.value {
                            from_field = path.path.get_ident().cloned();
                            if from_field.is_none() {
                                errors.push(syn::Error::new(
                                    path.path.span(),
                                    format!(
                                        "`from` for field `{field_name}` must be a single Theme field identifier"
                                    ),
                                ));
                            }
                        } else {
                            errors.push(syn::Error::new(
                                nv.value.span(),
                                format!(
                                    "`from` for field `{field_name}` must be a single Theme field identifier"
                                ),
                            ));
                        }
                    } else if ident == "from_expr" {
                        if let Expr::Lit(lit) = &nv.value
                            && let Lit::Str(s) = &lit.lit
                        {
                            from_expr = Some(s.value());
                        } else {
                            errors.push(syn::Error::new(
                                nv.value.span(),
                                format!(
                                    "`from_expr` for field `{field_name}` must be a string literal containing a Rust expression"
                                ),
                            ));
                        }
                    } else {
                        errors.push(syn::Error::new(
                            ident.span(),
                            format!("Unknown theme attribute: {ident}"),
                        ));
                    }
                }
                _ => {
                    errors.push(syn::Error::new(
                        meta.span(),
                        "Expected name = value in theme attribute",
                    ));
                }
            }
        }

        // Generate Default field based on type
        if let Some(expr_str) = default_expr_str {
            // Arbitrary expression (for Option types, nested themes, etc.)
            let expr: syn::Expr = match syn::parse_str(&expr_str) {
                Ok(e) => e,
                Err(error) => {
                    errors.push(syn::Error::new(
                        field_span,
                        format!("Failed to parse default_expr for field `{field_name}`: {error}"),
                    ));
                    continue;
                }
            };
            default_fields.push(quote! {
                #field_name: #expr
            });
        } else if let Some(f32_val) = default_f32 {
            // f32 field
            default_fields.push(quote! {
                #field_name: #f32_val as f32
            });
        } else if let Some(default_val) = default_value {
            // Check if it's RGB (6 hex digits) or RGBA (8 hex digits) by
            // inspecting the original literal string rather than the numeric
            // value. This avoids misclassifying transparent colors such as
            // 0x00000000 as RGB.
            let is_rgba = default_int_lit.as_ref().is_some_and(|literal| {
                let raw = literal.to_string();
                let literal_without_suffix =
                    raw.strip_suffix(literal.suffix()).unwrap_or(raw.as_str());
                literal_without_suffix
                    .strip_prefix("0x")
                    .or_else(|| literal_without_suffix.strip_prefix("0X"))
                    .is_some_and(|digits| {
                        digits.chars().filter(|character| *character != '_').count() == 8
                    })
            }) || default_val > 0xFFFFFF;

            let default_expr = if is_rgba {
                quote! { #gpui_path::rgba(#default_val) }
            } else {
                quote! { #gpui_path::rgb(#default_val) }
            };

            default_fields.push(quote! {
                #field_name: #default_expr
            });
        } else {
            errors.push(syn::Error::new(
                field_span,
                format!(
                    "Field `{field_name}` is missing `default`, `default_f32`, or `default_expr` in #[theme(...)]"
                ),
            ));
            continue;
        }

        // Generate From<&Theme> field
        if let Some(expr_str) = from_expr {
            theme_sources.extend(theme_refs_in_expr(&expr_str));
            let expr: syn::Expr = match syn::parse_str(&expr_str) {
                Ok(e) => e,
                Err(error) => {
                    errors.push(syn::Error::new(
                        field_span,
                        format!("Failed to parse from_expr for field `{field_name}`: {error}"),
                    ));
                    continue;
                }
            };
            from_fields.push(quote! {
                #field_name: #expr
            });
        } else if let Some(from) = from_field {
            theme_sources.push(from.to_string());
            from_fields.push(quote! {
                #field_name: theme.#from
            });
        } else {
            errors.push(syn::Error::new(
                field_span,
                format!("Field `{field_name}` needs either `from` or `from_expr` in #[theme(...)]"),
            ));
            continue;
        }
    }

    if !errors.is_empty() {
        return combined_compile_error(errors);
    }

    theme_sources.sort();
    theme_sources.dedup();

    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #(#default_fields),*
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics From<&#theme_path> for #name #ty_generics #where_clause {
            fn from(theme: &#theme_path) -> Self {
                Self {
                    #(#from_fields),*
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics From<std::sync::Arc<#theme_path>> for #name #ty_generics #where_clause {
            fn from(theme: std::sync::Arc<#theme_path>) -> Self {
                Self::from(theme.as_ref())
            }
        }

        #[automatically_derived]
        impl #impl_generics From<&std::sync::Arc<#theme_path>> for #name #ty_generics #where_clause {
            fn from(theme: &std::sync::Arc<#theme_path>) -> Self {
                Self::from(theme.as_ref())
            }
        }

        #[automatically_derived]
        impl #impl_generics #name #ty_generics #where_clause {
            /// Global theme fields consumed by this component theme.
            ///
            /// Sorted, deduplicated list of `from = <field>` targets plus
            /// `theme.<field>` references scraped from `from_expr` strings.
            /// The theme-coverage test asserts that every global `Theme` field
            /// appears in at least one component theme's list, catching drift
            /// when theme fields are added or renamed.
            pub const THEME_SOURCES: &'static [&'static str] = &[#(#theme_sources),*];
        }
    };

    expanded
}

/// Derive a fluent builder API for GPUI component structs.
///
/// Field attributes use the documented `#[field(...)]` syntax:
///
/// - `required` includes the field in `new(...)` as `impl Into<T>`
/// - `optional` initializes the field as `None` and makes the setter wrap `Some(...)`
/// - `into` makes a non-required setter accept `impl Into<T>`; required constructor
///   arguments already do
/// - `builder = false` or `skip` omits the setter
/// - `default = "expr"` uses an explicit default expression
/// - `rename = "method_name"` changes the generated setter name
pub(crate) fn derive_component_builder_impl(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(e) => return e.to_compile_error(),
    };
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "ComponentBuilder only supports structs with named fields",
                )
                .to_compile_error();
            }
        },
        _ => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "ComponentBuilder only supports structs",
            )
            .to_compile_error();
        }
    };

    let field_count = fields.len();
    let mut errors = Vec::with_capacity(field_count);
    let mut parsed_fields = Vec::with_capacity(field_count);

    for field in fields {
        match BuilderField::parse(field) {
            Ok(parsed) => parsed_fields.push(parsed),
            Err(error) => errors.push(error),
        }
    }

    if !errors.is_empty() {
        return combined_compile_error(errors);
    }

    let new_args = parsed_fields
        .iter()
        .filter(|field| field.required)
        .map(BuilderField::new_arg);
    let initializers = parsed_fields.iter().map(BuilderField::initializer);
    let setters = parsed_fields
        .iter()
        .filter(|field| field.generate_setter)
        .map(BuilderField::setter);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let docs_json = super::prop_docs::build_prop_docs_json(fields, &parsed_fields);
    let docs_literal = proc_macro2::Literal::string(&docs_json);

    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn new(#(#new_args),*) -> Self {
                Self {
                    #(#initializers),*
                }
            }

            #(#setters)*

            /// Machine-readable prop table for showcase/Storybook-style docs.
            ///
            /// Generated from the `#[field(...)]` attributes and `///` doc
            /// comments. Parse with any JSON reader; see
            /// `prop_docs` module docs for the entry shape.
            pub const __PROP_DOCS_JSON: &'static str = #docs_literal;
        }
    };

    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme_derive(input: &str) -> String {
        derive_component_theme_impl(input.parse().unwrap()).to_string()
    }

    #[test]
    fn theme_happy_path_generates_default_and_from() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                pub primary: u32,
            }
        "#,
        );
        assert!(out.contains("impl Default for MyTheme"));
        assert!(out.contains("impl From < & crate :: theme :: Theme > for MyTheme"));
    }

    #[test]
    fn theme_generates_from_arc_and_arc_ref() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                pub primary: u32,
            }
        "#,
        );
        assert!(
            out.contains("impl From < std :: sync :: Arc < crate :: theme :: Theme >> for MyTheme")
        );
        assert!(
            out.contains(
                "impl From < & std :: sync :: Arc < crate :: theme :: Theme >> for MyTheme"
            )
        );
    }

    #[test]
    fn generated_impls_are_marked_automatically_derived() {
        let theme = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                pub primary: u32,
            }
            "#,
        );
        assert_eq!(theme.matches("automatically_derived").count(), 5);

        let builder = builder_derive(
            r#"
            #[derive(ComponentBuilder)]
            pub struct MyBuilder {
                #[field(required)]
                pub id: String,
            }
            "#,
        );
        assert_eq!(builder.matches("automatically_derived").count(), 1);
    }

    #[test]
    fn theme_missing_attribute_emits_error() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                pub primary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("missing #[theme(...)] attribute"));
    }

    #[test]
    fn theme_unknown_attribute_emits_error() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent, unknown = 1)]
                pub primary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("Unknown theme attribute"));
    }

    #[test]
    fn theme_duplicate_attributes_and_keys_emit_errors() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                #[theme(default = 0x00ff00, from = surface)]
                pub primary: u32,
                #[theme(default = 0x007acc, default = 0x00ff00, from = accent)]
                pub secondary: u32,
            }
            "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("has multiple #[theme(...)] attributes"));
        assert!(out.contains("Duplicate theme attribute"));
    }

    #[test]
    fn theme_struct_path_attributes_require_string_literals() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            #[theme_path = 1]
            #[gpui_path = gpui]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                pub primary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("`theme_path` must be a string literal"));
        assert!(out.contains("`gpui_path` must be a string literal"));
    }

    #[test]
    fn theme_invalid_struct_paths_report_parse_errors() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            #[theme_path = "::"]
            #[gpui_path = "crate::"]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                pub primary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("Invalid theme_path"));
        assert!(out.contains("Invalid gpui_path"));
    }

    #[test]
    fn theme_field_attributes_reject_wrong_literal_shapes() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = "blue", default_f32 = "1.0", default_expr = 1, from = theme.accent, from_expr = 1)]
                pub primary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("`default` for field `primary` must be an integer literal"));
        assert!(
            out.contains("`default_f32` for field `primary` must be an integer or float literal")
        );
        assert!(out.contains("`default_expr` for field `primary` must be a string literal"));
        assert!(out.contains("`from` for field `primary` must be a single Theme field identifier"));
        assert!(out.contains("`from_expr` for field `primary` must be a string literal"));
    }

    #[test]
    fn theme_expression_parse_errors_include_field_context() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default_expr = "Some(", from_expr = "theme.accent")]
                pub primary: u32,
                #[theme(default_expr = "0", from_expr = "theme.")]
                pub secondary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("Failed to parse default_expr for field `primary`"));
        assert!(out.contains("Failed to parse from_expr for field `secondary`"));
    }

    #[test]
    fn theme_multiple_errors_are_combined() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                pub primary: u32,
                pub secondary: u32,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        // Two missing-attribute errors should be combined into one compile_error.
        assert!(out.contains("missing #[theme(...)] attribute"));
    }

    fn builder_derive(input: &str) -> String {
        derive_component_builder_impl(input.parse().unwrap()).to_string()
    }

    #[test]
    fn theme_sources_lists_from_fields_and_expr_refs() {
        let out = theme_derive(
            r#"
            #[derive(ComponentTheme)]
            pub struct MyTheme {
                #[theme(default = 0x007acc, from = accent)]
                pub primary: u32,
                #[theme(default = 0x007acc, from = accent)]
                pub secondary: u32,
                #[theme(default_expr = "0", from_expr = "with_alpha(theme.surface, 0.2)")]
                pub tinted: u32,
            }
            "#,
        );
        assert!(out.contains("pub const THEME_SOURCES"), "{out}");
        assert!(out.contains("\"accent\""), "{out}");
        assert!(out.contains("\"surface\""), "{out}");
        // Deduplicated: accent appears once despite two `from = accent` fields.
        assert_eq!(out.matches("\"accent\"").count(), 1, "{out}");
    }

    #[test]
    fn builder_emits_prop_docs_json() {
        let out = builder_derive(
            r#"
            #[derive(ComponentBuilder)]
            pub struct MyBuilder {
                /// Element id.
                #[field(required)]
                pub id: String,
                #[field(optional)]
                pub label: Option<String>,
            }
            "#,
        );
        assert!(out.contains("__PROP_DOCS_JSON"), "{out}");
        assert!(out.contains("Element id."), "{out}");
    }

    #[test]
    fn builder_happy_path_generates_constructor_and_setters() {
        let out = builder_derive(
            r#"
            #[derive(ComponentBuilder)]
            pub struct MyBuilder {
                #[field(required)]
                pub id: String,
                #[field(optional)]
                pub label: Option<String>,
            }
        "#,
        );
        assert!(out.contains("impl MyBuilder"));
        assert!(out.contains("pub fn new"));
        assert!(out.contains("pub fn label"));
    }

    #[test]
    fn builder_unknown_attribute_emits_error() {
        let out = builder_derive(
            r#"
            #[derive(ComponentBuilder)]
            pub struct MyBuilder {
                #[field(unknown)]
                pub id: String,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("unknown builder field attribute"));
    }

    #[test]
    fn builder_required_and_optional_emits_error() {
        let out = builder_derive(
            r#"
            #[derive(ComponentBuilder)]
            pub struct MyBuilder {
                #[field(required, optional)]
                pub id: String,
            }
        "#,
        );
        assert!(out.contains("compile_error !"));
        assert!(out.contains("cannot be both required and optional"));
    }
}
