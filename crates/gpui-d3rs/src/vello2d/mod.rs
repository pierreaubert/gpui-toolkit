//! vello-backed 2D chart rendering.
//!
//! Charts emit draw commands into the backend-neutral [`ChartScene`] IR.
//! `gpu_scene` replays it into a `vello::Scene`; `cpu` replays it through
//! `vello_cpu`. The GPUI element and wgpu custom draw live behind the
//! `vello-gpui` feature.

mod cpu;
mod gpu_scene;
mod scene;

// Populated by Task 5 (files do not exist yet).
// #[cfg(feature = "vello-gpui")]
// mod element;
// #[cfg(feature = "vello-gpui")]
// mod wgpu_draw;

// Re-exported so integration tests and downstream crates (gpui-px) use the
// exact kurbo/peniko versions vello is compiled against.
pub use kurbo;
pub use peniko;

// Populated by Tasks 3-4 (symbols do not exist yet).
// pub use cpu::CpuRasterizer;
// pub use gpu_scene::to_vello_scene;
pub use scene::{ChartCmd, ChartScene};

// #[cfg(feature = "vello-gpui")]
// pub use element::{RasterBackend, VelloChartElement};
// #[cfg(feature = "vello-gpui")]
// pub use wgpu_draw::physical_size as wgpu_draw_physical_size;
