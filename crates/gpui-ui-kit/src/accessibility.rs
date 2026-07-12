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

/// Schema version for platform-neutral accessibility bridge snapshots.
pub const ACCESSIBILITY_BRIDGE_SCHEMA_VERSION: u32 = 1;

/// Stable report type for platform-neutral accessibility bridge snapshots.
pub const ACCESSIBILITY_BRIDGE_REPORT_TYPE: &str = "gpui-ui-kit-accessibility-bridge";

/// Schema version for host/native accessibility adapter payloads.
pub const ACCESSIBILITY_ADAPTER_SCHEMA_VERSION: u32 = 1;

/// Stable report type for host/native accessibility adapter payloads.
pub const ACCESSIBILITY_ADAPTER_REPORT_TYPE: &str = "gpui-ui-kit-native-accessibility-adapter";

/// Schema version for native accessibility bridge readiness reports.
pub const ACCESSIBILITY_READINESS_SCHEMA_VERSION: u32 = 1;

/// Stable report type for native accessibility bridge readiness reports.
pub const ACCESSIBILITY_READINESS_REPORT_TYPE: &str = "gpui-ui-kit-accessibility-readiness";

/// Release-readiness status for an accessibility bridge surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityReadinessStatus {
    /// UI-kit emits the metadata needed by this surface.
    Implemented,
    /// The surface is covered by component or bridge snapshot tests.
    ComponentTested,
    /// Real platform screen-reader QA still needs to be executed.
    PlatformQaPending,
}

impl AccessibilityReadinessStatus {
    /// Stable status label for generated reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::ComponentTested => "component-tested",
            Self::PlatformQaPending => "platform-qa-pending",
        }
    }

    /// Whether this status is enough for a UI-kit native-accessibility claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(self, Self::Implemented | Self::ComponentTested)
    }
}

/// One accessibility bridge readiness row for release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessibilityReadinessEntry {
    /// Stable row id.
    pub id: &'static str,
    /// Human-readable bridge surface.
    pub surface: &'static str,
    /// Current readiness state.
    pub status: AccessibilityReadinessStatus,
    /// Evidence recorded by this crate.
    pub evidence: &'static str,
    /// Remaining requirement before native accessibility can be claimed.
    pub release_requirement: &'static str,
}

/// Versioned native accessibility readiness report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessibilityReadinessReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [AccessibilityReadinessEntry],
}

impl AccessibilityReadinessReport {
    /// Return true only when every accessibility row is release-ready.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block a native-accessibility claim.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static AccessibilityReadinessEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_release_ready())
    }

    /// Render the report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI UI Kit Accessibility Readiness\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Surface | Status | Evidence | Release requirement |\n\
             | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                entry.surface,
                entry.status.as_str(),
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const ACCESSIBILITY_READINESS_ENTRIES: &[AccessibilityReadinessEntry] = &[
    AccessibilityReadinessEntry {
        id: "aria-metadata",
        surface: "ARIA role/state/value metadata",
        status: AccessibilityReadinessStatus::Implemented,
        evidence: "AriaRole, AriaState, AriaLive, AriaProps, and AccessibilityNode provide stable role, state, live-region, level, value, label, and description metadata for UI-kit components.",
        release_requirement: "Keep semantic defaults and component accessibility tests green before release.",
    },
    AccessibilityReadinessEntry {
        id: "bridge-snapshot",
        surface: "Platform-neutral bridge snapshot",
        status: AccessibilityReadinessStatus::ComponentTested,
        evidence: "AccessibilityTree::to_bridge_snapshot() exports ordered AccessibilityBridgeNode payloads with schema_version, report_type, role names, labels, states, live metadata, levels, and values.",
        release_requirement: "Keep bridge snapshot contract tests green and attach snapshot examples for release QA.",
    },
    AccessibilityReadinessEntry {
        id: "accessible-name-audit",
        surface: "Accessible-name release audit",
        status: AccessibilityReadinessStatus::ComponentTested,
        evidence: "AccessibilityBridgeSnapshot::blocking_entries() reports interactive/non-separator nodes without accessible names, and all_nodes_named() provides a release gate helper.",
        release_requirement: "Run app-level snapshots and resolve unnamed controls before claiming app accessibility readiness.",
    },
    AccessibilityReadinessEntry {
        id: "focus-keyboard-handoff",
        surface: "Keyboard focus handoff",
        status: AccessibilityReadinessStatus::ComponentTested,
        evidence: "focus_integration_report() records FocusGroup, form, overlay, data-navigation, mobile-surface, and native-accessibility readiness rows for keyboard handoff.",
        release_requirement: "Attach product-level keyboard-only walkthrough evidence before claiming end-to-end accessibility parity.",
    },
    AccessibilityReadinessEntry {
        id: "native-host-adapter",
        surface: "Host/native adapter payload",
        status: AccessibilityReadinessStatus::ComponentTested,
        evidence: "AccessibilityBridgeSnapshot::to_native_adapter_payload() validates adapter ids and accessible names, then exports deterministic native adapter nodes with stable ids, roles, names, descriptions, states, values, focusability, visibility, and action hints.",
        release_requirement: "Keep native adapter contract tests green and wire the payload into each selected host platform accessibility layer.",
    },
    AccessibilityReadinessEntry {
        id: "cross-platform-screen-reader-qa",
        surface: "Cross-platform screen-reader QA",
        status: AccessibilityReadinessStatus::PlatformQaPending,
        evidence: "No VoiceOver, Narrator, Orca/AT-SPI, iOS VoiceOver, or Android TalkBack walkthrough artifact is recorded in this crate.",
        release_requirement: "Run and attach platform screen-reader walkthrough results for the selected release targets.",
    },
];

