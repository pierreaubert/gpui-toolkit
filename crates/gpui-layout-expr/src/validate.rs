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
    /// `Hd`: heading.
    Heading,
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
        "Hd" => Some(ComponentKind::Heading),
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

/// Maps a heading level value to its constructor method name.
///
/// Returns `None` for unknown levels.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::heading_level_ident;
///
/// assert_eq!(heading_level_ident("2"), Some("h2"));
/// assert_eq!(heading_level_ident("9"), None);
/// ```
pub fn heading_level_ident(level: &str) -> Option<&'static str> {
    match level {
        "1" => Some("h1"),
        "2" => Some("h2"),
        "3" => Some("h3"),
        "4" => Some("h4"),
        _ => None,
    }
}

/// Maps a text weight name to its variant identifier.
///
/// Returns `None` for unknown weights.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::text_weight_ident;
///
/// assert_eq!(text_weight_ident("bold"), Some("Bold"));
/// assert_eq!(text_weight_ident("heavy"), None);
/// ```
pub fn text_weight_ident(weight: &str) -> Option<&'static str> {
    match weight {
        "light" => Some("Light"),
        "normal" => Some("Normal"),
        "medium" => Some("Medium"),
        "semibold" => Some("Semibold"),
        "bold" => Some("Bold"),
        _ => None,
    }
}

/// Maps a text size name to its variant identifier.
///
/// Returns `None` for unknown sizes.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::text_size_ident;
///
/// assert_eq!(text_size_ident("sm"), Some("Sm"));
/// assert_eq!(text_size_ident("huge"), None);
/// ```
pub fn text_size_ident(size: &str) -> Option<&'static str> {
    match size {
        "xs" => Some("Xs"),
        "sm" => Some("Sm"),
        "md" => Some("Md"),
        "lg" => Some("Lg"),
        "xl" => Some("Xl"),
        "xxl" => Some("Xxl"),
        _ => None,
    }
}

/// Maps a stack spacing name to its variant identifier.
///
/// Returns `None` for unknown spacings.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::stack_spacing_ident;
///
/// assert_eq!(stack_spacing_ident("lg"), Some("Lg"));
/// assert_eq!(stack_spacing_ident("cozy"), None);
/// ```
pub fn stack_spacing_ident(spacing: &str) -> Option<&'static str> {
    match spacing {
        "none" => Some("None"),
        "xs" => Some("Xs"),
        "sm" => Some("Sm"),
        "md" => Some("Md"),
        "lg" => Some("Lg"),
        "xl" => Some("Xl"),
        "xxl" => Some("Xxl"),
        _ => None,
    }
}

/// Maps a fused Astryx-style gap attr to a spacing identifier.
///
/// `gN` means an N×4px gap; only widths with a `StackSpacing` variant
/// are accepted. Returns `None` for other keys.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::gap_spacing_ident;
///
/// assert_eq!(gap_spacing_ident("g4"), Some("Lg"));
/// assert_eq!(gap_spacing_ident("g3"), None);
/// assert_eq!(gap_spacing_ident("spacing"), None);
/// ```
pub fn gap_spacing_ident(attr: &str) -> Option<&'static str> {
    let digits = attr.strip_prefix('g')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    match digits {
        "0" => Some("None"),
        "1" => Some("Sm"),
        "2" => Some("Md"),
        "4" => Some("Lg"),
        "6" => Some("Xl"),
        "8" => Some("Xxl"),
        _ => None,
    }
}

/// Whether a key has fused-gap shape (`g` plus digits).
fn is_gap_attr(key: &str) -> bool {
    key.strip_prefix('g').is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Resolves the effective stack spacing identifier, if any.
///
/// Reads the `spacing` attr or a fused `gN` gap; validated trees
/// never carry both.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{parse_layout, stack_spacing_of};
///
/// let nodes = parse_layout("V[spacing=lg] > Tx\"x\"").unwrap();
/// assert_eq!(stack_spacing_of(&nodes[0]), Some("Lg"));
/// let nodes = parse_layout("H[g2] > Tx\"x\"").unwrap();
/// assert_eq!(stack_spacing_of(&nodes[0]), Some("Md"));
/// ```
pub fn stack_spacing_of(node: &LayoutNode) -> Option<&'static str> {
    for attr in &node.attrs {
        if attr.key == "spacing" {
            return attr.value.as_deref().and_then(stack_spacing_ident);
        }
        if let Some(spacing) = gap_spacing_ident(&attr.key) {
            return Some(spacing);
        }
    }
    None
}

/// Resolves the heading constructor method, defaulting to `h1`.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{heading_level_of, parse_layout};
///
/// let nodes = parse_layout("Hd\"T\"").unwrap();
/// assert_eq!(heading_level_of(&nodes[0]), "h1");
/// let nodes = parse_layout("Hd\"T\"[level=3]").unwrap();
/// assert_eq!(heading_level_of(&nodes[0]), "h3");
/// ```
pub fn heading_level_of(node: &LayoutNode) -> &'static str {
    node.attrs
        .iter()
        .find(|attr| attr.key == "level")
        .and_then(|attr| attr.value.as_deref())
        .and_then(heading_level_ident)
        .unwrap_or("h1")
}

/// Input attributes the registry accepts.
const INPUT_ATTRS: &[&str] = &["label", "placeholder", "value"];

/// Text attributes the registry accepts.
const TEXT_ATTRS: &[&str] = &["weight", "muted"];

/// Stack attributes the registry accepts.
const STACK_ATTRS: &[&str] = &["spacing"];

