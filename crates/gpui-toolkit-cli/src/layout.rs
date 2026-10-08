//! Layout expression check, expand, and grammar.
//!
//! [`layout_check`] parses and validates one expression, echoing its
//! canonical form; [`layout_expand`] emits a complete compilable Rust
//! unit for single-root expressions; [`layout_grammar`] prints the
//! normative reference. Expansion assigns deterministic auto ids
//! (`layout-N` in pre-order) where no `#id` is given, and imports only
//! what the tree uses.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use crate::init::is_plain_file_name;
use gpui_layout_expr::{
    ComponentKind, CustomComponents, LayoutNode, button_variant_ident, canonical_layout,
    column_for_offset, component_kind, layout_node_count, parse_layout, resolve_layout,
    validate_layout,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Discriminator for layout check payloads.
pub const LAYOUT_CHECK_TYPE: &str = "layout.check";

/// Discriminator for layout expansion payloads.
pub const LAYOUT_EXPAND_TYPE: &str = "layout.expand";

/// Discriminator for grammar payloads.
pub const LAYOUT_GRAMMAR_TYPE: &str = "layout.grammar";

/// Default function name for expansions.
pub const DEFAULT_LAYOUT_FN: &str = "expanded_layout";

/// Rust strict keywords rejected as expansion function names.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

/// Layout check payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutCheck {
    /// Echoed expression.
    pub expr: String,
    /// Canonical re-serialization.
    pub canonical: String,
    /// Node count with repeats expanded.
    pub nodes: usize,
    /// Custom components the expression used, in first-seen order.
    pub custom_components: Vec<String>,
}

/// Layout expansion payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutExpand {
    /// Echoed expression.
    pub expr: String,
    /// Generated function name.
    pub function: String,
    /// Complete Rust unit.
    pub rust: String,
    /// Emitted bytes.
    pub bytes: usize,
    /// Written path when `--out` was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

/// Grammar payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutGrammar {
    /// Normative grammar reference.
    pub grammar: String,
}

/// Parses and validates one expression.
///
/// Custom aliases from `customs` resolve before validation; the
/// canonical form keeps names as written. Returns the canonical form,
/// the expanded node count, and the custom components used.
///
/// # Errors
///
/// Returns [`ErrorCode::LayoutParse`] for syntax failures (with a
/// 1-based column) and [`ErrorCode::LayoutInvalid`] for registry
/// violations and preset-modifier collisions.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::CustomComponents;
/// use gpui_toolkit_cli::layout_check;
///
/// let checked = layout_check("V>B\"Hi\"", &CustomComponents::empty()).unwrap();
/// assert_eq!(checked.canonical, "V > B\"Hi\"");
/// assert_eq!(checked.nodes, 2);
/// ```
pub fn layout_check(expr: &str, customs: &CustomComponents) -> Result<LayoutCheck, ToolkitError> {
    let nodes = parse_expr(expr)?;
    resolve_and_validate(expr, &nodes, customs)?;
    Ok(LayoutCheck {
        expr: expr.to_owned(),
        nodes: layout_node_count(&nodes),
        canonical: canonical_layout(&nodes),
        custom_components: used_custom_names(&nodes, customs),
    })
}

/// Collects custom names used in a parsed tree, first-seen order.
///
/// Callers pass unresolved nodes: resolution rewrites names to bases,
/// so usage is only visible before resolving.
fn used_custom_names(nodes: &[LayoutNode], customs: &CustomComponents) -> Vec<String> {
    let mut used = Vec::new();
    collect_custom_names(nodes, customs, &mut used);
    used
}

/// Walks one level, recording custom names before descending.
fn collect_custom_names(nodes: &[LayoutNode], customs: &CustomComponents, used: &mut Vec<String>) {
    for node in nodes {
        if customs.get(&node.name).is_some() && !used.iter().any(|name| name == &node.name) {
            used.push(node.name.clone());
        }
        collect_custom_names(&node.children, customs, used);
    }
}

/// Parses one expression, mapping syntax failures to codes.
fn parse_expr(expr: &str) -> Result<Vec<LayoutNode>, ToolkitError> {
    parse_layout(expr).map_err(|error| ToolkitError::new(ErrorCode::LayoutParse, error.to_string()))
}

/// Resolves customs plus validates, mapping failures to codes.
fn resolve_and_validate(
    expr: &str,
    nodes: &[LayoutNode],
    customs: &CustomComponents,
) -> Result<Vec<LayoutNode>, ToolkitError> {
    let resolved = resolve_layout(nodes, customs).map_err(|error| {
        ToolkitError::new(
            ErrorCode::LayoutInvalid,
            format!(
                "column {}: {}",
                column_for_offset(expr, error.offset),
                error.message
            ),
        )
    })?;
    validate_layout(&resolved).map_err(|error| {
        ToolkitError::new(
            ErrorCode::LayoutInvalid,
            format!(
                "column {}: {}",
                column_for_offset(expr, error.offset),
                error.message
            ),
        )
    })?;
    Ok(resolved)
}