/// Return the current native accessibility readiness report.
pub const fn accessibility_readiness_report() -> AccessibilityReadinessReport {
    AccessibilityReadinessReport {
        schema_version: ACCESSIBILITY_READINESS_SCHEMA_VERSION,
        report_type: ACCESSIBILITY_READINESS_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: ACCESSIBILITY_READINESS_ENTRIES,
    }
}

/// Return all native accessibility readiness rows.
pub const fn accessibility_readiness_entries() -> &'static [AccessibilityReadinessEntry] {
    ACCESSIBILITY_READINESS_ENTRIES
}

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

impl AriaRole {
    /// Stable lowercase role name for native accessibility adapters.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Button => "button",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Textbox => "textbox",
            Self::Spinbutton => "spinbutton",
            Self::Slider => "slider",
            Self::Combobox => "combobox",
            Self::Listbox => "listbox",
            Self::Option => "option",
            Self::Switch => "switch",
            Self::Tab => "tab",
            Self::Tabpanel => "tabpanel",
            Self::Tablist => "tablist",
            Self::Dialog => "dialog",
            Self::Alertdialog => "alertdialog",
            Self::Alert => "alert",
            Self::Status => "status",
            Self::Progressbar => "progressbar",
            Self::Menu => "menu",
            Self::Menuitem => "menuitem",
            Self::Menubar => "menubar",
            Self::Toolbar => "toolbar",
            Self::Table => "table",
            Self::Row => "row",
            Self::Columnheader => "columnheader",
            Self::Cell => "cell",
            Self::Tree => "tree",
            Self::Treeitem => "treeitem",
            Self::Navigation => "navigation",
            Self::Search => "search",
            Self::Heading => "heading",
            Self::Link => "link",
            Self::Img => "img",
            Self::Group => "group",
            Self::Separator => "separator",
            Self::Tooltip => "tooltip",
            Self::Region => "region",
        }
    }
}

impl AriaState {
    /// Stable state name and optional boolean value for native accessibility adapters.
    pub const fn bridge_name_value(self) -> (&'static str, Option<bool>) {
        match self {
            Self::Checked(value) => ("checked", Some(value)),
            Self::Mixed => ("mixed", None),
            Self::Pressed(value) => ("pressed", Some(value)),
            Self::Expanded(value) => ("expanded", Some(value)),
            Self::Selected(value) => ("selected", Some(value)),
            Self::Disabled => ("disabled", None),
            Self::Hidden => ("hidden", None),
        }
    }
}

impl AriaLive {
    /// Stable live-region name for native accessibility adapters.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Polite => "polite",
            Self::Assertive => "assertive",
        }
    }
}

/// A node in the accessibility tree
#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    pub element_id: ElementId,
    pub label: SharedString,
    pub props: AriaProps,
}

/// State exported in a platform-neutral bridge snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityBridgeState {
    pub name: &'static str,
    pub value: Option<bool>,
}

