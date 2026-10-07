//! Registry validation for parsed layout nodes.
//!
//! Parsing accepts any well-formed shape; validation enforces the node
//! registry: known components, required payloads, allowed modifiers,
//! attrs, and ids, plus the no-repeat-explicit-id rule. Emitters share
//! [`ComponentKind`] so the CLI and the `layout!` macro agree on what
//! each name means.

// Rust guideline compliant 2026-02-21

use super::LayoutNode;
use std::fmt::{Display, Formatter};

/// Registry failure with the offending node offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Byte offset of the offending node.
    pub offset: usize,
    /// What went wrong.
    pub message: String,
}

impl Display for ValidationError {
    /// Renders `offset N: message`.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "offset {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Known component behind a node name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// `V`: vertical stack.
    VStack,
    /// `H`: horizontal stack.
    HStack,
    /// `B`: button.
    Button,
    /// `I`: text input.
    Input,
    /// `Tx`: text run.
    Text,
    /// `D`: plain div.
    Div,
}

/// Looks up the registry kind for a node name.
///
/// Returns `None` for unknown components.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{ComponentKind, component_kind};
///
/// assert_eq!(component_kind("B"), Some(ComponentKind::Button));
/// assert_eq!(component_kind("Table"), None);
/// ```
pub fn component_kind(name: &str) -> Option<ComponentKind> {
    match name {
        "V" => Some(ComponentKind::VStack),
        "H" => Some(ComponentKind::HStack),
        "B" => Some(ComponentKind::Button),
        "I" => Some(ComponentKind::Input),
        "Tx" => Some(ComponentKind::Text),
        "D" => Some(ComponentKind::Div),
        _ => None,
    }
}

/// Maps a button modifier to its variant identifier.
///
/// Returns `None` for unknown modifiers.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::button_variant_ident;
///
/// assert_eq!(button_variant_ident("primary"), Some("Primary"));
/// assert_eq!(button_variant_ident("fancy"), None);
/// ```
pub fn button_variant_ident(modifier: &str) -> Option<&'static str> {
    match modifier {
        "primary" => Some("Primary"),
        "secondary" => Some("Secondary"),
        "destructive" => Some("Destructive"),
        "ghost" => Some("Ghost"),
        "outline" => Some("Outline"),
        _ => None,
    }
}

/// Input attributes the registry accepts.
const INPUT_ATTRS: &[&str] = &["label", "placeholder", "value"];

/// Validates parsed nodes against the registry.
///
/// # Errors
///
/// Returns [`ValidationError`] for unknown components, missing
/// required payloads, illegal modifiers, attrs, or ids, repeated
/// explicit ids, and zero repeats.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{parse_layout, validate_layout};
///
/// let nodes = parse_layout("V > B.primary\"Save\"").unwrap();
/// validate_layout(&nodes).unwrap();
/// assert!(validate_layout(&parse_layout("V > X\"?\"").unwrap()).is_err());
/// ```
pub fn validate_layout(nodes: &[LayoutNode]) -> Result<(), ValidationError> {
    for node in nodes {
        validate_node(node)?;
    }
    Ok(())
}

