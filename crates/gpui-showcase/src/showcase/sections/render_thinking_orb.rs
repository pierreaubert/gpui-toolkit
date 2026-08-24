use super::prelude::*;
use gpui_ui_kit::thinking_orb::{engine, presets};
use gpui_ui_kit::{OrbSize, OrbState, ThinkingOrb};
use std::time::Duration;

/// Default slider position: points rendered per sphere.
const DEFAULT_POINTS_PER_SPHERE: f32 = 256.0;
/// On-canvas orb size in the demo grid.
const ORB_SIZE: f32 = 96.0;

/// Interactive lab for the `ThinkingOrb` component: all nine states in a 3×3
/// grid with a shared density slider and a live-load stats line.
pub(crate) struct ThinkingOrbsLab {
    orbs: Vec<(OrbState, Entity<ThinkingOrb>)>,
    points_per_sphere: f32,
    /// Preset-native dot counts at t=0 (Px64 tuning), per state — the slider
    /// maps its absolute point target to a per-orb `count_scale` factor.
    base_counts: Vec<usize>,
}

impl ThinkingOrbsLab {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let mut orbs = Vec::with_capacity(OrbState::ALL.len());
        let mut base_counts = Vec::with_capacity(OrbState::ALL.len());
        for state in OrbState::ALL {
            let resolved = presets::resolve_preset(state, OrbSize::Px64);
            let base = engine::frame(resolved.mode, 64.0, 0.0, &resolved.opts)
                .dots
                .len()
                .max(1);
            base_counts.push(base);
            let orb = cx.new(|cx| ThinkingOrb::new(state, px(ORB_SIZE), cx));
            let factor = f64::from(DEFAULT_POINTS_PER_SPHERE) / base as f64;
            orb.update(cx, |orb, cx| orb.set_count_scale(factor, cx));
            orbs.push((state, orb));
        }
        Self {
            orbs,
            points_per_sphere: DEFAULT_POINTS_PER_SPHERE,
            base_counts,
        }
    }

    fn apply_density(&mut self, target: f32, cx: &mut Context<Self>) {
        self.points_per_sphere = target;
        for (index, (_state, orb)) in self.orbs.iter().enumerate() {
            let scale = f64::from(target) / self.base_counts[index] as f64;
            orb.update(cx, |orb, cx| orb.set_count_scale(scale, cx));
        }
    }
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = cx.entity().clone();

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
            .child(Heading::h2("Thinking Orbs"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Text::new(format!("Points per sphere: {:.0}", self.points_per_sphere))
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
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .children(self.orbs.iter().map(|(state, orb)| {
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .w(px(128.0))
                            .child(capitalize_state(*state))
                            .child(orb.clone())
                    })),
            )
    }
}
