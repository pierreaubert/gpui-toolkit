/// Target position for a chart annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum ChartAnnotationTarget {
    /// Annotation pinned to an x/y chart coordinate.
    Point { x: f64, y: f64 },
    /// Annotation tied to a vertical x-axis marker.
    XValue { x: f64 },
    /// Annotation tied to a horizontal y-axis marker.
    YValue { y: f64 },
    /// Annotation tied to a categorical bar/chart entry.
    Category { category: String },
}

impl ChartAnnotationTarget {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Point { .. } => "point",
            Self::XValue { .. } => "x-value",
            Self::YValue { .. } => "y-value",
            Self::Category { .. } => "category",
        }
    }
}

/// Non-rendering chart annotation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartAnnotation {
    pub id: String,
    pub label: String,
    pub target: ChartAnnotationTarget,
    pub color: Option<u32>,
    pub series_index: Option<usize>,
}

impl ChartAnnotation {
    pub fn point(id: impl Into<String>, label: impl Into<String>, x: f64, y: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            target: ChartAnnotationTarget::Point { x, y },
            color: None,
            series_index: None,
        }
    }

    pub fn x_value(id: impl Into<String>, label: impl Into<String>, x: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            target: ChartAnnotationTarget::XValue { x },
            color: None,
            series_index: None,
        }
    }

    pub fn y_value(id: impl Into<String>, label: impl Into<String>, y: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            target: ChartAnnotationTarget::YValue { y },
            color: None,
            series_index: None,
        }
    }

    pub fn category(
        id: impl Into<String>,
        label: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            target: ChartAnnotationTarget::Category {
                category: category.into(),
            },
            color: None,
            series_index: None,
        }
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn series_index(mut self, series_index: usize) -> Self {
        self.series_index = Some(series_index);
        self
    }
}

/// Machine-readable annotation metadata for chart QA and host integrations.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartAnnotationSummary {
    pub chart_type: &'static str,
    pub annotations: Vec<ChartAnnotation>,
    pub description: String,
}

impl ChartAnnotationSummary {
    pub(crate) fn new(chart_type: &'static str, annotations: Vec<ChartAnnotation>) -> Self {
        let count = annotations.len();
        let mut target_kinds = annotations
            .iter()
            .map(|annotation| annotation.target.kind())
            .collect::<Vec<_>>();
        target_kinds.sort_unstable();
        target_kinds.dedup();
        let description = if count == 0 {
            format!("{chart_type} chart has no annotations.")
        } else {
            format!(
                "{chart_type} chart has {count} annotations across {} target types: {}.",
                target_kinds.len(),
                target_kinds.join(", ")
            )
        };

        Self {
            chart_type,
            annotations,
            description,
        }
    }

    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }
}
