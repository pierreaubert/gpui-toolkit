//! Custom GPUI element painting one resolved orb frame through the
//! `d3rs::vello2d` layer (vello-on-wgpu zero-copy custom draw, with an
//! automatic `vello_cpu` fallback). Modeled on
//! `gpui_audio_kit::spectrum::SpectrumElement`.

use super::engine::OrbFrame;
use d3rs::vello2d::kurbo::Stroke;
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, VelloScenePainter};
use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Rgba, Style, Window, size,
};
use std::panic::Location;

/// Solid grayscale brush for one ink value. `white` is paper-theme ink
/// (0 = darkest on paper); dark substrates mirror it (`1 - white`).
fn ink_brush(white: f64, a: Option<f64>, dark: bool) -> Brush {
    let white = if dark { 1.0 - white } else { white };
    let gray = white.clamp(0.0, 1.0) as f32;
    let alpha = a.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
    Brush::Solid(Color::new([gray, gray, gray, alpha]))
}

/// Tint preset ink while retaining its light/dark depth fade and alpha.
fn tint_brush(tint: Rgba, white: f64, a: Option<f64>, dark: bool) -> Brush {
    let strength = white.clamp(0.0, 1.0) as f32;
    let background = if dark { 0.0 } else { 1.0 };
    let mix = |component: f32| background + (component - background) * strength;
    let alpha = tint.a * a.unwrap_or(1.0).clamp(0.0, 1.0) as f32;
    Brush::Solid(Color::new([mix(tint.r), mix(tint.g), mix(tint.b), alpha]))
}

/// Vello-painted element for a single finished [`OrbFrame`], laid out as a
/// fixed `size` × `size` square. Engine coordinates remain local to that
/// square: both Vello backends rasterize an element-local scene.
pub struct ThinkingOrbElement {
    id: ElementId,
    source_location: &'static Location<'static>,
    frame: OrbFrame,
    size: Pixels,
    dark: bool,
    dot_color: Option<Rgba>,
    dot_scale: f64,
    painter: VelloScenePainter,
}

impl ThinkingOrbElement {
    #[track_caller]
    pub fn new(
        id: ElementId,
        frame: OrbFrame,
        size: Pixels,
        dark: bool,
        dot_color: Option<Rgba>,
        dot_scale: f64,
    ) -> Self {
        Self {
            id,
            source_location: Location::caller(),
            frame,
            size,
            dark,
            dot_color,
            dot_scale,
            painter: VelloScenePainter::new(),
        }
    }
}

fn scene_for_frame(
    frame: &OrbFrame,
    dark: bool,
    dot_color: Option<Rgba>,
    dot_scale: f64,
) -> ChartScene {
    let mut scene = ChartScene::new();
    // Lines first, then the z-sorted (far→near) dots.
    for line in &frame.lines {
        scene.stroke_polyline(
            &[(line.x1, line.y1), (line.x2, line.y2)],
            Stroke::new(line.w),
            ink_brush(line.white, line.a, dark),
        );
    }
    for dot in &frame.dots {
        let brush = dot_color.map_or_else(
            || ink_brush(dot.white, dot.a, dark),
            |color| tint_brush(color, dot.white, dot.a, dark),
        );
        scene.fill_circle(dot.x, dot.y, dot.r * dot_scale, brush);
    }
    scene
}

impl IntoElement for ThinkingOrbElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ThinkingOrbElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        Some(self.source_location)
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = window.request_layout(
            Style {
                size: size(self.size.into(), self.size.into()),
                ..Default::default()
            },
            [],
            cx,
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let scene = scene_for_frame(&self.frame, self.dark, self.dot_color, self.dot_scale);
        self.painter.paint_retained(id, &scene, bounds, window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thinking_orb::engine;
    use crate::thinking_orb::presets::{OrbSize, OrbState, resolve_preset};
    use d3rs::vello2d::CpuRasterizer;

    #[test]
    fn cpu_scene_has_coverage_at_its_local_raster_origin() {
        let resolved = resolve_preset(OrbState::Working, OrbSize::Px64);
        let frame = engine::frame(resolved.mode, 96.0, 0.0, &resolved.opts);
        let scene = scene_for_frame(&frame, true, None, 1.0);

        let pixels = CpuRasterizer::new(96, 96).rasterize(&scene, 96, 96);
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel[3] != 0),
            "local orb geometry must cover the CPU fallback raster"
        );
    }
}
