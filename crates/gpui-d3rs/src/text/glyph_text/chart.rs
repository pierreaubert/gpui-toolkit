use super::consts::CHART_FONT_FAMILY;
use super::glyph_text_config::GlyphTextConfig;
use super::glyph_text_config::logical_text_box;
use super::misc::escape_svg_text;
use super::misc::svg_text_anchor;
use super::types::ChartTextLayout;
use super::types::HorizontalTextAnchor;
use super::types::LogicalTextBox;
use super::types::VerticalTextAnchor;
use gpui::SharedString;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(super) fn chart_text_layout(
    text: &str,
    config: &GlyphTextConfig,
    horizontal_anchor: HorizontalTextAnchor,
    vertical_anchor: VerticalTextAnchor,
) -> ChartTextLayout {
    let logical = logical_text_box(text, config, horizontal_anchor, vertical_anchor);
    let padding = chart_text_padding(config.font_size);
    let cos_r = config.rotation.cos();
    let sin_r = config.rotation.sin();
    let corners = [
        [0.0, 0.0],
        [logical.width, 0.0],
        [0.0, logical.height],
        [logical.width, logical.height],
    ];

    let rotate_point = |point: [f32; 2]| -> [f32; 2] {
        let dx = point[0] - logical.anchor[0];
        let dy = point[1] - logical.anchor[1];
        [
            logical.anchor[0] + dx * cos_r - dy * sin_r,
            logical.anchor[1] + dx * sin_r + dy * cos_r,
        ]
    };

    let mut min_x = logical.anchor[0];
    let mut max_x = logical.anchor[0];
    let mut min_y = logical.anchor[1];
    let mut max_y = logical.anchor[1];
    for corner in corners {
        let rotated = rotate_point(corner);
        min_x = min_x.min(rotated[0]);
        max_x = max_x.max(rotated[0]);
        min_y = min_y.min(rotated[1]);
        max_y = max_y.max(rotated[1]);
    }

    let width = (max_x - min_x + padding * 2.0).ceil().max(1.0);
    let height = (max_y - min_y + padding * 2.0).ceil().max(1.0);
    let translate_x = padding - min_x;
    let translate_y = padding - min_y;
    let anchor = [
        logical.anchor[0] + translate_x,
        logical.anchor[1] + translate_y,
    ];
    let svg = chart_text_svg(
        text,
        config,
        &logical,
        width,
        height,
        translate_x,
        translate_y,
    );
    let cache_key = chart_text_cache_key(
        text,
        config,
        horizontal_anchor,
        vertical_anchor,
        width,
        height,
    );

    ChartTextLayout {
        svg,
        cache_key,
        width,
        height,
        anchor,
    }
}

pub(super) fn chart_line_height(font_size: f32) -> f32 {
    (font_size * 1.25).ceil().max(1.0)
}

pub(super) fn chart_baseline_y(font_size: f32, line_height: f32) -> f32 {
    (font_size * 0.85).ceil().clamp(0.0, line_height)
}

pub(super) fn chart_text_padding(font_size: f32) -> f32 {
    (font_size * 0.75).ceil().max(4.0)
}

pub(super) fn chart_text_svg(
    text: &str,
    config: &GlyphTextConfig,
    logical: &LogicalTextBox,
    width: f32,
    height: f32,
    translate_x: f32,
    translate_y: f32,
) -> String {
    let escaped = escape_svg_text(text);
    let text_anchor = svg_text_anchor(logical.anchor[0], logical.width);
    let letter_spacing = if config.letter_spacing.abs() > f32::EPSILON {
        format!(r#" letter-spacing="{:.3}px""#, config.letter_spacing)
    } else {
        String::new()
    };
    let rotation = config.rotation.to_degrees();

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.3}" height="{height:.3}" viewBox="0 0 {width:.3} {height:.3}">
<g transform="translate({translate_x:.3} {translate_y:.3}) rotate({rotation:.6} {anchor_x:.3} {anchor_y:.3})">
<text x="{anchor_x:.3}" y="{baseline_y:.3}" text-anchor="{text_anchor}" dominant-baseline="alphabetic" font-family="{font_family}" font-size="{font_size:.3}" fill="black" xml:space="preserve"{letter_spacing}>{escaped}</text>
</g>
</svg>"#,
        anchor_x = logical.anchor[0],
        anchor_y = logical.anchor[1],
        baseline_y = logical.baseline_y,
        font_family = CHART_FONT_FAMILY,
        font_size = config.font_size,
    )
}

pub(super) fn chart_text_cache_key(
    text: &str,
    config: &GlyphTextConfig,
    horizontal_anchor: HorizontalTextAnchor,
    vertical_anchor: VerticalTextAnchor,
    width: f32,
    height: f32,
) -> SharedString {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    config.font_size.to_bits().hash(&mut hasher);
    config.rotation.to_bits().hash(&mut hasher);
    config.letter_spacing.to_bits().hash(&mut hasher);
    horizontal_anchor.hash(&mut hasher);
    vertical_anchor.hash(&mut hasher);
    width.to_bits().hash(&mut hasher);
    height.to_bits().hash(&mut hasher);
    SharedString::from(format!("d3rs-chart-text:{:016x}", hasher.finish()))
}
