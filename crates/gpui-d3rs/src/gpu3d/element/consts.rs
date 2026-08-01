pub(super) const MAX_SURFACE_RENDER_DIMENSION: f32 = 4096.0;

/// Bound the RGBA readback and RenderImage allocation for one surface frame.
/// The current GPUI bridge consumes a CPU-backed image, so an area budget is
/// needed in addition to the per-axis limit.
pub(super) const MAX_SURFACE_RENDER_PIXELS: f32 = 4_194_304.0;

pub(super) const ISOLINE_OCCLUSION_SAMPLE_PX: f32 = 2.0;

pub(super) const ISOLINE_DEPTH_EPSILON: f32 = 0.004;
