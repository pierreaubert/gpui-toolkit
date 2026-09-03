use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Current JSON schema version for Python-authored app IR payloads.
pub const PYTHON_APP_IR_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum UiIrError {
    #[error("unsupported {schema} schema version {version}; supported version is {supported}")]
    UnsupportedSchemaVersion {
        schema: &'static str,
        version: u32,
        supported: u32,
    },

    #[error("app requires at least one section")]
    EmptySections,

    #[error("section {section:?} has empty id")]
    EmptySectionId { section: String },

    #[error("chart {id:?} is missing required data: {field}")]
    MissingChartData { id: String, field: &'static str },

    #[error("chart {id:?} has mismatched lengths: {left}={left_len}, {right}={right_len}")]
    ChartLengthMismatch {
        id: String,
        left: &'static str,
        left_len: usize,
        right: &'static str,
        right_len: usize,
    },

    #[error("heatmap {id:?} has {z_len} values but expected {width} x {height} = {expected}")]
    HeatmapDimensionMismatch {
        id: String,
        z_len: usize,
        width: usize,
        height: usize,
        expected: usize,
    },
    #[error("unknown UI node id {id:?}")]
    UnknownNodeId { id: String },
    #[error("invalid patch operation: {message}")]
    InvalidPatch { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniAppShellConfig {
    pub title: String,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub app_name: String,
    #[serde(default = "default_miniapp_scrollable")]
    pub scrollable: bool,
    #[serde(default)]
    pub with_theme: bool,
    #[serde(default)]
    pub with_i18n: bool,
    #[serde(default = "default_miniapp_theme")]
    pub initial_theme: String,
    #[serde(default = "default_miniapp_language")]
    pub initial_language: String,
}

fn default_miniapp_scrollable() -> bool {
    true
}
fn default_miniapp_theme() -> String {
    "dark".into()
}
fn default_miniapp_language() -> String {
    "english".into()
}

impl MiniAppShellConfig {
    pub fn validate(&self) -> Result<(), UiIrError> {
        if self.title.trim().is_empty()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.0
            || self.height <= 0.0
            || self.app_name.trim().is_empty()
        {
            return Err(UiIrError::InvalidPatch {
                message: "miniapp shell requires title, app name, and positive finite dimensions"
                    .into(),
            });
        }
        if !matches!(
            self.initial_theme.to_ascii_lowercase().as_str(),
            "dark"
                | "light"
                | "midnight"
                | "forest"
                | "black_and_white"
                | "onyx"
                | "carbon_white"
                | "carbon_gray_10"
                | "carbon_gray_90"
                | "carbon_gray_100"
        ) || !matches!(
            self.initial_language.to_ascii_lowercase().as_str(),
            "english" | "french" | "german" | "spanish" | "japanese"
        ) {
            return Err(UiIrError::InvalidPatch {
                message: "miniapp shell has an unsupported theme or language".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PythonAppIr {
    #[serde(default = "default_python_app_ir_schema_version")]
    pub schema_version: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(default = "default_sidebar_title")]
    pub sidebar_title: String,
    #[serde(default)]
    pub sidebar_subtitle: String,
    /// Optional native mini-app shell configuration. The host applies this to
    /// its existing window; Python never receives a raw window handle.
    #[serde(default)]
    pub miniapp: Option<MiniAppShellConfig>,
    #[serde(default)]
    pub sections: Vec<UiSection>,
}

impl PythonAppIr {
    pub fn validate(&self) -> Result<(), UiIrError> {
        if self.schema_version != PYTHON_APP_IR_SCHEMA_VERSION {
            return Err(UiIrError::UnsupportedSchemaVersion {
                schema: "python_app_ir",
                version: self.schema_version,
                supported: PYTHON_APP_IR_SCHEMA_VERSION,
            });
        }
        if self.sections.is_empty() {
            return Err(UiIrError::EmptySections);
        }
        if self
            .width
            .is_some_and(|width| !width.is_finite() || width <= 0.0)
            || self
                .height
                .is_some_and(|height| !height.is_finite() || height <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "app width and height must be finite and positive when specified".into(),
            });
        }
        if let Some(miniapp) = &self.miniapp {
            miniapp.validate()?;
        }
        for section in &self.sections {
            section.validate()?;
        }
        Ok(())
    }

    /// Apply a transaction of session patch operations. Validation happens on a
    /// cloned JSON tree, so a bad operation never leaves a partially-mutated
    /// application snapshot behind.
    pub fn apply_patch_ops(&mut self, ops: &[crate::session::PatchOp]) -> Result<(), UiIrError> {
        let mut tree = serde_json::to_value(&*self).map_err(|error| UiIrError::InvalidPatch {
            message: error.to_string(),
        })?;
        for op in ops {
            apply_patch_op(&mut tree, op)?;
        }
        let next: Self = serde_json::from_value(tree).map_err(|error| UiIrError::InvalidPatch {
            message: error.to_string(),
        })?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    /// Apply operations to the canonical JSON transaction tree. Hosts that
    /// also need to inspect mesh-resource references can reuse this value for
    /// validation and commit, avoiding repeated typed-IR serialization.
    pub fn apply_patch_ops_to_value(
        tree: &mut Value,
        ops: &[crate::session::PatchOp],
    ) -> Result<(), UiIrError> {
        for op in ops {
            apply_patch_op(tree, op)?;
        }
        Ok(())
    }

    /// Deserialize and validate a successfully patched transaction tree.
    pub fn from_patched_value(tree: &Value) -> Result<Self, UiIrError> {
        let next = Self::deserialize(tree).map_err(|error| UiIrError::InvalidPatch {
            message: error.to_string(),
        })?;
        next.validate()?;
        Ok(next)
    }
}

fn node_id(value: &Value) -> Option<&str> {
    value.get("id")?.as_str()
}

fn find_node_mut<'a>(value: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    if node_id(value) == Some(id) {
        return Some(value);
    }
    match value {
        Value::Object(object) => {
            for child in object.values_mut() {
                if let Some(found) = find_node_mut(child, id) {
                    return Some(found);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                if let Some(found) = find_node_mut(child, id) {
                    return Some(found);
                }
            }
        }
        _ => {}
    }
    None
}

fn for_each_section_content_mut(value: &mut Value, mut f: impl FnMut(&mut Value) -> bool) -> bool {
    let Some(sections) = value.get_mut("sections").and_then(Value::as_array_mut) else {
        return false;
    };
    for section in sections {
        if let Some(content) = section.get_mut("content")
            && f(content)
        {
            return true;
        }
    }
    false
}

fn with_node_mut(
    value: &mut Value,
    id: &str,
    f: impl FnOnce(&mut Value) -> Result<(), UiIrError>,
) -> Result<(), UiIrError> {
    let mut f = Some(f);
    let mut outcome = None;
    let found = for_each_section_content_mut(value, |content| {
        if let Some(node) = find_node_mut(content, id) {
            if let Some(callback) = f.take() {
                outcome = Some(callback(node));
            }
            true
        } else {
            false
        }
    });
    if !found {
        return Err(UiIrError::UnknownNodeId { id: id.into() });
    }
    outcome.ok_or_else(|| UiIrError::InvalidPatch {
        message: format!("node {id:?} was matched more than once"),
    })?
}

fn apply_patch_op(tree: &mut Value, op: &crate::session::PatchOp) -> Result<(), UiIrError> {
    use crate::session::PatchOp;
    match op {
        PatchOp::Set {
            id,
            property,
            value,
        } => with_node_mut(tree, id, |node| {
            let object = node
                .as_object_mut()
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: "node is not an object".into(),
                })?;
            if !object.contains_key(property) {
                return Err(UiIrError::InvalidPatch {
                    message: format!("unknown property {property:?} for node {id:?}"),
                });
            }
            object.insert(property.clone(), value.clone());
            Ok(())
        }),
        PatchOp::Replace { id, node } => with_node_mut(tree, id, |target| {
            if node.get("kind").is_none() {
                return Err(UiIrError::InvalidPatch {
                    message: "replacement node needs a kind".into(),
                });
            }
            *target = node.clone();
            Ok(())
        }),
        PatchOp::Insert {
            parent_id,
            index,
            node,
        } => with_node_mut(tree, parent_id, |parent| {
            if node.get("kind").is_none() {
                return Err(UiIrError::InvalidPatch {
                    message: "inserted node needs a kind".into(),
                });
            }
            let children = parent
                .get_mut("children")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: format!("node {parent_id:?} cannot contain children"),
                })?;
            if *index > children.len() {
                return Err(UiIrError::InvalidPatch {
                    message: "insert index is out of bounds".into(),
                });
            }
            children.insert(*index, node.clone());
            Ok(())
        }),
        PatchOp::Remove { id } => {
            let mut removed = false;
            for_each_section_content_mut(tree, |content| {
                removed = remove_child(content, id);
                removed
            });
            if removed {
                Ok(())
            } else {
                Err(UiIrError::UnknownNodeId { id: id.clone() })
            }
        }
        PatchOp::Reorder {
            parent_id,
            child_ids,
        } => with_node_mut(tree, parent_id, |parent| {
            let children = parent
                .get_mut("children")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: format!("node {parent_id:?} cannot contain children"),
                })?;
            if children.len() != child_ids.len() {
                return Err(UiIrError::InvalidPatch {
                    message: "reorder must name every child exactly once".into(),
                });
            }
            let mut ordered = Vec::with_capacity(children.len());
            for id in child_ids {
                let index = children
                    .iter()
                    .position(|child| node_id(child) == Some(id.as_str()))
                    .ok_or_else(|| UiIrError::UnknownNodeId { id: id.clone() })?;
                ordered.push(children[index].clone());
            }
            *children = ordered;
            Ok(())
        }),
        PatchOp::ReplaceMeshGeometry {
            plot_id, geometry, ..
        } => with_node_mut(tree, plot_id, |node| {
            if node.get("kind").and_then(Value::as_str) != Some("mesh_plot") {
                return Err(UiIrError::InvalidPatch {
                    message: format!("node {plot_id:?} is not a mesh_plot"),
                });
            }
            node.get_mut("spec")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: "mesh_plot spec is not an object".into(),
                })?
                .insert("geometry".into(), geometry.clone());
            Ok(())
        }),
        PatchOp::ReplaceMeshField { plot_id, field, .. } => with_node_mut(tree, plot_id, |node| {
            if node.get("kind").and_then(Value::as_str) != Some("mesh_plot") {
                return Err(UiIrError::InvalidPatch {
                    message: format!("node {plot_id:?} is not a mesh_plot"),
                });
            }
            node.get_mut("spec")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: "mesh_plot spec is not an object".into(),
                })?
                .insert("field".into(), field.clone());
            Ok(())
        }),
        PatchOp::SetMeshPlotProp {
            plot_id,
            property,
            value,
            ..
        } => with_node_mut(tree, plot_id, |node| {
            if node.get("kind").and_then(Value::as_str) != Some("mesh_plot") {
                return Err(UiIrError::InvalidPatch {
                    message: format!("node {plot_id:?} is not a mesh_plot"),
                });
            }
            let spec = node
                .get_mut("spec")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: "mesh_plot spec is not an object".into(),
                })?;
            if !matches!(
                property.as_str(),
                "view"
                    | "mode"
                    | "color_scale"
                    | "color_range"
                    | "wireframe"
                    | "title"
                    | "width"
                    | "height"
                    | "selection"
                    | "camera"
                    | "viewport"
                    | "contour_levels"
                    | "equal_aspect"
                    | "axes"
                    | "missing_value_policy"
                    | "revolve"
                    | "interactions"
            ) {
                return Err(UiIrError::InvalidPatch {
                    message: format!("unknown mesh_plot property {property:?}"),
                });
            }
            spec.insert(property.clone(), value.clone());
            Ok(())
        }),
        PatchOp::SetMeshPlotSelection {
            plot_id, selection, ..
        } => with_node_mut(tree, plot_id, |node| {
            mesh_plot_spec_object(node, plot_id)?.insert("selection".into(), selection.clone());
            Ok(())
        }),
        PatchOp::ClearMeshPlotSelection { plot_id, .. } => with_node_mut(tree, plot_id, |node| {
            mesh_plot_spec_object(node, plot_id)?.remove("selection");
            Ok(())
        }),
        PatchOp::SetMeshPlotCamera {
            plot_id, camera, ..
        } => with_node_mut(tree, plot_id, |node| {
            mesh_plot_spec_object(node, plot_id)?.insert("camera".into(), camera.clone());
            Ok(())
        }),
        PatchOp::ResetMeshPlotCamera { plot_id, .. } => with_node_mut(tree, plot_id, |node| {
            mesh_plot_spec_object(node, plot_id)?.remove("camera");
            Ok(())
        }),
        PatchOp::SetMeshPlotViewport {
            plot_id, viewport, ..
        } => with_node_mut(tree, plot_id, |node| {
            mesh_plot_spec_object(node, plot_id)?.insert("viewport".into(), viewport.clone());
            Ok(())
        }),
        PatchOp::ResetMeshPlotViewport { plot_id, .. } => with_node_mut(tree, plot_id, |node| {
            mesh_plot_spec_object(node, plot_id)?.remove("viewport");
            Ok(())
        }),
    }
}

fn mesh_plot_spec_object<'a>(
    node: &'a mut Value,
    plot_id: &str,
) -> Result<&'a mut serde_json::Map<String, Value>, UiIrError> {
    if node.get("kind").and_then(Value::as_str) != Some("mesh_plot") {
        return Err(UiIrError::InvalidPatch {
            message: format!("node {plot_id:?} is not a mesh_plot"),
        });
    }
    node.get_mut("spec")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| UiIrError::InvalidPatch {
            message: "mesh_plot spec is not an object".into(),
        })
}

fn remove_child(value: &mut Value, id: &str) -> bool {
    let Some(children) = value.get_mut("children").and_then(Value::as_array_mut) else {
        return false;
    };
    if let Some(index) = children.iter().position(|child| node_id(child) == Some(id)) {
        children.remove(index);
        return true;
    }
    children.iter_mut().any(|child| remove_child(child, id))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSection {
    pub id: String,
    pub label: String,
    pub content: UiNode,
}

impl UiSection {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::EmptySectionId {
                section: self.label.clone(),
            });
        }
        self.content.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
// This is the public, serde-tagged wire model. Boxing only the largest variants
// would add per-node allocations and change the Rust construction API while
// leaving the JSON contract unchanged; retain the explicit inline layout.
#[allow(clippy::large_enum_variant)]
pub enum UiNode {
    Vstack(StackNode),
    Hstack(StackNode),
    Wrap(StackNode),
    Heading(TextNode),
    Text(TextNode),
    Code(TextNode),
    SectionHeader(SectionHeaderNode),
    Card(CardNode),
    Form(FormNode),
    Button(ButtonNode),
    Badge(BadgeNode),
    Metric(MetricNode),
    Progress(ProgressNode),
    Spinner(SpinnerNode),
    ThinkingOrb(ThinkingOrbNode),
    Breadcrumbs(BreadcrumbsNode),
    Alert(AlertNode),
    Toast(ToastNode),
    Tooltip(TooltipNode),
    EmptyState(EmptyStateNode),
    Dialog(DialogNode),
    ConfirmDialog(ConfirmDialogNode),
    Menu(MenuNode),
    MenuBar(MenuBarNode),
    ContextMenu(ContextMenuNode),
    Popover(PopoverNode),
    Tabs(TabsNode),
    Stepper(StepperNode),
    Accordion(AccordionNode),
    ListEditor(ListEditorNode),
    Table(TableNode),
    /// Resource-backed table declaration; payload values travel separately.
    TableV2(TableV2Node),
    TextInput(TextInputNode),
    NumberInput(NumberInputNode),
    Slider(SliderNode),
    AudioPotentiometer(AudioControlNode),
    AudioVerticalSlider(AudioControlNode),
    AudioVolumeKnob(AudioControlNode),
    AudioHorizontalMeter(AudioMeterNode),
    AudioLevelMeter(AudioMeterNode),
    AudioSpectrum(AudioSpectrumNode),
    Select(SelectNode),
    ColorPicker(ColorPickerNode),
    PathInput(PathInputNode),
    Checkbox(BooleanInputNode),
    Toggle(BooleanInputNode),
    Divider(SimpleNode),
    Spacer(SimpleNode),
    /// Removed v1 chart document. Deserialization is retained only to return a
    /// Resource-backed gpui-px declaration; payload values travel separately.
    PxChartV2(PxChartV2Node),
    Scene3d(Scene3dNode),
    MeshPlot(MeshPlotNode),
}

