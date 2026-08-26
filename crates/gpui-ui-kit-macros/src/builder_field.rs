use super::misc::option_inner_type;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, Ident, Lit, Meta, Token, Type};

fn is_rust_keyword(identifier: &str) -> bool {
    matches!(
        identifier,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "gen"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "macro_rules"
            | "union"
    )
}

pub(super) struct BuilderField<'a> {
    pub(super) ident: &'a Ident,
    pub(super) ty: &'a Type,
    pub(super) required: bool,
    pub(super) optional: bool,
    pub(super) into: bool,
    pub(super) generate_setter: bool,
    pub(super) default_expr: Option<Expr>,
    pub(super) setter_name: Ident,
    pub(super) option_inner_ty: Option<Type>,
}

impl<'a> BuilderField<'a> {
    pub(super) fn parse(field: &'a syn::Field) -> Result<Self, syn::Error> {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "expected named field"))?;

        let mut required = false;
        let mut required_span = None;
        let mut optional = false;
        let mut optional_span = None;
        let mut into = false;
        let mut generate_setter = true;
        let mut default_expr = None;
        let mut setter_name = ident.clone();

        for attr in &field.attrs {
            if !attr.path().is_ident("field") && !attr.path().is_ident("builder") {
                continue;
            }

            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in nested {
                match meta {
                    Meta::Path(path) => {
                        if path.is_ident("required") {
                            required = true;
                            required_span = Some(path.span());
                        } else if path.is_ident("optional") {
                            optional = true;
                            optional_span = Some(path.span());
                        } else if path.is_ident("into") {
                            into = true;
                        } else if path.is_ident("skip") {
                            generate_setter = false;
                        } else {
                            return Err(syn::Error::new(
                                path.span(),
                                "unknown builder field attribute; expected one of `required`, `optional`, `into`, `skip`, `builder`, `default`, `rename`, or `name`",
                            ));
                        }
                    }
                    Meta::NameValue(nv) => {
                        let Some(name) = nv.path.get_ident() else {
                            return Err(syn::Error::new(nv.path.span(), "expected identifier"));
                        };
                        if name == "builder" {
                            if let Expr::Lit(lit) = &nv.value
                                && let Lit::Bool(value) = &lit.lit
                            {
                                generate_setter = value.value;
                            } else {
                                return Err(syn::Error::new(
                                    nv.value.span(),
                                    "builder must be a boolean",
                                ));
                            }
                        } else if name == "default" {
                            if let Expr::Lit(lit) = &nv.value
                                && let Lit::Str(value) = &lit.lit
                            {
                                let expr = value.parse().map_err(|error| {
                                    syn::Error::new(
                                        value.span(),
                                        format!("default must parse as a Rust expression: {error}"),
                                    )
                                })?;
                                default_expr = Some(expr);
                            } else {
                                return Err(syn::Error::new(
                                    nv.value.span(),
                                    "default must be a string expression",
                                ));
                            }
                        } else if name == "rename" || name == "name" {
                            if let Expr::Lit(lit) = &nv.value
                                && let Lit::Str(value) = &lit.lit
                            {
                                let rename = value.value();
                                if is_rust_keyword(&rename) {
                                    return Err(syn::Error::new(
                                        value.span(),
                                        "rename must not be a Rust keyword; use a raw identifier such as `r#type` instead",
                                    ));
                                }
                                setter_name =
                                    syn::parse_str::<Ident>(&rename).map_err(|error| {
                                        syn::Error::new(
                                            value.span(),
                                            format!(
                                                "rename must be a valid Rust identifier: {error}"
                                            ),
                                        )
                                    })?;
                            } else {
                                return Err(syn::Error::new(
                                    nv.value.span(),
                                    "rename must be a string",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new(
                                name.span(),
                                "unknown builder field attribute; expected one of `required`, `optional`, `into`, `skip`, `builder`, `default`, `rename`, or `name`",
                            ));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new(
                            meta.span(),
                            "expected path or name = value in builder field attribute",
                        ));
                    }
                }
            }
        }

        if required && optional {
            let mut error = syn::Error::new(
                required_span.unwrap_or_else(|| field.span()),
                "field cannot be both required and optional; remove `required` or `optional`",
            );
            error.combine(syn::Error::new(
                optional_span.unwrap_or_else(|| field.span()),
                "`optional` conflicts with `required` on the same field",
            ));
            return Err(error);
        }

        let option_inner_ty = option_inner_type(&field.ty);
        if optional && option_inner_ty.is_none() {
            return Err(syn::Error::new(
                optional_span.unwrap_or_else(|| field.span()),
                "`optional` requires a field whose type is `Option<T>`, `std::option::Option<T>`, or `core::option::Option<T>`",
            ));
        }

        Ok(Self {
            ident,
            ty: &field.ty,
            required,
            optional,
            into,
            generate_setter,
            default_expr,
            setter_name,
            option_inner_ty,
        })
    }

    pub(super) fn effective_arg_ty(&self) -> Type {
        if self.optional {
            self.option_inner_ty
                .clone()
                .unwrap_or_else(|| self.ty.clone())
        } else {
            self.ty.clone()
        }
    }

    pub(super) fn new_arg(&self) -> proc_macro2::TokenStream {
        let ident = self.ident;
        let arg_ty = self.effective_arg_ty();
        if self.required || self.into {
            quote! { #ident: impl Into<#arg_ty> }
        } else {
            quote! { #ident: #arg_ty }
        }
    }

    pub(super) fn initializer(&self) -> proc_macro2::TokenStream {
        let ident = self.ident;
        if self.required {
            if self.optional {
                quote! { #ident: Some(#ident.into()) }
            } else {
                quote! { #ident: #ident.into() }
            }
        } else if let Some(default_expr) = &self.default_expr {
            quote! { #ident: #default_expr }
        } else if self.optional {
            quote! { #ident: None }
        } else {
            quote! { #ident: ::core::default::Default::default() }
        }
    }

    pub(super) fn setter(&self) -> proc_macro2::TokenStream {
        let field = self.ident;
        let method = &self.setter_name;
        let arg_ty = self.effective_arg_ty();
        let assignment = if self.optional {
            if self.into {
                quote! { self.#field = Some(#field.into()); }
            } else {
                quote! { self.#field = Some(#field); }
            }
        } else if self.into {
            quote! { self.#field = #field.into(); }
        } else {
            quote! { self.#field = #field; }
        };

        if self.into {
            quote! {
                pub fn #method(mut self, #field: impl Into<#arg_ty>) -> Self {
                    #assignment
                    self
                }
            }
        } else {
            quote! {
                pub fn #method(mut self, #field: #arg_ty) -> Self {
                    #assignment
                    self
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{Data, Fields};

    fn extract_field(input: &syn::DeriveInput) -> &syn::Field {
        match &input.data {
            Data::Struct(data) => match &data.fields {
                Fields::Named(fields) => fields.named.first().expect("expected a field"),
                _ => panic!("expected named fields"),
            },
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn parse_required_optional_into_skip_default_rename() {
        let src = r#"
            #[field(required, into, default = "String::from(\"x\")", rename = "id_value")]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let field = extract_field(&input);
        let f = BuilderField::parse(field).unwrap();
        assert!(f.required);
        assert!(f.into);
        assert!(f.default_expr.is_some());
        assert_eq!(f.setter_name.to_string(), "id_value");
    }

    #[test]
    fn parse_skip_omits_setter() {
        let src = r#"
            #[field(skip, default = "99")]
            pub ignored: usize
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let field = extract_field(&input);
        let f = BuilderField::parse(field).unwrap();
        assert!(!f.generate_setter);
    }

    #[test]
    fn parse_unknown_attribute_errors() {
        let src = r#"
            #[field(unknown)]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let field = extract_field(&input);
        let err = match BuilderField::parse(field) {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("unknown builder field attribute"));
    }

    #[test]
    fn parse_invalid_default_expression_reports_literal_span() {
        let src = r#"
            #[field(default = "String::from(")]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let field = extract_field(&input);
        let err = match BuilderField::parse(field) {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("default must parse as a Rust expression")
        );
    }

    #[test]
    fn parse_invalid_rename_reports_error_instead_of_panicking() {
        let src = r#"
            #[field(rename = "not-valid")]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let field = extract_field(&input);
        let err = match BuilderField::parse(field) {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("rename must be a valid Rust identifier")
        );
    }

    #[test]
    fn parse_keyword_rename_reports_attribute_error() {
        let src = r#"
            #[field(rename = "fn")]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let err = match BuilderField::parse(extract_field(&input)) {
            Ok(_) => panic!("expected an error"),
            Err(error) => error,
        };
        assert!(err.to_string().contains("must not be a Rust keyword"));

        let src = r#"
            #[field(rename = "r#type")]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        assert_eq!(
            BuilderField::parse(extract_field(&input))
                .unwrap()
                .setter_name
                .to_string(),
            "r#type"
        );
    }

    #[test]
    fn parse_required_optional_reports_both_conflicting_attributes() {
        let src = r#"
            #[field(required, optional)]
            pub id: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let field = extract_field(&input);
        let err = match BuilderField::parse(field) {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        };
        let out = err.to_compile_error().to_string();
        assert!(out.contains("remove `required` or `optional`"));
        assert!(out.contains("`optional` conflicts with `required`"));
    }

    #[test]
    fn parse_optional_requires_a_standard_option_type() {
        let src = r#"
            #[field(optional)]
            pub label: String
        "#;
        let wrapped = format!("struct __TestStruct {{ {src} }}");
        let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
        let err = match BuilderField::parse(extract_field(&input)) {
            Ok(_) => panic!("expected an error"),
            Err(error) => error,
        };
        assert!(
            err.to_string()
                .contains("`optional` requires a field whose type")
        );

        for ty in [
            "Option<String>",
            "std::option::Option<String>",
            "core::option::Option<String>",
        ] {
            let wrapped = format!("struct __TestStruct {{ #[field(optional)] pub label: {ty} }}");
            let input: syn::DeriveInput = syn::parse_str(&wrapped).unwrap();
            assert!(
                BuilderField::parse(extract_field(&input))
                    .unwrap()
                    .option_inner_ty
                    .is_some()
            );
        }

        let wrapped = "struct __TestStruct { pub label: my_mod::Option<String> }";
        let input: syn::DeriveInput = syn::parse_str(wrapped).unwrap();
        assert!(
            BuilderField::parse(extract_field(&input))
                .unwrap()
                .option_inner_ty
                .is_none()
        );
    }
}