/// Numeric/text value metadata exported in a platform-neutral bridge snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityBridgeValue {
    pub now: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub text: Option<SharedString>,
}

/// Platform family a host/native accessibility adapter targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAccessibilityTarget {
    Desktop,
    Macos,
    Windows,
    Linux,
    Ios,
    Android,
    Test,
}

impl NativeAccessibilityTarget {
    /// Stable target label for adapter manifests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Test => "test",
        }
    }
}

/// Platform-neutral action hint for host/native accessibility adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAccessibilityAction {
    Focus,
    Press,
    SetValue,
    Increment,
    Decrement,
    Expand,
    Collapse,
}

impl NativeAccessibilityAction {
    /// Stable action label for native adapter payloads.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Press => "press",
            Self::SetValue => "set-value",
            Self::Increment => "increment",
            Self::Decrement => "decrement",
            Self::Expand => "expand",
            Self::Collapse => "collapse",
        }
    }
}

/// Validation failure while converting a bridge snapshot into a native adapter payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeAccessibilityAdapterError {
    EmptyAdapterId,
    MissingAccessibleNames(Vec<String>),
}

impl std::fmt::Display for NativeAccessibilityAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAdapterId => write!(f, "native accessibility adapter id is empty"),
            Self::MissingAccessibleNames(element_ids) => write!(
                f,
                "native accessibility adapter payload has unnamed interactive nodes: {}",
                element_ids.join(", ")
            ),
        }
    }
}

impl std::error::Error for NativeAccessibilityAdapterError {}

/// One deterministic node ready for host/native accessibility adapter consumption.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAccessibilityAdapterNode {
    pub index: usize,
    pub element_key: String,
    pub role: AriaRole,
    pub role_name: &'static str,
    pub name: SharedString,
    pub description: Option<SharedString>,
    pub states: Vec<AccessibilityBridgeState>,
    pub live: Option<&'static str>,
    pub level: Option<u8>,
    pub value: AccessibilityBridgeValue,
    pub focusable: bool,
    pub disabled: bool,
    pub hidden: bool,
    pub actions: Vec<NativeAccessibilityAction>,
}

/// Validated payload handed from UI-kit to a host/native accessibility adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAccessibilityAdapterPayload {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub adapter_id: SharedString,
    pub target: NativeAccessibilityTarget,
    pub nodes: Vec<NativeAccessibilityAdapterNode>,
}

impl NativeAccessibilityAdapterPayload {
    pub fn action_labels(&self) -> Vec<&'static str> {
        self.nodes
            .iter()
            .flat_map(|node| node.actions.iter().map(|action| action.as_str()))
            .collect()
    }
}

/// Node exported for native platform accessibility adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityBridgeNode {
    pub element_id: ElementId,
    pub role: AriaRole,
    pub role_name: &'static str,
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub states: Vec<AccessibilityBridgeState>,
    pub live: Option<&'static str>,
    pub level: Option<u8>,
    pub value: AccessibilityBridgeValue,
}

impl AccessibilityBridgeNode {
    pub fn from_node(node: &AccessibilityNode) -> Self {
        let states = node
            .props
            .states
            .iter()
            .map(|state| {
                let (name, value) = state.bridge_name_value();
                AccessibilityBridgeState { name, value }
            })
            .collect();

        Self {
            element_id: node.element_id.clone(),
            role: node.props.role,
            role_name: node.props.role.as_str(),
            label: node.label.clone(),
            description: node.props.description.clone(),
            states,
            live: node.props.live.map(AriaLive::as_str),
            level: node.props.level,
            value: AccessibilityBridgeValue {
                now: node.props.value_now,
                min: node.props.value_min,
                max: node.props.value_max,
                text: node.props.value_text.clone(),
            },
        }
    }

    pub fn has_accessible_name(&self) -> bool {
        self.role == AriaRole::None
            || self.role == AriaRole::Separator
            || !self.label.as_ref().trim().is_empty()
    }

    pub fn element_key(&self) -> String {
        format!("{:?}", self.element_id)
    }

    pub fn is_disabled(&self) -> bool {
        self.states
            .iter()
            .any(|state| state.name == "disabled" && state.value.unwrap_or(true))
    }