impl UiNode {
    fn validate(&self) -> Result<(), UiIrError> {
        match self {
            Self::Vstack(node) | Self::Hstack(node) | Self::Wrap(node) => {
                for child in &node.children {
                    child.validate()?;
                }
                Ok(())
            }
            Self::Card(node) => {
                for child in &node.children {
                    child.validate()?;
                }
                Ok(())
            }
            Self::Form(node) => node.validate(),
            Self::Button(node) => node.validate(),
            Self::Breadcrumbs(node) => node.validate(),
            Self::Alert(node) => node.validate(),
            Self::Toast(node) => node.validate(),
            Self::Tooltip(node) => node.validate(),
            Self::EmptyState(node) => node.validate(),
            Self::Dialog(node) => node.validate(),
            Self::ConfirmDialog(node) => node.validate(),
            Self::Menu(node) => node.validate(),
            Self::MenuBar(node) => node.validate(),
            Self::ContextMenu(node) => node.validate(),
            Self::Popover(node) => node.validate(),
            Self::Tabs(node) => node.validate(),
            Self::Accordion(node) => node.validate(),
            Self::ListEditor(node) => node.validate(),
            Self::Stepper(node) => node.validate(),
            Self::PxChartV2(node) => node.validate(),
            Self::Scene3d(node) => node.validate(),
            Self::MeshPlot(node) => node.validate(),
            Self::TextInput(node) => node.validate(),
            Self::NumberInput(node) => node.validate(),
            Self::Slider(node) => node.validate(),
            Self::ThinkingOrb(node) => node.validate(),
            Self::AudioPotentiometer(node)
            | Self::AudioVerticalSlider(node)
            | Self::AudioVolumeKnob(node) => node.validate(),
            Self::AudioHorizontalMeter(node) | Self::AudioLevelMeter(node) => node.validate(),
            Self::AudioSpectrum(node) => node.validate(),
            Self::Table(node) => node.validate(),
            Self::TableV2(node) => node.validate(),
            Self::Select(node) => node.validate(),
            Self::ColorPicker(node) => node.validate(),
            Self::PathInput(node) => node.validate(),
            Self::Checkbox(node) | Self::Toggle(node) => node.validate(),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimpleNode {
    #[serde(default)]
    pub id: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StackNode {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub children: Vec<UiNode>,
    pub gap: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextNode {
    #[serde(default)]
    pub id: Option<String>,
    pub text: String,
    #[serde(default = "default_tone")]
    pub tone: String,
    pub level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionHeaderNode {
    #[serde(default)]
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CardNode {
    #[serde(default)]
    pub id: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub children: Vec<UiNode>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Application-declared form layout and cross-control validation summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormNode {
    pub id: String,
    pub label: Option<String>,
    #[serde(default)]
    pub children: Vec<UiNode>,
    #[serde(default)]
    pub errors: Vec<FormValidationError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormValidationError {
    pub control_id: String,
    pub message: String,
    #[serde(default = "default_validation_severity")]
    pub severity: String,
}

fn default_validation_severity() -> String {
    "error".into()
}

impl FormNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form id is empty".into(),
            });
        }
        for child in &self.children {
            child.validate()?;
        }
        if self.errors.iter().any(|error| {
            error.control_id.trim().is_empty()
                || error.message.trim().is_empty()
                || !self
                    .children
                    .iter()
                    .any(|child| child_contains_id(child, &error.control_id))
        }) {
            return Err(UiIrError::InvalidPatch {
                message: "form validation errors require an existing control ID and message".into(),
            });
        }
        Ok(())
    }
}

fn child_contains_id(node: &UiNode, target: &str) -> bool {
    match node {
        UiNode::MeshPlot(_) => false,
        UiNode::Vstack(stack) | UiNode::Hstack(stack) | UiNode::Wrap(stack) => stack
            .children
            .iter()
            .any(|child| child_contains_id(child, target)),
        UiNode::Card(card) => card
            .children
            .iter()
            .any(|child| child_contains_id(child, target)),
        UiNode::Form(form) => {
            form.id == target
                || form
                    .children
                    .iter()
                    .any(|child| child_contains_id(child, target))
        }
        UiNode::Accordion(accordion) => accordion.items.iter().any(|item| {
            item.id == target
                || item
                    .children
                    .iter()
                    .any(|child| child_contains_id(child, target))
        }),
        UiNode::Tooltip(tooltip) => child_contains_id(&tooltip.child, target),
        UiNode::EmptyState(empty) => empty
            .action
            .as_ref()
            .is_some_and(|action| child_contains_id(action, target)),
        UiNode::Dialog(dialog) => dialog
            .content
            .iter()
            .chain(dialog.footer.iter())
            .any(|child| child_contains_id(child, target)),
        UiNode::Popover(popover) => {
            child_contains_id(&popover.trigger, target)
                || popover
                    .content
                    .iter()
                    .any(|child| child_contains_id(child, target))
        }
        UiNode::MenuBar(menu_bar) => menu_bar.items.iter().any(|item| {
            item.id == target || item.items.iter().any(|menu_item| menu_item.id == target)
        }),
        UiNode::TextInput(input) => input.id == target,
        UiNode::NumberInput(input) => input.id == target,
        UiNode::Slider(slider) => slider.id == target,
        UiNode::Select(select) => select.id == target,
        UiNode::PathInput(input) => input.id == target,
        UiNode::Checkbox(input) | UiNode::Toggle(input) => input.id == target,
        UiNode::ListEditor(list) => list.id == target,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonNode {
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    pub action: Option<String>,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub disabled: bool,
}

impl ButtonNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.label.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "button label is empty".into(),
            });
        }
        // `select:<section>` is the legacy host-navigation shorthand. Every
        // Python-directed click is correlated through an application ID.
        if self
            .action
            .as_deref()
            .is_some_and(|action| !action.starts_with("select:"))
            && self.id.as_deref().is_none_or(str::is_empty)
        {
            return Err(UiIrError::InvalidPatch {
                message: "interactive button requires a stable id".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadgeNode {
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    #[serde(default = "default_tone")]
    pub tone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricNode {
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressNode {
    #[serde(default)]
    pub id: Option<String>,
    pub value: f32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpinnerNode {
    #[serde(default)]
    pub id: Option<String>,
    pub label: Option<String>,
}

fn default_thinking_orb_size() -> f32 {
    96.0
}

fn default_thinking_orb_points() -> f32 {
    256.0
}

fn default_thinking_orb_speed() -> f32 {
    0.5
}

fn default_thinking_orb_dot_scale() -> f64 {
    1.0
}

fn default_thinking_orb_dot_color() -> String {
    "#60a5fa".into()
}

/// Animated dotted-sphere status indicator rendered by the native
/// `gpui-ui-kit` ThinkingOrb component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingOrbNode {
    pub id: String,
    pub state: String,
    #[serde(default = "default_thinking_orb_size")]
    pub size: f32,
    #[serde(default = "default_thinking_orb_points")]
    pub points_per_sphere: f32,
    #[serde(default = "default_thinking_orb_speed")]
    pub speed: f32,
    #[serde(default = "default_thinking_orb_dot_scale")]
    pub dot_scale: f64,
    #[serde(default = "default_thinking_orb_dot_color")]
    pub dot_color: String,
    #[serde(default)]
    pub paused: bool,
    pub aria_label: Option<String>,
}

impl ThinkingOrbNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "thinking orb id must not be empty".into(),
            });
        }
        if !matches!(
            self.state.as_str(),
            "working"
                | "searching"
                | "solving"
                | "listening"
                | "connecting"
                | "weaving"
                | "composing"
                | "breathing"
                | "shaping"
        ) {
            return Err(UiIrError::InvalidPatch {
                message: format!("unknown thinking orb state {:?}", self.state),
            });
        }
        if !self.size.is_finite()
            || self.size <= 0.0
            || !self.points_per_sphere.is_finite()
            || self.points_per_sphere <= 0.0
            || !self.speed.is_finite()
            || self.speed < 0.0
            || !self.dot_scale.is_finite()
            || self.dot_scale <= 0.0
        {
            return Err(UiIrError::InvalidPatch {
                message: format!("thinking orb {:?} has invalid numeric properties", self.id),
            });
        }
        let color = self.dot_color.strip_prefix('#').unwrap_or(&self.dot_color);
        if !matches!(color.len(), 6 | 8) || !color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(UiIrError::InvalidPatch {
                message: "thinking orb dot_color must be #RRGGBB or #RRGGBBAA".into(),
            });
        }
        Ok(())
    }
}

/// Native navigation breadcrumbs.  Items use application-stable IDs so the
/// host can return semantic navigation events without exposing pointer data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreadcrumbItemNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub href: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreadcrumbsNode {
    pub id: String,
    #[serde(default)]
    pub items: Vec<BreadcrumbItemNode>,
    #[serde(default = "default_breadcrumb_separator")]
    pub separator: String,
    #[serde(default)]
    pub action: Option<String>,
}

fn default_breadcrumb_separator() -> String {
    "chevron".into()
}

impl BreadcrumbsNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() || self.items.is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "breadcrumbs require an id and at least one item".into(),
            });
        }
        if !matches!(self.separator.as_str(), "slash" | "chevron" | "dot") {
            return Err(UiIrError::InvalidPatch {
                message: "breadcrumbs have an unsupported separator".into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        if self.items.iter().any(|item| {
            item.id.trim().is_empty() || item.label.trim().is_empty() || !ids.insert(&item.id)
        }) {
            return Err(UiIrError::InvalidPatch {
                message: "breadcrumb item IDs and labels must be unique and non-empty".into(),
            });
        }
        Ok(())
    }
}

/// Native contextual feedback. Close events are correlated to the declared
/// alert ID and never leak raw window handles into the Python session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertNode {
    pub id: String,
    pub message: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_alert_variant")]
    pub variant: String,
    #[serde(default)]
    pub closeable: bool,
    #[serde(default)]
    pub action: Option<String>,
}

fn default_alert_variant() -> String {
    "info".into()
}

impl AlertNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() || self.message.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "alert requires a stable id and non-empty message".into(),
            });
        }
        if !matches!(
            self.variant.as_str(),
            "info" | "success" | "warning" | "error"
        ) {
            return Err(UiIrError::InvalidPatch {
                message: "alert has an unsupported variant".into(),
            });
        }
        if self.action.is_some() && !self.closeable {
            return Err(UiIrError::InvalidPatch {
                message: "an alert action requires a closeable alert".into(),
            });
        }
        Ok(())
    }
}

/// A native non-blocking feedback item. The host owns visual rendering and
/// accessibility announcement; Python only receives an explicit close event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToastNode {
    pub id: String,
    pub message: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_alert_variant")]
    pub variant: String,
    #[serde(default = "default_true")]
    pub closeable: bool,
    #[serde(default)]
    pub duration_secs: Option<f32>,
    #[serde(default)]
    pub action: Option<String>,
}

fn default_true() -> bool {
    true
}

impl ToastNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() || self.message.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "toast requires a stable id and non-empty message".into(),
            });
        }
        if !matches!(
            self.variant.as_str(),
            "info" | "success" | "warning" | "error"
        ) {
            return Err(UiIrError::InvalidPatch {
                message: "toast has an unsupported variant".into(),
            });
        }
        if self
            .duration_secs
            .is_some_and(|duration| !duration.is_finite() || duration <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "toast duration must be positive and finite".into(),
            });
        }
        if self.action.is_some() && !self.closeable {
            return Err(UiIrError::InvalidPatch {
                message: "a toast action requires a closeable toast".into(),
            });
        }
        Ok(())
    }
}

/// Native hover/focus tooltip around exactly one retained child. The host owns
/// timing and placement, so pointer-rate hover state never crosses the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TooltipNode {
    pub id: String,
    pub content: String,
    #[serde(default = "default_tooltip_placement")]
    pub placement: String,
    #[serde(default = "default_tooltip_delay")]
    pub delay_ms: u32,
    #[serde(default)]
    pub show: Option<bool>,
    pub child: Box<UiNode>,
}

fn default_tooltip_placement() -> String {
    "top".into()
}

fn default_tooltip_delay() -> u32 {
    200
}

impl TooltipNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() || self.content.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "tooltip requires a stable id and non-empty content".into(),
            });
        }
        if !matches!(self.placement.as_str(), "top" | "bottom" | "left" | "right") {
            return Err(UiIrError::InvalidPatch {
                message: "tooltip has an unsupported placement".into(),
            });
        }
        self.child.validate()
    }
}

/// Host-rendered empty-state presentation, optionally with one declarative
/// action element such as a Python-directed button.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmptyStateNode {
    #[serde(default)]
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub action: Option<Box<UiNode>>,
}

impl EmptyStateNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.title.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "empty state title is empty".into(),
            });
        }
        if let Some(action) = &self.action {
            action.validate()?;
        }
        Ok(())
    }
}

/// Retained native dialog with typed content/footer slots. Backdrop, escape,
/// focus restoration and modal accessibility are owned by gpui-ui-kit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogNode {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_dialog_size")]
    pub size: String,
    #[serde(default)]
    pub content: Vec<UiNode>,
    #[serde(default)]
    pub footer: Vec<UiNode>,
    #[serde(default = "default_true")]
    pub show_close_button: bool,
    #[serde(default = "default_true")]
    pub close_on_backdrop: bool,
    #[serde(default)]
    pub close_action: Option<String>,
}

fn default_dialog_size() -> String {
    "md".into()
}

impl DialogNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "dialog id is empty".into(),
            });
        }
        if !matches!(self.size.as_str(), "sm" | "md" | "lg" | "xl" | "full") {
            return Err(UiIrError::InvalidPatch {
                message: "dialog has an unsupported size".into(),
            });
        }
        for child in self.content.iter().chain(self.footer.iter()) {
            child.validate()?;
        }
        Ok(())
    }
}

/// Native confirmation dialog; keyboard dismissal and focus restoration remain
/// host-owned while Python receives typed confirm or cancel actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfirmDialogNode {
    pub id: String,
    pub message: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_confirm_variant")]
    pub variant: String,
    #[serde(default = "default_confirm_label")]
    pub confirm_label: String,
    #[serde(default = "default_cancel_label")]
    pub cancel_label: String,
    #[serde(default)]
    pub confirm_action: Option<String>,
    #[serde(default)]
    pub cancel_action: Option<String>,
}

fn default_confirm_variant() -> String {
    "default".into()
}
fn default_confirm_label() -> String {
    "Confirm".into()
}
fn default_cancel_label() -> String {
    "Cancel".into()
}

impl ConfirmDialogNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || self.message.trim().is_empty()
            || self.confirm_label.trim().is_empty()
            || self.cancel_label.trim().is_empty()
            || !matches!(self.variant.as_str(), "default" | "destructive" | "warning")
        {
            return Err(UiIrError::InvalidPatch {
                message:
                    "confirmation dialog requires an ID, message, labels, and supported variant"
                        .into(),
            });
        }
        Ok(())
    }
}

/// Inline native menu with semantic selection and normalized keyboard events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuNode {
    pub id: String,
    #[serde(default)]
    pub items: Vec<MenuItemNode>,
    #[serde(default = "default_context_menu_width")]
    pub min_width: f32,
    #[serde(default)]
    pub focused_index: Option<usize>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub close_action: Option<String>,
    #[serde(default)]
    pub focus_action: Option<String>,
}

impl MenuNode {
    fn validate(&self) -> Result<(), UiIrError> {
        ContextMenuNode {
            id: self.id.clone(),
            items: self.items.clone(),
            position: [0.0, 0.0],
            min_width: self.min_width,
            focused_index: self.focused_index,
            action: self.action.clone(),
            close_action: self.close_action.clone(),
            focus_action: self.focus_action.clone(),
        }
        .validate()
    }
}

