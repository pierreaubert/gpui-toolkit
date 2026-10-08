//! `layout!` function-like macro: token DSL expanding to element chains.
//!
//! The token syntax mirrors layout expressions with braces instead of
//! `>`/`+`:
//!
//! ```ignore
//! layout! {
//!     V {
//!         H {
//!             B.primary("Save") #save,
//!             I #name [label = "Name"],
//!         }
//!     }
//! }
//! ```
//!
//! Parsing builds the shared [`LayoutNode`](gpui_layout_expr::LayoutNode)
//! AST (with a span table for diagnostics), validation reuses
//! [`validate_layout`](gpui_layout_expr::validate_layout), and emission
//! quotes the same element chains `layout expand` prints, with
//! fully-qualified paths so call sites need no imports.

// Rust guideline compliant 2026-02-21

use gpui_layout_expr::{LayoutAttr, LayoutNode, button_variant_ident, validate_layout};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Lit, LitStr, Token, braced, bracketed, parenthesized};

/// Expands `layout! { … }` to an element expression.
///
/// Failures become `compile_error!` diagnostics at the offending span.
pub fn layout_impl(input: TokenStream2) -> TokenStream2 {
    match run_layout(input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

/// Parses, validates, and emits one layout expression.
fn run_layout(input: TokenStream2) -> syn::Result<TokenStream2> {
    let root: DslRoot = syn::parse2(input)?;
    let mut forest = ForestBuilder { spans: Vec::new() };
    let node = forest.convert(&root.0);
    let nodes = vec![node];
    if let Err(error) = validate_layout(&nodes) {
        let span = forest
            .spans
            .get(error.offset)
            .copied()
            .expect("span table covers validated nodes");
        return Err(syn::Error::new(span, error.message));
    }
    let root = &nodes[0];
    if root.repeat != 1 {
        let span = forest
            .spans
            .first()
            .copied()
            .unwrap_or_else(Span::call_site);
        return Err(syn::Error::new(span, "root must not repeat"));
    }
    let mut emitter = MacroEmitter::default();
    let body = emit_single(root, &mut emitter);
    if emitter.uses_div {
        Ok(quote! {{
            use ::gpui::prelude::ParentElement;
            #body
        }})
    } else {
        Ok(body)
    }
}

/// Top-level wrapper enforcing a single root node.
struct DslRoot(DslNode);

impl Parse for DslRoot {
    /// Parses one root node and rejects trailing input.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let node: DslNode = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("expected a single root node"));
        }
        Ok(DslRoot(node))
    }
}

/// One parsed token-DSL node.
struct DslNode {
    name: Ident,
    id: Option<Ident>,
    modifier: Option<Ident>,
    payload: Option<LitStr>,
    attrs: Vec<DslAttr>,
    repeat: Option<usize>,
    children: Vec<DslNode>,
}

/// One parsed bracket attribute.
struct DslAttr {
    key: Ident,
    value: Option<LitStr>,
}

impl Parse for DslAttr {
    /// Parses `key` or `key = "value"`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        let value = if input.peek(Token![=]) {
            let _eq: Token![=] = input.parse()?;
            Some(parse_string_literal(input)?)
        } else {
            None
        };
        Ok(DslAttr { key, value })
    }
}

/// Parses a string literal, rejecting other literal kinds.
fn parse_string_literal(input: ParseStream) -> syn::Result<LitStr> {
    match input.parse::<Lit>()? {
        Lit::Str(text) => Ok(text),
        _ => Err(input.error("expected string literal")),
    }
}

impl Parse for DslNode {
    /// Parses `Name` with unordered at-most-once suffixes and children.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let mut node = DslNode {
            name,
            id: None,
            modifier: None,
            payload: None,
            attrs: Vec::new(),
            repeat: None,
            children: Vec::new(),
        };
        loop {
            if input.peek(Token![#]) {
                if node.id.is_some() {
                    return Err(input.error("duplicate '#id'"));
                }
                let _hash: Token![#] = input.parse()?;
                node.id = Some(input.parse()?);
            } else if input.peek(Token![.]) {
                if node.modifier.is_some() {
                    return Err(input.error("duplicate '.modifier'"));
                }
                let _dot: Token![.] = input.parse()?;
                node.modifier = Some(input.parse()?);
            } else if input.peek(Lit) {
                if node.payload.is_some() {
                    return Err(input.error("duplicate payload"));
                }
                node.payload = Some(parse_string_literal(input)?);
            } else if input.peek(syn::token::Paren) {
                if node.payload.is_some() {
                    return Err(input.error("duplicate payload"));
                }
                let content;
                parenthesized!(content in input);
                node.payload = Some(parse_string_literal(&content)?);
                if !content.is_empty() {
                    return Err(content.error("expected a single string payload"));
                }
            } else if input.peek(syn::token::Bracket) {
                if !node.attrs.is_empty() {
                    return Err(input.error("duplicate '[attrs]'"));
                }
                let content;
                bracketed!(content in input);
                let attrs = Punctuated::<DslAttr, Token![,]>::parse_terminated(&content)?;
                if attrs.is_empty() {
                    return Err(input.error("expected attribute"));
                }
                node.attrs = attrs.into_iter().collect();
            } else if input.peek(Token![*]) {
                if node.repeat.is_some() {
                    return Err(input.error("duplicate '*N'"));
                }
                let _star: Token![*] = input.parse()?;
                let count: syn::LitInt = input.parse()?;
                let count: usize = count.base10_parse()?;
                if count < 1 {
                    return Err(input.error("repeat count must be at least 1"));
                }
                node.repeat = Some(count);
            } else {
                break;
            }
        }
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            let children = Punctuated::<DslNode, Token![,]>::parse_terminated(&content)?;
            node.children = children.into_iter().collect();
        }
        Ok(node)
    }
}

