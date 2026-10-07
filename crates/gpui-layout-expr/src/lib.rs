#![forbid(unsafe_code)]

//! Compact layout expression AST, parser, and validation.
//!
//! Expressions describe a component tree in one line (`V > (H >
//! B.primary"Save"#save + I#name)`); the parser builds [`LayoutNode`]
//! trees, validation enforces the node registry, and [`canonical_layout`]
//! re-serializes the normalized form. This crate is pure `std` with no
//! GPUI dependencies, so both the CLI string emitter and the `layout!`
//! proc macro share it without bloating macro builds.
//!
//! Grammar summary (`>` nests right-associatively, `+` separates
//! siblings, suffixes take any order at most once):
//!
//! ```text
//! seq    := chain ("+" chain)*
//! chain  := atom (">" seq)?
//! atom   := node | "(" seq ")" ["*" N]
//! node   := Name suffix*
//! suffix := "#" id | "." modifier | '"' payload '"' | "[" attrs "]" | "*" N
//! ```
//!
//! See `docs/superpowers/2026-10-07-layout-expressions.md` for the
//! full design.

// Rust guideline compliant 2026-02-21

mod canonical;
mod parser;
mod registry;
mod validate;

#[doc(inline)]
pub use canonical::{canonical_layout, layout_node_count};
#[doc(inline)]
pub use parser::{ParseError, column_for_offset, parse_layout};
#[doc(inline)]
pub use registry::{
    CustomComponent, CustomComponents, CustomRegistryError, resolve_layout, validate_layout_with,
};
#[doc(inline)]
pub use validate::{
    ComponentKind, ValidationError, button_variant_ident, component_kind, validate_layout,
};

/// One parsed layout node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    /// Component name as written, such as `V` or `Button`.
    pub name: String,
    /// Explicit `#id`, if any.
    pub id: Option<String>,
    /// Dot modifier, such as `primary`, if any.
    pub modifier: Option<String>,
    /// Double-quoted payload, unescaped, if any.
    pub payload: Option<String>,
    /// Bracket attributes in written order.
    pub attrs: Vec<LayoutAttr>,
    /// Repeat count; always at least 1.
    pub repeat: usize,
    /// Nested children.
    pub children: Vec<LayoutNode>,
    /// Byte offset of the node name for error reporting.
    pub offset: usize,
}

/// One bracket attribute: a flag or a key/value pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutAttr {
    /// Attribute key.
    pub key: String,
    /// Value; `None` for bare flags.
    pub value: Option<String>,
}

/// Normative grammar reference printed by `layout grammar`.
pub const GRAMMAR: &str = "\
layout expressions — one line describes a component tree

  seq    := chain (\"+\" chain)*
  chain  := atom (\">\" seq)?
  atom   := node | \"(\" seq \")\" [\"*\" N]
  node   := Name suffix*
  suffix := \"#\" id | \".\" modifier | '\"' payload '\"' | \"[\" attrs \"]\" | \"*\" N

`>` attaches the whole following sibling run (A > B + C gives A the
children [B, C]); A > B > C nests as A(B(C)). Suffixes take any order,
each at most once. Groups splice in place; only single nodes take `>`
children. `*N` repeats (N >= 1); explicit `#id` may not repeat.

nodes: V (VStack), H (HStack), B (Button, label payload required),
  I (Input, attrs: label, placeholder, value), Tx (text, payload
  required), D (plain div). Button modifiers: primary, secondary,
  destructive, ghost, outline. Auto ids (`layout-N`) fill in where no
  `#id` is given.

examples:
  V > (Tx\"Hi\" + B.primary\"Save\"#save)
  H > (I#name[label=\"Name\"] + B\"Go\")
";