/// Typed top-level menu bar and child menus. The active menu is application
/// state; host pointer and keyboard interactions emit semantic IDs only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuBarItemNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub items: Vec<MenuItemNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuBarNode {
    pub id: String,
    #[serde(default)]
    pub items: Vec<MenuBarItemNode>,
    #[serde(default)]
    pub active_menu: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub toggle_action: Option<String>,
}

impl MenuBarNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() || self.items.is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "menu bar requires an ID and at least one menu".into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.items {
            if item.id.trim().is_empty() || item.label.trim().is_empty() || !ids.insert(&item.id) {
                return Err(UiIrError::InvalidPatch {
                    message: "menu bar IDs and labels must be unique and non-empty".into(),
                });
            }
            let mut item_ids = std::collections::HashSet::new();
            for menu_item in &item.items {
                menu_item.validate()?;
                if !menu_item.separator && !item_ids.insert(&menu_item.id) {
                    return Err(UiIrError::InvalidPatch {
                        message: "menu sibling IDs must be unique".into(),
                    });
                }
            }
        }
        if self
            .active_menu
            .as_ref()
            .is_some_and(|active| !ids.contains(active))
        {
            return Err(UiIrError::InvalidPatch {
                message: "active menu is not declared by this menu bar".into(),
            });
        }
        Ok(())
    }
}

/// One semantic item in a native menu or context menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItemNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub checkbox: bool,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub danger: bool,
    #[serde(default)]
    pub separator: bool,
    #[serde(default)]
    pub children: Vec<MenuItemNode>,
}

impl MenuItemNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.separator {
            if !self.id.is_empty() || !self.label.is_empty() || !self.children.is_empty() {
                return Err(UiIrError::InvalidPatch {
                    message: "menu separators cannot have an ID, label, or children".into(),
                });
            }
            return Ok(());
        }
        if self.id.trim().is_empty() || self.label.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "menu items require a stable ID and label".into(),
            });
        }
        if self.checked && !self.checkbox {
            return Err(UiIrError::InvalidPatch {
                message: "only checkbox menu items can be checked".into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for child in &self.children {
            child.validate()?;
            if !child.separator && !ids.insert(&child.id) {
                return Err(UiIrError::InvalidPatch {
                    message: "menu sibling IDs must be unique".into(),
                });
            }
        }
        Ok(())
    }
}

/// Retained host-native context menu. Pointer coordinates and focus stay in
/// Rust; Python receives only semantic item, close, and focus events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextMenuNode {
    pub id: String,
    #[serde(default)]
    pub items: Vec<MenuItemNode>,
    #[serde(default)]
    pub position: [f32; 2],
    #[serde(default = "default_context_menu_width")]
    pub min_width: f32,
    #[serde(default)]
    pub focused_index: Option<usize>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub close_action: Option<String>,
    #[serde(default)]
    pub focus_action: Option<String>,
}

fn default_context_menu_width() -> f32 {
    180.0
}

impl ContextMenuNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || self.items.is_empty()
            || !self.min_width.is_finite()
            || self.min_width <= 0.0
            || self.position.iter().any(|value| !value.is_finite())
        {
            return Err(UiIrError::InvalidPatch {
                message: "context menu requires an ID, items, finite position, and positive width"
                    .into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.items {
            item.validate()?;
            if !item.separator && !ids.insert(&item.id) {
                return Err(UiIrError::InvalidPatch {
                    message: "menu sibling IDs must be unique".into(),
                });
            }
        }
        if self
            .focused_index
            .is_some_and(|index| index >= self.items.len())
        {
            return Err(UiIrError::InvalidPatch {
                message: "context menu focused index is out of range".into(),
            });
        }
        Ok(())
    }
}

/// Anchored native popover with explicit trigger and content slots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PopoverNode {
    pub id: String,
    pub trigger: Box<UiNode>,
    #[serde(default)]
    pub content: Vec<UiNode>,
    #[serde(default = "default_popover_placement")]
    pub placement: String,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default = "default_true")]
    pub show_backdrop: bool,
    #[serde(default)]
    pub close_action: Option<String>,
}

fn default_popover_placement() -> String {
    "bottom".into()
}

impl PopoverNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || !matches!(
                self.placement.as_str(),
                "top"
                    | "bottom"
                    | "left"
                    | "right"
                    | "top_start"
                    | "top_end"
                    | "bottom_start"
                    | "bottom_end"
            )
            || self
                .width
                .is_some_and(|width| !width.is_finite() || width <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "popover requires an ID, supported placement, and positive finite width"
                    .into(),
            });
        }
        self.trigger.validate()?;
        for child in &self.content {
            child.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabsNode {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub active: usize,
    pub action: Option<String>,
}

impl TabsNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.items.iter().any(|item| item.trim().is_empty())
            || (!self.items.is_empty() && self.active >= self.items.len())
        {
            return Err(UiIrError::InvalidPatch {
                message: "tabs have an invalid active index or empty label".into(),
            });
        }
        if self.action.is_some() && self.id.as_deref().is_none_or(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "interactive tabs require a stable id".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepperNode {
    pub id: String,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub active: usize,
    #[serde(default)]
    pub disabled_steps: Vec<usize>,
    pub action: Option<String>,
}

impl StepperNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() || self.steps.is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "stepper requires an id and at least one step".into(),
            });
        }
        if self.active >= self.steps.len()
            || self.steps.iter().any(|step| step.trim().is_empty())
            || self
                .disabled_steps
                .iter()
                .any(|index| *index >= self.steps.len())
        {
            return Err(UiIrError::InvalidPatch {
                message: "stepper has invalid active, label, or disabled step".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccordionItemNode {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub children: Vec<UiNode>,
    #[serde(default)]
    pub disabled: bool,
    pub trailing: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccordionNode {
    pub id: String,
    #[serde(default)]
    pub items: Vec<AccordionItemNode>,
    #[serde(default)]
    pub expanded: Vec<String>,
    #[serde(default)]
    pub multiple: bool,
    pub action: Option<String>,
}

impl AccordionNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "accordion id is empty".into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.items {
            if item.id.trim().is_empty() || item.title.trim().is_empty() || !ids.insert(&item.id) {
                return Err(UiIrError::InvalidPatch {
                    message: "accordion item IDs and titles must be unique and non-empty".into(),
                });
            }
            for child in &item.children {
                child.validate()?;
            }
        }
        if !self.multiple && self.expanded.len() > 1 {
            return Err(UiIrError::InvalidPatch {
                message: "single accordion may only expand one item".into(),
            });
        }
        if self.expanded.iter().any(|id| !ids.contains(id)) {
            return Err(UiIrError::InvalidPatch {
                message: "accordion expanded item does not exist".into(),
            });
        }
        Ok(())
    }
}

/// Reorderable application-owned rows, for frequency/evaluation-point editors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListEditorNode {
    pub id: String,
    pub label: Option<String>,
    #[serde(default)]
    pub rows: Vec<ListEditorRow>,
    pub add_action: Option<String>,
    pub remove_action: Option<String>,
    pub reorder_action: Option<String>,
    pub add_label: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListEditorRow {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub value: Value,
    pub validation: Option<ValidationState>,
    #[serde(default)]
    pub disabled: bool,
}

impl ListEditorNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "list editor id is empty".into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for row in &self.rows {
            if row.id.trim().is_empty() || row.label.trim().is_empty() || !ids.insert(&row.id) {
                return Err(UiIrError::InvalidPatch {
                    message: "list editor rows need unique non-empty IDs and labels".into(),
                });
            }
        }
        Ok(())
    }
}

/// Presentation-neutral validation shown by native form controls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationState {
    pub severity: String,
    pub message: String,
}

/// Common declarative form presentation. Values remain Python-owned; the
/// optional default is metadata for reset/migration UIs, not host state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormControlProps {
    pub help: Option<String>,
    pub default_value: Option<Value>,
    #[serde(default = "default_visible")]
    pub visible: bool,
    pub width: Option<f32>,
}

fn default_visible() -> bool {
    true
}

impl FormControlProps {
    fn validate(&self) -> Result<(), UiIrError> {
        if self
            .width
            .is_some_and(|width| !width.is_finite() || width <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "form control width must be positive and finite".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextInputNode {
    pub id: String,
    #[serde(default)]
    pub value: String,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub action: Option<String>,
    pub commit_action: Option<String>,
    pub selection_action: Option<String>,
    #[serde(default)]
    pub password: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub required: bool,
    pub validation: Option<ValidationState>,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

impl TextInputNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form node id is empty".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumberInputNode {
    pub id: String,
    /// A string is accepted for invalid intermediate editor text, while JSON
    /// numbers provide the normal committed representation.
    pub value: Value,
    pub label: Option<String>,
    pub unit: Option<String>,
    #[serde(rename = "min")]
    pub minimum: Option<f64>,
    #[serde(rename = "max")]
    pub maximum: Option<f64>,
    pub step: Option<f64>,
    pub precision: Option<u8>,
    pub action: Option<String>,
    pub commit_action: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub required: bool,
    pub validation: Option<ValidationState>,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

impl NumberInputNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form node id is empty".into(),
            });
        }
        if let (Some(minimum), Some(maximum)) = (self.minimum, self.maximum)
            && minimum > maximum
        {
            return Err(UiIrError::InvalidPatch {
                message: "number input minimum exceeds maximum".into(),
            });
        }
        if self
            .step
            .is_some_and(|step| !step.is_finite() || step <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "number input step must be positive and finite".into(),
            });
        }
        Ok(())
    }
}

/// A continuous numeric control. `action` is emitted while dragging and
/// `commit_action` is emitted once when the pointer is released.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SliderNode {
    pub id: String,
    pub value: f32,
    pub label: Option<String>,
    #[serde(rename = "min")]
    pub minimum: f32,
    #[serde(rename = "max")]
    pub maximum: f32,
    pub step: Option<f32>,
    pub action: Option<String>,
    pub commit_action: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub show_value: bool,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

/// Shared declaration for native audio controls. Pointer-rate interaction
/// stays in Rust; `action` emits previews and `commit_action` emits releases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioControlNode {
    pub id: String,
    pub value: f64,
    #[serde(rename = "min", default)]
    pub minimum: f64,
    #[serde(rename = "max", default = "default_audio_maximum")]
    pub maximum: f64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default = "default_audio_control_size")]
    pub size: String,
    #[serde(default = "default_audio_scale")]
    pub scale: String,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub muted: bool,
    pub peak: Option<f64>,
    #[serde(default)]
    pub with_ticks: bool,
    pub action: Option<String>,
    pub commit_action: Option<String>,
    pub mute_action: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub aria_label: Option<String>,
}

fn default_audio_maximum() -> f64 {
    1.0
}

fn default_audio_control_size() -> String {
    "md".into()
}

fn default_audio_scale() -> String {
    "linear".into()
}

impl AudioControlNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || !self.value.is_finite()
            || !self.minimum.is_finite()
            || !self.maximum.is_finite()
            || self.maximum <= self.minimum
            || self.value < self.minimum
            || self.value > self.maximum
            || self.peak.is_some_and(|value| !value.is_finite())
            || self
                .width
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            || self
                .height
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: format!("audio control {:?} has invalid bounds or value", self.id),
            });
        }
        if !matches!(self.size.as_str(), "xs" | "sm" | "md" | "lg")
            || !matches!(self.scale.as_str(), "linear" | "logarithmic")
            || self.action.as_deref().is_some_and(str::is_empty)
            || self.commit_action.as_deref().is_some_and(str::is_empty)
            || self.mute_action.as_deref().is_some_and(str::is_empty)
        {
            return Err(UiIrError::InvalidPatch {
                message: format!("audio control {:?} has invalid options", self.id),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMeterNode {
    pub id: String,
    #[serde(default)]
    pub levels: Vec<f64>,
    #[serde(default)]
    pub peaks: Vec<f64>,
    #[serde(default)]
    pub channel_names: Vec<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    /// Optional host-owned binary stream; embedded values remain a first-frame fallback.
    pub stream_id: Option<String>,
}

impl AudioMeterNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || (self.levels.is_empty() && self.stream_id.is_none())
            || self.stream_id.as_deref().is_some_and(str::is_empty)
            || self.levels.iter().any(|value| !value.is_finite())
            || self.peaks.iter().any(|value| !value.is_finite())
            || (!self.peaks.is_empty() && self.peaks.len() != self.levels.len())
            || (!self.channel_names.is_empty()
                && !self.levels.is_empty()
                && self.channel_names.len() != self.levels.len())
            || self.channel_names.len() > crate::audio_stream::MAX_METER_CHANNELS
            || self
                .width
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            || self
                .height
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: format!("audio meter {:?} has invalid channels", self.id),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSpectrumNode {
    pub id: String,
    #[serde(default)]
    pub magnitudes: Vec<f32>,
    #[serde(default)]
    pub previous: Vec<f32>,
    #[serde(default = "default_spectrum_minimum")]
    pub minimum_frequency: f32,
    #[serde(default = "default_spectrum_maximum")]
    pub maximum_frequency: f32,
    #[serde(default = "default_spectrum_smoothing")]
    pub smoothing: f32,
    pub height: Option<f32>,
    pub bar_gap: Option<f32>,
    /// Optional host-owned binary stream; embedded values remain a first-frame fallback.
    pub stream_id: Option<String>,
}

fn default_spectrum_minimum() -> f32 {
    20.0
}
fn default_spectrum_maximum() -> f32 {
    20_000.0
}
fn default_spectrum_smoothing() -> f32 {
    0.8
}

impl AudioSpectrumNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || (self.magnitudes.is_empty() && self.stream_id.is_none())
            || self.stream_id.as_deref().is_some_and(str::is_empty)
            || self.magnitudes.iter().any(|value| !value.is_finite())
            || self.previous.iter().any(|value| !value.is_finite())
            || (!self.previous.is_empty() && self.previous.len() != self.magnitudes.len())
            || !self.minimum_frequency.is_finite()
            || !self.maximum_frequency.is_finite()
            || self.minimum_frequency <= 0.0
            || self.maximum_frequency <= self.minimum_frequency
            || !self.smoothing.is_finite()
            || !(0.0..=1.0).contains(&self.smoothing)
            || self
                .height
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            || self
                .bar_gap
                .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: format!("audio spectrum {:?} has invalid data", self.id),
            });
        }
        Ok(())
    }
}

impl SliderNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "slider id is empty".into(),
            });
        }
        if !self.value.is_finite() || !self.minimum.is_finite() || !self.maximum.is_finite() {
            return Err(UiIrError::InvalidPatch {
                message: "slider values must be finite".into(),
            });
        }
        if self.minimum > self.maximum {
            return Err(UiIrError::InvalidPatch {
                message: "slider minimum exceeds maximum".into(),
            });
        }
        if self
            .step
            .is_some_and(|step| !step.is_finite() || step <= 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "slider step must be positive and finite".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: Value,
    pub label: String,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectNode {
    pub id: String,
    pub value: Value,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    pub label: Option<String>,
    pub action: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

/// Native RGB/HSL color editor. The value is a CSS-style 6/8 digit hex
/// string, while user interaction remains in the host's ColorPickerView.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorPickerNode {
    pub id: String,
    pub value: String,
    pub label: Option<String>,
    pub action: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

impl ColorPickerNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form node id is empty".into(),
            });
        }
        let value = self.value.strip_prefix('#').unwrap_or(&self.value);
        if !matches!(value.len(), 6 | 8) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(UiIrError::InvalidPatch {
                message: "color picker value must be #RRGGBB or #RRGGBBAA".into(),
            });
        }
        Ok(())
    }
}

