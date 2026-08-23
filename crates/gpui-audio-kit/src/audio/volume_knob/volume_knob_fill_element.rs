use d3rs::render2d::{Renderer2D, VelloBackend};
use gpui::*;
use std::f32::consts::PI;

/// Custom element that paints the volume knob fill using paths
pub(super) struct VolumeKnobFillElement {
    pub(super) id: ElementId,
    pub(super) size: Pixels,
    pub(super) value: f32,
    pub(super) bg_color: Rgba,
    pub(super) fill_color: Rgba,
    pub(super) ring_color: Rgba,
    pub(super) renderer_2d: Renderer2D,
    pub(super) vello_backend: VelloBackend,
    #[cfg(feature = "vello")]
    pub(super) painter: d3rs::vello2d::VelloScenePainter,
}

impl VolumeKnobFillElement {
    pub(super) fn new(
        id: ElementId,
        size: Pixels,
        value: f32,
        bg_color: Rgba,
        fill_color: Rgba,
        ring_color: Rgba,
    ) -> Self {
        Self {
            id,
            size,
            value,
            bg_color,
            fill_color,
            ring_color,
            renderer_2d: Renderer2D::default(),
            vello_backend: VelloBackend::default(),
            #[cfg(feature = "vello")]
            painter: d3rs::vello2d::VelloScenePainter::new(),
        }
    }

    pub(super) fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    pub(super) fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        #[cfg(feature = "vello")]
        self.painter.set_backend(backend);
        self
    }
}

