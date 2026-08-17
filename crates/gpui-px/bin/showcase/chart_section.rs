#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ChartSection {
    #[default]
    Overview,
    Scatter,
    Line,
    Bar,
    BoxPlot,
    LogScales,
    Heatmap,
    Contour,
    Isoline,
    Treemap,
    Gallery,
    // Appended last so existing wasm-visual nav click coordinates for the
    // sections above stay valid.
    #[cfg(feature = "vello")]
    ScatterVello,
}

impl ChartSection {
    pub(super) fn all() -> &'static [ChartSection] {
        &[
            ChartSection::Overview,
            ChartSection::Scatter,
            ChartSection::Line,
            ChartSection::Bar,
            ChartSection::BoxPlot,
            ChartSection::LogScales,
            ChartSection::Heatmap,
            ChartSection::Contour,
            ChartSection::Isoline,
            ChartSection::Treemap,
            ChartSection::Gallery,
            #[cfg(feature = "vello")]
            ChartSection::ScatterVello,
        ]
    }

    pub(super) fn label(&self) -> &'static str {
        match self {
            ChartSection::Overview => "Overview",
            ChartSection::Scatter => "Scatter",
            ChartSection::Line => "Line",
            ChartSection::Bar => "Bar",
            ChartSection::BoxPlot => "Box Plot",
            ChartSection::LogScales => "Log Scales",
            ChartSection::Heatmap => "Heatmap",
            ChartSection::Contour => "Contour",
            ChartSection::Isoline => "Isoline",
            ChartSection::Treemap => "Treemap",
            ChartSection::Gallery => "Gallery",
            #[cfg(feature = "vello")]
            ChartSection::ScatterVello => "Scatter (vello)",
        }
    }
}