/// Heading attributes the registry accepts.
const HEADING_ATTRS: &[&str] = &["level"];

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
            ComponentKind::Button
            | ComponentKind::Input
            | ComponentKind::Text
            | ComponentKind::Heading => {
                return Err(fail(format!("'{}' takes no children", node.name)));
            }
            ComponentKind::VStack | ComponentKind::HStack | ComponentKind::Div => {}
        }
    }
    match kind {
        ComponentKind::VStack | ComponentKind::HStack => {
            if node.payload.is_some() {
                return Err(fail(format!("'{}' takes no payload", node.name)));
            }
            if node.id.is_some() {
                return Err(fail(format!("'{}' takes no id", node.name)));
            }
            if node.modifier.is_some() {
                return Err(fail(format!("'{}' takes no modifier", node.name)));
            }
            let mut spacing_sources = 0;
            for attr in &node.attrs {
                if is_gap_attr(&attr.key) {
                    if attr.value.is_some() {
                        return Err(fail(format!("gap attr '{}' takes no value", attr.key)));
                    }
                    if gap_spacing_ident(&attr.key).is_none() {
                        return Err(fail(format!(
                            "unknown gap '{}' (use g0, g1, g2, g4, g6, g8)",
                            attr.key
                        )));
                    }
                    spacing_sources += 1;
                    continue;
                }
                if !STACK_ATTRS.contains(&attr.key.as_str()) {
                    return Err(fail(format!("unknown stack attr '{}'", attr.key)));
                }
                let Some(value) = attr.value.as_deref() else {
                    return Err(fail(format!("stack attr '{}' needs a value", attr.key)));
                };
                if stack_spacing_ident(value).is_none() {
                    return Err(fail(format!("unknown spacing '{value}'")));
                }
                spacing_sources += 1;
            }
            if spacing_sources > 1 {
                return Err(fail("stack spacing given twice".to_owned()));
            }
        }
        ComponentKind::Div => {
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
            if let Some(modifier) = &node.modifier
                && text_size_ident(modifier).is_none()
            {
                return Err(fail(format!("unknown text size '.{modifier}'")));
            }
            for attr in &node.attrs {
                if !TEXT_ATTRS.contains(&attr.key.as_str()) {
                    return Err(fail(format!("unknown text attr '{}'", attr.key)));
                }
                match (attr.key.as_str(), attr.value.as_deref()) {
                    ("muted", None) => {}
                    ("muted", Some(_)) => {
                        return Err(fail("'muted' takes no value".to_owned()));
                    }
                    ("weight", Some(value)) => {
                        if text_weight_ident(value).is_none() {
                            return Err(fail(format!("unknown text weight '{value}'")));
                        }
                    }
                    _ => {
                        return Err(fail(format!("text attr '{}' needs a value", attr.key)));
                    }
                }
            }
        }
        ComponentKind::Heading => {
            if node.payload.is_none() {
                return Err(fail("'Hd' requires a text payload".to_owned()));
            }
            if node.id.is_some() {
                return Err(fail("'Hd' takes no id".to_owned()));
            }
            if node.modifier.is_some() {
                return Err(fail("'Hd' takes no modifier".to_owned()));
            }
            for attr in &node.attrs {
                if !HEADING_ATTRS.contains(&attr.key.as_str()) {
                    return Err(fail(format!("unknown heading attr '{}'", attr.key)));
                }
                let Some(value) = attr.value.as_deref() else {
                    return Err(fail(format!("heading attr '{}' needs a value", attr.key)));
                };
                if heading_level_ident(value).is_none() {
                    return Err(fail(format!("unknown heading level '{value}'")));
                }
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
    fn accepts_headings_text_styles_and_spacing() {
        check("Hd\"Title\"").unwrap();
        check("Hd\"Title\"[level=2]").unwrap();
        check("Tx.lg\"$42k\"").unwrap();
        check("Tx.sm\"Revenue\"[weight=bold muted]").unwrap();
        check("V[spacing=lg] > Tx\"Hi\"").unwrap();
        check("V[g6] > (Hd\"Analytics\"[level=2] + H[g2] > Tx.lg\"$42k\")").unwrap();
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
        assert!(
            check("Hd\"x\" > Tx\"y\"")
                .unwrap_err()
                .contains("no children")
        );
    }

    #[test]
    fn enforces_heading_text_and_spacing_rules() {
        assert!(check("Hd").unwrap_err().contains("text payload"));
        assert!(check("Hd.h2\"x\"").unwrap_err().contains("no modifier"));
        assert!(check("Hd\"x\"[level=9]").unwrap_err().contains("level"));
        assert!(
            check("Hd\"x\"[level]")
                .unwrap_err()
                .contains("needs a value")
        );
        assert!(
            check("Hd\"x\"[size=lg]")
                .unwrap_err()
                .contains("unknown heading attr")
        );
        assert!(check("Tx.huge\"x\"").unwrap_err().contains("text size"));
        assert!(
            check("Tx\"x\"[weight=heavy]")
                .unwrap_err()
                .contains("weight")
        );
        assert!(
            check("Tx\"x\"[muted=yes]")
                .unwrap_err()
                .contains("no value")
        );
        assert!(
            check("Tx\"x\"[align=center]")
                .unwrap_err()
                .contains("unknown text attr")
        );
        assert!(
            check("V[spacing=cozy] > Tx\"x\"")
                .unwrap_err()
                .contains("spacing")
        );
        assert!(
            check("V[g3] > Tx\"x\"")
                .unwrap_err()
                .contains("unknown gap")
        );
        assert!(
            check("V[spacing=md g2] > Tx\"x\"")
                .unwrap_err()
                .contains("twice")
        );
        assert!(check("D[g2] > Tx\"x\"").unwrap_err().contains("no attrs"));
    }
}