/// Validates one node and its children.
fn validate_node(node: &LayoutNode) -> Result<(), ValidationError> {
    let fail = |message: String| ValidationError {
        offset: node.offset,
        message,
    };
    let kind = component_kind(&node.name)
        .ok_or_else(|| fail(format!("unknown component '{}'", node.name)))?;
    if node.repeat < 1 {
        return Err(fail(format!(
            "repeat count for '{}' must be at least 1",
            node.name
        )));
    }
    if node.repeat > 1 && node.id.is_some() {
        return Err(fail(format!(
            "explicit id '#{}' cannot repeat",
            node.id.as_deref().unwrap_or_default()
        )));
    }
    if !node.children.is_empty() {
        match kind {
            ComponentKind::Button | ComponentKind::Input | ComponentKind::Text => {
                return Err(fail(format!("'{}' takes no children", node.name)));
            }
            ComponentKind::VStack | ComponentKind::HStack | ComponentKind::Div => {}
        }
    }
    match kind {
        ComponentKind::VStack | ComponentKind::HStack | ComponentKind::Div => {
            if node.payload.is_some() {
                return Err(fail(format!("'{}' takes no payload", node.name)));
            }
            if node.id.is_some() {
                return Err(fail(format!("'{}' takes no id", node.name)));
            }
            if node.modifier.is_some() {
                return Err(fail(format!("'{}' takes no modifier", node.name)));
            }
            if let Some(attr) = node.attrs.first() {
                return Err(fail(format!(
                    "'{}' takes no attrs (got '{}')",
                    node.name, attr.key
                )));
            }
        }
        ComponentKind::Text => {
            if node.payload.is_none() {
                return Err(fail("'Tx' requires a text payload".to_owned()));
            }
            if node.id.is_some() {
                return Err(fail("'Tx' takes no id".to_owned()));
            }
            if node.modifier.is_some() {
                return Err(fail("'Tx' takes no modifier".to_owned()));
            }
            if let Some(attr) = node.attrs.first() {
                return Err(fail(format!("'Tx' takes no attrs (got '{}')", attr.key)));
            }
        }
        ComponentKind::Button => {
            if node.payload.is_none() {
                return Err(fail("'B' requires a label payload".to_owned()));
            }
            if let Some(modifier) = &node.modifier
                && button_variant_ident(modifier).is_none()
            {
                return Err(fail(format!("unknown button modifier '.{modifier}'")));
            }
            if let Some(attr) = node.attrs.first() {
                return Err(fail(format!("'B' takes no attrs (got '{}')", attr.key)));
            }
        }
        ComponentKind::Input => {
            if node.modifier.is_some() {
                return Err(fail("'I' takes no modifier".to_owned()));
            }
            for attr in &node.attrs {
                if !INPUT_ATTRS.contains(&attr.key.as_str()) {
                    return Err(fail(format!("unknown input attr '{}'", attr.key)));
                }
                if attr.value.is_none() {
                    return Err(fail(format!("input attr '{}' needs a value", attr.key)));
                }
            }
            let has_value_attr = node.attrs.iter().any(|attr| attr.key == "value");
            if node.payload.is_some() && has_value_attr {
                return Err(fail(
                    "input value given twice (payload and value attr)".to_owned(),
                ));
            }
        }
    }
    for child in &node.children {
        validate_node(child)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_layout;
    use crate::parse_layout;

    /// Parses and validates, returning the message on failure.
    fn check(text: &str) -> Result<(), String> {
        let nodes = parse_layout(text).map_err(|error| error.to_string())?;
        validate_layout(&nodes).map_err(|error| error.message.clone())
    }

    #[test]
    fn accepts_registry_shapes() {
        check("V > (H > B.primary\"Save\"#save + I#name[label=\"Name\"])").unwrap();
        check("D > Tx\"Hi\"").unwrap();
        check("B\"Go\"*3").unwrap();
    }

    #[test]
    fn rejects_unknown_components() {
        assert!(
            check("Table\"x\"")
                .unwrap_err()
                .contains("unknown component")
        );
    }

    #[test]
    fn enforces_payload_rules() {
        assert!(check("B#save").unwrap_err().contains("label payload"));
        assert!(check("V\"x\"").unwrap_err().contains("no payload"));
        assert!(check("Tx").unwrap_err().contains("text payload"));
    }

    #[test]
    fn enforces_modifier_and_attr_rules() {
        assert!(check("B.fancy\"x\"").unwrap_err().contains("modifier"));
        assert!(check("B\"x\"[a=b]").unwrap_err().contains("no attrs"));
        assert!(
            check("I#n[cool=true]")
                .unwrap_err()
                .contains("unknown input attr")
        );
        assert!(check("I#n[label]").unwrap_err().contains("needs a value"));
        assert!(
            check("I#n\"v\"[value=\"w\"]")
                .unwrap_err()
                .contains("twice")
        );
    }

    #[test]
    fn rejects_repeated_explicit_ids() {
        assert!(check("B#x\"Go\"*2").unwrap_err().contains("cannot repeat"));
    }

    #[test]
    fn rejects_leaf_children() {
        assert!(
            check("B\"x\" > Tx\"y\"")
                .unwrap_err()
                .contains("no children")
        );
        assert!(check("I#n > Tx\"y\"").unwrap_err().contains("no children"));
        assert!(
            check("Tx\"x\" > B\"y\"")
                .unwrap_err()
                .contains("no children")
        );
    }
}
