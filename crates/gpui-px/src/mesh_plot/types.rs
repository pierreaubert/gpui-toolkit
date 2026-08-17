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

#[derive(Debug, Clone, PartialEq)]
pub struct Axes2d {
    pub equal_aspect: bool,
    horizontal_label: Option<Arc<str>>,
    vertical_label: Option<Arc<str>>,
    unit_label: Option<Arc<str>>,
    horizontal_range: Option<[f64; 2]>,
    vertical_range: Option<[f64; 2]>,
    show_grid: bool,
}
impl Axes2d {
    pub fn equal_aspect() -> Self {
        Self {
            equal_aspect: true,
            horizontal_label: None,
            vertical_label: None,
            unit_label: None,
            horizontal_range: None,
            vertical_range: None,
            show_grid: true,
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

    /// Set explicit horizontal and vertical data ranges.
    ///
    /// The ranges must be finite and strictly increasing. When omitted, each
    /// axis is fitted to the finite mesh bounds.
    pub fn ranges(mut self, horizontal: [f64; 2], vertical: [f64; 2]) -> Self {
        self.horizontal_range = Some(horizontal);
        self.vertical_range = Some(vertical);
        self
    }

    /// Set an explicit horizontal data range while keeping the vertical axis
    /// fitted to the mesh bounds.
    pub fn horizontal_range(mut self, min: f64, max: f64) -> Self {
        self.horizontal_range = Some([min, max]);
        self
    }

    /// Set an explicit vertical data range while keeping the horizontal axis
    /// fitted to the mesh bounds.
    pub fn vertical_range(mut self, min: f64, max: f64) -> Self {
        self.vertical_range = Some([min, max]);
        self
    }

    /// Show or hide the plot grid independently from the axes and labels.
    pub fn grid(mut self, show: bool) -> Self {
        self.show_grid = show;
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

    pub(crate) fn configured_ranges(&self) -> (Option<[f64; 2]>, Option<[f64; 2]>) {
        (self.horizontal_range, self.vertical_range)
    }

    pub(crate) fn show_grid(&self) -> bool {
        self.show_grid
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlotInteractions {
    #[default]
    InspectAndNavigate,
    None,
    Custom(u8),
}
impl PlotInteractions {
    const PAN: u8 = 1 << 0;
    const ZOOM: u8 = 1 << 1;
    const INSPECT: u8 = 1 << 2;
    const SELECT: u8 = 1 << 3;
    const RESET: u8 = 1 << 4;
    const FIT: u8 = 1 << 5;

    pub fn inspect_and_navigate() -> Self {
        Self::InspectAndNavigate
    }
    pub fn none() -> Self {
        Self::None
    }

    pub fn from_names(names: &[String]) -> Result<Self, String> {
        if names.is_empty() {
            return Ok(Self::None);
        }
        let mut flags = 0;
        for name in names {
            let flag = match name.as_str() {
                "pan" => Self::PAN,
                "zoom" => Self::ZOOM,
                "inspect" => Self::INSPECT,
                "select" => Self::SELECT,
                "reset" => Self::RESET,
                "fit" => Self::FIT,
                _ => return Err(format!("unsupported mesh plot interaction {name:?}")),
            };
            if flags & flag != 0 {
                return Err(format!("duplicate mesh plot interaction {name:?}"));
            }
            flags |= flag;
        }
        Ok(Self::Custom(flags))
    }

    pub fn is_interactive(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn allows_pan(self) -> bool {
        matches!(self, Self::InspectAndNavigate)
            || matches!(self, Self::Custom(flags) if flags & Self::PAN != 0)
    }

    pub fn allows_zoom(self) -> bool {
        matches!(self, Self::InspectAndNavigate)
            || matches!(self, Self::Custom(flags) if flags & Self::ZOOM != 0)
    }

    pub fn allows_inspect(self) -> bool {
        matches!(self, Self::InspectAndNavigate)
            || matches!(self, Self::Custom(flags) if flags & Self::INSPECT != 0)
    }

    pub fn allows_select(self) -> bool {
        matches!(self, Self::InspectAndNavigate)
            || matches!(self, Self::Custom(flags) if flags & Self::SELECT != 0)
    }

    pub fn allows_reset(self) -> bool {
        matches!(self, Self::InspectAndNavigate)
            || matches!(self, Self::Custom(flags) if flags & Self::RESET != 0)
    }

    pub fn allows_fit(self) -> bool {
        matches!(self, Self::InspectAndNavigate)
            || matches!(self, Self::Custom(flags) if flags & Self::FIT != 0)
    }

    pub fn controls_summary(self) -> String {
        let mut controls = Vec::new();
        if self.allows_inspect() {
            controls.push("inspect");
        }
        if self.allows_select() {
            controls.push("select");
        }
        if self.allows_pan() {
            controls.push("pan");
        }
        if self.allows_zoom() {
            controls.push("zoom");
        }
        if self.allows_fit() {
            controls.push("fit");
        }
        if self.allows_reset() {
            controls.push("reset");
        }
        if controls.is_empty() {
            "Available controls: none.".into()
        } else if controls.len() == 1 {
            format!("Available controls: {}.", controls[0])
        } else if controls.len() == 2 {
            format!("Available controls: {} and {}.", controls[0], controls[1])
        } else {
            let last = controls.pop().expect("controls has at least three items");
            format!("Available controls: {}, and {last}.", controls.join(", "))
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_titles_cover_defaults_custom_labels_and_units() {
        let defaults = Axes2d::default();
        assert_eq!(
            defaults.titles(
                &MeshPlotView::Planar {
                    horizontal: CoordinateAxis::X,
                    vertical: CoordinateAxis::Z,
                },
                CoordinateAxis::X,
                CoordinateAxis::Z,
            ),
            ("x".to_string(), "z".to_string())
        );
        assert_eq!(
            defaults.titles(
                &MeshPlotView::AxisymmetricSection {
                    radial: CoordinateAxis::X,
                    axial: CoordinateAxis::Y,
                },
                CoordinateAxis::X,
                CoordinateAxis::Y,
            ),
            ("r".to_string(), "z".to_string())
        );
        let custom = Axes2d::equal_aspect().labels("radius", "height").unit("m");
        assert_eq!(
            custom.titles(
                &MeshPlotView::Surface3d,
                CoordinateAxis::Y,
                CoordinateAxis::Z
            ),
            ("radius (m)".to_string(), "height (m)".to_string())
        );
        assert!(!Axes2d::equal_aspect().fill_aspect().equal_aspect);

        let configured = Axes2d::equal_aspect()
            .ranges([-1.0, 2.0], [3.0, 8.0])
            .grid(false);
        assert_eq!(
            configured.configured_ranges(),
            (Some([-1.0, 2.0]), Some([3.0, 8.0]))
        );
        assert!(!configured.show_grid());
    }

    #[test]
    fn public_type_constructors_preserve_their_enum_contracts() {
        assert_eq!(Wireframe::overlay(), Wireframe::Overlay);
        assert_eq!(Wireframe::hidden(), Wireframe::Hidden);
        assert_eq!(
            PlotInteractions::inspect_and_navigate(),
            PlotInteractions::InspectAndNavigate
        );
        assert_eq!(PlotInteractions::none(), PlotInteractions::None);
        let custom =
            PlotInteractions::from_names(&["pan".into(), "zoom".into(), "inspect".into()]).unwrap();
        assert!(custom.is_interactive());
        assert!(custom.allows_pan());
        assert!(custom.allows_zoom());
        assert!(custom.allows_inspect());
        assert!(!custom.allows_select());
        assert_eq!(
            custom.controls_summary(),
            "Available controls: inspect, pan, and zoom."
        );
        assert!(PlotInteractions::from_names(&["pan".into(), "pan".into()]).is_err());
        assert_eq!(coordinate_label(CoordinateAxis::X), "x");
        assert_eq!(coordinate_label(CoordinateAxis::Y), "y");
        assert_eq!(coordinate_label(CoordinateAxis::Z), "z");
    }
}
