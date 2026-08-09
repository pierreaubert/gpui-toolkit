//! Custom-draw support for the WGPU renderer.
//!
//! This is a local renderer extension layered on top of GPUI's custom-draw
//! registry. See `PATCHES.md` for the vendored-upstream rationale.

use crate::WgpuContext;
use gpui::{Bounds, CustomDraw, Pixels};
use std::rc::Rc;

/// A GPUI custom draw that can record commands directly into the WGPU frame.
pub trait WgpuCustomDraw: CustomDraw {
    /// Record a render pass against `target` without submitting `encoder`.
    ///
    /// Implementations should use `bounds` as their scissor rectangle. The
    /// bounds are in GPUI pixels and `scale_factor` is the scale to use when
    /// converting them to device pixels.
    fn draw_wgpu(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bounds: Bounds<Pixels>,
        scale_factor: f32,
    );
}

/// Adapts a WGPU custom draw to GPUI's platform-independent registry.
///
/// The renderer downcasts the registry's `Rc<dyn CustomDraw>` to this
/// concrete adapter. Draws registered for another platform are therefore
/// ignored safely by the WGPU renderer.
pub struct WgpuCustomDrawAdapter(pub Rc<dyn WgpuCustomDraw>);

impl CustomDraw for WgpuCustomDrawAdapter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
