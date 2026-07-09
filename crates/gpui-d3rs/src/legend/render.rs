//! GPUI legend renderer with automatic multi-column layout.
//!
//! Computes the optimal number of columns so that all legend items fit
//! within `available_width` without overlap, minimizing total height.

use gpui::*;

use super::{LegendConfig, LegendLayout, LegendSymbol};

/// Render a legend as a GPUI element with automatic multi-column layout.
///
/// Items are arranged into as many columns as fit within `available_width`.
/// Each column is as wide as its widest item. If all items fit on one row,
/// they are laid out horizontally.
///
/// # Arguments
/// * `config` - Legend configuration with items, colors, sizes
/// * `available_width` - Maximum width in pixels for the legend
/// * `text_color` - Color for label text (from theme)
/// * `bg_color` - Background color (from theme, `None` for transparent)
pub fn render_legend(
    config: &LegendConfig,
    available_width: f32,
    text_color: Rgba,
    bg_color: Option<Rgba>,
) -> Div {
    let items = &config.items;
    let padding = config.padding as f32;
    let layout = LegendLayout::from_config(config, f64::from(available_width));
    if layout.is_empty() {
        return div();
    }

    // Build grid: rows of items
    let mut container = div()
        .flex()
        .flex_col()
        .gap(px(config.item_spacing as f32))
        .w(px(layout.width as f32));

    if let Some(bg) = bg_color {
        container = container.bg(bg).rounded(px(4.0)).p(px(padding));
    }

    if let Some(title) = &layout.title {
        container = container.child(
            div()
                .text_size(px(config.font_size as f32))
                .text_color(text_color)
                .w(px(title.bounds.width as f32))
                .child(title.text.clone()),
        );
    }

    for row in 0..layout.rows {
        let mut row_div = div().flex().flex_row().gap(px(config.item_spacing as f32));
        for col in 0..layout.columns {
            let idx = row * layout.columns + col;
            let Some(item_layout) = layout.items.get(idx) else {
                // Empty cell — add spacer to keep alignment
                let col_w = layout.column_widths.get(col).copied().unwrap_or(0.0) as f32;
                row_div = row_div.child(div().w(px(col_w)));
                continue;
            };
            let item = &items[idx];
            let swatch_color = item.color.to_rgba();
            let swatch = match item.symbol {
                LegendSymbol::Line | LegendSymbol::DashedLine | LegendSymbol::LineWithMarker => {
                    div()
                        .w(px(item_layout.symbol_bounds.width as f32))
                        .h(px(2.0))
                        .bg(swatch_color)
                        .my_auto()
                }
                LegendSymbol::None => div().w(px(0.0)).h(px(0.0)),
                _ => div()
                    .w(px(item_layout.symbol_bounds.width as f32))
                    .h(px(item_layout.symbol_bounds.height as f32))
                    .bg(swatch_color)
                    .rounded(px(2.0)),
            };

            row_div = row_div.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px((config.symbol_size / 3.0) as f32))
                    .w(px(item_layout.item_bounds.width as f32))
                    .child(swatch)
                    .child(
                        div()
                            .text_size(px(config.font_size as f32))
                            .text_color(text_color)
                            .child(item.label.clone()),
                    ),
            );
        }
        container = container.child(row_div);
    }

    container
}

// Tests for render_legend are integration tests (require GPUI runtime).
// The column layout algorithm is verified by the legend::tests in mod.rs.
