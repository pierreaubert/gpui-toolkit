//! Renderer selection shared by GPUI-backed 2D chart consumers.

/// High-level 2D renderer preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renderer2D {
    /// Render through the Vello scene/painter path.
    Vello,
    /// Render through the existing GPUI/gpu2d implementation.
    Legacy,
}

impl Default for Renderer2D {
    #[cfg(feature = "vello-gpui")]
    fn default() -> Self {
        Self::Vello
    }

    #[cfg(not(feature = "vello-gpui"))]
    fn default() -> Self {
        Self::Legacy
    }
}

impl Renderer2D {
    pub const fn is_vello(self) -> bool {
        matches!(self, Self::Vello)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_defaults_to_auto() {
        assert_eq!(VelloBackend::default(), VelloBackend::Auto);
    }

    #[test]
    fn renderer_default_matches_feature_contract() {
        #[cfg(feature = "vello-gpui")]
        assert_eq!(Renderer2D::default(), Renderer2D::Vello);
        #[cfg(not(feature = "vello-gpui"))]
        assert_eq!(Renderer2D::default(), Renderer2D::Legacy);
    }
}

/// Vello raster path selected after [`Renderer2D::Vello`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VelloBackend {
    /// Probe GPU custom-draw support (Metal on macOS, WGPU elsewhere) and
    /// fall back to CPU if unavailable.
    #[default]
    Auto,
    /// Force the zero-copy WGPU custom-draw path. Never dispatches on the
    /// Metal renderer (elements paint nothing there); prefer [`Self::Metal`]
    /// or [`Self::Auto`] on macOS.
    Wgpu,
    /// Force the Metal custom-draw path: GPU rasterization via a headless
    /// wgpu device, blitted into the Metal drawable. macOS only; resolves
    /// to CPU anywhere else.
    Metal,
    /// Force deterministic `vello_cpu` rasterization.
    Cpu,
}
