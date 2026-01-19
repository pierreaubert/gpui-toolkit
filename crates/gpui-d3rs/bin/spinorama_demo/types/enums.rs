/// Contour rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContourRenderMode {
    #[default]
    Isoline,
    Surface,
    Heatmap,
}

impl ContourRenderMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Isoline => "Isoline",
            Self::Surface => "Surface",
            Self::Heatmap => "Heatmap",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Isoline => Self::Surface,
            Self::Surface => Self::Heatmap,
            Self::Heatmap => Self::Isoline,
        }
    }
}

/// Colormap selection for contour plots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Colormap {
    #[default]
    Viridis,
    Plasma,
    Magma,
    Inferno,
    Heat,
    Coolwarm,
}

impl Colormap {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Magma => "Magma",
            Self::Inferno => "Inferno",
            Self::Heat => "Heat",
            Self::Coolwarm => "Coolwarm",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Viridis => Self::Plasma,
            Self::Plasma => Self::Magma,
            Self::Magma => Self::Inferno,
            Self::Inferno => Self::Heat,
            Self::Heat => Self::Coolwarm,
            Self::Coolwarm => Self::Viridis,
        }
    }

    pub fn color_scale(
        &self,
    ) -> impl Fn(f64) -> d3rs::color::D3Color + Send + Sync + Clone + 'static {
        let colormap = *self;
        move |t: f64| {
            let t = t.clamp(0.0, 1.0);
            match colormap {
                Colormap::Viridis => {
                    let colors = [
                        d3rs::color::D3Color::from_hex(0x440154),
                        d3rs::color::D3Color::from_hex(0x482878),
                        d3rs::color::D3Color::from_hex(0x3e4a89),
                        d3rs::color::D3Color::from_hex(0x31688e),
                        d3rs::color::D3Color::from_hex(0x26838f),
                        d3rs::color::D3Color::from_hex(0x1f9e89),
                        d3rs::color::D3Color::from_hex(0x35b779),
                        d3rs::color::D3Color::from_hex(0x6ece58),
                        d3rs::color::D3Color::from_hex(0xb5de2b),
                        d3rs::color::D3Color::from_hex(0xfde725),
                    ];
                    super::super::utils::interpolate_colors(&colors, t)
                }
                Colormap::Plasma => {
                    let colors = [
                        d3rs::color::D3Color::from_hex(0x0d0887),
                        d3rs::color::D3Color::from_hex(0x46039f),
                        d3rs::color::D3Color::from_hex(0x7201a8),
                        d3rs::color::D3Color::from_hex(0x9c179e),
                        d3rs::color::D3Color::from_hex(0xbd3786),
                        d3rs::color::D3Color::from_hex(0xd8576b),
                        d3rs::color::D3Color::from_hex(0xed7953),
                        d3rs::color::D3Color::from_hex(0xfb9f3a),
                        d3rs::color::D3Color::from_hex(0xfdca26),
                        d3rs::color::D3Color::from_hex(0xf0f921),
                    ];
                    super::super::utils::interpolate_colors(&colors, t)
                }
                Colormap::Magma => {
                    let colors = [
                        d3rs::color::D3Color::from_hex(0x000004),
                        d3rs::color::D3Color::from_hex(0x180f3d),
                        d3rs::color::D3Color::from_hex(0x440f76),
                        d3rs::color::D3Color::from_hex(0x721f81),
                        d3rs::color::D3Color::from_hex(0x9e2f7f),
                        d3rs::color::D3Color::from_hex(0xcd4071),
                        d3rs::color::D3Color::from_hex(0xf1605d),
                        d3rs::color::D3Color::from_hex(0xfd9668),
                        d3rs::color::D3Color::from_hex(0xfeca8d),
                        d3rs::color::D3Color::from_hex(0xfcfdbf),
                    ];
                    super::super::utils::interpolate_colors(&colors, t)
                }
                Colormap::Inferno => {
                    let colors = [
                        d3rs::color::D3Color::from_hex(0x000004),
                        d3rs::color::D3Color::from_hex(0x1b0c41),
                        d3rs::color::D3Color::from_hex(0x4a0c6b),
                        d3rs::color::D3Color::from_hex(0x781c6d),
                        d3rs::color::D3Color::from_hex(0xa52c60),
                        d3rs::color::D3Color::from_hex(0xcf4446),
                        d3rs::color::D3Color::from_hex(0xed6925),
                        d3rs::color::D3Color::from_hex(0xfb9b06),
                        d3rs::color::D3Color::from_hex(0xf7d13d),
                        d3rs::color::D3Color::from_hex(0xfcffa4),
                    ];
                    super::super::utils::interpolate_colors(&colors, t)
                }
                Colormap::Heat => {
                    // Blue -> White -> Red
                    if t < 0.5 {
                        let local_t = t * 2.0;
                        d3rs::color::D3Color::from_hex(0x0571b0)
                            .interpolate(&d3rs::color::D3Color::from_hex(0xf7f7f7), local_t as f32)
                    } else {
                        let local_t = (t - 0.5) * 2.0;
                        d3rs::color::D3Color::from_hex(0xf7f7f7)
                            .interpolate(&d3rs::color::D3Color::from_hex(0xca0020), local_t as f32)
                    }
                }
                Colormap::Coolwarm => {
                    let colors = [
                        d3rs::color::D3Color::from_hex(0x3b4cc0),
                        d3rs::color::D3Color::from_hex(0x6688ee),
                        d3rs::color::D3Color::from_hex(0x99bbff),
                        d3rs::color::D3Color::from_hex(0xc9d8ef),
                        d3rs::color::D3Color::from_hex(0xf7f7f7),
                        d3rs::color::D3Color::from_hex(0xf6cdc4),
                        d3rs::color::D3Color::from_hex(0xee9977),
                        d3rs::color::D3Color::from_hex(0xd6604d),
                        d3rs::color::D3Color::from_hex(0xb40426),
                    ];
                    super::super::utils::interpolate_colors(&colors, t)
                }
            }
        }
    }
}

/// Chart identifiers for tracking brush interactions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ChartId {
    FreqSpl,
    SplContour,
    DirectivityContour,
}

/// View sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlotSection {
    #[default]
    CEA2034,
    HorizontalSPL,
    VerticalSPL,
    Contour,
    PolarDirectivity,
    Surface3D,
    SurfaceSphere,
    PolarContour,
}

impl PlotSection {
    pub fn all() -> Vec<Self> {
        vec![
            Self::CEA2034,
            Self::HorizontalSPL,
            Self::VerticalSPL,
            Self::Contour,
            Self::PolarDirectivity,
            Self::Surface3D,
            Self::SurfaceSphere,
            Self::PolarContour,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::CEA2034 => "CEA2034 (Spinorama)",
            Self::HorizontalSPL => "Horizontal SPL",
            Self::VerticalSPL => "Vertical SPL",
            Self::Contour => "Contour Plot",
            Self::PolarDirectivity => "Polar Directivity",
            Self::Surface3D => "3D Surface",
            Self::SurfaceSphere => "Sphere Plot",
            Self::PolarContour => "Polar Contour",
        }
    }
}

/// Directivity plane selection for polar plots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DirectivityPlane {
    #[default]
    Horizontal,
    Vertical,
}

#[allow(dead_code)]
impl DirectivityPlane {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

/// Loading state for async data
#[derive(Debug, Clone, PartialEq)]
pub enum LoadState {
    Idle,
    Loading,
    Loaded,
    Error(String),
}
