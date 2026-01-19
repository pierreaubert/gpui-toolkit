use gpui::*;

pub fn render(_app: &ShowcaseApp) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Welcome to d3rs"),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(0x666666))
                .max_w(px(600.0))
                .child("d3rs is a D3.js-inspired plotting library for GPUI. It brings familiar D3 concepts like scales, axes, and shape generators to Rust applications built with GPUI."),
        )
        .child(
            div()
                .mt_4()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Demos"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .ml_4()
                .child(feature_item("Scales", "Linear, logarithmic, ordinal, and band scales"))
                .child(feature_item("Axes", "Customizable axes with tick formatting"))
                .child(feature_item("Bar Charts", "Simple and grouped bar charts"))
                .child(feature_item("Line Charts", "Line charts with points and curves"))
                .child(feature_item("Scatter Plots", "Scatter plots with symbols"))
                .child(feature_item("Surface Plots", "2D surface and heatmap visualizations"))
                .child(feature_item("QuadTree", "Spatial indexing and nearest neighbor search"))
                .child(feature_item("Contours", "Contour lines and density estimation"))
                .child(feature_item("Transitions", "Animated transitions and easing"))
                .child(feature_item("Geo", "Geographic projections and maps"))
                .child(feature_item("Colors", "Color schemes and interpolation")),
        )
        .child(
            div()
                .mt_4()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child("D3 Observable Examples"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .ml_4()
                .child(feature_item("Volcano", "Volcano contour visualization"))
                .child(feature_item("KDE", "Kernel density estimation"))
                .child(feature_item("Treemap", "Hierarchical treemap layout"))
                .child(feature_item("Stacked Bars", "Stacked and grouped bar charts")),
        )
        .child(
            div()
                .mt_6()
                .p_4()
                .bg(rgb(0xf5f5f5))
                .rounded_md()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Use the sidebar to explore different demos"),
                ),
        )
}

pub fn feature_item(title: &str, desc: &str) -> Div {
    div()
        .flex()
        .gap_2()
        .child(
            div()
                .text_color(rgb(0x007acc))
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("{title}:")),
        )
        .child(div().text_color(rgb(0x666666)).child(desc.to_string()))
}

use super::ShowcaseApp;
