//! Replay a [`ChartScene`] into a `vello::Scene` for GPU rendering.

use crate::vello2d::{ChartCmd, ChartScene};
use vello::kurbo::Affine;

/// Build a fresh `vello::Scene` from the IR, with `transform` applied to
/// every draw call (used to map logical scene coordinates onto physical
/// pixels). Rebuilt per frame in `draw_wgpu`; encoding is cheap relative to
/// rasterization.
pub fn to_vello_scene(scene: &ChartScene, transform: Affine) -> vello::Scene {
    let mut out = vello::Scene::new();
    for cmd in scene.commands() {
        match cmd {
            ChartCmd::Fill { path, fill, brush } => {
                out.fill(*fill, transform, brush, None, path);
            }
            ChartCmd::Stroke {
                path,
                stroke,
                brush,
            } => {
                out.stroke(stroke, transform, brush, None, path);
            }
        }
    }
    out
}
