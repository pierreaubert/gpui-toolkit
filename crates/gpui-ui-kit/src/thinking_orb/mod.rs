//! Dotted 3D "thinking orb" status animations.
//!
//! The geometry engine and presets in this module are a faithful Rust port of
//! the TypeScript `thinking-orbs` library (version 0.3.1, MIT © Jakub
//! Antalik). The upstream MIT license text ships alongside this module in
//! `LICENSE`. The engine is pure math — no gpui imports — and its output is
//! verified against the upstream golden vectors in
//! `tests/components/thinking_orb_parity_test.rs`.
//!
//! With the `vello` feature (on by default) this module also ships the
//! [`ThinkingOrb`] GPUI component, which renders engine frames on the GPU via
//! `d3rs::vello2d` (vello-on-wgpu zero-copy custom draw, with an automatic
//! `vello_cpu` fallback).

pub mod engine;
pub mod presets;

#[cfg(feature = "vello")]
mod element;

#[cfg(feature = "vello")]
pub use element::ThinkingOrbElement;

#[cfg(feature = "vello")]
pub use component::{FrameStats, ThinkingOrb};

#[cfg(feature = "vello")]
mod component {
    use super::element::ThinkingOrbElement;
    use super::engine::profiles::{ModeOpts, scale_counts, scale_radii};
    use super::engine::{self};
    use super::presets::{OrbSize, OrbState, Resolved, resolve_preset};
    use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
    use crate::theme::ThemeExt;
    use gpui::{
        Context, ElementId, IntoElement, Pixels, Render, Rgba, SharedString, WeakEntity, Window, px,
    };
    use std::time::Duration;
    // `std::time::Instant` panics on wasm32-unknown-unknown ("time not
    // implemented on this platform"); web-time aliases std on native targets.
    use web_time::Instant;

    /// Animation repaint interval (~60 fps).
    const TICK_INTERVAL: Duration = Duration::from_millis(16);
    /// Lower clamp for [`ThinkingOrb::count_scale`] so radii scaling
    /// (`1 / √factor`) stays finite.
    const MIN_COUNT_SCALE: f64 = 0.01;
    /// Lower clamp for dot-radius scaling so invalid slider or API input never
    /// produces zero, negative, or non-finite paint geometry.
    const MIN_DOT_SCALE: f64 = 0.05;

