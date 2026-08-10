use d3rs::mesh::{ContourLevels, CoordinateAxis, RevolveSpec};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum MeshPlotView {
    Planar {
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
    },
    AxisymmetricSection {
        radial: CoordinateAxis,
        axial: CoordinateAxis,
    },
    AxisymmetricRevolve(RevolveSpec),
    Surface3d,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldInterpolation {
    Smooth,
    Flat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MeshRenderMode {
    Mesh,
    ScalarFill { interpolation: FieldInterpolation },
    FilledContours { levels: ContourLevels },
    Isolines { levels: ContourLevels },
    FillAndIsolines { levels: ContourLevels },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wireframe {
    Overlay,
    Hidden,
}
impl Wireframe {
    pub fn overlay() -> Self {
        Self::Overlay
    }
    pub fn hidden() -> Self {
        Self::Hidden
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axes2d {
    pub equal_aspect: bool,
    horizontal_label: Option<Arc<str>>,
    vertical_label: Option<Arc<str>>,
    unit_label: Option<Arc<str>>,
}
impl Axes2d {
    pub fn equal_aspect() -> Self {
        Self {
            equal_aspect: true,
            horizontal_label: None,
            vertical_label: None,
            unit_label: None,
        }
    }
    pub fn fill_aspect(mut self) -> Self {
        self.equal_aspect = false;
        self
    }
    /// Set the horizontal and vertical coordinate labels.
    pub fn labels(mut self, horizontal: impl Into<String>, vertical: impl Into<String>) -> Self {
        self.horizontal_label = Some(Arc::from(horizontal.into()));
        self.vertical_label = Some(Arc::from(vertical.into()));
        self
    }
    /// Apply one physical unit label to both coordinate titles.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit_label = Some(Arc::from(unit.into()));
        self
    }

    pub(crate) fn titles(
        &self,
        view: &MeshPlotView,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
    ) -> (String, String) {
        let default_horizontal = match view {
            MeshPlotView::AxisymmetricSection { .. } | MeshPlotView::AxisymmetricRevolve(_) => "r",
            _ => coordinate_label(horizontal),
        };
        let default_vertical = match view {
            MeshPlotView::AxisymmetricSection { .. } | MeshPlotView::AxisymmetricRevolve(_) => "z",
            _ => coordinate_label(vertical),
        };
        let horizontal = self
            .horizontal_label
            .as_deref()
            .unwrap_or(default_horizontal);
        let vertical = self.vertical_label.as_deref().unwrap_or(default_vertical);
        let with_unit = |label: &str| {
            self.unit_label
                .as_deref()
                .map_or_else(|| label.to_string(), |unit| format!("{label} ({unit})"))
        };
        (with_unit(horizontal), with_unit(vertical))
    }
}
impl Default for Axes2d {
    fn default() -> Self {
        Self::equal_aspect()
    }
}

fn coordinate_label(axis: CoordinateAxis) -> &'static str {
    match axis {
        CoordinateAxis::X => "x",
        CoordinateAxis::Y => "y",
        CoordinateAxis::Z => "z",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotInteractions {
    InspectAndNavigate,
    None,
}
impl PlotInteractions {
    pub fn inspect_and_navigate() -> Self {
        Self::InspectAndNavigate
    }
    pub fn none() -> Self {
        Self::None
    }
}
impl Default for PlotInteractions {
    fn default() -> Self {
        Self::InspectAndNavigate
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshPlotPick {
    pub plot_id: Arc<str>,
    pub mesh_id: Arc<str>,
    pub cell_index: u32,
    pub cell_id: Option<u64>,
    pub nearest_vertex_index: Option<u32>,
    pub vertex_id: Option<u64>,
    pub world_position: [f64; 3],
    pub displayed_value: Option<f64>,
    pub field_id: Option<Arc<str>>,
}