/// A file-system path editor. The application remains authoritative for
/// domain validation; `must_exist` provides a useful native preflight for the
/// common open-file/open-directory cases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathInputNode {
    pub id: String,
    #[serde(default)]
    pub value: String,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    #[serde(default = "default_path_input_mode")]
    pub mode: String,
    #[serde(default)]
    pub filters: Vec<PathFilter>,
    #[serde(default)]
    pub recent_values: Vec<String>,
    #[serde(default)]
    pub must_exist: bool,
    pub action: Option<String>,
    pub commit_action: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub required: bool,
    pub validation: Option<ValidationState>,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathFilter {
    pub label: String,
    #[serde(default)]
    pub extensions: Vec<String>,
}

fn default_path_input_mode() -> String {
    "open_file".into()
}

impl PathInputNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form node id is empty".into(),
            });
        }
        if !matches!(self.mode.as_str(), "open_file" | "directory" | "save_file") {
            return Err(UiIrError::InvalidPatch {
                message: format!("path input mode {:?} is unsupported", self.mode),
            });
        }
        if self.filters.iter().any(|filter| {
            filter.label.trim().is_empty()
                || filter
                    .extensions
                    .iter()
                    .any(|extension| extension.trim().is_empty())
        }) {
            return Err(UiIrError::InvalidPatch {
                message: "path input filters need non-empty labels and extensions".into(),
            });
        }
        Ok(())
    }
}

impl SelectNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form node id is empty".into(),
            });
        }
        if self.options.is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "select needs at least one option".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanInputNode {
    pub id: String,
    pub value: bool,
    #[serde(default)]
    pub indeterminate: bool,
    pub label: String,
    pub action: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(flatten)]
    pub presentation: FormControlProps,
}

impl BooleanInputNode {
    fn validate(&self) -> Result<(), UiIrError> {
        self.presentation.validate()?;
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "form node id is empty".into(),
            });
        }
        if self.indeterminate && self.label.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "indeterminate checkbox needs a label".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableNode {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
    #[serde(default)]
    pub columns: Vec<TableColumn>,
    #[serde(default)]
    pub typed_rows: Vec<TableRow>,
    pub selected_row: Option<String>,
    pub selection_action: Option<String>,
    pub row_action: Option<String>,
    /// Application action notified when a stable table column is resized.
    pub resize_action: Option<String>,
    /// Application action invoked when a sortable column header is activated.
    pub sort_action: Option<String>,
    pub sort_column: Option<String>,
    #[serde(default = "default_sort_direction")]
    pub sort_direction: String,
    #[serde(default)]
    pub row_offset: usize,
    /// Application-controlled virtual window size. The renderer caps omitted
    /// values to a conservative default rather than rebuilding unbounded rows.
    pub row_limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableColumn {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub sortable: bool,
    pub width: Option<f32>,
    #[serde(default)]
    pub pinned: bool,
}

/// Table metadata bound to a revisioned dataset or dataset view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableV2Node {
    pub id: String,
    pub data: Value,
    pub columns: Vec<TableV2Column>,
    #[serde(default)]
    pub selection_mode: String,
    pub virtualize: TableVirtualize,
    #[serde(default)]
    pub selection_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableV2Column {
    pub id: String,
    pub field: String,
    #[serde(default)]
    pub sortable: bool,
    #[serde(default)]
    pub min_width: Option<f32>,
    #[serde(default)]
    pub template: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableVirtualize {
    pub row_height: f32,
    pub overscan: usize,
}

impl TableV2Node {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || self.columns.is_empty()
            || !matches!(self.selection_mode.as_str(), "none" | "single" | "multiple")
            || !self.virtualize.row_height.is_finite()
            || self.virtualize.row_height <= 0.0
        {
            return Err(UiIrError::InvalidPatch {
                message:
                    "resource table requires ID, columns, selection mode, and positive row height"
                        .into(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for column in &self.columns {
            if column.id.trim().is_empty()
                || column.field.trim().is_empty()
                || !ids.insert(&column.id)
                || column
                    .min_width
                    .is_some_and(|width| !width.is_finite() || width <= 0.0)
            {
                return Err(UiIrError::InvalidPatch {
                    message:
                        "resource table columns require unique IDs, fields, and positive widths"
                            .into(),
                });
            }
        }
        if self.selection_action.as_deref().is_some_and(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "resource table selection action cannot be empty".into(),
            });
        }
        if self.selection_mode != "none" {
            let source = match self.data.get("kind").and_then(Value::as_str) {
                Some("dataset") => &self.data,
                Some("dataset_view") => {
                    self.data
                        .get("dataset")
                        .ok_or_else(|| UiIrError::InvalidPatch {
                            message: "dataset view requires dataset descriptor".into(),
                        })?
                }
                _ => {
                    return Err(UiIrError::InvalidPatch {
                        message: "resource table selection requires a keyed dataset".into(),
                    });
                }
            };
            if source
                .get("key")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                return Err(UiIrError::InvalidPatch {
                    message: "resource table selection requires a stable dataset key".into(),
                });
            }
        }
        if self.data.get("kind").and_then(Value::as_str) == Some("array_data") {
            return Err(UiIrError::InvalidPatch {
                message: "resource tables require Dataset or DatasetView; ArrayData is not tabular"
                    .into(),
            });
        }
        validate_resource_source(&self.data)?;
        validate_dataset_view_operations(&self.data, true, true)?;
        validate_dataset_view_projection(
            &self.data,
            self.columns.iter().map(|column| column.field.as_str()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
    pub id: String,
    #[serde(default)]
    pub cells: Vec<Value>,
}

fn default_sort_direction() -> String {
    "ascending".into()
}

impl TableNode {
    fn validate(&self) -> Result<(), UiIrError> {
        let mut column_ids = std::collections::HashSet::new();
        for column in &self.columns {
            if column.id.trim().is_empty()
                || column.label.trim().is_empty()
                || !column_ids.insert(&column.id)
                || column
                    .width
                    .is_some_and(|width| !width.is_finite() || width <= 0.0)
            {
                return Err(UiIrError::InvalidPatch {
                    message: "table columns require unique IDs, labels, and positive finite widths"
                        .into(),
                });
            }
        }
        if !matches!(self.sort_direction.as_str(), "ascending" | "descending") {
            return Err(UiIrError::InvalidPatch {
                message: "table sort direction must be ascending or descending".into(),
            });
        }
        if (self.sort_action.is_some()
            || self.resize_action.is_some()
            || self.selection_action.is_some()
            || self.row_action.is_some())
            && self.id.as_deref().is_none_or(str::is_empty)
        {
            return Err(UiIrError::InvalidPatch {
                message: "interactive table requires a stable table id".into(),
            });
        }
        if self
            .sort_column
            .as_ref()
            .is_some_and(|id| !column_ids.contains(id))
        {
            return Err(UiIrError::InvalidPatch {
                message: "table sort column does not exist".into(),
            });
        }
        Ok(())
    }
}

/// Declarative chart metadata bound to a Dataset, DatasetView, or ArrayData.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PxChartV2Node {
    pub chart: String,
    pub id: String,
    pub data: Value,
    pub lod: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub selection_action: Option<String>,
    #[serde(default)]
    pub viewport_action: Option<String>,
    #[serde(default)]
    pub accessibility_description: Option<String>,
    #[serde(default)]
    pub legend_position: Option<String>,
    #[serde(default)]
    pub annotations: Vec<ChartAnnotationNode>,
    #[serde(default)]
    pub tiling_method: Option<String>,
    #[serde(default)]
    pub padding: Option<f64>,
    #[serde(default)]
    pub color_scale: Option<String>,
    #[serde(default)]
    pub point_radius: Option<f32>,
    #[serde(default)]
    pub x_log: Option<bool>,
    #[serde(default)]
    pub y_log: Option<bool>,
    #[serde(default)]
    pub x_label: Option<String>,
    #[serde(default)]
    pub y_label: Option<String>,
    #[serde(default)]
    pub y2_label: Option<String>,
    #[serde(default)]
    pub z_label: Option<String>,
    #[serde(default)]
    pub x_range: Option<[f64; 2]>,
    #[serde(default)]
    pub y_range: Option<[f64; 2]>,
    #[serde(default)]
    pub y2_range: Option<[f64; 2]>,
    #[serde(default)]
    pub z_range: Option<[f64; 2]>,
    #[serde(default)]
    pub stroke_width: Option<f32>,
    #[serde(default)]
    pub opacity: Option<f32>,
    #[serde(default)]
    pub bar_gap: Option<f32>,
    #[serde(default)]
    pub border_radius: Option<f32>,
    #[serde(default)]
    pub box_color: Option<String>,
    #[serde(default)]
    pub median_color: Option<String>,
    #[serde(default)]
    pub whisker_color: Option<String>,
    #[serde(default)]
    pub outlier_color: Option<String>,
    #[serde(default)]
    pub box_opacity: Option<f32>,
    #[serde(default)]
    pub box_width: Option<f32>,
    #[serde(default)]
    pub outlier_radius: Option<f32>,
    #[serde(default)]
    pub bins: Option<usize>,
    #[serde(default)]
    pub wireframe: Option<bool>,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
    #[serde(default)]
    pub fill: Option<bool>,
    #[serde(default)]
    pub min_width: Option<f32>,
    #[serde(default)]
    pub min_height: Option<f32>,
    #[serde(default)]
    pub aspect_ratio: Option<f32>,
    #[serde(default)]
    pub thresholds: Option<Vec<f64>>,
    #[serde(default)]
    pub levels: Option<Vec<f64>>,
    #[serde(default)]
    pub hole: Option<f64>,
    #[serde(default)]
    pub colors: Option<Vec<String>>,
    #[serde(default)]
    pub hover: Option<bool>,
    #[serde(default)]
    pub renderer_2d: Option<String>,
    #[serde(default)]
    pub vello_backend: Option<String>,
    #[serde(default)]
    pub graph_ratio: Option<f32>,
    #[serde(default)]
    pub hidden_series: Option<Vec<usize>>,
    #[serde(default)]
    pub legend_action: Option<String>,
    #[serde(default)]
    pub fill_color: Option<String>,
    #[serde(default)]
    pub primary_color: Option<String>,
    #[serde(default)]
    pub stroke_color: Option<String>,
    #[serde(default)]
    pub pad_angle: Option<f64>,
    #[serde(default)]
    pub corner_radius: Option<f64>,
    #[serde(default)]
    pub sort: Option<bool>,
    #[serde(default)]
    pub curve: Option<String>,
    #[serde(default)]
    pub dash_style: Option<String>,
    #[serde(default)]
    pub show_points: Option<bool>,
    #[serde(default)]
    pub contour_upsample_factor: Option<usize>,
    #[serde(default)]
    pub smooth_strokes: Option<bool>,
    #[serde(default)]
    pub smoothing_iterations: Option<usize>,
    #[serde(default)]
    pub smoothing_max_deviation_px: Option<f32>,
}

impl PxChartV2Node {
    pub fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty()
            || !matches!(
                self.chart.as_str(),
                "scatter"
                    | "line"
                    | "area"
                    | "box_plot"
                    | "heatmap"
                    | "contour"
                    | "isoline"
                    | "surface"
                    | "pie"
                    | "donut"
                    | "bar"
                    | "treemap"
            )
            || !matches!(self.lod.as_str(), "auto" | "off" | "aggressive")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart requires ID, supported chart kind, and LOD policy".into(),
            });
        }
        if (self.viewport_action.is_some()
            && !matches!(self.chart.as_str(), "scatter" | "line" | "surface"))
            || (self.selection_action.is_some()
                && !matches!(self.chart.as_str(), "treemap" | "scatter" | "line"))
        {
            return Err(UiIrError::InvalidPatch {
                message:
                    "resource chart selection and viewport actions are unavailable in this host"
                        .into(),
            });
        }
        let source = self
            .data
            .get("source")
            .ok_or_else(|| UiIrError::InvalidPatch {
                message: "resource chart requires data binding source".into(),
            })?;
        if matches!(
            self.chart.as_str(),
            "heatmap" | "contour" | "isoline" | "surface"
        ) && source.get("kind").and_then(Value::as_str) != Some("array_data")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource grid charts require ArrayData".into(),
            });
        }
        if source.get("kind").and_then(Value::as_str) != Some("array_data")
            && !matches!(
                self.chart.as_str(),
                "heatmap" | "contour" | "isoline" | "surface"
            )
        {
            let roles = self
                .data
                .get("roles")
                .and_then(Value::as_object)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: "resource chart requires data-binding roles".into(),
                })?;
            let has_role = |name: &str| {
                roles
                    .get(name)
                    .and_then(Value::as_str)
                    .is_some_and(|field| !field.trim().is_empty())
            };
            if (has_role("series") || has_role("color"))
                && !matches!(self.chart.as_str(), "scatter" | "line" | "bar")
            {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart kind does not support series or color roles".into(),
                });
            }
            if has_role("dash") && self.chart != "line" {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart dash role is only supported by line charts".into(),
                });
            }
            if has_role("y0") && self.chart != "area" {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart y0 role is only supported by area charts".into(),
                });
            }
            if has_role("y2") && self.chart != "line" {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart y2 role is only supported by line charts".into(),
                });
            }
            if self.chart == "treemap" {
                if !has_role("row_id") || !has_role("parent") || !has_role("size") {
                    return Err(UiIrError::InvalidPatch {
                        message: "treemap resource chart requires row_id, parent, and size fields"
                            .into(),
                    });
                }
                if source.get("kind").and_then(Value::as_str) != Some("dataset") {
                    return Err(UiIrError::InvalidPatch {
                        message: "treemap resource chart currently requires Dataset".into(),
                    });
                }
            } else if matches!(self.chart.as_str(), "bar" | "pie" | "donut") {
                if !(has_role("label") || has_role("x")) || !has_role("y") {
                    return Err(UiIrError::InvalidPatch {
                        message: "categorical resource chart requires label (or x) and y fields"
                            .into(),
                    });
                }
            } else if !has_role("x") || !has_role("y") {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart requires x and y fields".into(),
                });
            }
        }
        if self.legend_position.as_deref().is_some_and(|position| {
            !matches!(position, "right" | "left" | "top" | "bottom" | "hidden")
        }) {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid legend position".into(),
            });
        }
        if self.tiling_method.as_deref().is_some_and(|method| {
            !matches!(
                method,
                "squarify" | "binary" | "slice" | "dice" | "slice_dice"
            )
        }) || self
            .padding
            .is_some_and(|padding| !padding.is_finite() || padding < 0.0)
            || ((self.tiling_method.is_some() || self.padding.is_some()) && self.chart != "treemap")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid treemap configuration".into(),
            });
        }
        if (self.legend_position.is_some() || !self.annotations.is_empty())
            && !matches!(self.chart.as_str(), "scatter" | "line" | "bar")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart kind does not support legends or annotations".into(),
            });
        }
        if let Some(scale) = self.color_scale.as_deref() {
            let supported = matches!(
                scale,
                "viridis" | "plasma" | "inferno" | "magma" | "heat" | "coolwarm" | "greys"
            );
            let supported_chart = matches!(self.chart.as_str(), "heatmap" | "contour" | "surface");
            let supported_surface = self.chart != "surface"
                || matches!(scale, "viridis" | "plasma" | "inferno" | "coolwarm");
            if !supported || !supported_chart || !supported_surface {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart has unsupported color scale".into(),
                });
            }
        }
        if self
            .point_radius
            .is_some_and(|radius| !radius.is_finite() || radius <= 0.0)
            || (self.point_radius.is_some() && self.chart != "scatter")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart invalid scatter point radius".into(),
            });
        }
        if self.hidden_series.as_ref().is_some_and(|indices| {
            let mut unique = std::collections::HashSet::new();
            indices.iter().any(|index| !unique.insert(*index))
        }) || ((self.hidden_series.is_some() || self.legend_action.is_some())
            && self.chart != "line")
            || self.legend_action.as_deref().is_some_and(str::is_empty)
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid line legend interaction configuration".into(),
            });
        }
        if self
            .graph_ratio
            .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
            || (self.graph_ratio.is_some()
                && !matches!(self.chart.as_str(), "scatter" | "line" | "bar"))
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid graph ratio".into(),
            });
        }
        let valid_range =
            |range: [f64; 2]| range[0].is_finite() && range[1].is_finite() && range[0] < range[1];
        let valid_dimension = |value: f32| value.is_finite() && value > 0.0;
        if self.width.is_some() != self.height.is_some()
            || self.min_width.is_some() != self.min_height.is_some()
            || self.width.is_some_and(|value| !valid_dimension(value))
            || self.height.is_some_and(|value| !valid_dimension(value))
            || self.min_width.is_some_and(|value| !valid_dimension(value))
            || self.min_height.is_some_and(|value| !valid_dimension(value))
            || (self.fill.unwrap_or(false) && self.width.is_some())
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart sizing must use paired positive dimensions and cannot combine fixed size with fill".into(),
            });
        }
        if self.x_range.is_some_and(|range| !valid_range(range))
            || self.y_range.is_some_and(|range| !valid_range(range))
            || self.y2_range.is_some_and(|range| !valid_range(range))
            || (self.x_log.unwrap_or(false) && self.x_range.is_some_and(|range| range[0] <= 0.0))
            || (self.y_log.unwrap_or(false) && self.y_range.is_some_and(|range| range[0] <= 0.0))
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart axis ranges must be finite, increasing, and positive for log axes"
                    .into(),
            });
        }
        if (self.y2_label.is_some() || self.y2_range.is_some()) && self.chart != "line" {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart y2 presentation requires line chart".into(),
            });
        }
        if (self.x_log.is_some() || self.x_label.is_some() || self.x_range.is_some())
            && !matches!(
                self.chart.as_str(),
                "scatter" | "line" | "area" | "surface" | "heatmap" | "contour" | "isoline"
            )
            || (self.y_log.is_some() || self.y_range.is_some())
                && !matches!(
                    self.chart.as_str(),
                    "scatter"
                        | "line"
                        | "area"
                        | "bar"
                        | "surface"
                        | "heatmap"
                        | "contour"
                        | "isoline"
                )
            || (self.x_label.is_some() || self.y_label.is_some())
                && !matches!(self.chart.as_str(), "line" | "surface")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart kind does not support requested axis configuration".into(),
            });
        }
        if self.z_range.is_some_and(|range| {
            !range[0].is_finite() || !range[1].is_finite() || range[0] >= range[1]
        }) || ((self.z_label.is_some() || self.z_range.is_some() || self.wireframe.is_some())
            && self.chart != "surface")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid surface axis configuration".into(),
            });
        }
        if self
            .stroke_width
            .is_some_and(|width| !width.is_finite() || width <= 0.0)
            || self
                .opacity
                .is_some_and(|opacity| !opacity.is_finite() || !(0.0..=1.0).contains(&opacity))
            || self
                .aspect_ratio
                .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
            || self
                .hole
                .is_some_and(|hole| !hole.is_finite() || !(0.0..1.0).contains(&hole))
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid style configuration".into(),
            });
        }
        if self
            .bar_gap
            .is_some_and(|gap| !gap.is_finite() || gap < 0.0)
            || self
                .border_radius
                .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
            || ((self.bar_gap.is_some() || self.border_radius.is_some()) && self.chart != "bar")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid bar layout configuration".into(),
            });
        }
        let invalid_box_color = [
            self.box_color.as_deref(),
            self.median_color.as_deref(),
            self.whisker_color.as_deref(),
            self.outlier_color.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|color| {
            let digits = color.strip_prefix('#').unwrap_or(color);
            digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
        let has_box_configuration = self.box_color.is_some()
            || self.median_color.is_some()
            || self.whisker_color.is_some()
            || self.outlier_color.is_some()
            || self.box_opacity.is_some()
            || self.box_width.is_some()
            || self.outlier_radius.is_some()
            || self.bins.is_some();
        if invalid_box_color
            || self
                .box_opacity
                .is_some_and(|opacity| !opacity.is_finite() || !(0.0..=1.0).contains(&opacity))
            || self
                .box_width
                .is_some_and(|width| !width.is_finite() || width <= 0.0)
            || self
                .outlier_radius
                .is_some_and(|radius| !radius.is_finite() || radius <= 0.0)
            || self.bins == Some(0)
            || (has_box_configuration && self.chart != "box_plot")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid box plot configuration".into(),
            });
        }
        if self.stroke_width.is_some()
            && !matches!(self.chart.as_str(), "line" | "isoline" | "box_plot")
            || self.opacity.is_some()
                && !matches!(
                    self.chart.as_str(),
                    "scatter" | "line" | "area" | "bar" | "heatmap" | "contour" | "isoline"
                )
            || self.thresholds.is_some() && self.chart != "contour"
            || self.levels.is_some() && self.chart != "isoline"
            || self.hole.is_some() && !matches!(self.chart.as_str(), "pie" | "donut")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart kind does not support requested style configuration"
                    .into(),
            });
        }
        let valid_hex = |color: &str| {
            let digits = color.strip_prefix('#').unwrap_or(color);
            digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        };
        if self
            .fill_color
            .as_deref()
            .is_some_and(|color| !valid_hex(color))
            || (self.fill_color.is_some() && self.chart != "area")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid area fill color".into(),
            });
        }
        if self
            .primary_color
            .as_deref()
            .is_some_and(|color| !valid_hex(color))
            || (self.primary_color.is_some()
                && !matches!(self.chart.as_str(), "scatter" | "line" | "bar"))
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid primary series color".into(),
            });
        }
        if self
            .stroke_color
            .as_deref()
            .is_some_and(|color| !valid_hex(color))
            || (self.stroke_color.is_some() && self.chart != "isoline")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid isoline stroke color".into(),
            });
        }
        let has_pie_configuration =
            self.pad_angle.is_some() || self.corner_radius.is_some() || self.sort.is_some();
        if self
            .colors
            .as_ref()
            .is_some_and(|colors| colors.is_empty() || colors.iter().any(|color| !valid_hex(color)))
            || (self.colors.is_some()
                && !matches!(self.chart.as_str(), "pie" | "donut" | "treemap"))
            || self
                .pad_angle
                .is_some_and(|angle| !angle.is_finite() || angle < 0.0)
            || self
                .corner_radius
                .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
            || (has_pie_configuration && !matches!(self.chart.as_str(), "pie" | "donut"))
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid pie presentation configuration".into(),
            });
        }
        if self.hover.is_some() && self.chart != "treemap" {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart hover configuration requires treemap".into(),
            });
        }
        if self
            .renderer_2d
            .as_deref()
            .is_some_and(|renderer| !matches!(renderer, "vello" | "legacy"))
            || self
                .vello_backend
                .as_deref()
                .is_some_and(|backend| !matches!(backend, "auto" | "wgpu" | "cpu"))
            || ((self.renderer_2d.is_some() || self.vello_backend.is_some())
                && self.chart == "surface")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid 2D renderer configuration".into(),
            });
        }
        if self.curve.as_deref().is_some_and(|curve| {
            !matches!(
                curve,
                "linear"
                    | "step"
                    | "step_before"
                    | "step_after"
                    | "basis"
                    | "cardinal"
                    | "catmull_rom"
                    | "monotone_x"
                    | "natural"
            )
        }) || self
            .dash_style
            .as_deref()
            .is_some_and(|dash| !matches!(dash, "solid" | "dashed" | "dotted" | "dash_dot"))
            || (self.curve.is_some() && !matches!(self.chart.as_str(), "line" | "area"))
            || ((self.dash_style.is_some() || self.show_points.is_some()) && self.chart != "line")
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart invalid line style configuration".into(),
            });
        }
        if self
            .contour_upsample_factor
            .is_some_and(|factor| !(1..=8).contains(&factor))
            || (self.contour_upsample_factor.is_some()
                && !matches!(self.chart.as_str(), "contour" | "isoline"))
            || ((self.smooth_strokes.is_some()
                || self.smoothing_iterations.is_some()
                || self.smoothing_max_deviation_px.is_some())
                && self.chart != "isoline")
            || self
                .smoothing_iterations
                .is_some_and(|iterations| iterations > 4)
            || self
                .smoothing_max_deviation_px
                .is_some_and(|deviation| !deviation.is_finite() || deviation < 0.0)
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart has invalid contour or isoline sampling configuration"
                    .into(),
            });
        }
        let finite_increasing = |values: &[f64]| {
            !values.is_empty()
                && values.iter().all(|value| value.is_finite())
                && values.windows(2).all(|pair| pair[0] < pair[1])
        };
        if self
            .thresholds
            .as_deref()
            .is_some_and(|values| !finite_increasing(values))
            || self
                .levels
                .as_deref()
                .is_some_and(|values| !finite_increasing(values))
        {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart thresholds and levels must be finite and increasing"
                    .into(),
            });
        }
        for annotation in &self.annotations {
            let target_valid = match annotation.target.as_str() {
                "point" => annotation.x.is_some() && annotation.y.is_some(),
                "x_value" => annotation.x.is_some(),
                "y_value" => annotation.y.is_some(),
                "category" => annotation
                    .category
                    .as_deref()
                    .is_some_and(|value| !value.is_empty()),
                _ => false,
            };
            if annotation.id.trim().is_empty()
                || annotation.label.trim().is_empty()
                || !target_valid
                || annotation.x.is_some_and(|value| !value.is_finite())
                || annotation.y.is_some_and(|value| !value.is_finite())
            {
                return Err(UiIrError::InvalidPatch {
                    message: format!("resource chart {:?} has invalid annotation", self.id),
                });
            }
        }
        if self.selection_action.as_deref().is_some_and(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart selection action cannot be empty".into(),
            });
        }
        if self.viewport_action.as_deref().is_some_and(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "resource chart viewport action cannot be empty".into(),
            });
        }
        if self.selection_action.is_some() {
            let dataset = match source.get("kind").and_then(Value::as_str) {
                Some("dataset") => source,
                Some("dataset_view") => {
                    source
                        .get("dataset")
                        .ok_or_else(|| UiIrError::InvalidPatch {
                            message: "dataset view requires dataset descriptor".into(),
                        })?
                }
                _ => {
                    return Err(UiIrError::InvalidPatch {
                        message: "resource chart selection requires a keyed dataset".into(),
                    });
                }
            };
            if dataset
                .get("key")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart selection requires a stable dataset key".into(),
                });
            }
            {
                let key = dataset
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let row_id = self
                    .data
                    .get("roles")
                    .and_then(Value::as_object)
                    .and_then(|roles| roles.get("row_id"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if key != row_id {
                    return Err(UiIrError::InvalidPatch {
                        message: format!(
                            "{} selection requires row_id to match dataset key",
                            self.chart
                        ),
                    });
                }
            }
        }
        validate_resource_source(source)?;
        validate_dataset_view_operations(
            source,
            true,
            matches!(self.chart.as_str(), "scatter" | "line"),
        )?;
        if let Some(sort_field) = source
            .get("operations")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|operation| operation.get("op").and_then(Value::as_str) == Some("sort"))
            .and_then(|operation| operation.get("field"))
            .and_then(Value::as_str)
        {
            if source
                .get("operations")
                .and_then(Value::as_array)
                .is_some_and(|operations| {
                    operations.iter().any(|operation| {
                        operation.get("op").and_then(Value::as_str) == Some("range")
                    })
                })
            {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart DatasetView sort plus range is unavailable".into(),
                });
            }
            let roles = self.data.get("roles").and_then(Value::as_object);
            if roles.is_none_or(|roles| {
                roles.get("x").and_then(Value::as_str) != Some(sort_field)
                    && roles.get("y").and_then(Value::as_str) != Some(sort_field)
            }) {
                return Err(UiIrError::InvalidPatch {
                    message: "resource chart DatasetView sort must match x or y role".into(),
                });
            }
        }
        validate_dataset_view_projection(
            source,
            self.data
                .get("roles")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|roles| roles.values())
                .filter_map(Value::as_str),
        )
    }
}

