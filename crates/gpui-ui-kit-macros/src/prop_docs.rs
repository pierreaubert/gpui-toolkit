//! Machine-readable prop tables for showcase/Storybook parity.
//!
//! [`build_prop_docs_json`] renders the `#[field(...)]` metadata of a
//! `ComponentBuilder`/`FormField` struct as a JSON array. Component-lab and
//! the showcase can parse the emitted `__PROP_DOCS_JSON` const to generate
//! prop tables and controls without hand-maintained documentation.
//!
//! Entry shape:
//!
//! ```json
//! [
//!   {
//!     "name": "id",
//!     "setter": "id",
//!     "type": "String",
//!     "required": true,
//!     "optional": false,
//!     "into": true,
//!     "has_setter": true,
//!     "default": null,
//!     "doc": "Element id"
//!   }
//! ]
//! ```

use super::builder_field::BuilderField;
use syn::punctuated::Punctuated;
use syn::{Lit, Meta, Token};

/// Escape a string for embedding in JSON output.
fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

/// Collect `///` doc lines from a field's attributes.
fn field_doc(field: &syn::Field) -> String {
    let mut lines = Vec::new();
    for attr in &field.attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(lit) = &nv.value
            && let Lit::Str(text) = &lit.lit
        {
            lines.push(text.value().trim().to_string());
        }
    }
    lines.join("\n")
}

/// Render whitespace-normalized type text (`Option < String >` becomes
/// `Option<String>`) so tables and snapshots stay readable.
fn normalized_type(ty: &syn::Type) -> String {
    quote::quote!(#ty)
        .to_string()
        .split_whitespace()
        .collect::<String>()
}

/// Build the JSON prop table for one builder struct.
///
/// `fields` and `parsed` must be in the same order (both derived from the
/// struct's named fields).
pub(crate) fn build_prop_docs_json(
    fields: &Punctuated<syn::Field, Token![,]>,
    parsed: &[BuilderField<'_>],
) -> String {
    let mut json = String::from("[");
    for (index, field) in fields.iter().enumerate() {
        let Some(entry) = parsed.get(index) else {
            continue;
        };
        let name = entry.ident.to_string();
        let setter = entry.setter_name.to_string();
        let ty = normalized_type(&entry.effective_arg_ty());
        let doc = field_doc(field);
        let default = if entry.required {
            "null".to_string()
        } else if let Some(default_expr) = &entry.default_expr {
            format!(
                "\"{}\"",
                escape_json(
                    &quote::quote!(#default_expr)
                        .to_string()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            )
        } else if entry.optional {
            "\"None\"".to_string()
        } else {
            "\"Default::default()\"".to_string()
        };
        if index != 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"name\":\"{}\",\"setter\":\"{}\",\"type\":\"{}\",\"required\":{},\"optional\":{},\"into\":{},\"has_setter\":{},\"default\":{},\"doc\":\"{}\"}}",
            escape_json(&name),
            escape_json(&setter),
            escape_json(&ty),
            entry.required,
            entry.optional,
            entry.into,
            entry.generate_setter,
            default,
            escape_json(&doc),
        ));
    }
    json.push(']');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_struct(input: &str) -> syn::DeriveInput {
        syn::parse_str(input).unwrap()
    }

    fn named_fields(input: &syn::DeriveInput) -> &Punctuated<syn::Field, Token![,]> {
        match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(fields) => &fields.named,
                _ => panic!("expected named fields"),
            },
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn renders_required_optional_and_defaults() {
        let input = parse_struct(
            r#"
            pub struct Props {
                /// Element id.
                #[field(required, into)]
                pub id: String,
                /// Optional label.
                #[field(optional, into)]
                pub label: Option<String>,
                #[field(default = "true")]
                pub enabled: bool,
                #[field(skip, default = "99")]
                pub skipped: usize,
            }
            "#,
        );
        let fields = named_fields(&input);
        let parsed: Vec<BuilderField<'_>> = fields
            .iter()
            .map(BuilderField::parse)
            .collect::<Result<_, _>>()
            .unwrap();
        let json = build_prop_docs_json(fields, &parsed);
        assert!(json.contains(r#""name":"id""#), "{json}");
        assert!(json.contains(r#""required":true"#), "{json}");
        assert!(json.contains(r#""type":"String""#), "{json}");
        assert!(json.contains(r#""doc":"Element id.""#), "{json}");
        assert!(json.contains(r#""name":"label""#), "{json}");
        assert!(json.contains(r#""optional":true"#), "{json}");
        assert!(json.contains(r#""type":"String""#), "{json}");
        assert!(json.contains(r#""default":"None""#), "{json}");
        assert!(json.contains(r#""default":"true""#), "{json}");
        assert!(json.contains(r#""has_setter":false"#), "{json}");
    }

    #[test]
    fn escapes_json_specials_in_docs() {
        let input = parse_struct(
            r#"
            pub struct Props {
                /// Say "hi" with a backslash \ here.
                #[field(optional)]
                pub label: Option<String>,
            }
            "#,
        );
        let fields = named_fields(&input);
        let parsed: Vec<BuilderField<'_>> = fields
            .iter()
            .map(BuilderField::parse)
            .collect::<Result<_, _>>()
            .unwrap();
        let json = build_prop_docs_json(fields, &parsed);
        assert!(
            json.contains(r#"Say \"hi\" with a backslash \\ here."#),
            "{json}"
        );
    }
}
