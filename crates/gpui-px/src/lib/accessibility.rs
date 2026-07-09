use super::ScaleType;
use gpui::ElementId;
use gpui_ui_kit::{
    AccessibilityBridgeSnapshot, AccessibilityNode, AccessibilityTree, AriaProps, AriaRole,
};

/// Machine-readable and screen-reader-friendly chart summary.
///
/// GPUI accessibility bridges can consume `description` directly, while tests
/// and downstream apps can inspect the structured fields for release QA.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartAccessibilitySummary {
    pub chart_type: &'static str,
    pub title: Option<String>,
    pub series_count: usize,
    pub datum_count: usize,
    pub x_range: Option<[f64; 2]>,
    pub y_range: Option<[f64; 2]>,
    pub value_range: Option<[f64; 2]>,
    pub x_scale: Option<ScaleType>,
    pub y_scale: Option<ScaleType>,
    pub series_labels: Vec<String>,
    pub description: String,
}

impl ChartAccessibilitySummary {
    /// Return the accessible label a host/native adapter should use for this chart.
    pub fn accessible_label(&self) -> String {
        self.title
            .as_ref()
            .filter(|title| !title.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{} chart", self.chart_type))
    }

    /// Return compact value text suitable for native accessibility adapters.
    pub fn accessible_value_text(&self) -> String {
        let mut parts = vec![
            format!("{} series", self.series_count),
            format!("{} data points", self.datum_count),
        ];

        if let Some(scale) = self.x_scale {
            parts.push(format!("x scale {}", format_scale(scale)));
        }
        if let Some(scale) = self.y_scale {
            parts.push(format!("y scale {}", format_scale(scale)));
        }

        push_range_part(&mut parts, "x", self.x_range);
        push_range_part(&mut parts, "y", self.y_range);
        push_range_part(&mut parts, "value", self.value_range);

        if !self.series_labels.is_empty() {
            parts.push(format!("series {}", self.series_labels.join(", ")));
        }

        parts.join("; ")
    }

    /// Convert this chart summary into a UI-kit accessibility tree.
    ///
    /// The tree uses a single image node because chart internals remain owned by
    /// the renderer/host app, while the summary carries the chart type, data
    /// volume, finite ranges, scale types, and series labels needed by native
    /// accessibility adapters.
    pub fn to_accessibility_tree(&self, element_id: impl Into<ElementId>) -> AccessibilityTree {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: element_id.into(),
            label: self.accessible_label().into(),
            props: AriaProps::with_role(AriaRole::Img)
                .description(self.description.clone())
                .value_text(self.accessible_value_text()),
        });
        tree
    }

    /// Convert this chart summary into a platform-neutral bridge snapshot.
    pub fn to_bridge_snapshot(
        &self,
        element_id: impl Into<ElementId>,
    ) -> AccessibilityBridgeSnapshot {
        self.to_accessibility_tree(element_id).to_bridge_snapshot()
    }
}

fn push_range_part(parts: &mut Vec<String>, label: &str, range: Option<[f64; 2]>) {
    if let Some([min, max]) = range {
        parts.push(format!("{label} range {min:.3} to {max:.3}"));
    }
}

pub(crate) fn finite_range<'a>(values: impl IntoIterator<Item = &'a f64>) -> Option<[f64; 2]> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut seen = false;

    for value in values {
        if value.is_finite() {
            min = min.min(*value);
            max = max.max(*value);
            seen = true;
        }
    }

    seen.then_some([min, max])
}

pub(crate) fn finite_range_owned(values: impl IntoIterator<Item = f64>) -> Option<[f64; 2]> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut seen = false;

    for value in values {
        if value.is_finite() {
            min = min.min(value);
            max = max.max(value);
            seen = true;
        }
    }

    seen.then_some([min, max])
}

pub(crate) fn indexed_label(label: &Option<String>, fallback: &str, index: usize) -> String {
    label
        .clone()
        .unwrap_or_else(|| format!("{fallback} {}", index + 1))
}

pub(crate) fn format_scale(scale: ScaleType) -> &'static str {
    match scale {
        ScaleType::Linear => "linear",
        ScaleType::Log => "log",
    }
}

pub(crate) fn format_range(label: &str, range: Option<[f64; 2]>) -> String {
    range.map_or_else(
        || format!("{label} range unavailable"),
        |[min, max]| format!("{label} range {min:.3} to {max:.3}"),
    )
}