fn validate_resource_source(value: &Value) -> Result<(), UiIrError> {
    let source = match value.get("kind").and_then(Value::as_str) {
        Some("dataset") | Some("array_data") => value,
        Some("dataset_view") => value
            .get("dataset")
            .ok_or_else(|| UiIrError::InvalidPatch {
                message: "dataset view requires dataset descriptor".into(),
            })?,
        _ => {
            return Err(UiIrError::InvalidPatch {
                message: "resource binding requires dataset, dataset view, or array descriptor"
                    .into(),
            });
        }
    };
    if source.get("values").is_some()
        || source
            .get("id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || source
            .get("generation")
            .and_then(Value::as_u64)
            .is_none_or(|generation| generation == 0)
    {
        return Err(UiIrError::InvalidPatch {
            message:
                "resource descriptors require stable ID/generation and cannot contain inline values"
                    .into(),
        });
    }
    Ok(())
}

/// Validate the DatasetView subset this generic host can execute today.
/// Declarations are kept as a serializable AST, but unsupported operations
/// must fail validation rather than silently rendering the unfiltered source.
fn validate_dataset_filter_expression(
    value: &Value,
    fields: &mut Vec<String>,
) -> Result<(), UiIrError> {
    let invalid = |message: &str| UiIrError::InvalidPatch {
        message: message.to_owned(),
    };
    if value.is_null() || value.is_boolean() || value.is_number() || value.is_string() {
        return Ok(());
    }
    let operation = value
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("dataset view filter expression requires op"))?;
    let arguments = value
        .get("args")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("dataset view filter expression requires args"))?;
    match operation {
        "field" => {
            let field = (arguments.len() == 1)
                .then(|| arguments[0].as_str())
                .flatten()
                .filter(|field| !field.is_empty())
                .ok_or_else(|| invalid("dataset view field filter requires one non-empty field"))?;
            if !fields.iter().any(|candidate| candidate == field) {
                fields.push(field.to_owned());
            }
        }
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "and" | "or" => {
            if arguments.len() != 2 {
                return Err(invalid(
                    "dataset view binary filter requires exactly two arguments",
                ));
            }
            validate_dataset_filter_expression(&arguments[0], fields)?;
            validate_dataset_filter_expression(&arguments[1], fields)?;
        }
        "not" | "is_null" => {
            if arguments.len() != 1 {
                return Err(invalid(
                    "dataset view unary filter requires exactly one argument",
                ));
            }
            validate_dataset_filter_expression(&arguments[0], fields)?;
        }
        "in" => {
            if arguments.len() != 2 {
                return Err(invalid(
                    "dataset view membership filter requires exactly two arguments",
                ));
            }
            validate_dataset_filter_expression(&arguments[0], fields)?;
            let values = arguments[1]
                .as_array()
                .filter(|values| !values.is_empty())
                .ok_or_else(|| {
                    invalid("dataset view membership filter requires a non-empty literal array")
                })?;
            if values.iter().any(|value| {
                !(value.is_null() || value.is_boolean() || value.is_number() || value.is_string())
            }) {
                return Err(invalid(
                    "dataset view membership values must be scalar literals",
                ));
            }
        }
        unsupported => {
            return Err(UiIrError::InvalidPatch {
                message: format!(
                    "dataset view filter operation {unsupported:?} is unavailable for this host"
                ),
            });
        }
    }
    Ok(())
}

