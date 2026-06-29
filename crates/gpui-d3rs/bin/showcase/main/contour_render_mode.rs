/// Contour rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContourRenderMode {
    #[default]
    Isoline,
    Surface,
    Heatmap,
}

impl ContourRenderMode {
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Isoline => "Isoline",
            Self::Surface => "Surface",
            Self::Heatmap => "Heatmap",
        }
    }

    pub(super) fn next(&self) -> Self {
        match self {
            Self::Isoline => Self::Surface,
            Self::Surface => Self::Heatmap,
            Self::Heatmap => Self::Isoline,
        }
    }
}