impl IntoElement for VolumeKnobFillElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for VolumeKnobFillElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
                    width: self.size.into(),
                    height: self.size.into(),
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
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        // Extract bounds into f32 for calculations
        let size_f32 = self.size.to_f64() as f32;
        let origin_x = bounds.origin.x;
        let origin_y = bounds.origin.y;
        let radius = size_f32 / 2.0;

        #[cfg(feature = "vello")]
        if self.renderer_2d == Renderer2D::Vello {
            use d3rs::vello2d::kurbo::{BezPath, Circle, PathEl, Shape, Stroke};
            use d3rs::vello2d::peniko::{Brush, Color};
            let mut scene = d3rs::vello2d::ChartScene::new();
            let color = |c: Rgba| Brush::Solid(Color::new([c.r, c.g, c.b, c.a]));
            scene.fill_path(
                Circle::new((radius as f64, radius as f64), radius as f64).to_path(0.1),
                color(self.bg_color),
            );
            if self.value > 0.001 {
                let value = self.value.clamp(0.0, 1.0);
                let center = radius;
                let water = center + radius - value * 2.0 * radius;
                let dy = water - center;
                let dx2 = radius * radius - dy * dy;
                if dx2 > 0.0 {
                    let dx = dx2.sqrt();
                    let left = center - dx;
                    let start = (dy / radius).clamp(-1.0, 1.0).asin();
                    let end = PI - start;
                    let mut path = BezPath::new();
                    path.push(PathEl::MoveTo((left as f64, water as f64).into()));
                    for index in 1..=32 {
                        let t = index as f32 / 32.0;
                        let angle = PI - (start + t * (end - start));
                        path.push(PathEl::LineTo(
                            (
                                (center + radius * angle.cos()) as f64,
                                (center + radius * angle.sin()) as f64,
                            )
                                .into(),
                        ));
                    }
                    path.push(PathEl::ClosePath);
                    scene.fill_path(path, color(self.fill_color));
                } else if value > 0.99 {
                    scene.fill_path(
                        Circle::new((radius as f64, radius as f64), (radius - 1.0) as f64)
                            .to_path(0.1),
                        color(self.fill_color),
                    );
                }
            }
            let ring = Rgba {
                r: self.ring_color.r,
                g: self.ring_color.g,
                b: self.ring_color.b,
                a: self.ring_color.a * 0.3,
            };
            scene.stroke_path(
                Circle::new((radius as f64, radius as f64), (radius - 3.0) as f64).to_path(0.1),
                Stroke::new(2.0),
                color(ring),
            );
            self.painter.set_backend(self.vello_backend);
            self.painter.paint_retained(id, &scene, bounds, window);
            return;
        }

        // Transparent color for borders we don't want to render
        let transparent = Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };

        // Draw background circle
        window.paint_quad(PaintQuad {
            bounds,
            corner_radii: Corners::all(px(radius)),
            background: self.bg_color.into(),
            border_widths: Edges::default(),
            border_color: transparent.into(),
            border_style: BorderStyle::default(),
        });

        // Draw fill - a circular segment from bottom
        if self.value > 0.001 {
            // Calculate geometry in f32
            let center_x = radius;
            let center_y = radius;

            // Calculate the y-coordinate of the "water line" (relative to element origin)
            // At 0%: water_line_y = center_y + radius (bottom of circle)
            // At 100%: water_line_y = center_y - radius (top of circle)
            let water_line_y = center_y + radius - (self.value * 2.0 * radius);

            // Only draw if the water line is within the circle
            if water_line_y < center_y + radius {
                // Calculate intersection points of horizontal line with circle
                // Circle equation: (x - cx)^2 + (y - cy)^2 = r^2
                // At y = water_line_y: (x - cx)^2 = r^2 - (water_line_y - cy)^2
                let dy = water_line_y - center_y;
                let dx_squared = radius * radius - dy * dy;

                if dx_squared > 0.0 {
                    let dx = dx_squared.sqrt();
                    let left_x = center_x - dx;

                    // Build a path for the filled portion using PathBuilder
                    let mut builder = PathBuilder::fill();

                    // Start at left intersection point
                    builder.move_to(point(origin_x + px(left_x), origin_y + px(water_line_y)));

                    // Draw arc from left to right along the bottom of the circle
                    // We'll approximate with line segments for a smooth curve
                    let start_angle = (dy / radius).clamp(-1.0, 1.0).asin();
                    let end_angle = PI - start_angle;

                    // Number of segments for smooth arc
                    let segments = 32;
                    for i in 1..=segments {
                        let t = i as f32 / segments as f32;
                        let angle = start_angle + t * (end_angle - start_angle);
                        // Angle measured from right (0) going counter-clockwise
                        // We want bottom arc, so we go from left intersection to right
                        let arc_angle = PI - angle; // Convert to standard angle
                        let x = center_x + radius * arc_angle.cos();
                        let y = center_y + radius * arc_angle.sin();
                        builder.line_to(point(origin_x + px(x), origin_y + px(y)));
                    }

                    // Close the path back to start
                    builder.line_to(point(origin_x + px(left_x), origin_y + px(water_line_y)));

                    if let Ok(path) = builder.build() {
                        window.paint_path(path, self.fill_color);
                    }
                } else if self.value > 0.99 {
                    // Nearly full - draw full circle
                    let inset = px(1.0);
                    window.paint_quad(PaintQuad {
                        bounds: Bounds {
                            origin: point(bounds.origin.x + inset, bounds.origin.y + inset),
                            size: size(
                                bounds.size.width - inset * 2.0,
                                bounds.size.height - inset * 2.0,
                            ),
                        },
                        corner_radii: Corners::all(px(radius - 1.0)),
                        background: self.fill_color.into(),
                        border_widths: Edges::default(),
                        border_color: transparent.into(),
                        border_style: BorderStyle::default(),
                    });
                }
            }
        }

        // Draw border ring
        let ring_inset = px(3.0);
        let ring_bounds = Bounds {
            origin: point(bounds.origin.x + ring_inset, bounds.origin.y + ring_inset),
            size: size(
                bounds.size.width - ring_inset * 2.0,
                bounds.size.height - ring_inset * 2.0,
            ),
        };
        // Create ring color with 30% opacity
        let ring_with_opacity = Rgba {
            r: self.ring_color.r,
            g: self.ring_color.g,
            b: self.ring_color.b,
            a: self.ring_color.a * 0.3,
        };
        window.paint_quad(PaintQuad {
            bounds: ring_bounds,
            corner_radii: Corners::all(px(radius - 3.0)),
            background: transparent.into(),
            border_widths: Edges::all(px(2.0)),
            border_color: ring_with_opacity.into(),
            border_style: BorderStyle::default(),
        });
    }
}
