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
pub struct PythonAppIr {
    #[serde(default = "default_python_app_ir_schema_version")]
    pub schema_version: u32,
    pub title: String,
    #[serde(default = "default_width")]
    pub width: f32,
    #[serde(default = "default_height")]
    pub height: f32,
    #[serde(default = "default_sidebar_title")]
    pub sidebar_title: String,
    #[serde(default)]
    pub sidebar_subtitle: String,
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
}

fn node_id(value: &Value) -> Option<&str> {
    value.get("id")?.as_str()
}

fn find_node_mut<'a>(value: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    if node_id(value) == Some(id) {
        return Some(value);
    }
    if let Some(children) = value.get_mut("children")?.as_array_mut() {
        for child in children {
            if let Some(found) = find_node_mut(child, id) {
                return Some(found);
            }
        }
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
            outcome = Some(f.take().expect("single callback")(node));
            true
        } else {
            false
        }
    });
    if !found {
        return Err(UiIrError::UnknownNodeId { id: id.into() });
    }
    outcome.expect("found callback runs")
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
        PatchOp::ReplaceChartSeries { chart_id, series } => with_node_mut(tree, chart_id, |chart| {
            if chart.get("kind").and_then(Value::as_str) != Some("chart") {
                return Err(UiIrError::InvalidPatch {
                    message: format!("node {chart_id:?} is not a chart"),
                });
            }
            let replacement_id = node_id(series).ok_or_else(|| UiIrError::InvalidPatch {
                message: "replacement chart series needs an id".into(),
            })?;
            let series_values = chart
                .get_mut("series")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: format!("chart {chart_id:?} has invalid series data"),
                })?;
            let index = series_values
                .iter()
                .position(|candidate| node_id(candidate) == Some(replacement_id))
                .ok_or_else(|| UiIrError::InvalidPatch {
                    message: format!("chart {chart_id:?} has no series {replacement_id:?}"),
                })?;
            series_values[index] = series.clone();
            Ok(())
        }),
        PatchOp::AppendChartSeries { chart_id, series_id, x, y } => {
            if x.len() != y.len() {
                return Err(UiIrError::ChartLengthMismatch {
                    id: format!("{chart_id}:{series_id}"),
                    left: "x",
                    left_len: x.len(),
                    right: "y",
                    right_len: y.len(),
                });
            }
            with_node_mut(tree, chart_id, |chart| {
                if chart.get("kind").and_then(Value::as_str) != Some("chart") {
                    return Err(UiIrError::InvalidPatch {
                        message: format!("node {chart_id:?} is not a chart"),
                    });
                }
                let series_values = chart
                    .get_mut("series")
                    .and_then(Value::as_array_mut)
                    .ok_or_else(|| UiIrError::InvalidPatch {
                        message: format!("chart {chart_id:?} has invalid series data"),
                    })?;
                let series = series_values
                    .iter_mut()
                    .find(|candidate| node_id(candidate) == Some(series_id))
                    .ok_or_else(|| UiIrError::InvalidPatch {
                        message: format!("chart {chart_id:?} has no series {series_id:?}"),
                    })?;
                let series_x = series.get_mut("x").and_then(Value::as_array_mut).ok_or_else(|| UiIrError::InvalidPatch {
                    message: format!("chart {chart_id:?} series {series_id:?} has invalid x data"),
                })?;
                series_x.extend(x.iter().map(|value| serde_json::json!(value)));
                let series_y = series.get_mut("y").and_then(Value::as_array_mut).ok_or_else(|| UiIrError::InvalidPatch {
                    message: format!("chart {chart_id:?} series {series_id:?} has invalid y data"),
                })?;
                series_y.extend(y.iter().map(|value| serde_json::json!(value)));
                Ok(())
            })
        }
    }
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
    Tabs(TabsNode),
    Stepper(StepperNode),
    Accordion(AccordionNode),
    ListEditor(ListEditorNode),
    Table(TableNode),
    TextInput(TextInputNode),
    NumberInput(NumberInputNode),
    Slider(SliderNode),
    Select(SelectNode),
    PathInput(PathInputNode),
    Checkbox(BooleanInputNode),
    Toggle(BooleanInputNode),
    Divider(SimpleNode),
    Spacer(SimpleNode),
    Chart(ChartNode),
    Scene3d(Scene3dNode),
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
            Self::Tabs(node) => node.validate(),
            Self::Accordion(node) => node.validate(),
            Self::ListEditor(node) => node.validate(),
            Self::Stepper(node) => node.validate(),
            Self::Chart(node) => node.validate(),
            Self::Scene3d(node) => node.validate(),
            Self::TextInput(node) => node.validate(),
            Self::NumberInput(node) => node.validate(),
            Self::Slider(node) => node.validate(),
            Self::Table(node) => node.validate(),
            Self::Select(node) => node.validate(),
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
            return Err(UiIrError::InvalidPatch { message: "form id is empty".into() });
        }
        for child in &self.children {
            child.validate()?;
        }
        if self.errors.iter().any(|error| {
            error.control_id.trim().is_empty()
                || error.message.trim().is_empty()
                || !self.children.iter().any(|child| child_contains_id(child, &error.control_id))
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
        UiNode::Vstack(stack) | UiNode::Hstack(stack) | UiNode::Wrap(stack) => {
            stack.children.iter().any(|child| child_contains_id(child, target))
        }
        UiNode::Card(card) => card.children.iter().any(|child| child_contains_id(child, target)),
        UiNode::Form(form) => form.id == target || form.children.iter().any(|child| child_contains_id(child, target)),
        UiNode::Accordion(accordion) => accordion.items.iter().any(|item| {
            item.id == target || item.children.iter().any(|child| child_contains_id(child, target))
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
            return Err(UiIrError::InvalidPatch { message: "stepper requires an id and at least one step".into() });
        }
        if self.active >= self.steps.len()
            || self.steps.iter().any(|step| step.trim().is_empty())
            || self.disabled_steps.iter().any(|index| *index >= self.steps.len())
        {
            return Err(UiIrError::InvalidPatch { message: "stepper has invalid active, label, or disabled step".into() });
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
            return Err(UiIrError::InvalidPatch { message: "accordion id is empty".into() });
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
            return Err(UiIrError::InvalidPatch { message: "single accordion may only expand one item".into() });
        }
        if self.expanded.iter().any(|id| !ids.contains(id)) {
            return Err(UiIrError::InvalidPatch { message: "accordion expanded item does not exist".into() });
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
            return Err(UiIrError::InvalidPatch { message: "list editor id is empty".into() });
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

fn default_visible() -> bool { true }

impl FormControlProps {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.width.is_some_and(|width| !width.is_finite() || width <= 0.0) {
            return Err(UiIrError::InvalidPatch { message: "form control width must be positive and finite".into() });
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
        if self.step.is_some_and(|step| !step.is_finite() || step <= 0.0) {
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
                || column.width.is_some_and(|width| !width.is_finite() || width <= 0.0)
            {
                return Err(UiIrError::InvalidPatch {
                    message: "table columns require unique IDs, labels, and positive finite widths".into(),
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
        if self.sort_column.as_ref().is_some_and(|id| !column_ids.contains(id)) {
            return Err(UiIrError::InvalidPatch {
                message: "table sort column does not exist".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartNode {
    pub id: String,
    pub chart: ChartKind,
    #[serde(default)]
    pub title: String,
    pub x: Option<Vec<f64>>,
    pub y: Option<Vec<f64>>,
    pub categories: Option<Vec<String>>,
    pub values: Option<Vec<f64>>,
    /// `None` is an intentional missing heatmap cell; present values must be finite.
    pub z: Option<Vec<Option<f64>>>,
    pub width_count: Option<usize>,
    pub height_count: Option<usize>,
    pub color: Option<String>,
    #[serde(default = "default_color_scale")]
    pub color_scale: String,
    #[serde(default)]
    pub x_log: bool,
    #[serde(default)]
    pub y_log: bool,
    #[serde(default = "default_chart_width")]
    pub width: f32,
    #[serde(default = "default_chart_height")]
    pub height: f32,
    #[serde(default = "default_point_radius")]
    pub point_radius: f32,
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f32,
    #[serde(default)]
    pub series: Vec<ChartSeries>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub x_range: Option<[f64; 2]>,
    pub y_range: Option<[f64; 2]>,
    pub color_label: Option<String>,
    pub color_unit: Option<String>,
    pub color_range: Option<[f64; 2]>,
    pub aspect_ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSeries {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub color: Option<String>,
    #[serde(default = "default_series_visible")]
    pub visible: bool,
    pub stroke_width: Option<f32>,
    pub point_radius: Option<f32>,
}

fn default_series_visible() -> bool {
    true
}

impl ChartNode {
    fn validate(&self) -> Result<(), UiIrError> {
        let finite = |field: &str, values: &[f64]| -> Result<(), UiIrError> {
            if values.iter().any(|value| !value.is_finite()) {
                return Err(UiIrError::InvalidPatch {
                    message: format!("chart {:?} contains NaN or Infinity in {field}", self.id),
                });
            }
            Ok(())
        };
        let valid_range = |field: &str, range: Option<[f64; 2]>| -> Result<(), UiIrError> {
            if let Some([min, max]) = range
                && (!min.is_finite() || !max.is_finite() || min >= max)
            {
                return Err(UiIrError::InvalidPatch {
                    message: format!("chart {:?} has invalid {field}", self.id),
                });
            }
            Ok(())
        };
        valid_range("x_range", self.x_range)?;
        valid_range("y_range", self.y_range)?;
        valid_range("color_range", self.color_range)?;
        if self.aspect_ratio.is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0) {
            return Err(UiIrError::InvalidPatch {
                message: format!("chart {:?} has invalid aspect_ratio", self.id),
            });
        }
        match self.chart {
            ChartKind::Scatter | ChartKind::Line => {
                if !self.series.is_empty() {
                    for series in &self.series {
                        if series.id.trim().is_empty() {
                            return Err(UiIrError::InvalidPatch {
                                message: "chart series id is empty".into(),
                            });
                        }
                        if series.x.len() != series.y.len() {
                            return Err(UiIrError::ChartLengthMismatch {
                                id: format!("{}:{}", self.id, series.id),
                                left: "x",
                                left_len: series.x.len(),
                                right: "y",
                                right_len: series.y.len(),
                            });
                        }
                        finite("series.x", &series.x)?;
                        finite("series.y", &series.y)?;
                    }
                    return Ok(());
                }
                let x = self.x.as_ref().ok_or_else(|| UiIrError::MissingChartData {
                    id: self.id.clone(),
                    field: "x",
                })?;
                let y = self.y.as_ref().ok_or_else(|| UiIrError::MissingChartData {
                    id: self.id.clone(),
                    field: "y",
                })?;
                if x.len() != y.len() {
                    return Err(UiIrError::ChartLengthMismatch {
                        id: self.id.clone(),
                        left: "x",
                        left_len: x.len(),
                        right: "y",
                        right_len: y.len(),
                    });
                }
                finite("x", x)?;
                finite("y", y)?;
            }
            ChartKind::Bar => {
                let categories =
                    self.categories
                        .as_ref()
                        .ok_or_else(|| UiIrError::MissingChartData {
                            id: self.id.clone(),
                            field: "categories",
                        })?;
                let values = self
                    .values
                    .as_ref()
                    .ok_or_else(|| UiIrError::MissingChartData {
                        id: self.id.clone(),
                        field: "values",
                    })?;
                if categories.len() != values.len() {
                    return Err(UiIrError::ChartLengthMismatch {
                        id: self.id.clone(),
                        left: "categories",
                        left_len: categories.len(),
                        right: "values",
                        right_len: values.len(),
                    });
                }
                finite("values", values)?;
            }
            ChartKind::Heatmap => {
                let z = self.z.as_ref().ok_or_else(|| UiIrError::MissingChartData {
                    id: self.id.clone(),
                    field: "z",
                })?;
                let width = self
                    .width_count
                    .ok_or_else(|| UiIrError::MissingChartData {
                        id: self.id.clone(),
                        field: "width_count",
                    })?;
                let height = self
                    .height_count
                    .ok_or_else(|| UiIrError::MissingChartData {
                        id: self.id.clone(),
                        field: "height_count",
                    })?;
                let expected = width * height;
                if z.len() != expected {
                    return Err(UiIrError::HeatmapDimensionMismatch {
                        id: self.id.clone(),
                        z_len: z.len(),
                        width,
                        height,
                        expected,
                    });
                }
                if z.iter().flatten().any(|value| !value.is_finite()) {
                    return Err(UiIrError::InvalidPatch {
                        message: format!("chart {:?} contains NaN or Infinity in z", self.id),
                    });
                }
                if z.iter().all(Option::is_none) {
                    return Err(UiIrError::InvalidPatch {
                        message: format!("chart {:?} heatmap requires a non-missing z value", self.id),
                    });
                }
                if let Some(x) = &self.x {
                    if x.len() != width {
                        return Err(UiIrError::ChartLengthMismatch { id: self.id.clone(), left: "x", left_len: x.len(), right: "width_count", right_len: width });
                    }
                    finite("x", x)?;
                    if x.windows(2).any(|pair| pair[0] >= pair[1]) {
                        return Err(UiIrError::InvalidPatch { message: format!("chart {:?} heatmap x coordinates must increase", self.id) });
                    }
                }
                if let Some(y) = &self.y {
                    if y.len() != height {
                        return Err(UiIrError::ChartLengthMismatch { id: self.id.clone(), left: "y", left_len: y.len(), right: "height_count", right_len: height });
                    }
                    finite("y", y)?;
                    if y.windows(2).any(|pair| pair[0] >= pair[1]) {
                        return Err(UiIrError::InvalidPatch { message: format!("chart {:?} heatmap y coordinates must increase", self.id) });
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Scatter,
    Line,
    Bar,
    Heatmap,
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

impl Scene3dNode {
    fn validate(&self) -> Result<(), UiIrError> {
        if self.id.trim().is_empty() {
            return Err(UiIrError::InvalidPatch {
                message: "scene3d requires a stable id".into(),
            });
        }
        if self
            .selection_action
            .as_deref()
            .is_some_and(str::is_empty)
        {
            return Err(UiIrError::InvalidPatch {
                message: "scene3d selection action is empty".into(),
            });
        }
        Ok(())
    }
}

fn default_width() -> f32 {
    1240.0
}

fn default_python_app_ir_schema_version() -> u32 {
    PYTHON_APP_IR_SCHEMA_VERSION
}

fn default_height() -> f32 {
    820.0
}

fn default_sidebar_title() -> String {
    "Python UI".to_string()
}

fn default_tone() -> String {
    "primary".to_string()
}

fn default_color_scale() -> String {
    "viridis".to_string()
}

fn default_chart_width() -> f32 {
    360.0
}

fn default_chart_height() -> f32 {
    260.0
}

fn default_point_radius() -> f32 {
    4.0
}

fn default_stroke_width() -> f32 {
    2.0
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(invalid.validate(), Err(UiIrError::InvalidPatch { .. })));
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
        assert!(matches!(invalid.validate(), Err(UiIrError::InvalidPatch { .. })));

        let invalid_resize: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "table", "resize_action": "resize_column",
                "columns": [{"id": "frequency", "label": "Frequency"}]
            }}]
        }))
        .unwrap();
        assert!(matches!(invalid_resize.validate(), Err(UiIrError::InvalidPatch { .. })));
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
        assert!(matches!(invalid_button.validate(), Err(UiIrError::InvalidPatch { .. })));

        let invalid_tabs: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "tabs", "items": ["Configuration"], "action": "set_tab"
            }}]
        }))
        .unwrap();
        assert!(matches!(invalid_tabs.validate(), Err(UiIrError::InvalidPatch { .. })));
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
        assert!(matches!(invalid.validate(), Err(UiIrError::InvalidPatch { .. })));
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
        assert!(matches!(invalid.validate(), Err(UiIrError::InvalidPatch { .. })));
    }

    #[test]
    fn allows_missing_heatmap_cells_but_rejects_all_missing_grid() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "chart", "id": "field", "chart": "heatmap",
                "z": [1.0, null, 3.0, 4.0], "width_count": 2, "height_count": 2
            }}]
        }))
        .unwrap();
        assert!(app.validate().is_ok());

        let all_missing: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "main", "label": "Main", "content": {
                "kind": "chart", "id": "field", "chart": "heatmap",
                "z": [null, null, null, null], "width_count": 2, "height_count": 2
            }}]
        }))
        .unwrap();
        assert!(matches!(all_missing.validate(), Err(UiIrError::InvalidPatch { .. })));
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
        assert!(matches!(invalid.validate(), Err(UiIrError::InvalidPatch { .. })));
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
    fn validates_chart_lengths() {
        let app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{
                "id": "charts",
                "label": "Charts",
                "content": {
                    "kind": "chart",
                    "id": "bad",
                    "chart": "scatter",
                    "x": [1.0, 2.0],
                    "y": [1.0]
                }
            }]
        }))
        .expect("app ir");

        assert!(matches!(
            app.validate(),
            Err(UiIrError::ChartLengthMismatch { .. })
        ));
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
    fn validates_bar_and_heatmap_charts() {
        let bar_missing_values: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{
                "id": "bar",
                "label": "Bar",
                "content": {"kind": "chart", "id": "bar", "chart": "bar", "categories": ["a"]}
            }]
        }))
        .unwrap();
        assert!(matches!(
            bar_missing_values.validate(),
            Err(UiIrError::MissingChartData {
                field: "values",
                ..
            })
        ));

        let bar_mismatch: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{
                "id": "bar",
                "label": "Bar",
                "content": {"kind": "chart", "id": "bar", "chart": "bar", "categories": ["a","b"], "values": [1.0]}
            }]
        }))
        .unwrap();
        assert!(matches!(
            bar_mismatch.validate(),
            Err(UiIrError::ChartLengthMismatch { .. })
        ));

        let heatmap_bad_dim: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{
                "id": "heatmap",
                "label": "Heatmap",
                "content": {"kind": "chart", "id": "h", "chart": "heatmap", "z": [1.0,2.0,3.0], "width_count": 2, "height_count": 2}
            }]
        }))
        .unwrap();
        assert!(matches!(
            heatmap_bad_dim.validate(),
            Err(UiIrError::HeatmapDimensionMismatch { .. })
        ));

        let heatmap_coordinates: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "heatmap", "label": "Heatmap", "content": {
                "kind": "chart", "id": "h", "chart": "heatmap", "z": [0.0, 1.0, 2.0, 3.0],
                "width_count": 2, "height_count": 2, "x": [20.0], "y": [0.0, 30.0]
            }}]
        }))
        .unwrap();
        assert!(matches!(heatmap_coordinates.validate(), Err(UiIrError::ChartLengthMismatch { .. })));

        let nan_series = serde_json::from_value::<PythonAppIr>(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "line", "label": "Line", "content": {
                "kind": "chart", "id": "line", "chart": "line", "x": [1.0], "y": [null]
            }}]
        }));
        // JSON cannot carry IEEE NaN; malformed/non-number data is rejected at
        // deserialization before it can reach the renderer.
        assert!(nan_series.is_err());
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
        assert!(matches!(invalid_mode.validate(), Err(UiIrError::InvalidPatch { .. })));
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
        assert!(matches!(invalid.validate(), Err(UiIrError::InvalidPatch { .. })));
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
    fn chart_series_patches_use_stable_series_ids() {
        let mut app: PythonAppIr = serde_json::from_value(serde_json::json!({
            "title": "Demo",
            "sections": [{"id": "charts", "label": "Charts", "content": {
                "kind": "chart", "id": "response", "chart": "line",
                "series": [{"id": "measured", "x": [20.0], "y": [0.0]}]
            }}]
        }))
        .unwrap();
        app.apply_patch_ops(&[crate::session::PatchOp::AppendChartSeries {
            chart_id: "response".into(),
            series_id: "measured".into(),
            x: vec![100.0],
            y: vec![1.5],
        }])
        .unwrap();
        let json = serde_json::to_value(&app).unwrap();
        assert_eq!(json["sections"][0]["content"]["series"][0]["x"], serde_json::json!([20.0, 100.0]));

        app.apply_patch_ops(&[crate::session::PatchOp::ReplaceChartSeries {
            chart_id: "response".into(),
            series: serde_json::json!({"id": "measured", "x": [50.0], "y": [-1.0], "label": "Latest"}),
        }])
        .unwrap();
        let json = serde_json::to_value(&app).unwrap();
        assert_eq!(json["sections"][0]["content"]["series"][0]["label"], "Latest");

        let before = app.clone();
        assert!(matches!(
            app.apply_patch_ops(&[crate::session::PatchOp::AppendChartSeries {
                chart_id: "response".into(),
                series_id: "measured".into(),
                x: vec![1.0],
                y: vec![],
            }]),
            Err(UiIrError::ChartLengthMismatch { .. })
        ));
        assert_eq!(app, before);
    }
}
