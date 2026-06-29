use std::sync::Arc;

/// A single series in a bar chart (for grouped/stacked bars)
#[derive(Debug, Clone)]
pub(super) struct BarSeries {
    pub(super) values: Arc<[f64]>,
    pub(super) label: Option<String>,
    pub(super) color: u32,
    #[allow(dead_code)] // Reserved for future per-series opacity support
    pub(super) opacity: f32,
}
