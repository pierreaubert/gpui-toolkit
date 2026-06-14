//! Level meter primitives for audio UIs.

use crate::TickConfig;
use gpui::prelude::*;
use gpui::*;
use std::panic;

/// dB scale positions: maps dB value to visual position (0.0 = bottom, 1.0 = top).
pub fn db_to_position(db: f64) -> f32 {
    let normalized = if db <= -60.0 {
        0.0
    } else if db <= -30.0 {
        ((db + 60.0) / 30.0) * 0.33
    } else if db <= -10.0 {
        0.33 + ((db + 30.0) / 20.0) * 0.33
    } else {
        0.66 + ((db + 10.0) / 10.0) * 0.34
    };
    normalized.clamp(0.0, 1.0) as f32
}

/// Colors used by the level meter.
#[derive(Clone)]
pub struct MeterColors {
    pub background: Rgba,
    pub green: Rgba,
    pub yellow: Rgba,
    pub red: Rgba,
    pub peak: Rgba,
    pub text: Rgba,
    pub corner_radius: f32,
    pub use_gradient: bool,
}

impl Default for MeterColors {
    fn default() -> Self {
        Self {
            background: rgba(0x1f1f1fff),
            green: rgba(0x4caf50ff),
            yellow: rgba(0xffc107ff),
            red: rgba(0xf44336ff),
            peak: rgba(0xffffffff),
            text: rgba(0xd0d0d0ff),
            corner_radius: 2.0,
            use_gradient: false,
        }
    }
}

/// Theme for reusable horizontal meter bars.
#[derive(Clone)]
pub struct HorizontalMeterTheme {
    pub color_normal: Rgba,
    pub color_warning: Rgba,
    pub color_critical: Rgba,
    pub color_info: Rgba,
    pub color_background: Rgba,
    pub color_border: Rgba,
    pub color_text: Rgba,
    pub bar_height: f32,
    pub border_radius: f32,
    pub border_width: f32,
    pub label_width: f32,
    pub value_width: f32,
    pub warning_threshold: f32,
    pub critical_threshold: f32,
    pub use_gradient: bool,
    pub text_size: Rems,
    pub gap: Pixels,
}

impl Default for HorizontalMeterTheme {
    fn default() -> Self {
        Self {
            color_normal: rgba(0x4caf50ff),
            color_warning: rgba(0xffc107ff),
            color_critical: rgba(0xf44336ff),
            color_info: rgba(0x38bdf8ff),
            color_background: rgba(0x1f1f1fff),
            color_border: rgba(0x3f3f46ff),
            color_text: rgba(0xd0d0d0ff),
            bar_height: 20.0,
            border_radius: 2.0,
            border_width: 1.0,
            label_width: 32.0,
            value_width: 50.0,
            warning_threshold: 0.75,
            critical_threshold: 0.90,
            use_gradient: false,
            text_size: rems(0.75),
            gap: px(4.0),
        }
    }
}

impl HorizontalMeterTheme {
    /// Get the semantic meter color for a fill ratio from 0.0 to 1.0.
    pub fn color_for_ratio(&self, ratio: f32) -> Rgba {
        if ratio >= self.critical_threshold {
            self.color_critical
        } else if ratio >= self.warning_threshold {
            self.color_warning
        } else {
            self.color_normal
        }
    }
}

/// Render a horizontal meter bar using a tick config for value positioning.
pub fn render_horizontal_meter_bar(
    label: impl Into<SharedString>,
    value: f64,
    tick_config: &TickConfig,
    theme: HorizontalMeterTheme,
) -> impl IntoElement {
    let ratio = tick_config.value_to_position(value);
    let bar_color = theme.color_for_ratio(ratio);
    render_horizontal_meter_bar_with(label, ratio, bar_color, format!("{value:.1}"), theme)
}

/// Single-element gradient fill for horizontal meter bars.
///
/// Paints the gradient directly instead of creating a child `div` per strip,
/// removing per-frame allocations for the gradient meter path.
struct GradientMeterFillElement {
    bar_color: Rgba,
    strips: usize,
}

impl IntoElement for GradientMeterFillElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for GradientMeterFillElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
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
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
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
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let strips = self.strips.max(1);
        let strip_width = width / strips as f32;
        for index in 0..strips {
            let t = (index as f32 + 0.5) / strips as f32;
            let alpha = 0.35 + 0.65 * t;
            let strip_color = Rgba {
                r: self.bar_color.r,
                g: self.bar_color.g,
                b: self.bar_color.b,
                a: self.bar_color.a * alpha,
            };
            let x = bounds.origin.x + px(strip_width * index as f32);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(x, bounds.origin.y),
                    size: size(px(strip_width), px(height)),
                },
                corner_radii: Corners::default(),
                background: strip_color.into(),
                border_widths: Edges::default(),
                border_color: Hsla::transparent_black(),
                border_style: Default::default(),
            });
        }
    }
}