    /// Per-frame geometry statistics for debugging overlays.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct FrameStats {
        /// Dots in the last evaluated frame.
        pub dots: usize,
        /// Lines in the last evaluated frame.
        pub lines: usize,
        /// Wall time of the last `engine::frame` evaluation.
        pub geometry_time: Duration,
    }

    /// A GPU-rendered dotted "thinking orb" status animation.
    ///
    /// Entity component (modelled on [`crate::AnimatedQrCode`]): a `cx.spawn`
    /// ticker advances an accumulated clock at ~60 fps and repaints; the clock
    /// freezes while paused so resume never jumps. Frames come from the
    /// pure-math [`engine`] and are painted by [`ThinkingOrbElement`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In a Context<Parent>:
    /// let orb = cx.new(|cx| ThinkingOrb::new(OrbState::Working, px(64.0), cx));
    /// // In render:
    /// parent.child(orb)
    /// ```
    pub struct ThinkingOrb {
        /// Element/accessibility identity: the view's entity id.
        id: ElementId,
        state: OrbState,
        size: Pixels,
        /// User clock multiplier on top of the preset speed.
        speed: f32,
        /// Density multiplier (1.0 = preset-native density).
        count_scale: f64,
        /// Optional dot tint. Lines retain their preset-native ink.
        dot_color: Option<Rgba>,
        /// Dot-radius multiplier (1.0 = preset-native radius).
        dot_scale: f64,
        paused: bool,
        aria_label: Option<SharedString>,
        /// Preset-native resolution (mode + clock multiplier + base opts).
        resolved: Resolved,
        /// `resolved.opts` with the density scaling applied; recomputed by
        /// [`Self::set_count_scale`].
        opts: ModeOpts,
        /// Accumulated animation clock (frozen while paused).
        elapsed: Duration,
        /// Wall clock of the last tick; `elapsed` grows by the delta.
        last_tick: Instant,
        /// Bumped on every ticker (re)spawn so stale loops exit.
        generation: u64,
        stats: FrameStats,
    }

    impl ThinkingOrb {
        /// Create an orb for `state` rendered at `size` (preset selected by
        /// nearest [`OrbSize`]: ≤ 40 px → 20 px tuning, else 64 px tuning).
        pub fn new(state: OrbState, size: Pixels, cx: &mut Context<Self>) -> Self {
            let resolved = resolve_preset(state, nearest_orb_size(size));
            let opts = resolved.opts.clone();
            let mut this = Self {
                id: ElementId::View(cx.entity_id()),
                state,
                size,
                speed: 1.0,
                count_scale: 1.0,
                dot_color: None,
                dot_scale: 1.0,
                paused: false,
                aria_label: None,
                resolved,
                opts,
                elapsed: Duration::ZERO,
                last_tick: Instant::now(),
                generation: 0,
                stats: FrameStats::default(),
            };
            this.spawn_ticker(cx);
            this
        }

        /// Multiply the preset animation speed (1.0 = preset speed).
        pub fn speed(mut self, speed: f32) -> Self {
            self.speed = valid_speed(speed);
            self
        }

        /// Tint dots while preserving their depth and alpha shading. Lines
        /// retain their preset-native monochrome ink.
        pub fn dot_color(mut self, color: Rgba) -> Self {
            self.dot_color = Some(color);
            self
        }

        /// Scale every dot radius (1.0 = preset-native radius).
        pub fn dot_scale(mut self, scale: f64) -> Self {
            self.dot_scale = valid_dot_scale(scale);
            self
        }

        /// Scale dot density relative to the preset (1.0 = preset-native).
        /// Radii are counter-scaled by `1 / √factor` so ink coverage stays
        /// constant.
        pub fn count_scale(mut self, scale: f64) -> Self {
            self.count_scale = scale.max(MIN_COUNT_SCALE);
            self.opts = scaled_opts(&self.resolved, self.count_scale);
            self
        }

        /// Freeze (`true`) or resume (`false`) the animation clock.
        pub fn paused(mut self, paused: bool) -> Self {
            self.paused = paused;
            self
        }

        /// Override the accessibility label (default: the state's label).
        pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
            self.aria_label = Some(label.into());
            self
        }

        /// Update the dot density from a live control (e.g. a slider).
        pub fn set_count_scale(&mut self, scale: f64, cx: &mut Context<Self>) {
            self.count_scale = scale.max(MIN_COUNT_SCALE);
            self.opts = scaled_opts(&self.resolved, self.count_scale);
            cx.notify();
        }

        /// Update the animation clock multiplier without resetting its phase.
        pub fn set_speed(&mut self, speed: f32, cx: &mut Context<Self>) {
            let speed = valid_speed(speed);
            if self.speed == speed {
                return;
            }
            self.speed = speed;
            cx.notify();
        }

        /// Update the live dot tint.
        pub fn set_dot_color(&mut self, color: Rgba, cx: &mut Context<Self>) {
            if self.dot_color == Some(color) {
                return;
            }
            self.dot_color = Some(color);
            cx.notify();
        }

        /// Update the live dot-radius multiplier.
        pub fn set_dot_scale(&mut self, scale: f64, cx: &mut Context<Self>) {
            let scale = valid_dot_scale(scale);
            if self.dot_scale == scale {
                return;
            }
            self.dot_scale = scale;
            cx.notify();
        }

        /// Update the square render size without recreating the animation.
        pub fn set_size(&mut self, size: Pixels, cx: &mut Context<Self>) {
            let size = size.max(px(1.0));
            if self.size == size {
                return;
            }
            self.size = size;
            cx.notify();
        }

        /// Freeze or resume the animation clock at runtime. Resuming does not
        /// jump: the accumulated clock simply starts advancing again.
        pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
            if self.paused == paused {
                return;
            }
            self.paused = paused;
            if !paused {
                self.spawn_ticker(cx);
            }
            cx.notify();
        }

        /// Statistics of the last evaluated frame.
        pub fn frame_stats(&self) -> FrameStats {
            self.stats
        }

        /// Spawn the repaint ticker. The loop exits when the entity is
        /// dropped, when paused, or when a newer ticker supersedes it.
        fn spawn_ticker(&mut self, cx: &mut Context<Self>) {
            self.generation += 1;
            let generation = self.generation;
            self.last_tick = Instant::now();
            cx.spawn(async move |this: WeakEntity<Self>, cx| {
                loop {
                    cx.background_executor().timer(TICK_INTERVAL).await;
                    let keep_running = this.update(cx, |this, cx| {
                        if this.paused || this.generation != generation {
                            return false;
                        }
                        let now = Instant::now();
                        this.elapsed += (now - this.last_tick).mul_f32(this.speed);
                        this.last_tick = now;
                        cx.notify();
                        true
                    });
                    match keep_running {
                        Ok(true) => {}
                        _ => break,
                    }
                }
            })
            .detach();
        }
    }

    impl Render for ThinkingOrb {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            cx.register_accessible(AccessibilityNode {
                element_id: self.id.clone(),
                label: self
                    .aria_label
                    .clone()
                    .unwrap_or_else(|| self.state.label().into()),
                props: AriaProps::with_role(AriaRole::Img),
            });

            let theme = cx.theme();
            let background = theme.background;
            let luminance = 0.2126 * background.r + 0.7152 * background.g + 0.0722 * background.b;
            let dark = luminance <= 0.5;

            let size_px: f32 = self.size.into();
            // `Resolved.speed` is the renderer-side clock multiplier — the
            // engine takes a raw t.
            let t = self.elapsed.as_secs_f64() * self.resolved.speed;
            let start = Instant::now();
            let frame = engine::frame(self.resolved.mode, f64::from(size_px), t, &self.opts);
            self.stats = FrameStats {
                dots: frame.dots.len(),
                lines: frame.lines.len(),
                geometry_time: start.elapsed(),
            };

            ThinkingOrbElement::new(
                self.id.clone(),
                frame,
                self.size,
                dark,
                self.dot_color,
                self.dot_scale,
            )
        }
    }

    /// Nearest tuned preset size for a render size.
    fn nearest_orb_size(size: Pixels) -> OrbSize {
        if size <= px(40.0) {
            OrbSize::Px20
        } else {
            OrbSize::Px64
        }
    }

    /// Apply density scaling to the preset-native opts: counts × `factor`,
    /// radii × `1 / √factor` (constant ink coverage).
    fn scaled_opts(resolved: &Resolved, factor: f64) -> ModeOpts {
        let opts = scale_counts(&resolved.opts, factor);
        scale_radii(&opts, 1.0 / factor.sqrt())
    }

    fn valid_dot_scale(scale: f64) -> f64 {
        if scale.is_finite() {
            scale.max(MIN_DOT_SCALE)
        } else {
            1.0
        }
    }

    fn valid_speed(speed: f32) -> f32 {
        if speed.is_finite() {
            speed.max(0.0)
        } else {
            1.0
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{MIN_DOT_SCALE, valid_dot_scale, valid_speed};

        #[test]
        fn live_control_values_are_bounded_and_finite() {
            assert_eq!(valid_dot_scale(-1.0), MIN_DOT_SCALE);
            assert_eq!(valid_dot_scale(f64::INFINITY), 1.0);
            assert_eq!(valid_speed(-1.0), 0.0);
            assert_eq!(valid_speed(f32::NAN), 1.0);
        }
    }
}