/// Parses, resolves customs, plus validates, mapping failures to codes.
fn parse_validated(
    expr: &str,
    customs: &CustomComponents,
) -> Result<Vec<LayoutNode>, ToolkitError> {
    let nodes = parse_expr(expr)?;
    resolve_and_validate(expr, &nodes, customs)
}

/// Checks that a function name is a usable Rust identifier.
fn check_fn_name(name: &str) -> Result<(), ToolkitError> {
    let mut chars = name.chars();
    let head = chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');
    let tail = chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if !head || !tail {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("function name '{name}' is not a Rust identifier"),
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("function name '{name}' is a Rust keyword"),
        ));
    }
    Ok(())
}

/// Expands one single-root expression to a Rust unit.
///
/// `function` names the generated function (default
/// [`DEFAULT_LAYOUT_FN`]); `customs` resolves project aliases first.
/// Forests and repeated roots fail: expansion targets one
/// `impl IntoElement` value.
///
/// # Errors
///
/// Returns parse/validation errors like [`layout_check`], plus
/// [`ErrorCode::LayoutInvalid`] for multi-root input and
/// [`ErrorCode::InvalidArgument`] for bad function names.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::CustomComponents;
/// use gpui_toolkit_cli::layout_expand;
///
/// let unit = layout_expand("B\"Hi\"", None, &CustomComponents::empty()).unwrap();
/// assert!(unit.contains("Button::new"));
/// ```
pub fn layout_expand(
    expr: &str,
    function: Option<&str>,
    customs: &CustomComponents,
) -> Result<String, ToolkitError> {
    let nodes = parse_validated(expr, customs)?;
    if nodes.len() != 1 {
        return Err(ToolkitError::new(
            ErrorCode::LayoutInvalid,
            format!("expand needs a single root; got {} nodes", nodes.len()),
        ));
    }
    let root = &nodes[0];
    if root.repeat != 1 {
        return Err(ToolkitError::new(
            ErrorCode::LayoutInvalid,
            "expand needs a single root; the root repeats",
        ));
    }
    let name = function.unwrap_or(DEFAULT_LAYOUT_FN);
    check_fn_name(name)?;
    Ok(render_unit(root, name))
}

/// Expands one expression and writes the unit to a file.
///
/// The file defaults to `<function>.rs`; existing files are never
/// overwritten.
///
/// # Errors
///
/// Returns the same parse, validation, and function-name errors as
/// [`layout_expand`], plus [`ErrorCode::InvalidArgument`] for a missing
/// directory or a non-plain file name, [`ErrorCode::FileExists`] when
/// the target already exists, and [`ErrorCode::WriteFailed`] for I/O
/// failures.
pub fn layout_expand_to_file(
    expr: &str,
    function: Option<&str>,
    dir: &Path,
    file: Option<&str>,
    customs: &CustomComponents,
) -> Result<LayoutExpand, ToolkitError> {
    let rust = layout_expand(expr, function, customs)?;
    if !dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", dir.display()),
        ));
    }
    let name = function.unwrap_or(DEFAULT_LAYOUT_FN);
    let file_name = file.map_or_else(|| format!("{name}.rs"), str::to_owned);
    if !is_plain_file_name(&file_name) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("file name '{file_name}' must be a plain file name"),
        ));
    }
    let path = dir.join(file_name);
    if path.exists() {
        return Err(ToolkitError::new(
            ErrorCode::FileExists,
            format!("refusing to overwrite '{}'", path.display()),
        ));
    }
    std::fs::write(&path, &rust).map_err(|error| {
        ToolkitError::new(
            ErrorCode::WriteFailed,
            format!("cannot write '{}': {error}", path.display()),
        )
    })?;
    Ok(LayoutExpand {
        expr: expr.to_owned(),
        function: name.to_owned(),
        bytes: rust.len(),
        rust,
        path: Some(path),
    })
}

/// Returns the normative grammar reference.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::layout_grammar;
///
/// assert!(layout_grammar().grammar.contains("suffix"));
/// ```
pub fn layout_grammar() -> LayoutGrammar {
    LayoutGrammar {
        grammar: gpui_layout_expr::GRAMMAR.to_owned(),
    }
}