/// Render a horizontal meter bar with explicit fill ratio, color, and value text.
pub fn render_horizontal_meter_bar_with(
    label: impl Into<SharedString>,
    ratio: f32,
    bar_color: Rgba,
    value_text: impl Into<SharedString>,
    theme: HorizontalMeterTheme,
) -> impl IntoElement {
    let ratio = ratio.clamp(0.0, 1.0);
    let fill: Div = if theme.use_gradient {
        div()
            .h_full()
            .w(relative(ratio))
            .flex()
            .child(GradientMeterFillElement {
                bar_color,
                strips: 10,
            })
    } else {
        div().h_full().w(relative(ratio)).bg(bar_color)
    };

    div()
        .flex()
        .items_center()
        .gap(theme.gap)
        .child(
            div()
                .w(px(theme.label_width))
                .text_size(theme.text_size)
                .text_color(theme.color_text)
                .child(label.into()),
        )
        .child(
            div()
                .flex_1()
                .h(px(theme.bar_height))
                .rounded(px(theme.border_radius))
                .border(px(theme.border_width))
                .border_color(theme.color_border)
                .bg(theme.color_background)
                .overflow_hidden()
                .child(fill),
        )
        .child(
            div()
                .w(px(theme.value_width))
                .text_size(theme.text_size)
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.color_text)
                .text_align(TextAlign::Right)
                .child(value_text.into()),
        )
}

/// GPU-accelerated vertical level meter element.
pub struct LevelMeterElement {
    level_db: f64,
    peak_db: Option<f64>,
    #[allow(dead_code)]
    channel_name: SharedString,
    is_clipping: bool,
    bar_width: Pixels,
    colors: MeterColors,
}

impl LevelMeterElement {
    pub fn new(level_db: f64, channel_name: impl Into<SharedString>) -> Self {
        Self {
            level_db,
            peak_db: None,
            channel_name: channel_name.into(),
            is_clipping: level_db > -0.1,
            bar_width: px(16.0),
            colors: MeterColors::default(),
        }
    }

    pub fn peak(mut self, peak_db: f64) -> Self {
        self.peak_db = Some(peak_db);
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.bar_width = width;
        self
    }

    pub fn colors(mut self, colors: MeterColors) -> Self {
        self.colors = colors;
        self
    }

    fn fill_ratio(&self) -> f32 {
        db_to_position(self.level_db)
    }

    fn yellow_threshold(&self) -> f32 {
        db_to_position(-6.0)
    }

    fn red_threshold(&self) -> f32 {
        db_to_position(-1.0)
    }
}

