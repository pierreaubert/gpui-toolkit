use super::prelude::*;
use gpui::Subscription;
use gpui::prelude::FluentBuilder as _;
use gpui_builder::{Axis, ContainerNode, LayoutNode, LayoutPreferences, Sizing, SlotNode, solve};
use gpui_ui_kit::color::Color;
use gpui_ui_kit::thinking_orb::{engine, presets};
use gpui_ui_kit::{ColorPickerView, OrbSize, OrbState, ThinkingOrb};
use std::time::Duration;

/// Default slider position: points rendered per sphere.
const DEFAULT_POINTS_PER_SPHERE: f32 = 256.0;
/// On-canvas orb size in the demo grid.
const ORB_SIZE: f32 = 96.0;
/// Largest interactive sphere size, expressed as a multiple of [`ORB_SIZE`].
const MAX_ORB_SIZE_SCALE: f32 = 8.0;
const DEFAULT_DOT_SIZE_SCALE: f32 = 1.0;
const MIN_DOT_SIZE_SCALE: f32 = 0.25;
const MAX_DOT_SIZE_SCALE: f32 = 20.0;
const DEFAULT_ORB_SPEED: f32 = 0.5;
const MIN_ORB_SPEED: f32 = 0.05;
const MAX_ORB_SPEED: f32 = 2.0;
const ORB_GRID_GAP: f32 = 16.0;
const SHOWCASE_SIDEBAR_WIDTH: f32 = 220.0;
const SHOWCASE_WIDE_CONTENT_PADDING: f32 = 64.0;
const SHOWCASE_COMPACT_CONTENT_PADDING: f32 = 32.0;
const SHOWCASE_COMPACT_BREAKPOINT: f32 = 600.0;
const ORB_GRID_SLOT_IDS: [&str; 9] = [
    "orb-grid-slot-0",
    "orb-grid-slot-1",
    "orb-grid-slot-2",
    "orb-grid-slot-3",
    "orb-grid-slot-4",
    "orb-grid-slot-5",
    "orb-grid-slot-6",
    "orb-grid-slot-7",
    "orb-grid-slot-8",
];

/// Interactive lab for the `ThinkingOrb` component: all nine states in a 3×3
/// grid with a shared density slider and a live-load stats line.
pub(crate) struct ThinkingOrbsLab {
    orbs: Vec<(OrbState, Entity<ThinkingOrb>)>,
    points_per_sphere: f32,
    sphere_size_scale: f32,
    dot_size_scale: f32,
    speed_scale: f32,
    dot_color_picker: Entity<ColorPickerView>,
    _dot_color_subscription: Subscription,
    /// Preset-native dot counts at t=0 (Px64 tuning), per state — the slider
    /// maps its absolute point target to a per-orb `count_scale` factor.
    base_counts: Vec<usize>,
}

impl ThinkingOrbsLab {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let dot_color = Color::rgb(96, 165, 250);
        let mut orbs = Vec::with_capacity(OrbState::ALL.len());
        let mut base_counts = Vec::with_capacity(OrbState::ALL.len());
        for state in OrbState::ALL {
            let resolved = presets::resolve_preset(state, OrbSize::Px64);
            let base = engine::frame(resolved.mode, 64.0, 0.0, &resolved.opts)
                .dots
                .len()
                .max(1);
            base_counts.push(base);
            let orb = cx.new(|cx| {
                ThinkingOrb::new(state, px(ORB_SIZE), cx)
                    .speed(DEFAULT_ORB_SPEED)
                    .dot_color(dot_color.to_rgba())
            });
            let factor = f64::from(DEFAULT_POINTS_PER_SPHERE) / base as f64;
            orb.update(cx, |orb, cx| orb.set_count_scale(factor, cx));
            orbs.push((state, orb));
        }
        let dot_color_picker = cx.new(|_| ColorPickerView::new("Small dot color", dot_color));
        let dot_color_subscription = cx.observe(&dot_color_picker, |this, picker, cx| {
            this.apply_dot_color(picker.read(cx).color(), cx);
        });
        Self {
            orbs,
            points_per_sphere: DEFAULT_POINTS_PER_SPHERE,
            sphere_size_scale: 1.0,
            dot_size_scale: DEFAULT_DOT_SIZE_SCALE,
            speed_scale: DEFAULT_ORB_SPEED,
            dot_color_picker,
            _dot_color_subscription: dot_color_subscription,
            base_counts,
        }
    }

    fn apply_density(&mut self, target: f32, cx: &mut Context<Self>) {
        self.points_per_sphere = target;
        for (index, (_state, orb)) in self.orbs.iter().enumerate() {
            let scale = f64::from(target) / self.base_counts[index] as f64;
            orb.update(cx, |orb, cx| orb.set_count_scale(scale, cx));
        }
        cx.notify();
    }

    fn apply_size_scale(&mut self, scale: f32, cx: &mut Context<Self>) {
        self.sphere_size_scale = scale;
        let size = px(ORB_SIZE * scale);
        for (_state, orb) in &self.orbs {
            orb.update(cx, |orb, cx| orb.set_size(size, cx));
        }
        cx.notify();
    }

    fn apply_dot_size_scale(&mut self, scale: f32, cx: &mut Context<Self>) {
        self.dot_size_scale = scale;
        for (_state, orb) in &self.orbs {
            orb.update(cx, |orb, cx| orb.set_dot_scale(f64::from(scale), cx));
        }
        cx.notify();
    }

    fn apply_speed_scale(&mut self, scale: f32, cx: &mut Context<Self>) {
        self.speed_scale = scale;
        for (_state, orb) in &self.orbs {
            orb.update(cx, |orb, cx| orb.set_speed(scale, cx));
        }
        cx.notify();
    }

    fn apply_dot_color(&mut self, color: Color, cx: &mut Context<Self>) {
        let rgba = color.to_rgba();
        for (_state, orb) in &self.orbs {
            orb.update(cx, |orb, cx| orb.set_dot_color(rgba, cx));
        }
        cx.notify();
    }
}

