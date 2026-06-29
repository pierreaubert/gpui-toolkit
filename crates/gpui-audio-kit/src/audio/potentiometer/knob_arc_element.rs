use gpui::prelude::*;
use gpui::*;

/// Custom element that paints an arc indicating the current knob value.
///
/// The arc sweeps from the start angle (7:30 position) clockwise to the
/// current value angle. It is rendered as a filled annular sector using
/// `PathBuilder`, sitting just outside the knob circle.
pub(super) struct KnobArcElement {
    /// Width of the container (matches knob_container)
    pub(super) container_width: f32,
    /// Height of the container (matches knob_container)
    pub(super) container_height: f32,
    /// Horizontal offset of the knob within the container
    pub(super) knob_offset_x: f32,
    /// Vertical offset of the knob within the container
    pub(super) knob_offset_y: f32,
    /// Knob diameter
    pub(super) knob_size: f32,
    /// Normalized value [0, 1]
    pub(super) normalized: f32,
    /// Arc color (typically the plugin accent color)
    pub(super) arc_color: Rgba,
    /// Arc thickness in pixels (filled / value portion)
    pub(super) arc_width: f32,
    /// Track (unfilled) arc thickness in pixels. `0.0` hides the track.
    pub(super) track_arc_width: f32,
    /// Glow intensity [0.0, 1.0]. 0.0 disables the outer halo.
    pub(super) arc_glow: f32,
    /// Arc start angle in radians (from design tokens)
    pub(super) arc_start_rad: f32,
    /// Arc end angle in radians (from design tokens)
    pub(super) arc_end_rad: f32,
    /// Number of segments for arc smoothness (from design tokens)
    pub(super) arc_segments: u32,
}

impl IntoElement for KnobArcElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for KnobArcElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
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
                size: Size {
                    width: px(self.container_width).into(),
                    height: px(self.container_height).into(),
                },
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
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let origin_x = bounds.origin.x;
        let origin_y = bounds.origin.y;

        let center_x = self.knob_offset_x + self.knob_size / 2.0;
        let center_y = self.knob_offset_y + self.knob_size / 2.0;

        let start_rad = self.arc_start_rad;
        let end_rad = self.arc_end_rad;
        let value_rad = start_rad + (end_rad - start_rad) * self.normalized;
        let segments = self.arc_segments;
        let total_span = end_rad - start_rad;

        // Helper that builds a filled annular sector path between two angles.
        let build_sector = |from_rad: f32,
                            to_rad: f32,
                            outer_r: f32,
                            inner_r: f32|
         -> Option<gpui::Path<Pixels>> {
            let span = to_rad - from_rad;
            if span <= 0.01 || outer_r <= inner_r {
                return None;
            }
            let actual_segments = ((segments as f32 * (span / total_span.max(0.0001)))
                .ceil()
                .max(4.0)) as usize;

            let mut builder = PathBuilder::fill();
            let start_x = center_x + outer_r * from_rad.cos();
            let start_y = center_y + outer_r * from_rad.sin();
            builder.move_to(point(origin_x + px(start_x), origin_y + px(start_y)));

            for i in 1..=actual_segments {
                let t = i as f32 / actual_segments as f32;
                let angle = from_rad + span * t;
                let x = center_x + outer_r * angle.cos();
                let y = center_y + outer_r * angle.sin();
                builder.line_to(point(origin_x + px(x), origin_y + px(y)));
            }
            for i in 0..=actual_segments {
                let t = 1.0 - (i as f32 / actual_segments as f32);
                let angle = from_rad + span * t;
                let x = center_x + inner_r * angle.cos();
                let y = center_y + inner_r * angle.sin();
                builder.line_to(point(origin_x + px(x), origin_y + px(y)));
            }

            builder.build().ok()
        };

        // Geometry: arc sits just outside the knob border.
        let outer_radius = self.knob_size / 2.0 + 1.0;
        let value_inner_radius = outer_radius - self.arc_width;

        // Glow halo behind the value arc — three concentric copies with falling
        // alpha to fake a soft outer bloom. Skipped at zero glow or zero value.
        if self.arc_glow > 0.0 && self.normalized > 0.005 {
            let glow_alpha_base = (self.arc_glow.clamp(0.0, 1.0) * 0.45) * self.arc_color.a;
            for (i, mult) in [(1.0_f32, 0.55_f32), (2.5, 0.30), (4.5, 0.15)] {
                let halo_color = Rgba {
                    r: self.arc_color.r,
                    g: self.arc_color.g,
                    b: self.arc_color.b,
                    a: (glow_alpha_base * mult).clamp(0.0, 1.0),
                };
                let halo_outer = outer_radius + i;
                let halo_inner = (value_inner_radius - i).max(self.knob_size / 2.0);
                if let Some(path) = build_sector(start_rad, value_rad, halo_outer, halo_inner) {
                    window.paint_path(path, halo_color);
                }
            }
        }

        // Value arc (filled portion).
        if self.normalized > 0.005
            && let Some(path) = build_sector(start_rad, value_rad, outer_radius, value_inner_radius)
        {
            window.paint_path(path, self.arc_color);
        }

        // Track (unfilled portion). Hidden when the configured track width is 0.
        if self.track_arc_width > 0.0 {
            let track_outer = outer_radius;
            let track_inner = outer_radius - self.track_arc_width;
            let track_color = Rgba {
                r: self.arc_color.r,
                g: self.arc_color.g,
                b: self.arc_color.b,
                a: 0.12,
            };
            if let Some(path) = build_sector(value_rad, end_rad, track_outer, track_inner) {
                window.paint_path(path, track_color);
            }
        }
    }
}
