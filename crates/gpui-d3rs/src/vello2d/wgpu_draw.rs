//! Zero-copy vello backend: render the scene to an offscreen texture with
//! the shared wgpu device, then alpha-composite into the GPUI frame.
//!
//! Skeleton for Task 6: the type, constructor, registry adapter, and trait
//! impls exist so `VelloChartElement` compiles; Task 7 fills in the GPU body
//! of `draw_wgpu` (offscreen texture + vello renderer + composite pass).

use crate::vello2d::ChartScene;
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::RefCell;
use std::rc::Rc;

/// Shared scene handle; GPU state is added by Task 7.
pub struct WgpuVelloDraw {
    scene: Rc<RefCell<ChartScene>>,
}

/// Bounds size (GPUI px) → physical texture size, clamped to >= 1.
pub fn physical_size(width: f32, height: f32, scale_factor: f32) -> [u32; 2] {
    [
        (width * scale_factor).max(1.0) as u32,
        (height * scale_factor).max(1.0) as u32,
    ]
}

impl WgpuVelloDraw {
    pub fn new(scene: Rc<RefCell<ChartScene>>) -> Self {
        Self { scene }
    }

    pub fn into_custom_draw(self) -> Rc<dyn CustomDraw> {
        Rc::new(WgpuCustomDrawAdapter(Rc::new(self)))
    }
}

impl CustomDraw for WgpuVelloDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WgpuCustomDraw for WgpuVelloDraw {
    fn draw_wgpu(
        &self,
        _ctx: &WgpuContext,
        _encoder: &mut wgpu::CommandEncoder,
        _target: &wgpu::TextureView,
        _target_format: wgpu::TextureFormat,
        _target_size: [u32; 2],
        _bounds: Bounds<Pixels>,
        _scale_factor: f32,
    ) {
        // Task 7: render_to_texture into an offscreen texture, then composite.
        let _ = &self.scene;
    }
}
