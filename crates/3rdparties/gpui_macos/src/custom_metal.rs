//! Embedder custom-draw subtrait for the Metal renderer.
//!
//! This is a local extension to the vendored zed v1.9.0 `gpui` renderer; see
//! `PATCHES.md` for the corresponding patch record.

use gpui::{Bounds, CustomDraw, Pixels};
use metal::{CommandBufferRef, DeviceRef, TextureRef};
use std::{any::Any, rc::Rc};

/// A custom draw callback that records Metal commands into GPUI's command
/// buffer and renders directly into the current drawable texture.
pub trait MetalCustomDraw: CustomDraw {
    /// Record a render pass against `drawable_texture`.
    ///
    /// The callback owns the render encoder it creates from `command_buffer`.
    /// It must use a `Load` color attachment action and scissor its draws to
    /// `bounds` (converted to device pixels with `scale_factor`). It must not
    /// commit or submit `command_buffer`.
    fn draw_metal(
        &self,
        device: &DeviceRef,
        command_buffer: &CommandBufferRef,
        drawable_texture: &TextureRef,
        bounds: Bounds<Pixels>,
        scale_factor: f32,
    );
}

/// Registry adapter for a Metal custom draw.
///
/// GPUI's registry stores `Rc<dyn CustomDraw>`. The adapter gives the Metal
/// renderer a concrete type to downcast while retaining the Metal-specific
/// callback vtable for embedders.
pub struct MetalCustomDrawAdapter(pub Rc<dyn MetalCustomDraw>);

impl CustomDraw for MetalCustomDrawAdapter {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
