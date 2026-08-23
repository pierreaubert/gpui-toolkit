//! vello-backed 2D chart rendering.
//!
//! Charts emit draw commands into the backend-neutral [`ChartScene`] IR.
//! `gpu_scene` replays it into a `vello::Scene`; `cpu` replays it through
//! `vello_cpu`. The GPUI element and wgpu custom draw live behind the
//! `vello-gpui` feature.

mod cpu;
mod gpu_scene;
mod scene;

#[cfg(feature = "vello-gpui")]
mod element;
#[cfg(feature = "vello-gpui")]
mod wgpu_draw;

// Re-exported so integration tests and downstream crates (gpui-px) use the
// exact kurbo/peniko versions vello is compiled against.
pub use crate::render2d::VelloBackend;
pub use kurbo;
pub use peniko;

pub use cpu::CpuRasterizer;
pub use gpu_scene::to_vello_scene;
pub use scene::{ChartCmd, ChartScene};

#[cfg(feature = "vello-gpui")]
pub use element::{
    RasterBackend, RetainedVelloChart, RetainedVelloChartElement, VelloChartElement,
    VelloScenePainter,
};
#[cfg(feature = "vello-gpui")]
pub use wgpu_draw::{
    clip_src_rect as wgpu_draw_clip_src_rect, physical_size as wgpu_draw_physical_size,
    scene_scale as wgpu_draw_scene_scale,
};