/// Escapes a string for a Rust literal.
fn escape_rust_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// Tracks used constructors and auto ids during emission.
#[derive(Default)]
struct Emitter {
    next_id: usize,
    button: bool,
    variant: bool,
    input: bool,
    vstack: bool,
    hstack: bool,
    div: bool,
}

impl Emitter {
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

/// Renders the complete compilable unit for one root.
fn render_unit(root: &LayoutNode, name: &str) -> String {
    let mut emitter = Emitter::default();
    let mut instances = Vec::new();
    emit_instances(root, &mut emitter, &mut instances);
    let body = instances.into_iter().next().unwrap_or_default();
    let mut unit = String::from("// Expanded by `gpui-toolkit layout expand`; do not hand-edit.\n");
    unit.push_str("use gpui::IntoElement;\n");
    if emitter.div {
        unit.push_str("use gpui::div;\n");
        unit.push_str("use gpui::prelude::*;\n");
    }
    let mut kit = Vec::new();
    if emitter.button {
        kit.push("Button");
    }
    if emitter.variant {
        kit.push("ButtonVariant");
    }
    if emitter.hstack {
        kit.push("HStack");
    }
    if emitter.input {
        kit.push("Input");
    }
    if emitter.vstack {
        kit.push("VStack");
    }
    if !kit.is_empty() {
        unit.push_str(&format!("use gpui_ui_kit::{{{}}};\n", kit.join(", ")));
    }
    unit.push('\n');
    unit.push_str(&format!(
        "pub fn {name}() -> impl IntoElement {{\n    {body}\n}}\n"
    ));
    unit
}

/// Emits one expression per repeat instance.
fn emit_instances(node: &LayoutNode, emitter: &mut Emitter, out: &mut Vec<String>) {
    for _ in 0..node.repeat {
        out.push(emit_single(node, emitter));
    }
}

/// Emits one node expression; children splice per repeat.
fn emit_single(node: &LayoutNode, emitter: &mut Emitter) -> String {
    let kind = component_kind(&node.name).expect("validated node name");
    let mut children = Vec::new();
    for child in &node.children {
        emit_instances(child, emitter, &mut children);
    }
    let mut chained = String::new();
    for child in &children {
        use std::fmt::Write as _;
        write!(chained, ".child({child})").expect("write to String");
    }
    match kind {
        ComponentKind::VStack => {
            emitter.vstack = true;
            format!("VStack::new(){chained}")
        }
        ComponentKind::HStack => {
            emitter.hstack = true;
            format!("HStack::new(){chained}")
        }
        ComponentKind::Div => {
            emitter.div = true;
            format!("div(){chained}")
        }
        ComponentKind::Text => {
            emitter.div = true;
            let text = escape_rust_string(node.payload.as_deref().unwrap_or_default());
            format!("div().child(\"{text}\"){chained}")
        }
        ComponentKind::Button => {
            emitter.button = true;
            let id = emitter.resolve_id(node.id.as_ref());
            let label = escape_rust_string(node.payload.as_deref().unwrap_or_default());
            let mut expr = format!("Button::new(\"{id}\", \"{label}\")");
            if let Some(modifier) = &node.modifier {
                emitter.variant = true;
                let variant = button_variant_ident(modifier).expect("validated modifier");
                expr.push_str(&format!(".variant(ButtonVariant::{variant})"));
            }
            expr.push_str(&chained);
            expr
        }
        ComponentKind::Input => {
            emitter.input = true;
            let id = emitter.resolve_id(node.id.as_ref());
            let mut expr = format!("Input::new(\"{id}\")");
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
                    expr.push_str(&format!(".{key}(\"{}\")", escape_rust_string(value)));
                }
            }
            expr.push_str(&chained);
            expr
        }
    }
}

/// Renders a check payload as human-readable text.
pub fn render_layout_check_text(check: &LayoutCheck, dense: bool) -> String {
    if dense {
        return format!("{} {}\n", check.nodes, check.canonical);
    }
    format!("OK: {} nodes\n{}\n", check.nodes, check.canonical)
}

/// Renders an expansion receipt as human-readable text.
pub fn render_layout_expand_text(expand: &LayoutExpand) -> String {
    match &expand.path {
        Some(path) => format!(
            "Wrote {} bytes to {} ({} nodes)\n",
            expand.bytes,
            path.display(),
            layout_node_count_str(&expand.expr),
        ),
        None => expand.rust.clone(),
    }
}

/// Counts nodes for receipts; validated input always parses.
///
/// Parse-only on purpose: the expression already validated upstream,
/// and counting needs no registry.
fn layout_node_count_str(expr: &str) -> usize {
    parse_expr(expr).map_or(0, |nodes| gpui_layout_expr::layout_node_count(&nodes))
}
