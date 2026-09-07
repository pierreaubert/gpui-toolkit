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
            ChartCmd::Text { runs } => {
                for run in runs {
                    // Rotation composes with the logical→physical map; hint
                    // only axis-aligned runs (grid-fitting rotated outlines
                    // smears strokes across pixels).
                    let composed = transform * run.transform;
                    out.draw_glyphs(&run.font)
                        .font_size(run.size)
                        .transform(composed)
                        .hint(is_axis_aligned(composed))
                        .brush(&run.brush)
                        .draw(vello::peniko::Fill::NonZero, run.glyphs.iter().copied());
                }
            }
        }
    }
    out
}

/// Whether `transform` has no rotation or shear (scale + translate only).
fn is_axis_aligned(transform: Affine) -> bool {
    let coeffs = transform.as_coeffs();
    coeffs[1] == 0.0 && coeffs[2] == 0.0
}