impl IntoElement for LevelMeterElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for LevelMeterElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
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
                size: size(self.bar_width.into(), relative(1.0).into()),
                min_size: size(self.bar_width.into(), px(60.0).into()),
                flex_grow: 1.0,
                flex_shrink: 0.0,
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
        let fill_ratio = self.fill_ratio();
        let yellow_threshold = self.yellow_threshold();
        let red_threshold = self.red_threshold();
        let text_height = px(16.0);
        let meter_bounds = Bounds {
            origin: bounds.origin,
            size: size(bounds.size.width, bounds.size.height - text_height),
        };

        let meter_w_f: f32 = meter_bounds.size.width.into();
        let meter_height_f: f32 = meter_bounds.size.height.into();
        let meter_origin_y_f: f32 = meter_bounds.origin.y.into();
        let bar_radius = self
            .colors
            .corner_radius
            .clamp(0.0, (meter_w_f / 2.0).min(8.0));
        let corner_radii = Corners::all(px(bar_radius));

        window.paint_quad(PaintQuad {
            bounds: meter_bounds,
            corner_radii,
            background: self.colors.background.into(),
            border_widths: Edges::default(),
            border_color: Hsla::transparent_black(),
            border_style: Default::default(),
        });

        let green_height = fill_ratio.min(yellow_threshold);
        let yellow_height = if fill_ratio > yellow_threshold {
            (fill_ratio - yellow_threshold).min(red_threshold - yellow_threshold)
        } else {
            0.0
        };
        let red_height = if fill_ratio > red_threshold {
            fill_ratio - red_threshold
        } else {
            0.0
        };

        let use_gradient = self.colors.use_gradient;
        let mut paint_segment = |y_top: f32, y_bottom: f32, color: Rgba, is_bottom: bool| {
            if y_bottom - y_top < 0.5 {
                return;
            }
            let seg_corner = if is_bottom {
                Corners {
                    top_left: px(0.0),
                    top_right: px(0.0),
                    bottom_left: px(bar_radius),
                    bottom_right: px(bar_radius),
                }
            } else {
                Corners::default()
            };

            if use_gradient {
                let strips = 12usize;
                let total_h = (y_bottom - y_top).max(0.0);
                for i in 0..strips {
                    let t0 = i as f32 / strips as f32;
                    let t1 = (i + 1) as f32 / strips as f32;
                    let strip_top = y_top + total_h * t0;
                    let strip_bot = y_top + total_h * t1;
                    let mid = (strip_top + strip_bot) * 0.5;
                    let local_pos = if meter_height_f > 0.0 {
                        ((meter_origin_y_f + meter_height_f - mid) / meter_height_f).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let alpha = 0.4 + 0.6 * local_pos;
                    let stripe_color = Rgba {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: color.a * alpha,
                    };
                    window.paint_quad(PaintQuad {
                        bounds: Bounds {
                            origin: point(meter_bounds.origin.x, px(strip_top)),
                            size: size(meter_bounds.size.width, px(strip_bot - strip_top)),
                        },
                        corner_radii: if is_bottom && i == strips - 1 {
                            seg_corner
                        } else {
                            Corners::default()
                        },
                        background: stripe_color.into(),
                        border_widths: Edges::default(),
                        border_color: Hsla::transparent_black(),
                        border_style: Default::default(),
                    });
                }
            } else {
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(meter_bounds.origin.x, px(y_top)),
                        size: size(meter_bounds.size.width, px(y_bottom - y_top)),
                    },
                    corner_radii: seg_corner,
                    background: color.into(),
                    border_widths: Edges::default(),
                    border_color: Hsla::transparent_black(),
                    border_style: Default::default(),
                });
            }
        };

        let bar_bottom_y = meter_origin_y_f + meter_height_f;
        if green_height > 0.001 {
            let segment_height = meter_height_f * green_height;
            paint_segment(
                bar_bottom_y - segment_height,
                bar_bottom_y,
                self.colors.green,
                true,
            );
        }
        if yellow_height > 0.001 {
            let segment_height = meter_height_f * yellow_height;
            let segment_bottom = meter_height_f * yellow_threshold;
            let y_top = bar_bottom_y - segment_bottom - segment_height;
            paint_segment(y_top, y_top + segment_height, self.colors.yellow, false);
        }
        if red_height > 0.001 {
            let segment_height = meter_height_f * red_height;
            let segment_bottom = meter_height_f * red_threshold;
            let y_top = bar_bottom_y - segment_bottom - segment_height;
            paint_segment(y_top, y_top + segment_height, self.colors.red, false);
        }

        if let Some(peak_db) = self.peak_db {
            let peak_pos = db_to_position(peak_db);
            let peak_thickness = 2.0_f32;
            let peak_center_y = meter_origin_y_f + meter_height_f * (1.0 - peak_pos);
            let peak_color = if self.is_clipping {
                self.colors.red
            } else {
                self.colors.peak
            };

            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(
                        meter_bounds.origin.x,
                        px(peak_center_y - peak_thickness / 2.0),
                    ),
                    size: size(meter_bounds.size.width, px(peak_thickness)),
                },
                corner_radii: Corners::default(),
                background: peak_color.into(),
                border_widths: Edges::default(),
                border_color: Hsla::transparent_black(),
                border_style: Default::default(),
            });
        }
    }
}

fn rgba(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 24) & 0xff) as f32 / 255.0,
        g: ((hex >> 16) & 0xff) as f32 / 255.0,
        b: ((hex >> 8) & 0xff) as f32 / 255.0,
        a: (hex & 0xff) as f32 / 255.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn horizontal_meter_theme_uses_threshold_colors() {
        let theme = HorizontalMeterTheme::default();
        assert_eq!(theme.color_for_ratio(0.25), theme.color_normal);
        assert_eq!(theme.color_for_ratio(0.80), theme.color_warning);
        assert_eq!(theme.color_for_ratio(0.95), theme.color_critical);
    }

    #[::core::prelude::v1::test]
    fn horizontal_meter_bar_helper_is_constructible() {
        let theme = HorizontalMeterTheme::default();
        let config = TickConfig::lufs();
        let _bar = render_horizontal_meter_bar("L", -18.0, &config, theme.clone());
        let _custom = render_horizontal_meter_bar_with("W", 0.72, theme.color_info, "72%", theme);
    }

    #[::core::prelude::v1::test]
    fn gradient_meter_bar_is_constructible() {
        let mut theme = HorizontalMeterTheme::default();
        theme.use_gradient = true;
        let _custom = render_horizontal_meter_bar_with("G", 0.5, theme.color_info, "50%", theme);
    }
}