    pub fn is_hidden(&self) -> bool {
        self.states
            .iter()
            .any(|state| state.name == "hidden" && state.value.unwrap_or(true))
    }

    pub fn state_value(&self, name: &str) -> Option<bool> {
        self.states
            .iter()
            .find(|state| state.name == name)
            .and_then(|state| state.value)
    }

    pub fn is_focusable_for_native_adapter(&self) -> bool {
        if self.is_disabled() || self.is_hidden() {
            return false;
        }

        matches!(
            self.role,
            AriaRole::Button
                | AriaRole::Checkbox
                | AriaRole::Radio
                | AriaRole::Textbox
                | AriaRole::Spinbutton
                | AriaRole::Slider
                | AriaRole::Combobox
                | AriaRole::Listbox
                | AriaRole::Option
                | AriaRole::Switch
                | AriaRole::Tab
                | AriaRole::Dialog
                | AriaRole::Alertdialog
                | AriaRole::Menuitem
                | AriaRole::Treeitem
                | AriaRole::Link
                | AriaRole::Search
        )
    }

    pub fn native_adapter_actions(&self) -> Vec<NativeAccessibilityAction> {
        if !self.is_focusable_for_native_adapter() {
            return Vec::new();
        }

        let mut actions = vec![NativeAccessibilityAction::Focus];

        if matches!(
            self.role,
            AriaRole::Button
                | AriaRole::Checkbox
                | AriaRole::Radio
                | AriaRole::Switch
                | AriaRole::Tab
                | AriaRole::Menuitem
                | AriaRole::Option
                | AriaRole::Link
        ) {
            actions.push(NativeAccessibilityAction::Press);
        }

        if matches!(
            self.role,
            AriaRole::Textbox | AriaRole::Spinbutton | AriaRole::Slider | AriaRole::Combobox
        ) {
            actions.push(NativeAccessibilityAction::SetValue);
        }

        if matches!(self.role, AriaRole::Spinbutton | AriaRole::Slider)
            && self.value.now.is_some()
            && self.value.min.is_some()
            && self.value.max.is_some()
        {
            actions.push(NativeAccessibilityAction::Increment);
            actions.push(NativeAccessibilityAction::Decrement);
        }

        match self.state_value("expanded") {
            Some(true) => actions.push(NativeAccessibilityAction::Collapse),
            Some(false) => actions.push(NativeAccessibilityAction::Expand),
            None => {}
        }

        actions
    }

    pub fn to_native_adapter_node(&self, index: usize) -> NativeAccessibilityAdapterNode {
        NativeAccessibilityAdapterNode {
            index,
            element_key: self.element_key(),
            role: self.role,
            role_name: self.role_name,
            name: self.label.clone(),
            description: self.description.clone(),
            states: self.states.clone(),
            live: self.live,
            level: self.level,
            value: self.value.clone(),
            focusable: self.is_focusable_for_native_adapter(),
            disabled: self.is_disabled(),
            hidden: self.is_hidden(),
            actions: self.native_adapter_actions(),
        }
    }
}

/// Ordered, platform-neutral accessibility tree export for native adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityBridgeSnapshot {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub nodes: Vec<AccessibilityBridgeNode>,
}

impl AccessibilityBridgeSnapshot {
    pub fn blocking_entries(&self) -> impl Iterator<Item = &AccessibilityBridgeNode> {
        self.nodes.iter().filter(|node| !node.has_accessible_name())
    }

    pub fn all_nodes_named(&self) -> bool {
        self.blocking_entries().next().is_none()
    }

