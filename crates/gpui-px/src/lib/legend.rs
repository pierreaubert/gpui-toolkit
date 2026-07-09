use crate::line::LegendPosition;

/// Marker shape used by a chart legend entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartLegendMarker {
    /// A short line segment, used for line-series legends.
    Line,
    /// A circle marker, used for scatter-series legends.
    Circle,
    /// A square marker, used for grouped bar legends.
    Square,
}

impl ChartLegendMarker {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Circle => "circle",
            Self::Square => "square",
        }
    }
}

/// One native legend entry exposed for release QA and host integrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartLegendItem {
    pub series_index: usize,
    pub label: String,
    pub color: u32,
    pub marker: ChartLegendMarker,
    pub hidden: bool,
    pub uses_secondary_axis: bool,
}

/// Machine-readable legend metadata for native-rendered chart families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartLegendSummary {
    pub chart_type: &'static str,
    pub visible: bool,
    pub position: LegendPosition,
    pub position_explicit: bool,
    pub items: Vec<ChartLegendItem>,
    pub description: String,
}

impl ChartLegendSummary {
    pub(crate) fn new(
        chart_type: &'static str,
        show_legend: bool,
        position: LegendPosition,
        position_explicit: bool,
        items: Vec<ChartLegendItem>,
    ) -> Self {
        let visible = show_legend && position != LegendPosition::Hidden && !items.is_empty();
        let item_count = items.len();
        let hidden_count = items.iter().filter(|item| item.hidden).count();
        let secondary_count = items.iter().filter(|item| item.uses_secondary_axis).count();
        let state = if visible { "visible" } else { "hidden" };
        let secondary = if secondary_count > 0 {
            format!(" {secondary_count} entries use the secondary Y axis.")
        } else {
            String::new()
        };
        let hidden = if hidden_count > 0 {
            format!(" {hidden_count} entries are hidden.")
        } else {
            String::new()
        };
        let description =
            format!("{chart_type} legend is {state} with {item_count} entries.{secondary}{hidden}");

        Self {
            chart_type,
            visible,
            position,
            position_explicit,
            items,
            description,
        }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}
