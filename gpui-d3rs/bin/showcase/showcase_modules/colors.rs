use d3rs::color::{ColorScheme, D3Color};
use gpui::*;

pub fn render(_app: &ShowcaseApp) -> Div {
    let category10 = ColorScheme::category10();
    let tableau10 = ColorScheme::tableau10();
    let pastel = ColorScheme::pastel();

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Colors Demo"),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(0x666666))
                .max_w(px(700.0))
                .child("The d3-color module provides color manipulation utilities including RGB/HSL conversion, interpolation, and categorical color schemes for data visualization."),
        )
        // Category10
        .child(render_color_scheme_section(
            "Category10",
            "D3's classic 10-color categorical scheme. Vibrant colors ideal for distinguishing data series.",
            &category10,
        ))
        // Tableau10
        .child(render_color_scheme_section(
            "Tableau10",
            "A perceptually distinct 10-color scheme from Tableau. Optimized for visual clarity.",
            &tableau10,
        ))
        // Pastel
        .child(render_color_scheme_section(
            "Pastel",
            "Soft, muted colors for backgrounds or less prominent data elements.",
            &pastel,
        ))
        // RGB Interpolation
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("RGB Interpolation"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .mb_2()
                        .child("Smooth gradients between colors using RGB interpolation:"),
                )
                .child(render_gradient_row("Red → Blue", D3Color::rgb(255, 0, 0), D3Color::rgb(0, 0, 255)))
                .child(render_gradient_row("Green → Purple", D3Color::rgb(0, 200, 0), D3Color::rgb(128, 0, 128)))
                .child(render_gradient_row("Orange → Teal", D3Color::rgb(255, 165, 0), D3Color::rgb(0, 128, 128)))
                .child(render_gradient_row("Yellow → Navy", D3Color::rgb(255, 255, 0), D3Color::rgb(0, 0, 128))),
        )
        // HSL Gradient
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("HSL Color Wheel"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .mb_2()
                        .child("Full hue rotation (0°-360°) at different saturation/lightness levels:"),
                )
                .child(render_hsl_row("High Saturation", 1.0, 0.5))
                .child(render_hsl_row("Medium Saturation", 0.6, 0.5))
                .child(render_hsl_row("Low Saturation", 0.3, 0.5))
                .child(render_hsl_row("Light", 0.8, 0.75))
                .child(render_hsl_row("Dark", 0.8, 0.25)),
        )
        // Lighten/Darken
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Lighten / Darken"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .mb_2()
                        .child("Adjusting color brightness while preserving hue:"),
                )
                .child(render_brightness_row("Blue", D3Color::rgb(0, 122, 204)))
                .child(render_brightness_row("Red", D3Color::rgb(214, 39, 40)))
                .child(render_brightness_row("Green", D3Color::rgb(44, 160, 44)))
                .child(render_brightness_row("Orange", D3Color::rgb(255, 127, 14))),
        )
        // Opacity Blending
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Opacity Levels"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .mb_2()
                        .child("The same color at different opacity levels (on checkered background):"),
                )
                .child(render_opacity_row("Category10[0]", category10.color(0)))
                .child(render_opacity_row("Category10[3]", category10.color(3)))
                .child(render_opacity_row("Tableau10[1]", tableau10.color(1))),
        )
        // Named Colors
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Color Construction"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .mb_2()
                        .child("Multiple ways to create colors:"),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_3()
                        .child(render_named_color("from_hex(0x1f77b4)", D3Color::from_hex(0x1f77b4)))
                        .child(render_named_color("rgb(255, 127, 14)", D3Color::rgb(255, 127, 14)))
                        .child(render_named_color("from_hsl(120, 0.8, 0.5)", D3Color::from_hsl(120.0, 0.8, 0.5)))
                        .child(render_named_color("from_hsl(240, 0.7, 0.6)", D3Color::from_hsl(240.0, 0.7, 0.6)))
                        .child(render_named_color("from_hsl(0, 0.9, 0.5)", D3Color::from_hsl(0.0, 0.9, 0.5)))
                        .child(render_named_color("from_hsl(60, 1.0, 0.5)", D3Color::from_hsl(60.0, 1.0, 0.5))),
                ),
        )
        // Chromatic Scales
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Chromatic Scales"),
                )
                .child(
                   div()
                       .text_sm()
                       .text_color(rgb(0x666666))
                       .mb_2()
                       .child("Sequential and Diverging scales from d3-scale-chromatic:"),
                )
                // TODO: Fix type inference/coercion for function pointers to render_chromatic_row
                // .child(render_chromatic_row("Turbo", turbo_wrapper))
                // .child(render_chromatic_row("Viridis", viridis_wrapper))
                // .child(render_chromatic_row("Magma", magma_wrapper))
                // .child(render_chromatic_row("RdBu", rdbu_wrapper)),
                .child(div().text_sm().text_color(rgb(0x888888)).child("(Chromatic scales disabled due to temporary compilation issue)")),
        )
        // Code example
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .bg(rgb(0xf5f5f5))
                .rounded_lg()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Usage Example:"),
                )
                .child(
                    div()
                        .font_family("monospace")
                        .text_sm()
                        .p_3()
                        .bg(rgb(0xffffff))
                        .rounded_md()
                        .child(
                            "use d3rs::color::{ColorScheme, D3Color};\n\n\
                             // Categorical schemes\n\
                             let scheme = ColorScheme::category10();\n\
                             let color = scheme.color(0);  // Blue\n\n\
                             // Direct construction\n\
                             let red = D3Color::rgb(255, 0, 0);\n\
                             let hsl = D3Color::from_hsl(120.0, 0.8, 0.5);\n\n\
                             // Manipulation\n\
                             let lighter = red.lighten(0.3);\n\
                             let mixed = red.interpolate(&blue, 0.5);",
                        ),
                ),
        )
}