fn orb_grid_columns(available_width: f32, card_width: f32) -> usize {
    (1..=ORB_GRID_SLOT_IDS.len())
        .rev()
        .find(|&columns| {
            let slots: Vec<LayoutNode<'_>> = ORB_GRID_SLOT_IDS[..columns]
                .iter()
                .map(|id| SlotNode::new(id, Sizing::Fixed(card_width)).into_node())
                .collect();
            let root = ContainerNode::new(
                "thinking-orbs-grid",
                Axis::Horizontal,
                Sizing::flex(0.0),
                &slots,
            )
            .divider_size(ORB_GRID_GAP)
            .into_node();
            solve(
                &root,
                available_width.max(card_width),
                card_width,
                &LayoutPreferences::default(),
            )
            .debug_report()
            .is_clean()
        })
        .unwrap_or(1)
}

/// Capitalize the lowercase engine state key for display ("working" → "Working").
fn capitalize_state(state: OrbState) -> String {
    let key = state.as_str();
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Format a geometry-eval duration as µs below 1 ms, ms above.
fn format_geometry_time(duration: Duration) -> String {
    if duration >= Duration::from_millis(1) {
        format!("{:.2} ms", duration.as_secs_f64() * 1_000.0)
    } else {
        format!("{} µs", duration.as_micros())
    }
}

impl Render for ThinkingOrbsLab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = cx.entity().clone();
        let sphere_size = ORB_SIZE * self.sphere_size_scale;
        let card_width = sphere_size.max(128.0);
        let viewport_width = window.viewport_size().width.as_f32();
        let content_width = if viewport_width < SHOWCASE_COMPACT_BREAKPOINT {
            viewport_width - SHOWCASE_COMPACT_CONTENT_PADDING
        } else {
            viewport_width - SHOWCASE_SIDEBAR_WIDTH - SHOWCASE_WIDE_CONTENT_PADDING
        };
        let grid_columns = orb_grid_columns(content_width, card_width);
        let control_columns = orb_grid_columns(content_width, 400.0).min(2);
        let grid_width =
            card_width * grid_columns as f32 + ORB_GRID_GAP * grid_columns.saturating_sub(1) as f32;

        let mut total_dots = 0usize;
        let mut slowest_geometry = Duration::ZERO;
        for (_state, orb) in &self.orbs {
            let stats = orb.read(cx).frame_stats();
            total_dots += stats.dots;
            slowest_geometry = slowest_geometry.max(stats.geometry_time);
        }
        let backend = if gpui::wgpu_custom_draw_available() {
            "Vello · wgpu (GPU)"
        } else {
            "Vello · CPU fallback"
        };

        div()
            .id("thinking-orbs-section")
            .flex()
            .flex_col()
            .gap_4()
            .child(Heading::h2(cx.t(TranslationKey::SectionThinkingOrbs)))
            .child(
                div()
                    .w_full()
                    .grid()
                    .grid_cols(control_columns as u16)
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(control_columns == 2, |el| el.col_start(1).col_end(2))
                            .child(
                                Text::new(format!(
                                    "Points per sphere: {:.0}",
                                    self.points_per_sphere
                                ))
                                .weight(TextWeight::Medium),
                            )
                            .child(
                                div().w(px(300.0)).child(
                                    Slider::new("orbs-density")
                                        .min(64.0)
                                        .max(1024.0)
                                        .step(1.0)
                                        .value(self.points_per_sphere)
                                        .size(SliderSize::Md)
                                        .on_change({
                                            let entity = entity.clone();
                                            move |value, _window, cx| {
                                                entity.update(cx, |lab, cx| {
                                                    lab.apply_density(value, cx);
                                                });
                                            }
                                        }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(control_columns == 2, |el| el.col_start(1).col_end(2))
                            .child(
                                Text::new(format!(
                                    "Sphere size: {:.2}× ({sphere_size:.0} px)",
                                    self.sphere_size_scale,
                                ))
                                .weight(TextWeight::Medium),
                            )
                            .child(
                                div().w(px(300.0)).child(
                                    Slider::new("orbs-size")
                                        .min(1.0)
                                        .max(MAX_ORB_SIZE_SCALE)
                                        .step(0.25)
                                        .value(self.sphere_size_scale)
                                        .size(SliderSize::Md)
                                        .on_change({
                                            let entity = entity.clone();
                                            move |value, _window, cx| {
                                                entity.update(cx, |lab, cx| {
                                                    lab.apply_size_scale(value, cx);
                                                });
                                            }
                                        }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(control_columns == 2, |el| el.col_start(1).col_end(2))
                            .child(
                                Text::new(format!("Small dot size: {:.2}×", self.dot_size_scale))
                                    .weight(TextWeight::Medium),
                            )
                            .child(
                                div().w(px(300.0)).child(
                                    Slider::new("orbs-dot-size")
                                        .min(MIN_DOT_SIZE_SCALE)
                                        .max(MAX_DOT_SIZE_SCALE)
                                        .step(0.05)
                                        .value(self.dot_size_scale)
                                        .size(SliderSize::Md)
                                        .on_change({
                                            let entity = entity.clone();
                                            move |value, _window, cx| {
                                                entity.update(cx, |lab, cx| {
                                                    lab.apply_dot_size_scale(value, cx);
                                                });
                                            }
                                        }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(control_columns == 2, |el| el.col_start(1).col_end(2))
                            .child(
                                Text::new(format!("Animation speed: {:.2}×", self.speed_scale))
                                    .weight(TextWeight::Medium),
                            )
                            .child(
                                div().w(px(300.0)).child(
                                    Slider::new("orbs-speed")
                                        .min(MIN_ORB_SPEED)
                                        .max(MAX_ORB_SPEED)
                                        .step(0.05)
                                        .value(self.speed_scale)
                                        .size(SliderSize::Md)
                                        .on_change({
                                            let entity = entity.clone();
                                            move |value, _window, cx| {
                                                entity.update(cx, |lab, cx| {
                                                    lab.apply_speed_scale(value, cx);
                                                });
                                            }
                                        }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .id("thinking-orbs-color-picker")
                            .w(px(400.0))
                            .when(control_columns == 2, |el| {
                                el.col_start(2).col_end(3).row_start(1).row_end(5)
                            })
                            .child(self.dot_color_picker.clone()),
                    ),
            )
            .child(
                div()
                    .id("thinking-orbs-stats")
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format!(
                        "Total dots: {total_dots} · slowest geometry eval: {} · backend: {backend}",
                        format_geometry_time(slowest_geometry),
                    )),
            )
            .child(
                div()
                    .id("thinking-orbs-grid-scroll")
                    .w_full()
                    .overflow_x_scroll()
                    .child(
                        div()
                            .w(px(grid_width))
                            .grid()
                            .grid_cols(grid_columns as u16)
                            .gap_4()
                            .children(self.orbs.iter().map(|(state, orb)| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_1()
                                    .w(px(card_width))
                                    .child(capitalize_state(*state))
                                    .child(orb.clone())
                            })),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_column_count_tracks_fixed_card_capacity() {
        assert_eq!(orb_grid_columns(128.0, 128.0), 1);
        assert_eq!(orb_grid_columns(272.0, 128.0), 2);
        assert_eq!(orb_grid_columns(9.0 * 128.0 + 8.0 * ORB_GRID_GAP, 128.0), 9);
    }
}
