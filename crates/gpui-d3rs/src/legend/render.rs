//! GPUI legend renderer with automatic multi-column layout.
//!
//! Computes the optimal number of columns so that all legend items fit
//! within `available_width` without overlap, minimizing total height.

use gpui::*;

use super::{LegendConfig, LegendSymbol};

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
    if items.is_empty() {
        return div();
    }

    let font_size = config.font_size as f32;
    let symbol_size = config.symbol_size as f32;
    let gap = config.item_spacing as f32;
    let padding = config.padding as f32;
    // Approximate char width as 0.6 * font_size for proportional fonts
    let char_w = font_size * 0.6;
    let symbol_gap = 4.0_f32; // gap between swatch and label

    // Compute width of each item: swatch + gap + label text
    let item_widths: Vec<f32> = items
        .iter()
        .map(|item| symbol_size + symbol_gap + item.label.len() as f32 * char_w)
        .collect();

    let usable = available_width - padding * 2.0;

    // Find maximum number of columns that fit.
    // Strategy: try N columns, assign items round-robin, check if the sum
    // of widest-per-column + gaps fits within usable width.
    let n = items.len();
    let mut best_cols = 1_usize;

    for cols in 1..=n {
        let rows = n.div_ceil(cols);
        // Width of each column = max item width in that column
        let mut col_widths = vec![0.0_f32; cols];
        for (i, &w) in item_widths.iter().enumerate() {
            let col = i % cols;
            col_widths[col] = col_widths[col].max(w);
        }
        let total_w: f32 = col_widths.iter().sum::<f32>() + gap * (cols as f32 - 1.0);
        if total_w <= usable {
            best_cols = cols;
        } else if cols > 1 {
            // Adding more columns only makes it wider — stop
            break;
        }
        // Stop if we're already down to 1 row
        if rows == 1 {
            break;
        }
    }

    let cols = best_cols;
    let rows = n.div_ceil(cols);

    // Compute final column widths
    let mut col_widths = vec![0.0_f32; cols];
    for (i, &w) in item_widths.iter().enumerate() {
        let col = i % cols;
        col_widths[col] = col_widths[col].max(w);
    }

    // Build grid: rows of items
    let mut container = div().flex().flex_col().gap(px(gap));

    if let Some(bg) = bg_color {
        container = container.bg(bg).rounded(px(4.0)).p(px(padding));
    }

    for row in 0..rows {
        let mut row_div = div().flex().flex_row().gap(px(gap));
        for (col, &col_w) in col_widths.iter().enumerate() {
            let idx = row * cols + col;
            if idx >= n {
                // Empty cell — add spacer to keep alignment
                row_div = row_div.child(div().w(px(col_w)));
                continue;
            }
            let item = &items[idx];
            let swatch_color = item.color.to_rgba();
            let swatch = match item.symbol {
                LegendSymbol::Line | LegendSymbol::DashedLine | LegendSymbol::LineWithMarker => {
                    div()
                        .w(px(symbol_size))
                        .h(px(2.0))
                        .bg(swatch_color)
                        .my_auto()
                }
                _ => div()
                    .w(px(symbol_size * 0.8))
                    .h(px(symbol_size * 0.8))
                    .bg(swatch_color)
                    .rounded(px(2.0)),
            };

            row_div = row_div.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(symbol_gap))
                    .w(px(col_w))
                    .child(swatch)
                    .child(
                        div()
                            .text_size(px(font_size))
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