fn validate_dataset_view_operations(
    value: &Value,
    supports_range: bool,
    supports_sort: bool,
) -> Result<(), UiIrError> {
    if value.get("kind").and_then(Value::as_str) != Some("dataset_view") {
        return Ok(());
    }
    let operations = value
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| UiIrError::InvalidPatch {
            message: "dataset view requires an operations array".into(),
        })?;
    if operations
        .iter()
        .filter(|operation| operation.get("op").and_then(Value::as_str) == Some("filter"))
        .count()
        > 1
    {
        return Err(UiIrError::InvalidPatch {
            message: "this host supports at most one DatasetView filter".into(),
        });
    }
    if operations
        .iter()
        .filter(|operation| operation.get("op").and_then(Value::as_str) == Some("range"))
        .count()
        > 1
    {
        return Err(UiIrError::InvalidPatch {
            message: "this host supports at most one DatasetView range".into(),
        });
    }
    if operations
        .iter()
        .filter(|operation| operation.get("op").and_then(Value::as_str) == Some("select"))
        .count()
        > 1
    {
        return Err(UiIrError::InvalidPatch {
            message: "this host supports at most one DatasetView projection".into(),
        });
    }
    if operations
        .iter()
        .filter(|operation| operation.get("op").and_then(Value::as_str) == Some("sort"))
        .count()
        > 1
    {
        return Err(UiIrError::InvalidPatch {
            message: "this host supports at most one DatasetView sort".into(),
        });
    }
    let mut saw_range = false;
    let mut projected_fields: Option<std::collections::HashSet<&str>> = None;
    for operation in operations {
        let kind =
            operation
                .get("op")
                .and_then(Value::as_str)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: "dataset view operation requires op".into(),
                })?;
        match kind {
            "filter" => {
                let expression =
                    operation
                        .get("expression")
                        .ok_or_else(|| UiIrError::InvalidPatch {
                            message: "dataset view filter requires expression".into(),
                        })?;
                let mut filter_fields = Vec::new();
                validate_dataset_filter_expression(expression, &mut filter_fields)?;
                if projected_fields.as_ref().is_some_and(|projected| {
                    filter_fields
                        .iter()
                        .any(|field| !projected.contains(field.as_str()))
                }) {
                    return Err(UiIrError::InvalidPatch {
                        message: "dataset view filter field unavailable after projection".into(),
                    });
                }
            }
            "select" => {
                let fields = operation
                    .get("fields")
                    .and_then(Value::as_array)
                    .ok_or_else(|| UiIrError::InvalidPatch {
                        message: "dataset view select requires a fields array".into(),
                    })?;
                let mut unique = std::collections::HashSet::new();
                if fields.is_empty()
                    || fields.iter().any(|field| {
                        field
                            .as_str()
                            .is_none_or(|field| field.is_empty() || !unique.insert(field))
                    })
                {
                    return Err(UiIrError::InvalidPatch {
                        message: "dataset view select requires unique non-empty fields".into(),
                    });
                }
                projected_fields = Some(unique);
            }
            "sort" if supports_sort => {
                if saw_range {
                    return Err(UiIrError::InvalidPatch {
                        message: "dataset view sort must precede range".into(),
                    });
                }
                if operation
                    .get("field")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                    || operation
                        .get("descending")
                        .is_some_and(|descending| !descending.is_boolean())
                {
                    return Err(UiIrError::InvalidPatch {
                        message: "dataset view sort requires field and boolean descending".into(),
                    });
                }
                let field = operation
                    .get("field")
                    .and_then(Value::as_str)
                    .expect("validated above");
                if projected_fields
                    .as_ref()
                    .is_some_and(|fields| !fields.contains(field))
                {
                    return Err(UiIrError::InvalidPatch {
                        message: "dataset view sort field unavailable after projection".into(),
                    });
                }
            }
            "range" if supports_range => {
                saw_range = true;
                let start = operation.get("start").and_then(Value::as_u64);
                let stop = operation.get("stop").and_then(Value::as_u64);
                if start.zip(stop).is_none_or(|(start, stop)| start > stop) {
                    return Err(UiIrError::InvalidPatch {
                        message: "dataset view range requires ordered integer start/stop".into(),
                    });
                }
            }
            unsupported => {
                return Err(UiIrError::InvalidPatch {
                    message: format!(
                        "DatasetView operation {unsupported:?} is unavailable for this host consumer"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_dataset_view_projection<'a>(
    value: &Value,
    required_fields: impl IntoIterator<Item = &'a str>,
) -> Result<(), UiIrError> {
    if value.get("kind").and_then(Value::as_str) != Some("dataset_view") {
        return Ok(());
    }
    let Some(fields) = value
        .get("operations")
        .and_then(Value::as_array)
        .and_then(|operations| {
            operations.iter().find_map(|operation| {
                (operation.get("op").and_then(Value::as_str) == Some("select"))
                    .then(|| operation.get("fields").and_then(Value::as_array))
                    .flatten()
            })
        })
    else {
        return Ok(());
    };
    let projected = fields
        .iter()
        .filter_map(Value::as_str)
        .collect::<std::collections::HashSet<_>>();
    if let Some(field) = required_fields
        .into_iter()
        .find(|field| !projected.contains(field))
    {
        return Err(UiIrError::InvalidPatch {
            message: format!("dataset view projection does not contain required field {field:?}"),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartAnnotationNode {
    pub id: String,
    pub label: String,
    pub target: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub category: Option<String>,
    pub color: Option<String>,
    pub series_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene3dNode {
    pub id: String,
    pub spec: Value,
    #[serde(default)]
    pub selection_action: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshPlotNode {
    pub id: String,
    pub spec: Value,
    #[serde(default)]
    pub selection_action: Option<String>,
    #[serde(default)]
    pub export_action: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl MeshPlotNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "mesh_plot requires a stable id".into(),
            });
        }
        if self.selection_action.as_deref().is_some_and(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "mesh_plot selection action is empty".into(),
            });
        }
        if self.export_action.as_deref().is_some_and(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "mesh_plot export action is empty".into(),
            });
        }
        crate::meshplot::MeshPlotSpec::from_value(self.spec.clone())
            .map_err(|message| UiIrError::InvalidPatch { message })?;
        Ok(())
    }
}

impl Scene3dNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "scene3d requires a stable id".into(),
            });
        }
        if self.selection_action.as_deref().is_some_and(str::is_empty) {
            return Err(UiIrError::InvalidPatch {
                message: "scene3d selection action is empty".into(),
            });
        }
        Ok(())
    }
}

fn default_python_app_ir_schema_version() -> u32 {
    PYTHON_APP_IR_SCHEMA_VERSION
}

fn default_sidebar_title() -> String {
    "Python UI".to_string()
}

