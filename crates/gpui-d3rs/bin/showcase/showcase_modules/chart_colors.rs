//! Theme-following colors for showcase charts.
//!
//! Chart CHROME (gridlines, axis lines, tick labels, missing-data marks)
//! reads directly from the active UI theme. DATA ink keeps its d3 hue but
//! is lightness-adapted ([`ink`]) so it contrasts with the theme
//! background. On the default dark theme d3 colors pass through unchanged,
//! so the examples keep their official look by default.

use d3rs::axis::AxisTheme;
use d3rs::color::{ColorScheme, D3Color};
use gpui::{Hsla, Rgba, rgb};
use gpui_ui_kit::theme::Theme;

/// [`AxisTheme`] wired to the active UI theme, for `d3rs::axis::render_axis`.
/// (A newtype because the orphan rule forbids implementing the d3rs trait
/// directly for the UI-kit theme.)
pub struct UiAxisTheme<'a>(pub &'a Theme);

impl AxisTheme for UiAxisTheme<'_> {
    fn axis_line_color(&self) -> Rgba {
        Rgba::from(axis_line(self.0))
    }

    fn axis_label_color(&self) -> Rgba {
        Rgba::from(tick(self.0))
    }

    fn background_color(&self) -> Option<Rgba> {
        Some(self.0.background)
    }
}

/// Minimum `|lightness - background lightness|` for data ink to pass through.
const MIN_GAP: f32 = 0.28;
/// Lightness gap enforced when a color clashes with the background.
const TARGET_GAP: f32 = 0.34;
/// Hard lightness limits when adapting.
const MIN_L: f32 = 0.08;
const MAX_L: f32 = 0.92;

/// Lightness of the theme background (HSL, 0..1).
pub fn background_lightness(theme: &Theme) -> f32 {
    Hsla::from(theme.background).l
}

/// Adapt a data color so it contrasts with the theme background.
///
/// Hue, saturation, and alpha are preserved; only lightness moves, and only
/// when the color sits too close to the background. d3 hues therefore render
/// exactly as authored on the default dark theme.
pub fn ink(theme: &Theme, color: Hsla) -> Hsla {
    adapt(background_lightness(theme), color)
}

/// [`ink`] for a `0xRRGGBB` data color.
pub fn ink_hex(theme: &Theme, hex: u32) -> Hsla {
    ink(theme, Hsla::from(rgb(hex)))
}

/// [`ink`] for an [`Rgba`] data color.
pub fn ink_rgba(theme: &Theme, color: Rgba) -> Hsla {
    ink(theme, Hsla::from(color))
}

/// [`ink`] for a d3 color value.
pub fn ink_d3(theme: &Theme, color: D3Color) -> Hsla {
    ink_rgba(theme, color.to_rgba())
}

/// Adapt a d3 `t -> color` ramp so every stop contrasts with the theme
/// background. Takes a precomputed [`background_lightness`] (a plain `f32`)
/// so the closure stays `'static` for GPU contour configs. The mapping is
/// constant inside the clash band, so ramps keep their stop order up to a
/// sub-perceptual step at the band edge.
pub fn ink_scale<F>(bg: f32, scale: F) -> impl Fn(f64) -> D3Color + Send + Sync
where
    F: Fn(f64) -> D3Color + Send + Sync,
{
    move |t: f64| {
        let rgba = scale(t).to_rgba();
        let adapted = adapt(bg, Hsla::from(rgba));
        let out = Rgba::from(adapted);
        D3Color {
            r: out.r,
            g: out.g,
            b: out.b,
            a: out.a,
        }
    }
}

/// Core of [`ink`] with a precomputed background lightness.
fn adapt(bg: f32, color: Hsla) -> Hsla {
    if (color.l - bg).abs() >= MIN_GAP {
        return color;
    }
    let up = (bg + TARGET_GAP).min(MAX_L);
    let down = (bg - TARGET_GAP).max(MIN_L);
    let l = if (up - bg).abs() >= (down - bg).abs() {
        up
    } else {
        down
    };
    Hsla { l, ..color }
}

/// [`ink`] for a d3 categorical scheme entry.
pub fn categorical(theme: &Theme, scheme: &ColorScheme, index: usize) -> Hsla {
    ink_d3(theme, scheme.color(index))
}

/// Gridlines: theme border, dimmed.
pub fn grid(theme: &Theme) -> Hsla {
    Hsla::from(theme.border).opacity(0.35)
}

/// Axis lines and tick marks.
pub fn axis_line(theme: &Theme) -> Hsla {
    Hsla::from(theme.text_muted)
}

/// Axis tick labels.
pub fn tick(theme: &Theme) -> Hsla {
    Hsla::from(theme.text_muted)
}