/// Render a color scheme section with swatches
fn render_color_scheme_section(
    name: &'static str,
    description: &'static str,
    scheme: &ColorScheme,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child(name),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x666666))
                .mb_1()
                .child(description),
        )
        .child(div().flex().gap_2().children((0..scheme.len()).map(|i| {
            let color = scheme.color(i);
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .w(px(50.0))
                        .h(px(50.0))
                        .rounded_lg()
                        .bg(color.to_rgba())
                        .shadow_sm(),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x888888))
                        .child(format!("{}", i)),
                )
        })))
}

/// Render a gradient row with label
fn render_gradient_row(label: &'static str, start: D3Color, end: D3Color) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(120.0))
                .text_sm()
                .text_color(rgb(0x333333))
                .child(label),
        )
        .child(div().flex().children((0..30).map(|i| {
            let t = i as f32 / 29.0;
            let color = start.interpolate(&end, t);
            div().w(px(14.0)).h(px(30.0)).bg(color.to_rgba())
        })))
}

/// Render an HSL hue rotation row
fn render_hsl_row(label: &'static str, saturation: f32, lightness: f32) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(120.0))
                .text_sm()
                .text_color(rgb(0x333333))
                .child(label),
        )
        .child(div().flex().children((0..36).map(|i| {
            let hue = i as f32 * 10.0;
            let color = D3Color::from_hsl(hue, saturation, lightness);
            div().w(px(12.0)).h(px(30.0)).bg(color.to_rgba())
        })))
}

/// Render a brightness adjustment row
fn render_brightness_row(label: &'static str, base: D3Color) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(120.0))
                .text_sm()
                .text_color(rgb(0x333333))
                .child(label),
        )
        .child(div().flex().children((0..11).map(|i| {
            let amount = (i as f32 - 5.0) / 5.0;
            let color = if amount < 0.0 {
                base.darken(-amount)
            } else {
                base.lighten(amount)
            };
            div().w(px(36.0)).h(px(30.0)).bg(color.to_rgba())
        })))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .ml_2()
                .child("← darker | lighter →"),
        )
}

/// Render an opacity row with checkered background
fn render_opacity_row(label: &'static str, color: D3Color) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(120.0))
                .text_sm()
                .text_color(rgb(0x333333))
                .child(label),
        )
        .child(div().flex().children((0..10).map(|i| {
            let opacity = (i + 1) as f32 / 10.0;
            let rgba = gpui::rgba(
                ((color.r * 255.0) as u32) << 24
                    | ((color.g * 255.0) as u32) << 16
                    | ((color.b * 255.0) as u32) << 8
                    | (opacity * 255.0) as u32,
            );
            div()
                .w(px(40.0))
                .h(px(30.0))
                .relative()
                // Checkered background
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgb(0xcccccc))
                        .child(div().absolute().w(px(20.0)).h(px(15.0)).bg(rgb(0xffffff)))
                        .child(
                            div()
                                .absolute()
                                .left(px(20.0))
                                .top(px(15.0))
                                .w(px(20.0))
                                .h(px(15.0))
                                .bg(rgb(0xffffff)),
                        ),
                )
                // Color overlay
                .child(div().absolute().inset_0().bg(rgba))
        })))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x888888))
                .ml_2()
                .child("10% → 100%"),
        )
}

/// Render a named color swatch
fn render_named_color(code: &'static str, color: D3Color) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .p_2()
        .bg(rgb(0xf8f8f8))
        .rounded_md()
        .child(
            div()
                .w(px(60.0))
                .h(px(40.0))
                .rounded_md()
                .bg(color.to_rgba())
                .shadow_sm(),
        )
        .child(
            div()
                .text_xs()
                .font_family("monospace")
                .text_color(rgb(0x666666))
                .child(code),
        )
}

use super::ShowcaseApp;

/// Render a chromatic scale row
/// TODO: Fix type inference for function pointers to enable usage
#[allow(dead_code)]
fn render_chromatic_row(label: &'static str, scale_fn: fn(f64) -> D3Color) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(120.0))
                .text_sm()
                .text_color(rgb(0x333333))
                .child(label),
        )
        .child(div().flex().children((0..50).map(|i| {
            let t = i as f64 / 49.0;
            let color = scale_fn(t);
            div().w(px(8.0)).h(px(30.0)).bg(color.to_rgba())
        })))
}