fn default_tone() -> String {
    "primary".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_content(content: Value) -> PythonAppIr {
        serde_json::from_value(serde_json::json!({
            "title": "Contract test",
            "sections": [{"id": "main", "label": "Main", "content": content}]
        }))
        .expect("valid test fixture shape")
    }

    fn assert_invalid_content(content: Value) {
        assert!(matches!(
            app_with_content(content).validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn mesh_plot_export_action_is_typed_and_non_empty() {
        let mut node = MeshPlotNode {
            id: "plot".into(),
            spec: serde_json::json!({
                "schema_version": 1,
                "id": "plot",
                "geometry": {
                    "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    "triangles": [[0, 1, 2]]
                }
            }),
            selection_action: None,
            export_action: Some("mesh-exported".into()),
            width: None,
            height: None,
        };
        assert!(node.validate().is_ok());
        node.export_action = Some(String::new());
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn resource_surface_and_treemap_sizing_is_strict() {
        let mut value = serde_json::json!({
            "chart": "surface",
            "id": "terrain",
            "data": {
                "source": {
                    "kind": "array_data",
                    "id": "terrain-values",
                    "generation": 1,
                    "dtype": "f32",
                    "shape": [2, 2]
                },
                "roles": {}
            },
            "lod": "auto",
            "fill": true,
            "min_width": 320.0,
            "min_height": 240.0,
            "aspect_ratio": 1.5
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        value["fill"] = Value::Null;
        value["width"] = serde_json::json!(640.0);
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        value["height"] = serde_json::json!(360.0);
        value["chart"] = serde_json::json!("scatter");
        value["data"] = serde_json::json!({
            "source": {"kind": "dataset", "id": "points", "generation": 1},
            "roles": {"x": "frequency", "y": "level"}
        });
        let node: PxChartV2Node = serde_json::from_value(value).unwrap();
        assert!(node.validate().is_ok());
    }

    #[test]
    fn dataset_view_validation_rejects_operations_the_host_cannot_execute() {
        let filtered = serde_json::json!({
            "kind": "dataset_view",
            "dataset": {"kind": "dataset", "id": "events", "generation": 1},
            "operations": [{
                "op": "filter",
                "expression": {"op": "field", "args": ["enabled"]}
            }]
        });
        assert!(validate_dataset_view_operations(&filtered, false, false).is_ok());
        let composed = serde_json::json!({
            "kind": "dataset_view",
            "dataset": {"kind": "dataset", "id": "events", "generation": 1},
            "operations": [{
                "op": "filter",
                "expression": {
                    "op": "and",
                    "args": [
                        {"op": "in", "args": [
                            {"op": "field", "args": ["channel"]}, ["L", "R"]
                        ]},
                        {"op": "gt", "args": [
                            {"op": "field", "args": ["spl"]}, 0.0
                        ]}
                    ]
                }
            }]
        });
        assert!(validate_dataset_view_operations(&composed, false, false).is_ok());
        let mut malformed = composed.clone();
        malformed["operations"][0]["expression"]["args"] = serde_json::json!([]);
        assert!(matches!(
            validate_dataset_view_operations(&malformed, false, false),
            Err(UiIrError::InvalidPatch { .. })
        ));
        let mut repeated_filter = filtered.clone();
        repeated_filter["operations"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "op": "filter",
                "expression": {"op": "field", "args": ["enabled"]}
            }));
        assert!(matches!(
            validate_dataset_view_operations(&repeated_filter, false, false),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let ranged = serde_json::json!({
            "kind": "dataset_view",
            "dataset": {"kind": "dataset", "id": "events", "generation": 1},
            "operations": [{"op": "range", "start": 1, "stop": 3}]
        });
        assert!(validate_dataset_view_operations(&ranged, true, false).is_ok());
        assert!(matches!(
            validate_dataset_view_operations(&ranged, false, false),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let projected = serde_json::json!({
            "kind": "dataset_view",
            "dataset": {"kind": "dataset", "id": "events", "generation": 1},
            "operations": [{"op": "select", "fields": ["frequency", "spl"]}]
        });
        assert!(validate_dataset_view_operations(&projected, true, false).is_ok());
        assert!(validate_dataset_view_projection(&projected, ["frequency", "spl"]).is_ok());
        assert!(matches!(
            validate_dataset_view_projection(&projected, ["channel"]),
            Err(UiIrError::InvalidPatch { .. })
        ));
        let mut invalid_projection = projected.clone();
        invalid_projection["operations"][0]["fields"] = serde_json::json!(["spl", "spl"]);
        assert!(matches!(
            validate_dataset_view_operations(&invalid_projection, true, false),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let sorted = serde_json::json!({
            "kind": "dataset_view",
            "dataset": {"kind": "dataset", "id": "events", "generation": 1},
            "operations": [{"op": "sort", "field": "frequency"}]
        });
        assert!(validate_dataset_view_operations(&sorted, true, true).is_ok());
        assert!(matches!(
            validate_dataset_view_operations(&sorted, true, false),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn resource_table_selection_requires_a_keyed_dataset_and_nonempty_action() {
        let mut value = serde_json::json!({
            "id": "events",
            "data": {"kind": "dataset", "id": "events", "generation": 1},
            "columns": [{"id": "frequency", "field": "frequency"}],
            "selection_mode": "single",
            "virtualize": {"row_height": 28.0, "overscan": 8},
            "selection_action": "selected"
        });
        let node: TableV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        value["data"]["key"] = serde_json::json!("event_id");
        let node: TableV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        value["selection_action"] = serde_json::json!("");
        let node: TableV2Node = serde_json::from_value(value).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn resource_chart_interactions_are_rejected_until_the_host_can_emit_them() {
        let array_chart: PxChartV2Node = serde_json::from_value(serde_json::json!({
            "chart": "scatter",
            "id": "grid",
            "data": {
                "source": {"kind": "array_data", "id": "grid", "generation": 1},
                "roles": {"x": "x", "y": "y"}
            },
            "lod": "auto"
        }))
        .unwrap();
        assert!(array_chart.validate().is_ok());

        let surface: PxChartV2Node = serde_json::from_value(serde_json::json!({
            "chart": "surface",
            "id": "surface",
            "data": {"source": {"kind": "array_data", "id": "grid", "generation": 1}},
            "lod": "auto",
            "viewport_action": "camera-changed"
        }))
        .unwrap();
        assert!(surface.validate().is_ok());

        for lod in ["auto", "off", "aggressive"] {
            let value = serde_json::json!({
                "chart": "heatmap",
                "id": "grid",
                "data": {"source": {"kind": "array_data", "id": "grid", "generation": 1}},
                "lod": lod,
            });
            let node: PxChartV2Node = serde_json::from_value(value).unwrap();
            assert!(node.validate().is_ok(), "LOD {lod} should validate");
        }
        let table_grid: PxChartV2Node = serde_json::from_value(serde_json::json!({
            "chart": "heatmap",
            "id": "grid",
            "data": {"source": {"kind": "dataset", "id": "events", "generation": 1}},
            "lod": "auto",
        }))
        .unwrap();
        assert!(matches!(
            table_grid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut value = serde_json::json!({
            "chart": "scatter",
            "id": "points",
            "data": {
                "source": {"kind": "dataset", "id": "events", "generation": 1},
                "roles": {"x": "frequency", "y": "spl"}
            },
            "lod": "auto",
            "selection_action": "selected"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        value["data"]["source"]["key"] = serde_json::json!("event_id");
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        value["data"]["roles"]["row_id"] = serde_json::json!("event_id");
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        value["selection_action"] = serde_json::json!("");
        let node: PxChartV2Node = serde_json::from_value(value).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn categorical_resource_charts_require_label_and_value_roles() {
        let valid: PxChartV2Node = serde_json::from_value(serde_json::json!({
            "chart": "bar",
            "id": "levels",
            "data": {
                "source": {"kind": "dataset", "id": "levels", "generation": 1},
                "roles": {"label": "band", "y": "level"}
            },
            "lod": "auto",
            "legend_position": "hidden",
            "annotations": [{
                "id": "mid", "label": "Mid band", "target": "category",
                "x": null, "y": null, "category": "Mid", "color": "#ff0000",
                "series_index": null
            }]
        }))
        .unwrap();
        assert!(valid.validate().is_ok());

        let missing_label: PxChartV2Node = serde_json::from_value(serde_json::json!({
            "chart": "bar",
            "id": "levels",
            "data": {
                "source": {"kind": "dataset", "id": "levels", "generation": 1},
                "roles": {"y": "level"}
            },
            "lod": "auto"
        }))
        .unwrap();
        assert!(matches!(
            missing_label.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let unsupported: PxChartV2Node = serde_json::from_value(serde_json::json!({
            "chart": "pie",
            "id": "levels",
            "data": {
                "source": {"kind": "dataset", "id": "levels", "generation": 1},
                "roles": {"label": "band", "y": "level"}
            },
            "lod": "auto",
            "legend_position": "right"
        }))
        .unwrap();
        assert!(matches!(
            unsupported.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn treemap_resource_chart_accepts_keyed_selection_and_rejects_bad_roles() {
        let value = serde_json::json!({
            "chart": "treemap",
            "id": "bands",
            "data": {
                "source": {
                    "kind": "dataset", "id": "bands", "generation": 1, "key": "id"
                },
                "roles": {"row_id": "id", "parent": "parent", "size": "value"}
            },
            "lod": "auto",
            "selection_action": "band-selected",
            "tiling_method": "binary",
            "padding": 2.0,
            "colors": ["#112233", "#abcdef"],
            "hover": false,
            "renderer_2d": "vello",
            "vello_backend": "cpu"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut bad_color = value.clone();
        bad_color["colors"] = serde_json::json!(["red"]);
        let node: PxChartV2Node = serde_json::from_value(bad_color).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut bad_key = value.clone();
        bad_key["data"]["source"]["key"] = serde_json::json!("other");
        let node: PxChartV2Node = serde_json::from_value(bad_key).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut bad_method = value;
        bad_method["tiling_method"] = serde_json::json!("spiral");
        let node: PxChartV2Node = serde_json::from_value(bad_method).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn cartesian_resource_chart_accepts_viewport_events_only() {
        let value = serde_json::json!({
            "chart": "scatter",
            "id": "points",
            "data": {
                "source": {"kind": "dataset", "id": "points", "generation": 1},
                "roles": {"x": "frequency", "y": "spl"}
            },
            "lod": "auto",
            "viewport_action": "viewport-changed"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut unsupported = value;
        unsupported["chart"] = serde_json::json!("bar");
        let node: PxChartV2Node = serde_json::from_value(unsupported).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn line_resource_chart_accepts_secondary_axis_series() {
        let value = serde_json::json!({
            "chart": "line",
            "id": "dual-axis",
            "data": {
                "source": {"kind": "dataset", "id": "dual-axis", "generation": 1},
                "roles": {
                    "x": "frequency",
                    "y": "level",
                    "y2": "phase",
                    "series": "channel",
                    "color": "channel",
                    "dash": "channel"
                }
            },
            "lod": "auto",
            "y2_label": "Phase (degrees)",
            "y2_range": [-180.0, 180.0],
            "renderer_2d": "legacy",
            "vello_backend": "cpu",
            "graph_ratio": 0.75,
            "hidden_series": [1],
            "legend_action": "series-toggled"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut invalid = value;
        invalid["chart"] = serde_json::json!("scatter");
        let node: PxChartV2Node = serde_json::from_value(invalid).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_contour_and_isoline_sampling_configuration() {
        let value = serde_json::json!({
            "chart": "isoline",
            "id": "levels",
            "data": {
                "source": {"kind": "array_data", "id": "grid", "generation": 1},
                "roles": {}
            },
            "lod": "auto",
            "contour_upsample_factor": 4,
            "smooth_strokes": true,
            "smoothing_iterations": 3,
            "smoothing_max_deviation_px": 1.25
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut invalid_factor = value.clone();
        invalid_factor["contour_upsample_factor"] = serde_json::json!(9);
        let node: PxChartV2Node = serde_json::from_value(invalid_factor).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("contour");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_bar_layout_configuration() {
        let value = serde_json::json!({
            "chart": "bar",
            "id": "bars",
            "data": {
                "source": {"kind": "dataset", "id": "bar-data", "generation": 1},
                "roles": {"x": "category", "y": "value"}
            },
            "lod": "auto",
            "bar_gap": 7.5,
            "border_radius": 4.0
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut negative = value.clone();
        negative["bar_gap"] = serde_json::json!(-1.0);
        let node: PxChartV2Node = serde_json::from_value(negative).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("line");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_box_plot_presentation_configuration() {
        let value = serde_json::json!({
            "chart": "box_plot",
            "id": "distribution",
            "data": {
                "source": {"kind": "dataset", "id": "box-data", "generation": 1},
                "roles": {"x": "x", "y": "y"}
            },
            "lod": "auto",
            "box_color": "#abcdef",
            "median_color": "#112233",
            "whisker_color": "#445566",
            "outlier_color": "#778899",
            "box_opacity": 0.75,
            "box_width": 24.0,
            "outlier_radius": 3.5,
            "bins": 8
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut invalid_color = value.clone();
        invalid_color["median_color"] = serde_json::json!("red");
        let node: PxChartV2Node = serde_json::from_value(invalid_color).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut invalid_bins = value.clone();
        invalid_bins["bins"] = serde_json::json!(0);
        let node: PxChartV2Node = serde_json::from_value(invalid_bins).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("line");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_surface_axis_and_wireframe_configuration() {
        let value = serde_json::json!({
            "chart": "surface",
            "id": "terrain",
            "data": {
                "source": {
                    "kind": "array_data",
                    "id": "surface-grid",
                    "generation": 1,
                    "shape": [3, 4],
                    "dtype": "f32"
                },
                "roles": {}
            },
            "lod": "auto",
            "wireframe": true,
            "x_log": true,
            "y_log": true,
            "z_range": [0.1, 12.0],
            "x_label": "Longitude",
            "y_label": "Latitude",
            "z_label": "Elevation"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut invalid_range = value.clone();
        invalid_range["z_range"] = serde_json::json!([2.0, 1.0]);
        let node: PxChartV2Node = serde_json::from_value(invalid_range).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("line");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_pie_presentation_configuration() {
        let value = serde_json::json!({
            "chart": "pie",
            "id": "share",
            "data": {
                "source": {"kind": "dataset", "id": "pie-data", "generation": 1},
                "roles": {"label": "label", "y": "value"}
            },
            "lod": "auto",
            "colors": ["#112233", "#445566"],
            "pad_angle": 0.02,
            "corner_radius": 3.0,
            "sort": false
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut invalid_colors = value.clone();
        invalid_colors["colors"] = serde_json::json!([]);
        let node: PxChartV2Node = serde_json::from_value(invalid_colors).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("bar");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_area_presentation_configuration() {
        let value = serde_json::json!({
            "chart": "area",
            "id": "filled",
            "data": {
                "source": {"kind": "dataset", "id": "area-data", "generation": 1},
                "roles": {"x": "x", "y": "y"}
            },
            "lod": "auto",
            "fill_color": "#336699",
            "curve": "natural",
            "x_log": true,
            "y_log": true
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut invalid_color = value.clone();
        invalid_color["fill_color"] = serde_json::json!("blue");
        let node: PxChartV2Node = serde_json::from_value(invalid_color).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("scatter");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_primary_series_color_configuration() {
        let value = serde_json::json!({
            "chart": "scatter",
            "id": "points",
            "data": {
                "source": {"kind": "dataset", "id": "points", "generation": 1},
                "roles": {"x": "x", "y": "y", "color": "group"}
            },
            "lod": "auto",
            "primary_color": "#abcdef"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("area");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_dense_grid_axes_and_isoline_color() {
        let value = serde_json::json!({
            "chart": "isoline",
            "id": "levels",
            "data": {
                "source": {
                    "kind": "array_data",
                    "id": "grid",
                    "generation": 1,
                    "shape": [3, 4],
                    "dtype": "f32"
                },
                "roles": {}
            },
            "lod": "auto",
            "x_log": true,
            "y_log": true,
            "x_range": [1.0, 4.0],
            "y_range": [1.0, 3.0],
            "stroke_color": "#123456"
        });
        let node: PxChartV2Node = serde_json::from_value(value.clone()).unwrap();
        assert!(node.validate().is_ok());

        let mut wrong_kind = value;
        wrong_kind["chart"] = serde_json::json!("heatmap");
        let node: PxChartV2Node = serde_json::from_value(wrong_kind).unwrap();
        assert!(matches!(
            node.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn app_window_size_is_optional_but_rejects_invalid_explicit_values() {
        let mut app = app_with_content(serde_json::json!({
            "kind": "text",
            "id": "body",
            "text": "Ready"
        }));
        assert_eq!(app.width, None);
        assert_eq!(app.height, None);
        assert!(app.validate().is_ok());

        app.width = Some(-1.0);
        assert!(matches!(
            app.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_color_picker_hex_contract() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "color_picker", "id": "accent", "value": "#ff00ffaa"
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "color_picker", "id": "accent", "value": "not-a-color"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_thinking_orb_contract() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "thinking_orb",
                "id": "working-orb",
                "state": "working",
                "size": 192.0,
                "points_per_sphere": 512.0,
                "speed": 0.25,
                "dot_scale": 4.0,
                "dot_color": "#60a5fa"
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        for (property, value) in [
            ("state", serde_json::json!("unknown")),
            ("size", serde_json::json!(-1.0)),
            ("points_per_sphere", serde_json::json!(0.0)),
            ("speed", serde_json::json!(-0.5)),
            ("dot_scale", serde_json::json!(0.0)),
            ("dot_color", serde_json::json!("blue")),
        ] {
            let mut content = serde_json::json!({
                "kind": "thinking_orb",
                "id": "working-orb",
                "state": "working"
            });
            content[property] = value;
            assert_invalid_content(content);
        }
    }

    #[test]
    fn validates_accordion_items_and_expansion() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "accordion", "id": "advanced", "expanded": ["solver"],
                "items": [{"id": "solver", "title": "Solver", "children": [
                    {"kind": "text", "text": "Tolerance"}
                ]}]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "accordion", "id": "advanced", "expanded": ["missing"],
                "items": [{"id": "solver", "title": "Solver"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_table_sorting_contract() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "table", "id": "runs", "sort_action": "sort_runs",
                "sort_column": "frequency", "sort_direction": "descending",
                "columns": [{"id": "frequency", "label": "Frequency", "sortable": true, "width": 120.0}]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "table", "id": "runs", "sort_column": "missing",
                "sort_direction": "sideways", "columns": [{"id": "frequency", "label": "Frequency"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let invalid_resize: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "table", "resize_action": "resize_column",
                "columns": [{"id": "frequency", "label": "Frequency"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_resize.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn interactive_actions_require_stable_node_ids() {
        let invalid_button: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "button", "label": "Run", "action": "run"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_button.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let invalid_tabs: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "tabs", "items": ["Configuration"], "action": "set_tab"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_tabs.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_list_editor_row_ids() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "list_editor", "id": "frequencies",
                "rows": [{"id": "f-100", "label": "100 Hz", "value": 100.0}]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "list_editor", "id": "frequencies",
                "rows": [{"id": "same", "label": "100 Hz"}, {"id": "same", "label": "200 Hz"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_form_summary_references() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "form", "id": "simulation", "errors": [
                    {"control_id": "frequency", "message": "Enter a frequency"}
                ],
                "children": [{"kind": "number_input", "id": "frequency", "value": 100.0}]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "form", "id": "simulation", "errors": [{"control_id": "", "message": "Missing"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_stepper_navigation_state() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "stepper", "id": "workflow", "steps": ["Model", "Run"], "active": 0
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());
        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "stepper", "id": "workflow", "steps": ["Model"], "active": 1
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn parses_minimal_app_ir() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{
                "id": "overview",
                "label": "Overview",
                "content": {
                    "kind": "vstack",
                    "children": [{"kind": "heading", "text": "Hello", "level": 1}]
                }
            }]
        }))
        .expect("app ir");

        assert_eq!(app.schema_version, PYTHON_APP_IR_SCHEMA_VERSION);
        assert_eq!(app.title, "Demo");
        app.validate().expect("valid app");
    }

    #[test]
    fn validates_app_ir_schema_version() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "schema_version": 999,
            "title": "Demo",
            "sections": [{
                "id": "overview",
                "label": "Overview",
                "content": {"kind": "text", "text": "Hello"}
            }]
        }))
        .expect("app ir");

        assert!(matches!(
            app.validate(),
            Err(UiIrError::UnsupportedSchemaVersion {
                schema: "python_app_ir",
                version: 999,
                supported: PYTHON_APP_IR_SCHEMA_VERSION,
            })
        ));
    }

    #[test]
    fn rejects_removed_v1_chart_protocol_at_deserialization() {
        let error = serde_json::from_value::<PythonAppIr>(serde_json::json!({
            "title": "legacy",
            "sections": [{
                "id": "chart",
                "label": "Chart",
                "content": {
                    "kind": "chart",
                    "chart": "line",
                    "id": "legacy-chart",
                    "x": [1.0],
                    "y": [2.0]
                }
            }]
        }))
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("unknown variant `chart`"));
        assert!(message.contains("px_chart_v2"));
    }

    #[test]
    fn validates_empty_sections_and_ids() {
        let empty: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": []
        }))
        .unwrap();
        assert!(matches!(empty.validate(), Err(UiIrError::EmptySections)));

        let missing_id: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "", "label": "Bad", "content": {"kind": "text", "text": "x"}}]
        }))
        .unwrap();
        assert!(matches!(
            missing_id.validate(),
            Err(UiIrError::EmptySectionId { .. })
        ));
    }

    #[test]
    fn validates_nested_cards_and_stacks() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{
                "id": "s",
                "label": "S",
                "content": {
                    "kind": "card",
                    "children": [{
                        "kind": "vstack",
                        "children": [{"kind": "hstack", "children": [{"kind": "text", "text": "x"}]}]
                    }]
                }
            }]
        }))
        .unwrap();
        assert!(app.validate().is_ok());
    }

    #[test]
    fn validates_path_input_modes_and_filters() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "path_input", "id": "model", "mode": "open_file",
                "filters": [{"label": "Models", "extensions": ["mlg", "json"]}],
                "value": "speaker.mlg", "must_exist": true
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid_mode: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "path_input", "id": "model", "mode": "remote_url"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_mode.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_breadcrumb_and_alert_contracts() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "vstack", "children": [
                    {"kind": "breadcrumbs", "id": "location", "separator": "chevron",
                     "action": "navigate", "items": [
                        {"id": "home", "label": "Home"}, {"id": "run", "label": "Run"}
                     ]},
                    {"kind": "alert", "id": "saved", "message": "Saved", "variant": "success",
                     "closeable": true, "action": "dismiss"},
                    {"kind": "toast", "id": "queued", "message": "Queued", "duration_secs": 3.0}
                    ,{"kind": "tooltip", "id": "help-tip", "content": "Explain this",
                      "placement": "bottom", "child": {"kind": "button", "id": "help", "label": "Help"}},
                    {"kind": "empty_state", "title": "No runs", "action": {
                        "kind": "button", "id": "create", "label": "Create"
                    }}
                ]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "alert", "id": "saved", "message": "Saved", "action": "dismiss"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let invalid_duration: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "toast", "id": "queued", "message": "Queued", "duration_secs": 0.0
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_duration.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let invalid_tooltip: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "tooltip", "id": "help-tip", "content": "Explain", "placement": "diagonal",
                "child": {"kind": "text", "text": "Help"}
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_tooltip.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));

        let invalid_empty: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "empty_state", "title": ""
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid_empty.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_dialog_slots_and_size() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "dialog", "id": "details", "title": "Details", "size": "lg",
                "content": [{"kind": "text", "text": "Ready"}],
                "footer": [{"kind": "button", "id": "close", "label": "Close", "action": "close"}],
                "close_action": "dismiss"
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "dialog", "id": "details", "size": "giant"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_context_menu_and_popover_slots() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "vstack", "children": [
                    {"kind": "context_menu", "id": "actions", "position": [12.0, 24.0],
                     "items": [{"id": "run", "label": "Run"}, {"separator": true}],
                     "action": "select_action", "close_action": "close_actions"},
                    {"kind": "popover", "id": "details", "placement": "bottom_end", "width": 220.0,
                     "trigger": {"kind": "button", "id": "more", "label": "More"},
                     "content": [{"kind": "text", "text": "Details"}]}
                ]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "context_menu", "id": "actions", "items": [
                    {"id": "run", "label": "Run", "checked": true}
                ]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_menu_and_menu_bar_state() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "vstack", "children": [
                    {"kind": "menu", "id": "actions", "focused_index": 0,
                     "items": [{"id": "run", "label": "Run"}]},
                    {"kind": "menu_bar", "id": "app-menu", "active_menu": "file",
                     "items": [{"id": "file", "label": "File", "items": [{"id": "quit", "label": "Quit"}]}]}
                ]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "menu_bar", "id": "app-menu", "active_menu": "unknown",
                "items": [{"id": "file", "label": "File"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_native_confirmation_dialog_actions() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "confirm_dialog", "id": "delete", "title": "Delete?",
                "message": "This cannot be undone.", "variant": "destructive",
                "confirm_action": "delete", "cancel_action": "keep"
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "confirm_dialog", "id": "delete", "message": "", "variant": "unknown"
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_slider_ranges_and_steps() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "slider", "id": "gain", "value": 0.5,
                "min": 0.0, "max": 1.0, "step": 0.1,
                "action": "preview", "commit_action": "commit"
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "slider", "id": "gain", "value": 0.5,
                "min": 1.0, "max": 0.0, "step": 0.0
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn patches_update_transactionally_and_reject_unknown_ids() {
        let mut app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "vstack", "id": "root", "children": [
                    {"kind": "metric", "id": "metric", "label": "Count", "value": "0"}
                ]
            }}]
        }))
        .unwrap();
        app.apply_patch_ops(&[crate::session::PatchOp::Set {
            id: "metric".into(),
            property: "value".into(),
            value: serde_json::json!("1"),
        }])
        .unwrap();
        assert_eq!(
            serde_json::to_value(&app).unwrap()["sections"][0]["content"]["children"][0]["value"],
            "1"
        );

        let before = app.clone();
        let error = app
            .apply_patch_ops(&[
                crate::session::PatchOp::Set {
                    id: "metric".into(),
                    property: "value".into(),
                    value: serde_json::json!("2"),
                },
                crate::session::PatchOp::Set {
                    id: "missing".into(),
                    property: "value".into(),
                    value: serde_json::json!("x"),
                },
            ])
            .unwrap_err();
        assert!(matches!(error, UiIrError::UnknownNodeId { .. }));
        assert_eq!(app, before);
    }

    #[test]
    fn mesh_plot_selection_camera_and_viewport_patches_are_transactional() {
        let mut app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Mesh",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "mesh_plot", "id": "plot", "spec": {
                    "schema_version": 1, "id": "plot",
                    "geometry": {
                        "id": "mesh",
                        "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                        "triangles": [[0, 1, 2]]
                    },
                    "field": {"values": [0.0, 0.5, 1.0], "association": "vertex"},
                    "mode": "scalar_fill"
                }
            }}]
        }))
        .unwrap();
        app.validate().unwrap();
        app.apply_patch_ops(&[
            crate::session::PatchOp::SetMeshPlotSelection {
                plot_id: "plot".into(),
                generation: 1,
                selection: serde_json::json!({"cell_index": 0}),
            },
            crate::session::PatchOp::SetMeshPlotCamera {
                plot_id: "plot".into(),
                generation: 1,
                camera: serde_json::json!({"azimuth": 0.5}),
            },
            crate::session::PatchOp::SetMeshPlotViewport {
                plot_id: "plot".into(),
                generation: 1,
                viewport: serde_json::json!({"x": [0.0, 1.0], "y": [0.0, 1.0]}),
            },
            crate::session::PatchOp::SetMeshPlotProp {
                plot_id: "plot".into(),
                generation: 1,
                property: "axes".into(),
                value: serde_json::json!({
                    "horizontal_label": "distance",
                    "vertical_label": "height",
                    "unit": "m",
                    "x_range": [0.0, 2.0],
                    "y_range": [-1.0, 3.0],
                    "show_grid": false
                }),
            },
            crate::session::PatchOp::SetMeshPlotProp {
                plot_id: "plot".into(),
                generation: 1,
                property: "missing_value_policy".into(),
                value: serde_json::json!("mask_nan"),
            },
        ])
        .unwrap();
        let value = serde_json::to_value(&app).unwrap();
        assert_eq!(
            value["sections"][0]["content"]["spec"]["selection"]["cell_index"],
            0
        );
        assert_eq!(
            value["sections"][0]["content"]["spec"]["camera"]["azimuth"],
            0.5
        );
        assert_eq!(value["sections"][0]["content"]["spec"]["axes"]["unit"], "m");
        assert_eq!(
            value["sections"][0]["content"]["spec"]["axes"]["show_grid"],
            false
        );
        assert_eq!(
            value["sections"][0]["content"]["spec"]["missing_value_policy"],
            "mask_nan"
        );
        app.apply_patch_ops(&[
            crate::session::PatchOp::ClearMeshPlotSelection {
                plot_id: "plot".into(),
                generation: 2,
            },
            crate::session::PatchOp::ResetMeshPlotCamera {
                plot_id: "plot".into(),
                generation: 2,
            },
            crate::session::PatchOp::ResetMeshPlotViewport {
                plot_id: "plot".into(),
                generation: 2,
            },
        ])
        .unwrap();
        let value = serde_json::to_value(&app).unwrap();
        assert!(value["sections"][0]["content"]["spec"]["selection"].is_null());
        assert!(value["sections"][0]["content"]["spec"]["camera"].is_null());
        assert!(value["sections"][0]["content"]["spec"]["viewport"].is_null());
    }

    #[test]
    fn mesh_plot_revolve_property_patch_is_transactional() {
        let mut app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Revolve",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "mesh_plot", "id": "plot", "spec": {
                    "schema_version": 1, "id": "plot",
                    "geometry": {
                        "id": "mesh",
                        "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                        "triangles": [[0, 1, 2]]
                    },
                    "view": "axisymmetric_revolve",
                    "mode": "mesh"
                }
            }}]
        }))
        .unwrap();
        app.validate().unwrap();
        app.apply_patch_ops(&[crate::session::PatchOp::SetMeshPlotProp {
            plot_id: "plot".into(),
            generation: 1,
            property: "revolve".into(),
            value: serde_json::json!({
                "radial": "y",
                "axial": "z",
                "start_angle": 0.25,
                "sweep_angle": 1.5,
                "segments": 32,
                "end_caps": true
            }),
        }])
        .unwrap();

        let value = serde_json::to_value(&app).unwrap();
        assert_eq!(
            value["sections"][0]["content"]["spec"]["revolve"]["segments"],
            32
        );
        assert_eq!(
            value["sections"][0]["content"]["spec"]["revolve"]["end_caps"],
            true
        );
    }

    #[test]
    fn validates_native_audio_controls_meters_and_spectrum() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Audio",
            "sections": [{"id": "audio", "label": "Audio", "content": {
                "kind": "vstack",
                "children": [
                    {"kind": "audio_potentiometer", "id": "gain", "value": 0.5, "min": 0.0, "max": 1.0, "label": "Gain", "action": "preview", "commit_action": "commit"},
                    {"kind": "audio_vertical_slider", "id": "frequency", "value": 1000.0, "min": 20.0, "max": 20000.0, "scale": "logarithmic", "with_ticks": true},
                    {"kind": "audio_volume_knob", "id": "volume", "value": 0.7, "muted": true},
                    {"kind": "audio_horizontal_meter", "id": "hm", "levels": [-12.0, -6.0], "peaks": [-3.0, -1.0], "channel_names": ["L", "R"]},
                    {"kind": "audio_level_meter", "id": "lm", "levels": [-12.0, -6.0], "peaks": [-3.0, -1.0], "channel_names": ["L", "R"]},
                    {"kind": "audio_spectrum", "id": "spectrum", "magnitudes": [-80.0, -40.0], "previous": [-90.0, -50.0], "minimum_frequency": 20.0, "maximum_frequency": 20000.0}
                ]
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let invalid: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Audio",
            "sections": [{"id": "audio", "label": "Audio", "content": {
                "kind": "audio_level_meter", "id": "meter", "levels": [-12.0, -6.0], "peaks": [-3.0]
            }}]
        }))
        .unwrap();
        assert!(matches!(
            invalid.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn validates_miniapp_defaults_dimensions_theme_and_language() {
        let mut app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Mini app",
            "miniapp": {
                "title": "Native shell",
                "width": 960.0,
                "height": 640.0,
                "app_name": "contract-test"
            },
            "sections": [{
                "id": "main",
                "label": "Main",
                "content": {"kind": "text", "text": "Ready"}
            }]
        }))
        .unwrap();
        let miniapp = app.miniapp.as_ref().unwrap();
        assert!(miniapp.scrollable);
        assert_eq!(miniapp.initial_theme, "dark");
        assert_eq!(miniapp.initial_language, "english");
        app.validate().unwrap();

        app.miniapp.as_mut().unwrap().width = 0.0;
        assert!(matches!(
            app.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
        app.miniapp.as_mut().unwrap().width = 960.0;
        app.miniapp.as_mut().unwrap().initial_theme = "sepia".into();
        assert!(matches!(
            app.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
        app.miniapp.as_mut().unwrap().initial_theme = "carbon_gray_100".into();
        app.miniapp.as_mut().unwrap().initial_language = "klingon".into();
        assert!(matches!(
            app.validate(),
            Err(UiIrError::InvalidPatch { .. })
        ));
    }

    #[test]
    fn patch_operations_cover_structural_edits_and_rejections() {
        use crate::session::PatchOp;

        let mut tree = serde_json::json!({
            "sections": [{"content": {
                "kind": "vstack", "id": "root", "children": [
                    {"kind": "metric", "id": "first", "label": "First", "value": "1"},
                    {"kind": "card", "id": "nested", "children": [
                        {"kind": "text", "id": "deep", "text": "Deep"}
                    ]}
                ]
            }}]
        });

        apply_patch_op(
            &mut tree,
            &PatchOp::Replace {
                id: "first".into(),
                node: serde_json::json!({"kind": "badge", "id": "first", "text": "New"}),
            },
        )
        .unwrap();
        apply_patch_op(
            &mut tree,
            &PatchOp::Insert {
                parent_id: "root".into(),
                index: 1,
                node: serde_json::json!({"kind": "text", "id": "middle", "text": "Middle"}),
            },
        )
        .unwrap();
        apply_patch_op(
            &mut tree,
            &PatchOp::Reorder {
                parent_id: "root".into(),
                child_ids: vec!["nested".into(), "middle".into(), "first".into()],
            },
        )
        .unwrap();
        apply_patch_op(&mut tree, &PatchOp::Remove { id: "deep".into() }).unwrap();
        let children = tree["sections"][0]["content"]["children"]
            .as_array()
            .unwrap();
        assert_eq!(node_id(&children[0]), Some("nested"));
        assert_eq!(node_id(&children[1]), Some("middle"));
        assert_eq!(node_id(&children[2]), Some("first"));
        assert!(
            tree["sections"][0]["content"]["children"][0]["children"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        let rejected = [
            PatchOp::Set {
                id: "first".into(),
                property: "missing".into(),
                value: Value::Null,
            },
            PatchOp::Replace {
                id: "first".into(),
                node: serde_json::json!({"id": "first"}),
            },
            PatchOp::Insert {
                parent_id: "root".into(),
                index: 99,
                node: serde_json::json!({"kind": "text", "text": "late"}),
            },
            PatchOp::Insert {
                parent_id: "first".into(),
                index: 0,
                node: serde_json::json!({"kind": "text", "text": "child"}),
            },
            PatchOp::Reorder {
                parent_id: "root".into(),
                child_ids: vec!["first".into()],
            },
            PatchOp::Reorder {
                parent_id: "root".into(),
                child_ids: vec!["nested".into(), "middle".into(), "unknown".into()],
            },
            PatchOp::Remove {
                id: "unknown".into(),
            },
        ];
        for operation in rejected {
            assert!(apply_patch_op(&mut tree.clone(), &operation).is_err());
        }

        assert!(
            apply_patch_op(
                &mut tree.clone(),
                &PatchOp::Insert {
                    parent_id: "root".into(),
                    index: 0,
                    node: serde_json::json!({"id": "missing-kind"}),
                }
            )
            .is_err()
        );
    }

    #[test]
    fn nested_form_targets_cover_every_retained_container_and_control() {
        let fixtures = [
            (
                serde_json::json!({"kind": "vstack", "children": [{"kind": "text_input", "id": "target"}]}),
                "target",
            ),
            (
                serde_json::json!({"kind": "card", "children": [{"kind": "number_input", "id": "target", "value": 1.0}]}),
                "target",
            ),
            (
                serde_json::json!({"kind": "form", "id": "target"}),
                "target",
            ),
            (
                serde_json::json!({"kind": "accordion", "id": "a", "items": [{"id": "target", "title": "T"}]}),
                "target",
            ),
            (
                serde_json::json!({"kind": "tooltip", "id": "tip", "content": "Tip", "child": {"kind": "slider", "id": "target", "value": 1.0, "min": 0.0, "max": 2.0}}),
                "target",
            ),
            (
                serde_json::json!({"kind": "empty_state", "title": "Empty", "action": {"kind": "select", "id": "target", "value": "a", "options": [{"value": "a", "label": "A"}]}}),
                "target",
            ),
            (
                serde_json::json!({"kind": "dialog", "id": "d", "content": [{"kind": "path_input", "id": "target"}]}),
                "target",
            ),
            (
                serde_json::json!({"kind": "popover", "id": "p", "trigger": {"kind": "checkbox", "id": "target", "value": false, "label": "T"}}),
                "target",
            ),
            (
                serde_json::json!({"kind": "menu_bar", "id": "m", "items": [{"id": "target", "label": "File"}]}),
                "target",
            ),
            (
                serde_json::json!({"kind": "toggle", "id": "target", "value": true, "label": "T"}),
                "target",
            ),
            (
                serde_json::json!({"kind": "list_editor", "id": "target"}),
                "target",
            ),
        ];
        for (fixture, target) in fixtures {
            let node: UiNode = serde_json::from_value(fixture).unwrap();
            assert!(child_contains_id(&node, target));
        }
        let unrelated: UiNode = serde_json::from_value(serde_json::json!({
            "kind": "text", "text": "no id"
        }))
        .unwrap();
        assert!(!child_contains_id(&unrelated, "target"));
    }

    #[test]
    fn rejects_invalid_component_and_form_contracts() {
        let invalid = [
            serde_json::json!({"kind": "button", "label": ""}),
            serde_json::json!({"kind": "breadcrumbs", "id": "", "items": []}),
            serde_json::json!({"kind": "breadcrumbs", "id": "b", "separator": "pipe", "items": [{"id": "a", "label": "A"}]}),
            serde_json::json!({"kind": "breadcrumbs", "id": "b", "items": [{"id": "a", "label": "A"}, {"id": "a", "label": "Again"}]}),
            serde_json::json!({"kind": "alert", "id": "", "message": ""}),
            serde_json::json!({"kind": "alert", "id": "a", "message": "A", "variant": "loud"}),
            serde_json::json!({"kind": "toast", "id": "t", "message": "T", "closeable": false, "action": "close"}),
            serde_json::json!({"kind": "tooltip", "id": "", "content": "", "child": {"kind": "text", "text": "x"}}),
            serde_json::json!({"kind": "dialog", "id": ""}),
            serde_json::json!({"kind": "menu_bar", "id": "", "items": []}),
            serde_json::json!({"kind": "menu_bar", "id": "m", "items": [{"id": "a", "label": "A"}, {"id": "a", "label": "Again"}]}),
            serde_json::json!({"kind": "menu_bar", "id": "m", "items": [{"id": "a", "label": "A", "items": [{"id": "x", "label": "X"}, {"id": "x", "label": "Again"}]}]}),
            serde_json::json!({"kind": "context_menu", "id": "", "items": []}),
            serde_json::json!({"kind": "context_menu", "id": "m", "items": [{"id": "x", "label": "X"}], "focused_index": 1}),
            serde_json::json!({"kind": "popover", "id": "", "placement": "diagonal", "trigger": {"kind": "text", "text": "x"}}),
            serde_json::json!({"kind": "tabs", "items": [""], "active": 0}),
            serde_json::json!({"kind": "stepper", "id": "", "steps": []}),
            serde_json::json!({"kind": "accordion", "id": ""}),
            serde_json::json!({"kind": "accordion", "id": "a", "multiple": false, "expanded": ["x", "y"], "items": [{"id": "x", "title": "X"}, {"id": "y", "title": "Y"}]}),
            serde_json::json!({"kind": "list_editor", "id": ""}),
            serde_json::json!({"kind": "text_input", "id": "", "width": -1.0}),
            serde_json::json!({"kind": "number_input", "id": "n", "value": 1.0, "min": 2.0, "max": 1.0}),
            serde_json::json!({"kind": "number_input", "id": "n", "value": 1.0, "step": 0.0}),
            serde_json::json!({"kind": "select", "id": "s", "value": "a", "options": []}),
            serde_json::json!({"kind": "checkbox", "id": "c", "value": false, "indeterminate": true, "label": ""}),
            serde_json::json!({"kind": "path_input", "id": "p", "filters": [{"label": "", "extensions": [""]}]}),
            serde_json::json!({"kind": "slider", "id": "", "value": 0.0, "min": 0.0, "max": 1.0}),
            serde_json::json!({"kind": "slider", "id": "s", "value": 0.0, "min": 0.0, "max": 1.0, "step": 0.0}),
            serde_json::json!({"kind": "audio_potentiometer", "id": "a", "value": 0.5, "min": 0.0, "max": 1.0, "size": "huge"}),
            serde_json::json!({"kind": "audio_spectrum", "id": "s", "magnitudes": [1.0], "minimum_frequency": 100.0, "maximum_frequency": 20.0}),
            serde_json::json!({"kind": "scene3d", "id": "", "spec": {}}),
            serde_json::json!({"kind": "scene3d", "id": "scene", "spec": {}, "selection_action": ""}),
        ];
        for content in invalid {
            assert_invalid_content(content);
        }
    }
}
