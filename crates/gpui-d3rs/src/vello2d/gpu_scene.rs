//! Replay a [`ChartScene`] into a `vello::Scene` for GPU rendering.

use crate::vello2d::{ChartCmd, ChartScene};
use vello::kurbo::Affine;

/// Build a fresh `vello::Scene` from the IR. Rebuilt per frame in
/// `draw_wgpu`; encoding is cheap relative to rasterization.
pub fn to_vello_scene(scene: &ChartScene) -> vello::Scene {
    let mut out = vello::Scene::new();
    for cmd in scene.commands() {
        match cmd {
            ChartCmd::Fill { path, fill, brush } => {
                out.fill(*fill, Affine::IDENTITY, brush, None, path);
            }
            ChartCmd::Stroke {
                path,
                stroke,
                brush,
            } => {
                out.stroke(stroke, Affine::IDENTITY, brush, None, path);
            }
        }
    }
    out
}
