//! CPU replay of a [`ChartScene`] via vello_cpu's sparse-strips rasterizer.
//!
//! Universal fallback (Metal renderer, missing wgpu hook) and the
//! deterministic QA oracle for GPU output. Output is premultiplied RGBA8,
//! matching what `gpu2d/element.rs` hands to `window.paint_image`.

use crate::vello2d::{ChartCmd, ChartScene};
use vello_cpu::peniko::Brush;
use vello_cpu::{Pixmap, RenderContext, Resources};

/// Reusable vello_cpu rasterizer; recreates its context only on resize.
pub struct CpuRasterizer {
    ctx: RenderContext,
    resources: Resources,
    size: (u16, u16),
}

impl CpuRasterizer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            ctx: RenderContext::new(width, height),
            resources: Resources::new(),
            size: (width, height),
        }
    }

    /// Rasterize `scene` at `width`x`height`; returns premultiplied RGBA8
    /// bytes (`width*4` row stride). An empty scene yields a cleared buffer.
    pub fn rasterize(&mut self, scene: &ChartScene, width: u16, height: u16) -> Vec<u8> {
        if self.size != (width, height) {
            *self = Self::new(width, height);
        } else {
            self.ctx.reset();
        }
        for cmd in scene.commands() {
            match cmd {
                ChartCmd::Fill { path, brush, .. } => {
                    apply_paint(&mut self.ctx, brush);
                    self.ctx.fill_path(path);
                }
                ChartCmd::Stroke {
                    path,
                    stroke,
                    brush,
                } => {
                    apply_paint(&mut self.ctx, brush);
                    self.ctx.set_stroke(stroke.clone());
                    self.ctx.stroke_path(path);
                }
            }
        }
        self.ctx.flush();
        let mut pixmap = Pixmap::new(width, height);
        self.ctx.render(&mut pixmap, &mut self.resources);
        // `Pixmap` stores premultiplied RGBA8 in a byte-backed buffer. Copy
        // that buffer directly instead of allocating an iterator item for
        // every pixel channel.
        pixmap.data_as_u8_slice().to_vec()
    }
}

fn apply_paint(ctx: &mut RenderContext, brush: &Brush) {
    match brush {
        Brush::Solid(color) => ctx.set_paint(*color),
        Brush::Gradient(gradient) => ctx.set_paint(gradient.clone()),
        // Charts never paint images; vello_cpu image paints are out of scope.
        Brush::Image(_) => log::warn!("vello2d: image brush unsupported on CPU backend, skipped"),
    }
}