/// Converts token nodes to the shared AST with span tracking.
struct ForestBuilder {
    spans: Vec<Span>,
}

impl ForestBuilder {
    /// Converts one node, recording its span by index.
    fn convert(&mut self, node: &DslNode) -> LayoutNode {
        let index = self.spans.len();
        self.spans.push(node.name.span());
        LayoutNode {
            name: node.name.to_string(),
            id: node.id.as_ref().map(ToString::to_string),
            modifier: node.modifier.as_ref().map(ToString::to_string),
            payload: node.payload.as_ref().map(LitStr::value),
            attrs: node
                .attrs
                .iter()
                .map(|attr| LayoutAttr {
                    key: attr.key.to_string(),
                    value: attr.value.as_ref().map(LitStr::value),
                })
                .collect(),
            repeat: node.repeat.unwrap_or(1),
            children: node
                .children
                .iter()
                .map(|child| self.convert(child))
                .collect(),
            offset: index,
        }
    }
}

/// Tracks auto ids and trait imports during emission.
#[derive(Default)]
struct MacroEmitter {
    next_id: usize,
    uses_div: bool,
}

impl MacroEmitter {
    /// Resolves an explicit id or mints the next auto id.
    fn resolve_id(&mut self, id: Option<&String>) -> String {
        if let Some(id) = id {
            return id.clone();
        }
        let id = format!("layout-{}", self.next_id);
        self.next_id += 1;
        id
    }
}

/// Emits one expression per repeat instance.
fn emit_instances(node: &LayoutNode, emitter: &mut MacroEmitter) -> Vec<TokenStream2> {
    (0..node.repeat)
        .map(|_| emit_single(node, emitter))
        .collect()
}

/// Emits one node expression; children splice per repeat.
fn emit_single(node: &LayoutNode, emitter: &mut MacroEmitter) -> TokenStream2 {
    let kind = gpui_layout_expr::component_kind(&node.name).expect("validated node name");
    let mut children = Vec::new();
    for child in &node.children {
        children.extend(emit_instances(child, emitter));
    }
    let mut expr = match kind {
        gpui_layout_expr::ComponentKind::VStack => {
            quote! { ::gpui_ui_kit::VStack::new() }
        }
        gpui_layout_expr::ComponentKind::HStack => {
            quote! { ::gpui_ui_kit::HStack::new() }
        }
        gpui_layout_expr::ComponentKind::Div => {
            emitter.uses_div = true;
            quote! { ::gpui::div() }
        }
        gpui_layout_expr::ComponentKind::Text => {
            emitter.uses_div = true;
            let text = node.payload.as_deref().expect("validated payload");
            let text = LitStr::new(text, Span::call_site());
            quote! { ::gpui::div().child(#text) }
        }
        gpui_layout_expr::ComponentKind::Button => {
            let id = emitter.resolve_id(node.id.as_ref());
            let label = node.payload.as_deref().expect("validated payload");
            let mut expr = quote! { ::gpui_ui_kit::Button::new(#id, #label) };
            if let Some(modifier) = &node.modifier {
                let variant = button_variant_ident(modifier).expect("validated modifier");
                let variant = Ident::new(variant, Span::call_site());
                expr.extend(quote! { .variant(::gpui_ui_kit::ButtonVariant::#variant) });
            }
            expr
        }
        gpui_layout_expr::ComponentKind::Input => {
            let id = emitter.resolve_id(node.id.as_ref());
            let mut expr = quote! { ::gpui_ui_kit::Input::new(#id) };
            for key in ["label", "placeholder", "value"] {
                let from_payload = key == "value" && node.payload.is_some();
                let value = if from_payload {
                    node.payload.as_deref()
                } else {
                    node.attrs
                        .iter()
                        .find(|attr| attr.key == key)
                        .and_then(|attr| attr.value.as_deref())
                };
                if let Some(value) = value {
                    let method = Ident::new(key, Span::call_site());
                    expr.extend(quote! { .#method(#value) });
                }
            }
            expr
        }
    };
    for child in children {
        expr.extend(quote! { .child(#child) });
    }
    expr
}
