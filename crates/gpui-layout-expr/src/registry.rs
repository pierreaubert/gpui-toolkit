//! Named custom components resolved before validation.
//!
//! Parsing accepts any well-formed shape and [`validate_layout`] enforces
//! the built-in registry. Custom components close the gap between the two:
//! a project registers aliases in `toolkit.toml` (`[layout.components]`),
//! [`resolve_layout`] rewrites them to their base nodes (merging the
//! preset modifier), and validation then runs on the resolved tree. The
//! canonical form keeps names as written, so `check` echoes the author's
//! vocabulary while `expand` emits the base constructors.

// Rust guideline compliant 2026-02-21

use super::{ComponentKind, LayoutNode, ValidationError, button_variant_ident, component_kind};
use std::fmt::{Display, Formatter};

impl ComponentKind {
    /// Canonical short name this kind parses as (`B`, `Tx`, …).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_layout_expr::ComponentKind;
    ///
    /// assert_eq!(ComponentKind::Button.canonical_name(), "B");
    /// ```
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::VStack => "V",
            Self::HStack => "H",
            Self::Button => "B",
            Self::Input => "I",
            Self::Text => "Tx",
            Self::Div => "D",
        }
    }
}

/// One named alias: `name` behaves like `base` with a preset modifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomComponent {
    /// Name as used in expressions, such as `PrimaryButton`.
    pub name: String,
    /// Built-in kind this alias expands to.
    pub base: ComponentKind,
    /// Preset dot modifier, if any (buttons only).
    pub modifier: Option<String>,
}

/// Custom-registry build failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomRegistryError {
    /// What went wrong.
    pub message: String,
}

impl Display for CustomRegistryError {
    /// Renders the message.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CustomRegistryError {}

/// Validated custom-component set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomComponents {
    /// Specs in registration order.
    specs: Vec<CustomComponent>,
}

impl CustomComponents {
    /// Builds a registry, rejecting bad names, builtin collisions,
    /// duplicates, and illegal preset modifiers.
    ///
    /// # Errors
    ///
    /// Returns [`CustomRegistryError`] describing the first rejected
    /// spec.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_layout_expr::{ComponentKind, CustomComponent, CustomComponents};
    ///
    /// let customs = CustomComponents::new(vec![CustomComponent {
    ///     name: "PrimaryButton".to_owned(),
    ///     base: ComponentKind::Button,
    ///     modifier: Some("primary".to_owned()),
    /// }])
    /// .unwrap();
    /// assert_eq!(customs.names(), vec!["PrimaryButton".to_owned()]);
    /// ```
    pub fn new(specs: Vec<CustomComponent>) -> Result<Self, CustomRegistryError> {
        let fail = |message: String| CustomRegistryError { message };
        for spec in &specs {
            if !valid_custom_name(&spec.name) {
                return Err(fail(format!(
                    "custom component '{}' is not a valid node name",
                    spec.name
                )));
            }
            if component_kind(&spec.name).is_some() {
                return Err(fail(format!(
                    "custom component '{}' collides with a built-in",
                    spec.name
                )));
            }
            if specs.iter().filter(|other| other.name == spec.name).count() > 1 {
                return Err(fail(format!(
                    "custom component '{}' is defined twice",
                    spec.name
                )));
            }
            if let Some(modifier) = &spec.modifier {
                if spec.base != ComponentKind::Button {
                    return Err(fail(format!(
                        "custom component '{}' presets a modifier on '{}', which takes none",
                        spec.name,
                        spec.base.canonical_name()
                    )));
                }
                if button_variant_ident(modifier).is_none() {
                    return Err(fail(format!(
                        "custom component '{}' presets unknown button modifier '.{modifier}'",
                        spec.name
                    )));
                }
            }
        }
        Ok(Self { specs })
    }

    /// Empty registry; resolution is the identity.
    pub fn empty() -> Self {
        Self { specs: Vec::new() }
    }

    /// Looks up one alias by name.
    pub fn get(&self, name: &str) -> Option<&CustomComponent> {
        self.specs.iter().find(|spec| spec.name == name)
    }

    /// Registered names in registration order.
    pub fn names(&self) -> Vec<String> {
        self.specs.iter().map(|spec| spec.name.clone()).collect()
    }

    /// Whether the registry holds no aliases.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Number of registered aliases.
    pub fn len(&self) -> usize {
        self.specs.len()
    }
}

