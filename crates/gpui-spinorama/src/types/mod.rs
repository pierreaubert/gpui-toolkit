#![allow(unused_imports, reason = "facade re-exports for batch and generic consumers")]
pub mod config;
pub mod enums;
pub mod graph;
pub mod plot_curve;
pub mod trace;

pub use config::{BrushOverlay, SecondaryAxisConfig};
pub use d3rs::shape::LinePoint;
pub use enums::{ChartId, Colormap, ContourRenderMode, DirectivityPlane, LoadState, PlotSection};
pub use graph::{GRAPH_CATALOG, GraphDef, GraphRenderer, graph_for_measurement};
pub use plot_curve::PlotCurve;
pub use trace::{
    Annotation, AxisLayout, GenericTrace, PlotData, PlotLayout, TraceFill, TraceKind, TraceMode,
    parse_plotly_color,
};