    pub fn to_native_adapter_payload(
        &self,
        adapter_id: impl Into<SharedString>,
        target: NativeAccessibilityTarget,
    ) -> Result<NativeAccessibilityAdapterPayload, NativeAccessibilityAdapterError> {
        let adapter_id = adapter_id.into();
        if adapter_id.as_ref().trim().is_empty() {
            return Err(NativeAccessibilityAdapterError::EmptyAdapterId);
        }

        let missing_names = self
            .blocking_entries()
            .map(AccessibilityBridgeNode::element_key)
            .collect::<Vec<_>>();
        if !missing_names.is_empty() {
            return Err(NativeAccessibilityAdapterError::MissingAccessibleNames(
                missing_names,
            ));
        }

        Ok(NativeAccessibilityAdapterPayload {
            schema_version: ACCESSIBILITY_ADAPTER_SCHEMA_VERSION,
            report_type: ACCESSIBILITY_ADAPTER_REPORT_TYPE,
            adapter_id,
            target,
            nodes: self
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| node.to_native_adapter_node(index))
                .collect(),
        })
    }

    pub fn to_markdown_table(&self) -> String {
        let mut markdown = format!(
            "# {}\n\nschema_version: {}\n\n| Element | Role | Label | States | Value |\n|---|---|---|---|---|\n",
            self.report_type, self.schema_version
        );

        for node in &self.nodes {
            let states = if node.states.is_empty() {
                String::new()
            } else {
                node.states
                    .iter()
                    .map(|state| match state.value {
                        Some(value) => format!("{}={}", state.name, value),
                        None => state.name.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let value = match (
                node.value.now,
                node.value.min,
                node.value.max,
                node.value.text.as_ref(),
            ) {
                (_, _, _, Some(text)) => text.as_ref().to_string(),
                (Some(now), Some(min), Some(max), None) => format!("{} [{}..{}]", now, min, max),
                (Some(now), _, _, None) => now.to_string(),
                _ => String::new(),
            };

            markdown.push_str(&format!(
                "| `{:?}` | `{}` | {} | {} | {} |\n",
                node.element_id,
                node.role_name,
                escape_markdown_cell(node.label.as_ref()),
                escape_markdown_cell(&states),
                escape_markdown_cell(&value)
            ));
        }

        markdown
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
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

    pub fn to_bridge_snapshot(&self) -> AccessibilityBridgeSnapshot {
        AccessibilityBridgeSnapshot {
            schema_version: ACCESSIBILITY_BRIDGE_SCHEMA_VERSION,
            report_type: ACCESSIBILITY_BRIDGE_REPORT_TYPE,
            nodes: self
                .nodes_in_order()
                .into_iter()
                .map(AccessibilityBridgeNode::from_node)
                .collect(),
        }
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

#[cfg(test)]
mod tests {
    use super::{
        ACCESSIBILITY_ADAPTER_REPORT_TYPE, ACCESSIBILITY_ADAPTER_SCHEMA_VERSION,
        ACCESSIBILITY_BRIDGE_REPORT_TYPE, ACCESSIBILITY_BRIDGE_SCHEMA_VERSION,
        ACCESSIBILITY_READINESS_REPORT_TYPE, ACCESSIBILITY_READINESS_SCHEMA_VERSION,
        AccessibilityNode, AccessibilityReadinessStatus, AccessibilityTree, AriaLive, AriaProps,
        AriaRole, AriaState, NativeAccessibilityAction, NativeAccessibilityAdapterError,
        NativeAccessibilityTarget, accessibility_readiness_entries, accessibility_readiness_report,
    };
    use gpui::ElementId;

    #[test]
    fn aria_names_are_stable_for_bridge_adapters() {
        assert_eq!(AriaRole::Button.as_str(), "button");
        assert_eq!(AriaRole::Alertdialog.as_str(), "alertdialog");
        assert_eq!(AriaRole::Columnheader.as_str(), "columnheader");
        assert_eq!(
            AriaState::Checked(true).bridge_name_value(),
            ("checked", Some(true))
        );
        assert_eq!(AriaState::Mixed.bridge_name_value(), ("mixed", None));
        assert_eq!(AriaLive::Assertive.as_str(), "assertive");
    }

    #[test]
    fn accessibility_tree_exports_ordered_bridge_snapshot() {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("volume".into()),
            label: "Volume".into(),
            props: AriaProps::with_role(AriaRole::Slider)
                .description("Playback volume")
                .state(AriaState::Disabled)
                .value_range(75.0, 0.0, 100.0)
                .value_text("75%"),
        });
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("status".into()),
            label: "Saved".into(),
            props: AriaProps::with_role(AriaRole::Status).live(AriaLive::Polite),
        });

        let snapshot = tree.to_bridge_snapshot();

        assert_eq!(snapshot.schema_version, ACCESSIBILITY_BRIDGE_SCHEMA_VERSION);
        assert_eq!(snapshot.report_type, ACCESSIBILITY_BRIDGE_REPORT_TYPE);
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.nodes[0].role, AriaRole::Slider);
        assert_eq!(snapshot.nodes[0].role_name, "slider");
        assert_eq!(snapshot.nodes[0].label.as_ref(), "Volume");
        assert_eq!(
            snapshot.nodes[0]
                .description
                .as_ref()
                .map(|text| text.as_ref()),
            Some("Playback volume")
        );
        assert_eq!(snapshot.nodes[0].states[0].name, "disabled");
        assert_eq!(snapshot.nodes[0].value.now, Some(75.0));
        assert_eq!(
            snapshot.nodes[0]
                .value
                .text
                .as_ref()
                .map(|text| text.as_ref()),
            Some("75%")
        );
        assert_eq!(snapshot.nodes[1].live, Some("polite"));
    }

    #[test]
    fn bridge_snapshot_reports_missing_accessible_names() {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("icon-only".into()),
            label: "".into(),
            props: AriaProps::with_role(AriaRole::Button),
        });
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("divider".into()),
            label: "".into(),
            props: AriaProps::with_role(AriaRole::Separator),
        });

        let snapshot = tree.to_bridge_snapshot();
        let blocking: Vec<_> = snapshot.blocking_entries().collect();

        assert_eq!(blocking.len(), 1);
        assert_eq!(blocking[0].role, AriaRole::Button);
        assert!(!snapshot.all_nodes_named());
    }

    #[test]
    fn bridge_snapshot_markdown_names_payload_shape() {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("ok".into()),
            label: "OK | Confirm".into(),
            props: AriaProps::with_role(AriaRole::Button).state(AriaState::Pressed(false)),
        });

        let markdown = tree.to_bridge_snapshot().to_markdown_table();

        assert!(markdown.contains("gpui-ui-kit-accessibility-bridge"));
        assert!(markdown.contains("schema_version: 1"));
        assert!(markdown.contains("`button`"));
        assert!(markdown.contains("OK \\| Confirm"));
        assert!(markdown.contains("pressed=false"));
    }

    #[test]
    fn bridge_snapshot_exports_native_adapter_payload() {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("volume".into()),
            label: "Volume".into(),
            props: AriaProps::with_role(AriaRole::Slider)
                .description("Playback volume")
                .value_range(75.0, 0.0, 100.0)
                .value_text("75%"),
        });
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("filters".into()),
            label: "Filters".into(),
            props: AriaProps::with_role(AriaRole::Combobox).state(AriaState::Expanded(false)),
        });
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("status".into()),
            label: "Saved".into(),
            props: AriaProps::with_role(AriaRole::Status).live(AriaLive::Polite),
        });

        let payload = tree
            .to_bridge_snapshot()
            .to_native_adapter_payload("macos-voiceover", NativeAccessibilityTarget::Macos)
            .expect("valid native adapter payload");

        assert_eq!(payload.schema_version, ACCESSIBILITY_ADAPTER_SCHEMA_VERSION);
        assert_eq!(payload.report_type, ACCESSIBILITY_ADAPTER_REPORT_TYPE);
        assert_eq!(payload.adapter_id.as_ref(), "macos-voiceover");
        assert_eq!(payload.target.as_str(), "macos");
        assert_eq!(payload.nodes.len(), 3);

        let slider = &payload.nodes[0];
        assert_eq!(slider.index, 0);
        assert!(slider.element_key.contains("volume"));
        assert_eq!(slider.role_name, "slider");
        assert!(slider.focusable);
        assert_eq!(slider.value.now, Some(75.0));
        assert!(slider.actions.contains(&NativeAccessibilityAction::Focus));
        assert!(
            slider
                .actions
                .contains(&NativeAccessibilityAction::SetValue)
        );
        assert!(
            slider
                .actions
                .contains(&NativeAccessibilityAction::Increment)
        );
        assert!(
            slider
                .actions
                .contains(&NativeAccessibilityAction::Decrement)
        );

        let combobox = &payload.nodes[1];
        assert!(
            combobox
                .actions
                .contains(&NativeAccessibilityAction::Expand)
        );
        assert!(
            !combobox
                .actions
                .contains(&NativeAccessibilityAction::Collapse)
        );

        let status = &payload.nodes[2];
        assert!(!status.focusable);
        assert!(status.actions.is_empty());
        assert_eq!(status.live, Some("polite"));
    }

    #[test]
    fn native_targets_preserve_tree_order_states_values_and_action_parity() {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("gain".into()),
            label: "Gain مستوى".into(),
            props: AriaProps::with_role(AriaRole::Slider)
                .state(AriaState::Disabled)
                .value_range(-6.0, -24.0, 12.0)
                .value_text("−6 dB"),
        });
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("details".into()),
            label: "פרטים Details".into(),
            props: AriaProps::with_role(AriaRole::Button).state(AriaState::Expanded(true)),
        });
        let snapshot = tree.to_bridge_snapshot();
        let targets = [
            NativeAccessibilityTarget::Macos,
            NativeAccessibilityTarget::Windows,
            NativeAccessibilityTarget::Linux,
            NativeAccessibilityTarget::Ios,
            NativeAccessibilityTarget::Android,
        ];
        let reference = snapshot
            .to_native_adapter_payload("reference", targets[0])
            .unwrap();
        for target in targets {
            let payload = snapshot
                .to_native_adapter_payload(target.as_str(), target)
                .unwrap();
            assert_eq!(
                payload.nodes,
                reference.nodes,
                "{} adapter drift",
                target.as_str()
            );
            assert_eq!(payload.action_labels(), reference.action_labels());
        }
    }

    #[test]
    fn native_adapter_payload_rejects_bad_release_inputs() {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::Name("icon-only".into()),
            label: "".into(),
            props: AriaProps::with_role(AriaRole::Button),
        });
        let snapshot = tree.to_bridge_snapshot();

        assert_eq!(
            snapshot
                .to_native_adapter_payload(" ", NativeAccessibilityTarget::Test)
                .expect_err("empty adapter id is invalid"),
            NativeAccessibilityAdapterError::EmptyAdapterId
        );

        let error = snapshot
            .to_native_adapter_payload("test-adapter", NativeAccessibilityTarget::Test)
            .expect_err("unnamed button blocks native adapter export");

        match error {
            NativeAccessibilityAdapterError::MissingAccessibleNames(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert!(nodes[0].contains("icon-only"));
            }
            NativeAccessibilityAdapterError::EmptyAdapterId => {
                panic!("expected unnamed-node error");
            }
        }
    }

    #[test]
    fn accessibility_readiness_report_has_stable_contract() {
        let report = accessibility_readiness_report();

        assert_eq!(
            report.schema_version,
            ACCESSIBILITY_READINESS_SCHEMA_VERSION
        );
        assert_eq!(report.report_type, ACCESSIBILITY_READINESS_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(!report.entries.is_empty());
        assert!(!report.all_release_ready());
    }

    #[test]
    fn accessibility_readiness_report_has_unique_rows() {
        let mut ids = std::collections::BTreeSet::new();

        for entry in accessibility_readiness_entries() {
            assert!(
                ids.insert(entry.id),
                "duplicate accessibility readiness row {}",
                entry.id
            );
            assert!(!entry.surface.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn accessibility_readiness_report_separates_snapshot_from_native_gates() {
        let report = accessibility_readiness_report();
        let entries = accessibility_readiness_entries();

        assert!(entries.iter().any(|entry| {
            entry.id == "bridge-snapshot"
                && entry.status == AccessibilityReadinessStatus::ComponentTested
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "native-host-adapter"
                && entry.status == AccessibilityReadinessStatus::ComponentTested
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "cross-platform-screen-reader-qa"
                && entry.status == AccessibilityReadinessStatus::PlatformQaPending
        }));

        let blocking = report
            .blocking_entries()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        assert!(!blocking.contains(&"aria-metadata"));
        assert!(!blocking.contains(&"bridge-snapshot"));
        assert!(!blocking.contains(&"native-host-adapter"));
        assert!(blocking.contains(&"cross-platform-screen-reader-qa"));
    }

    #[test]
    fn accessibility_readiness_markdown_names_remaining_screen_reader_blocker() {
        let markdown = accessibility_readiness_report().to_markdown_table();

        assert!(markdown.contains(ACCESSIBILITY_READINESS_REPORT_TYPE));
        assert!(markdown.contains("Platform-neutral bridge snapshot"));
        assert!(markdown.contains("Host/native adapter payload"));
        assert!(markdown.contains("component-tested"));
        assert!(markdown.contains("platform-qa-pending"));
        assert!(markdown.contains("VoiceOver"));
    }
}
