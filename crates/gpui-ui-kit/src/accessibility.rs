//! Accessibility support for gpui-ui-kit
//!
//! Provides ARIA roles, labels, and a runtime accessibility tree.
//! Since GPUI has no native accessibility support, this module stores
//! accessibility metadata at the UI-kit level so that:
//! 1. Components carry semantic meaning (role, label, description)
//! 2. A runtime tree can be queried by external code (future bridges, tests)
//! 3. Tooltip fallbacks can use aria_label when no tooltip is set

use gpui::{App, ElementId, Global, SharedString};
use std::collections::HashMap;

/// WAI-ARIA roles for UI components
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AriaRole {
    #[default]
    None,
    Button,
    Checkbox,
    Radio,
    Textbox,
    Spinbutton,
    Slider,
    Combobox,
    Listbox,
    Option,
    Switch,
    Tab,
    Tabpanel,
    Tablist,
    Dialog,
    Alertdialog,
    Alert,
    Status,
    Progressbar,
    Menu,
    Menuitem,
    Menubar,
    Toolbar,
    Table,
    Row,
    Columnheader,
    Cell,
    Tree,
    Treeitem,
    Navigation,
    Search,
    Heading,
    Link,
    Img,
    Group,
    Separator,
    Tooltip,
    Region,
}

/// ARIA state for components with checked/pressed/expanded states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AriaState {
    Checked(bool),
    Mixed,
    Pressed(bool),
    Expanded(bool),
    Selected(bool),
    Disabled,
    Hidden,
}

/// aria-live region behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaLive {
    Off,
    Polite,
    Assertive,
}

/// Accessibility properties that a component can carry.
///
/// The accessible name (label) lives on [`AccessibilityNode`], not here.
/// `AriaProps` carries the role, states, and value metadata.
#[derive(Debug, Clone, Default)]
pub struct AriaProps {
    pub role: AriaRole,
    pub description: Option<SharedString>,
    pub states: Vec<AriaState>,
    pub live: Option<AriaLive>,
    pub level: Option<u8>,
    pub value_now: Option<f64>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub value_text: Option<SharedString>,
}

impl AriaProps {
    pub fn with_role(role: AriaRole) -> Self {
        Self {
            role,
            ..Default::default()
        }
    }

    pub fn description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn state(mut self, state: AriaState) -> Self {
        self.states.push(state);
        self
    }

    /// Conditionally add a state
    pub fn maybe_state(self, condition: bool, state: AriaState) -> Self {
        if condition { self.state(state) } else { self }
    }

    pub fn live(mut self, live: AriaLive) -> Self {
        self.live = Some(live);
        self
    }

    pub fn level(mut self, level: u8) -> Self {
        self.level = Some(level);
        self
    }

    pub fn value_range(mut self, now: f64, min: f64, max: f64) -> Self {
        self.value_now = Some(now);
        self.value_min = Some(min);
        self.value_max = Some(max);
        self
    }

    pub fn value_text(mut self, text: impl Into<SharedString>) -> Self {
        self.value_text = Some(text.into());
        self
    }
}

/// A node in the accessibility tree
#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    pub element_id: ElementId,
    pub label: SharedString,
    pub props: AriaProps,
}

/// Runtime accessibility tree, rebuilt each render frame.
pub struct AccessibilityTree {
    nodes: HashMap<ElementId, AccessibilityNode>,
    order: Vec<ElementId>,
}

impl Global for AccessibilityTree {}

impl AccessibilityTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.order.clear();
    }

    pub fn register(&mut self, node: AccessibilityNode) {
        let id = node.element_id.clone();
        if !self.nodes.contains_key(&id) {
            self.order.push(id.clone());
        }
        self.nodes.insert(id, node);
    }

    pub fn get(&self, id: &ElementId) -> Option<&AccessibilityNode> {
        self.nodes.get(id)
    }

    pub fn nodes_in_order(&self) -> Vec<&AccessibilityNode> {
        self.order
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for AccessibilityTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for accessibility tree access on App
pub trait AccessibilityExt {
    fn register_accessible(&mut self, node: AccessibilityNode);
    fn accessibility_tree(&self) -> Option<&AccessibilityTree>;
}

impl AccessibilityExt for App {
    fn register_accessible(&mut self, node: AccessibilityNode) {
        if self.has_global::<AccessibilityTree>() {
            self.global_mut::<AccessibilityTree>().register(node);
        }
    }

    fn accessibility_tree(&self) -> Option<&AccessibilityTree> {
        self.try_global::<AccessibilityTree>()
    }
}