/// Missing-data marks: neutral gray, contrast-adapted like data ink.
pub fn missing(theme: &Theme) -> Hsla {
    ink(
        theme,
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.5,
            a: 1.0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark() -> Theme {
        Theme::dark()
    }

    fn light() -> Theme {
        Theme::light()
    }

    #[test]
    fn steelblue_passes_through_on_dark() {
        let theme = dark();
        let steel = Hsla::from(rgb(0x4682b4));
        assert_eq!(ink(&theme, steel), steel);
    }

    #[test]
    fn black_is_lifted_on_dark_and_kept_on_light() {
        let black = Hsla::from(rgb(0x111111));
        let on_dark = ink(&dark(), black);
        assert!(on_dark.l > 0.3, "black must lift off dark bg: {on_dark:?}");
        assert_eq!(on_dark.h, black.h);
        assert_eq!(on_dark.a, black.a);
        let on_light = ink(&light(), black);
        assert_eq!(on_light, black);
    }

    #[test]
    fn white_is_dimmed_on_light_and_kept_on_dark() {
        let white = Hsla::from(rgb(0xffffff));
        let on_light = ink(&light(), white);
        assert!(on_light.l < 0.7, "white must dim on light bg: {on_light:?}");
        assert_eq!(ink(&dark(), white), white);
    }

    #[test]
    fn category10_passes_through_on_dark() {
        let theme = dark();
        let scheme = ColorScheme::category10();
        for i in 0..10 {
            let raw = Hsla::from(scheme.color(i).to_rgba());
            assert_eq!(categorical(&theme, &scheme, i), raw, "entry {i} changed");
        }
    }

    #[test]
    fn chrome_comes_from_theme() {
        let theme = dark();
        assert_eq!(grid(&theme).h, Hsla::from(theme.border).h);
        assert_eq!(axis_line(&theme), Hsla::from(theme.text_muted));
        assert_eq!(tick(&theme), Hsla::from(theme.text_muted));
        let at = UiAxisTheme(&theme);
        assert_eq!(at.axis_line_color(), theme.text_muted);
        assert_eq!(at.axis_label_color(), theme.text_muted);
        assert_eq!(at.background_color(), Some(theme.background));
    }

    #[test]
    fn adapted_ramp_keeps_stop_order() {
        // A dark-to-light ramp adapted for the light theme must stay
        // non-decreasing up to the sub-perceptual band-edge step.
        let theme = light();
        let bg = background_lightness(&theme);
        let ramp = ink_scale(bg, |t: f64| D3Color {
            r: (t * 200.0) as f32 / 255.0,
            g: (t * 100.0) as f32 / 255.0,
            b: (t * 50.0) as f32 / 255.0,
            a: 1.0,
        });
        let mut prev = f32::NEG_INFINITY;
        let mut i = 0;
        while i <= 20 {
            let c = ramp(i as f64 / 20.0).to_rgba();
            let l = Hsla::from(c).l;
            assert!(
                l + 0.07 >= prev,
                "ramp inverts more than a band-edge step at {i}/20: {prev} -> {l}"
            );
            prev = prev.max(l);
            i += 1;
        }
    }

    #[test]
    fn adapted_colors_keep_hue_and_alpha() {
        // Near-black on every variant keeps hue/saturation/alpha, moves lightness.
        for variant in gpui_ui_kit::theme::ThemeVariant::all() {
            let theme = Theme::for_variant(*variant);
            let c = Hsla {
                h: 0.58,
                s: 0.5,
                l: 0.05,
                a: 0.8,
            };
            let out = ink(&theme, c);
            assert_eq!(out.h, c.h);
            assert_eq!(out.s, c.s);
            assert_eq!(out.a, c.a);
            let gap = (out.l - background_lightness(&theme)).abs();
            assert!(gap >= MIN_GAP - 1e-6, "{variant:?}: gap {gap}");
        }
    }

    #[test]
    fn scheme_entries_contrast_on_every_variant() {
        // Every categorical entry on every theme variant keeps its hue and
        // clears the background by MIN_GAP (d3 fidelity where it passes,
        // lightness shift where it would clash).
        for variant in gpui_ui_kit::theme::ThemeVariant::all() {
            let theme = Theme::for_variant(*variant);
            for scheme in [ColorScheme::category10(), ColorScheme::tableau10()] {
                for i in 0..scheme.len() {
                    let raw = Hsla::from(scheme.color(i).to_rgba());
                    let out = categorical(&theme, &scheme, i);
                    assert_eq!(out.h, raw.h, "{variant:?} entry {i}: hue moved");
                    assert_eq!(out.s, raw.s, "{variant:?} entry {i}: saturation moved");
                    assert_eq!(out.a, raw.a, "{variant:?} entry {i}: alpha moved");
                    let gap = (out.l - background_lightness(&theme)).abs();
                    assert!(gap >= MIN_GAP - 1e-6, "{variant:?} entry {i}: gap {gap}");
                }
            }
        }
    }
}
