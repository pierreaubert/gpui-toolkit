//! Canonical re-serialization and node counting.
//!
//! [`canonical_layout`] renders the normalized form `layout check`
//! echoes: fixed suffix order, single spaces, children parenthesized
//! when plural. Canonical output always reparses to an equal forest,
//! which round-trip tests assert.

// Rust guideline compliant 2026-02-21

use super::LayoutNode;

/// Renders the canonical form of a node forest.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{canonical_layout, parse_layout};
///
/// let nodes = parse_layout("V>B\"Hi\"").unwrap();
/// assert_eq!(canonical_layout(&nodes), "V > B\"Hi\"");
/// ```
pub fn canonical_layout(nodes: &[LayoutNode]) -> String {
    nodes
        .iter()
        .map(canonical_node)
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Renders one node with its children.
fn canonical_node(node: &LayoutNode) -> String {
    let mut text = node.name.clone();
    if let Some(id) = &node.id {
        text.push('#');
        text.push_str(id);
    }
    if let Some(modifier) = &node.modifier {
        text.push('.');
        text.push_str(modifier);
    }
    if let Some(payload) = &node.payload {
        text.push('"');
        text.push_str(&escape_quoted(payload));
        text.push('"');
    }
    if !node.attrs.is_empty() {
        let attrs = node
            .attrs
            .iter()
            .map(|attr| match &attr.value {
                Some(value) => format!("{}={}", attr.key, quote_attr_value(value)),
                None => attr.key.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ");
        text.push('[');
        text.push_str(&attrs);
        text.push(']');
    }
    if node.repeat > 1 {
        text.push('*');
        text.push_str(&node.repeat.to_string());
    }
    match node.children.as_slice() {
        [] => text,
        [single] => format!("{text} > {}", canonical_node(single)),
        children => {
            let inner = children
                .iter()
                .map(canonical_node)
                .collect::<Vec<_>>()
                .join(" + ");
            format!("{text} > ({inner})")
        }
    }
}

/// Escapes backslashes and quotes for canonical payloads.
fn escape_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch == '\\' || ch == '"' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Quotes attr values that need it for reparsing.
fn quote_attr_value(value: &str) -> String {
    if value
        .chars()
        .any(|ch| ch == ' ' || ch == '\t' || ch == ']' || ch == '"')
    {
        format!("\"{}\"", escape_quoted(value))
    } else {
        value.to_owned()
    }
}

/// Counts nodes with repeats expanded.
///
/// A node with `repeat` 3 counts 3, so the total matches the emitted
/// element count.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{layout_node_count, parse_layout};
///
/// let nodes = parse_layout("V > (B\"a\" + B\"b\"*2)").unwrap();
/// assert_eq!(layout_node_count(&nodes), 4);
/// ```
pub fn layout_node_count(nodes: &[LayoutNode]) -> usize {
    nodes.iter().map(count_node).sum()
}

/// Counts one node with its subtree.
fn count_node(node: &LayoutNode) -> usize {
    node.repeat * (1 + node.children.iter().map(count_node).sum::<usize>())
}

#[cfg(test)]
mod tests {
    use super::canonical_layout;
    use crate::parse_layout;

    /// Parses, canonicalizes, and reparses, returning both forests.
    fn round_trip(text: &str) -> bool {
        let first = parse_layout(text).unwrap();
        let canonical = canonical_layout(&first);
        let second = parse_layout(&canonical).unwrap();
        canonical_layout(&second) == canonical
    }

    #[test]
    fn canonicalizes_spacing_and_order() {
        let nodes = parse_layout("V>(Tx\"Hi\"+B\"Ok\")").unwrap();
        assert_eq!(canonical_layout(&nodes), "V > (Tx\"Hi\" + B\"Ok\")");
        let nodes = parse_layout("B\"Save\".primary#save").unwrap();
        assert_eq!(canonical_layout(&nodes), "B#save.primary\"Save\"");
    }

    #[test]
    fn round_trips() {
        for text in [
            "V > (H > B.primary\"Save\"#save + I#name[label=\"Display name\"])",
            "D > Tx\"Hi \\\"there\\\"\"",
            "V > (B\"a\" + B\"b\")*2",
            "I#n[label=\"A B\" value=x]",
            "V*2",
            "V[g6] > (Hd\"Analytics\"[level=2] + H[g2] > Tx.lg\"$42k\")",
            "V[spacing=lg] > Tx.sm\"Hi\"[weight=bold muted]",
        ] {
            assert!(round_trip(text), "{text}");
        }
    }
}