/// Mirrors the parser `Name` rule: ASCII alpha start, then alnum/underscore.
fn valid_custom_name(name: &str) -> bool {
    let mut chars = name.chars();
    let head_ok = chars.next().is_some_and(|ch| ch.is_ascii_alphabetic());
    head_ok && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Rewrites custom names to their base nodes, merging preset modifiers.
///
/// Unknown names pass through untouched so [`validate_layout`] still
/// reports `unknown component` from one place. A node carrying both a
/// preset modifier and an explicit one fails: the modifier is given
/// twice.
///
/// # Errors
///
/// Returns [`ValidationError`] at the offending node offset when a
/// preset and an explicit modifier collide.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{
///     ComponentKind, CustomComponent, CustomComponents, parse_layout, resolve_layout,
/// };
///
/// let customs = CustomComponents::new(vec![CustomComponent {
///     name: "PrimaryButton".to_owned(),
///     base: ComponentKind::Button,
///     modifier: Some("primary".to_owned()),
/// }])
/// .unwrap();
/// let nodes = parse_layout("PrimaryButton\"Save\"").unwrap();
/// let resolved = resolve_layout(&nodes, &customs).unwrap();
/// assert_eq!(resolved[0].name, "B");
/// assert_eq!(resolved[0].modifier.as_deref(), Some("primary"));
/// ```
pub fn resolve_layout(
    nodes: &[LayoutNode],
    customs: &CustomComponents,
) -> Result<Vec<LayoutNode>, ValidationError> {
    nodes
        .iter()
        .map(|node| resolve_node(node, customs))
        .collect()
}

/// Resolves one node and its children.
fn resolve_node(
    node: &LayoutNode,
    customs: &CustomComponents,
) -> Result<LayoutNode, ValidationError> {
    let mut resolved = node.clone();
    if let Some(spec) = customs.get(&node.name) {
        spec.base.canonical_name().clone_into(&mut resolved.name);
        match (&spec.modifier, &node.modifier) {
            (Some(preset), Some(explicit)) => {
                return Err(ValidationError {
                    offset: node.offset,
                    message: format!("modifier given twice (preset '.{preset}' and '.{explicit}')"),
                });
            }
            (Some(preset), None) => resolved.modifier = Some(preset.clone()),
            (None, _) => {}
        }
    }
    resolved.children = resolve_layout(&node.children, customs)?;
    Ok(resolved)
}

/// Validates parsed nodes with custom aliases resolved first.
///
/// Behaves exactly like [`validate_layout`] for trees without custom
/// names.
///
/// # Errors
///
/// Returns [`ValidationError`] for modifier collisions and for every
/// failure [`validate_layout`] reports.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::{
///     ComponentKind, CustomComponent, CustomComponents, parse_layout, validate_layout_with,
/// };
///
/// let customs = CustomComponents::new(vec![CustomComponent {
///     name: "PrimaryButton".to_owned(),
///     base: ComponentKind::Button,
///     modifier: Some("primary".to_owned()),
/// }])
/// .unwrap();
/// let nodes = parse_layout("V > PrimaryButton\"Save\"").unwrap();
/// validate_layout_with(&nodes, &customs).unwrap();
/// ```
pub fn validate_layout_with(
    nodes: &[LayoutNode],
    customs: &CustomComponents,
) -> Result<(), ValidationError> {
    let resolved = resolve_layout(nodes, customs)?;
    super::validate_layout(&resolved)
}

#[cfg(test)]
mod tests {
    use super::{CustomComponent, CustomComponents, resolve_layout, validate_layout_with};
    use crate::{ComponentKind, parse_layout};

    /// Builds a one-button registry for tests.
    fn button_registry() -> CustomComponents {
        CustomComponents::new(vec![CustomComponent {
            name: "PrimaryButton".to_owned(),
            base: ComponentKind::Button,
            modifier: Some("primary".to_owned()),
        }])
        .unwrap()
    }

    #[test]
    fn rejects_bad_registry_specs() {
        let spec = |name: &str, base: ComponentKind, modifier: Option<&str>| CustomComponent {
            name: name.to_owned(),
            base,
            modifier: modifier.map(str::to_owned),
        };
        CustomComponents::new(vec![spec("9bad", ComponentKind::Button, None)]).unwrap_err();
        CustomComponents::new(vec![spec("", ComponentKind::Button, None)]).unwrap_err();
        CustomComponents::new(vec![spec("B", ComponentKind::Button, None)]).unwrap_err();
        CustomComponents::new(vec![
            spec("Alias", ComponentKind::Button, None),
            spec("Alias", ComponentKind::Input, None),
        ])
        .unwrap_err();
        CustomComponents::new(vec![spec("Alias", ComponentKind::Input, Some("primary"))])
            .unwrap_err();
        CustomComponents::new(vec![spec("Alias", ComponentKind::Button, Some("fancy"))])
            .unwrap_err();
    }

    #[test]
    fn resolves_aliases_and_passes_unknown_through() {
        let customs = button_registry();
        let nodes = parse_layout("V > (PrimaryButton\"Save\" + Button\"x\")").unwrap();
        let resolved = resolve_layout(&nodes, &customs).unwrap();
        assert_eq!(resolved[0].children[0].name, "B");
        assert_eq!(resolved[0].children[0].modifier.as_deref(), Some("primary"));
        assert_eq!(resolved[0].children[1].name, "Button");
    }

    #[test]
    fn rejects_modifier_given_twice() {
        let customs = button_registry();
        let nodes = parse_layout("PrimaryButton.secondary\"Save\"").unwrap();
        let error = resolve_layout(&nodes, &customs).unwrap_err();
        assert!(error.message.contains("twice"));
    }

    #[test]
    fn validates_resolved_trees() {
        let customs = button_registry();
        let nodes = parse_layout("V > PrimaryButton\"Save\"").unwrap();
        validate_layout_with(&nodes, &customs).unwrap();
        let missing = parse_layout("PrimaryButton#id").unwrap();
        assert!(
            validate_layout_with(&missing, &customs)
                .unwrap_err()
                .message
                .contains("label payload")
        );
    }

    #[test]
    fn empty_registry_is_identity() {
        let customs = CustomComponents::empty();
        assert!(customs.is_empty());
        assert_eq!(customs.len(), 0);
        let nodes = parse_layout("V > B\"Hi\"").unwrap();
        assert_eq!(resolve_layout(&nodes, &customs).unwrap(), nodes);
    }
}
