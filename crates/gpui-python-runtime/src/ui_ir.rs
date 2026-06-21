use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum UiIrError {
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PythonAppIr {
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
        if self.sections.is_empty() {
            return Err(UiIrError::EmptySections);
        }
        for section in &self.sections {
            section.validate()?;
        }
        Ok(())
    }
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
    Button(ButtonNode),
    Badge(BadgeNode),
    Metric(MetricNode),
    Progress(ProgressNode),
    Spinner(SpinnerNode),
    Tabs(TabsNode),
    Table(TableNode),
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
            Self::Chart(node) => node.validate(),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimpleNode {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StackNode {
    #[serde(default)]
    pub children: Vec<UiNode>,
    pub gap: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextNode {
    pub text: String,
    #[serde(default = "default_tone")]
    pub tone: String,
    pub level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionHeaderNode {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CardNode {
    pub title: Option<String>,
    #[serde(default)]
    pub children: Vec<UiNode>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonNode {
    pub label: String,
    pub action: Option<String>,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadgeNode {
    pub label: String,
    #[serde(default = "default_tone")]
    pub tone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricNode {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressNode {
    pub value: f32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpinnerNode {
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabsNode {
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub active: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableNode {
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
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
    pub z: Option<Vec<f64>>,
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
}

impl ChartNode {
    fn validate(&self) -> Result<(), UiIrError> {
        match self.chart {
            ChartKind::Scatter | ChartKind::Line => {
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
    pub width: Option<f32>,
    pub height: Option<f32>,
}

fn default_width() -> f32 {
    1240.0
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

        assert_eq!(app.title, "Demo");
        app.validate().expect("valid app");
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
}
