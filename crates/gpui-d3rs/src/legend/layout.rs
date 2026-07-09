//! Renderer-independent legend layout.

use super::{LegendConfig, LegendOrientation, LegendSymbol};
use std::error::Error;
use std::fmt;

const SYMBOL_GAP_FACTOR: f64 = 1.0 / 3.0;
const TITLE_LINE_HEIGHT: f64 = 1.5;

/// Checked legend-layout error.
#[derive(Debug, Clone, PartialEq)]
pub enum LegendLayoutError {
    /// Available size contained a non-finite value.
    NonFiniteSize { field: &'static str },
    /// Available size contained a negative value.
    NegativeSize { field: &'static str },
    /// Legend visual configuration contained a non-finite value.
    NonFiniteConfig { field: &'static str },
    /// Legend visual configuration contained a negative value.
    NegativeConfig { field: &'static str },
    /// Average character width was not positive.
    NonPositiveAverageCharWidth { value: f64 },
}

impl fmt::Display for LegendLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteSize { field } => {
                write!(f, "legend size field {field} must be finite")
            }
            Self::NegativeSize { field } => {
                write!(f, "legend size field {field} must be non-negative")
            }
            Self::NonFiniteConfig { field } => {
                write!(f, "legend configuration field {field} must be finite")
            }
            Self::NegativeConfig { field } => {
                write!(f, "legend configuration field {field} must be non-negative")
            }
            Self::NonPositiveAverageCharWidth { value } => {
                write!(
                    f,
                    "legend average character width must be positive, got {value}"
                )
            }
        }
    }
}

impl Error for LegendLayoutError {}

/// Two-dimensional point in legend-local pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendPoint {
    pub x: f64,
    pub y: f64,
}

impl LegendPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Rectangle in legend-local pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendRect {
    pub origin: LegendPoint,
    pub width: f64,
    pub height: f64,
}

impl LegendRect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: LegendPoint::new(x, y),
            width,
            height,
        }
    }
}

/// Renderer-independent geometry for one legend title.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendTitleLayout {
    pub text: String,
    pub bounds: LegendRect,
}

/// Renderer-independent geometry for one legend item.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendItemLayout {
    pub index: usize,
    pub row: usize,
    pub column: usize,
    pub label: String,
    pub symbol: LegendSymbol,
    pub item_bounds: LegendRect,
    pub symbol_bounds: LegendRect,
    pub label_bounds: LegendRect,
}

/// Renderer-independent legend geometry derived from [`LegendConfig`].
#[derive(Debug, Clone, PartialEq)]
pub struct LegendLayout {
    pub width: f64,
    pub height: f64,
    pub columns: usize,
    pub rows: usize,
    pub column_widths: Vec<f64>,
    pub title: Option<LegendTitleLayout>,
    pub items: Vec<LegendItemLayout>,
}

impl LegendLayout {
    /// Build checked legend geometry using the crate's approximate text metrics.
    pub fn try_from_config(
        config: &LegendConfig,
        available_width: f64,
    ) -> Result<Self, LegendLayoutError> {
        Self::try_from_config_with_char_width(config, available_width, config.font_size * 0.6)
    }

    /// Build checked legend geometry with an explicit average character width.
    pub fn try_from_config_with_char_width(
        config: &LegendConfig,
        available_width: f64,
        avg_char_width: f64,
    ) -> Result<Self, LegendLayoutError> {
        validate_config(config, available_width, avg_char_width)?;

        if config.items.is_empty() && config.title.is_none() {
            return Ok(Self {
                width: 0.0,
                height: 0.0,
                columns: 0,
                rows: 0,
                column_widths: Vec::new(),
                title: None,
                items: Vec::new(),
            });
        }

        let available_width = config
            .max_width
            .map_or(available_width, |max_width| available_width.min(max_width));
        let symbol_gap = config.symbol_size * SYMBOL_GAP_FACTOR;
        let item_widths = config
            .items
            .iter()
            .map(|item| item_width(config, item.label.len(), avg_char_width, symbol_gap))
            .collect::<Vec<_>>();
        let usable_width = (available_width - config.padding * 2.0).max(0.0);
        let columns = choose_columns(config, &item_widths, usable_width);
        let rows = if columns == 0 {
            0
        } else {
            config.items.len().div_ceil(columns)
        };
        let column_widths = column_widths(columns, &item_widths);

        let title_height = config
            .title
            .as_ref()
            .map(|_| config.font_size * TITLE_LINE_HEIGHT)
            .unwrap_or(0.0);
        let item_block_y = config.padding + title_height;
        let item_step = config.symbol_size + config.item_spacing;
        let item_block_height = if rows == 0 {
            0.0
        } else {
            rows as f64 * config.symbol_size + rows.saturating_sub(1) as f64 * config.item_spacing
        };
        let content_width = if column_widths.is_empty() {
            config
                .title
                .as_ref()
                .map(|title| title.len() as f64 * avg_char_width)
                .unwrap_or(0.0)
        } else {
            column_widths.iter().sum::<f64>()
                + config.item_spacing * column_widths.len().saturating_sub(1) as f64
        };
        let title_width = config
            .title
            .as_ref()
            .map(|title| title.len() as f64 * avg_char_width)
            .unwrap_or(0.0);
        let width = config.padding * 2.0 + content_width.max(title_width);
        let height = config.padding * 2.0 + title_height + item_block_height;

        let title = config.title.as_ref().map(|text| LegendTitleLayout {
            text: text.clone(),
            bounds: LegendRect::new(config.padding, config.padding, title_width, title_height),
        });

        let mut items = Vec::with_capacity(config.items.len());
        for (index, item) in config.items.iter().enumerate() {
            let row = index.checked_div(columns).unwrap_or(0);
            let column = index.checked_rem(columns).unwrap_or(0);
            let x = config.padding
                + column_widths.iter().take(column).sum::<f64>()
                + config.item_spacing * column as f64;
            let y = item_block_y + row as f64 * item_step;
            let item_width = column_widths.get(column).copied().unwrap_or(0.0);
            let label_width = item.label.len() as f64 * avg_char_width;
            let symbol_size = symbol_box_size(config, item.symbol);
            let symbol_y = y + (config.symbol_size - symbol_size) / 2.0;
            let label_x = x + config.symbol_size + symbol_gap;

            items.push(LegendItemLayout {
                index,
                row,
                column,
                label: item.label.clone(),
                symbol: item.symbol,
                item_bounds: LegendRect::new(x, y, item_width, config.symbol_size),
                symbol_bounds: LegendRect::new(x, symbol_y, symbol_size, symbol_size),
                label_bounds: LegendRect::new(
                    label_x,
                    y,
                    label_width,
                    config.symbol_size.max(config.font_size),
                ),
            });
        }

        Ok(Self {
            width,
            height,
            columns,
            rows,
            column_widths,
            title,
            items,
        })
    }

    /// Build legend geometry and panic on invalid input, matching existing
    /// permissive legend helper behavior.
    pub fn from_config(config: &LegendConfig, available_width: f64) -> Self {
        Self::try_from_config(config, available_width).expect("invalid legend layout configuration")
    }

    /// Return true when no legend primitives will be drawn.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.items.is_empty()
    }
}

/// Convenience helper for checked legend layout.
pub fn legend_layout(
    config: &LegendConfig,
    available_width: f64,
) -> Result<LegendLayout, LegendLayoutError> {
    LegendLayout::try_from_config(config, available_width)
}

fn validate_config(
    config: &LegendConfig,
    available_width: f64,
    avg_char_width: f64,
) -> Result<(), LegendLayoutError> {
    if !available_width.is_finite() {
        return Err(LegendLayoutError::NonFiniteSize {
            field: "available_width",
        });
    }
    if available_width < 0.0 {
        return Err(LegendLayoutError::NegativeSize {
            field: "available_width",
        });
    }

    if !avg_char_width.is_finite() {
        return Err(LegendLayoutError::NonFiniteConfig {
            field: "avg_char_width",
        });
    }
    if avg_char_width <= 0.0 {
        return Err(LegendLayoutError::NonPositiveAverageCharWidth {
            value: avg_char_width,
        });
    }

    for (field, value) in [
        ("symbol_size", config.symbol_size),
        ("item_spacing", config.item_spacing),
        ("padding", config.padding),
        ("border_width", config.border_width),
        ("font_size", config.font_size),
    ] {
        if !value.is_finite() {
            return Err(LegendLayoutError::NonFiniteConfig { field });
        }
        if value < 0.0 {
            return Err(LegendLayoutError::NegativeConfig { field });
        }
    }

    if let Some(max_width) = config.max_width {
        if !max_width.is_finite() {
            return Err(LegendLayoutError::NonFiniteConfig { field: "max_width" });
        }
        if max_width < 0.0 {
            return Err(LegendLayoutError::NegativeConfig { field: "max_width" });
        }
    }

    Ok(())
}

fn choose_columns(config: &LegendConfig, item_widths: &[f64], usable_width: f64) -> usize {
    let n = item_widths.len();
    if n == 0 {
        return 0;
    }

    match config.orientation {
        LegendOrientation::Vertical => {
            let mut best_cols = 1_usize;
            for columns in 1..=n {
                let widths = column_widths(columns, item_widths);
                let total_width = widths.iter().sum::<f64>()
                    + config.item_spacing * columns.saturating_sub(1) as f64;
                if total_width <= usable_width {
                    best_cols = columns;
                } else if columns > 1 {
                    break;
                }
                if n.div_ceil(columns) == 1 {
                    break;
                }
            }
            best_cols
        }
        LegendOrientation::Horizontal => {
            for columns in (1..=n).rev() {
                let widths = column_widths(columns, item_widths);
                let total_width = widths.iter().sum::<f64>()
                    + config.item_spacing * columns.saturating_sub(1) as f64;
                if total_width <= usable_width || columns == 1 {
                    return columns;
                }
            }
            1
        }
    }
}

fn column_widths(columns: usize, item_widths: &[f64]) -> Vec<f64> {
    let mut widths = vec![0.0_f64; columns];
    for (index, &width) in item_widths.iter().enumerate() {
        widths[index % columns] = widths[index % columns].max(width);
    }
    widths
}

fn item_width(
    config: &LegendConfig,
    label_len: usize,
    avg_char_width: f64,
    symbol_gap: f64,
) -> f64 {
    config.symbol_size + symbol_gap + label_len as f64 * avg_char_width
}

fn symbol_box_size(config: &LegendConfig, symbol: LegendSymbol) -> f64 {
    match symbol {
        LegendSymbol::Line | LegendSymbol::DashedLine | LegendSymbol::LineWithMarker => {
            config.symbol_size
        }
        LegendSymbol::None => 0.0,
        LegendSymbol::Circle | LegendSymbol::Square => config.symbol_size * 0.8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::D3Color;
    use crate::legend::{LegendItem, LegendPosition};

    fn legend_items() -> Vec<LegendItem> {
        vec![
            LegendItem::color("Alpha", D3Color::rgb(31, 119, 180)),
            LegendItem::line("Beta", D3Color::rgb(255, 127, 14)),
            LegendItem::with_symbol(
                "Long Gamma",
                D3Color::rgb(44, 160, 44),
                LegendSymbol::Square,
            ),
        ]
    }

    #[test]
    fn legend_layout_builds_checked_item_geometry() {
        let config = LegendConfig::new()
            .title("Series")
            .items(legend_items())
            .symbol_size(12.0)
            .item_spacing(6.0)
            .padding(4.0)
            .font_size(10.0);

        let layout = LegendLayout::try_from_config_with_char_width(&config, 240.0, 5.0).unwrap();

        assert_eq!(layout.columns, 3);
        assert_eq!(layout.rows, 1);
        assert_eq!(layout.title.as_ref().unwrap().text, "Series");
        assert_eq!(layout.items.len(), 3);
        assert_eq!(layout.items[0].row, 0);
        assert_eq!(layout.items[0].column, 0);
        assert_eq!(
            layout.items[0].item_bounds.origin,
            LegendPoint::new(4.0, 19.0)
        );
        assert_eq!(layout.items[1].symbol, LegendSymbol::Line);
        assert!(layout.items[2].label_bounds.width > layout.items[0].label_bounds.width);
        assert!(layout.width > 0.0);
        assert!(layout.height > 0.0);
    }

    #[test]
    fn legend_layout_wraps_when_width_is_constrained() {
        let config = LegendConfig::new()
            .position(LegendPosition::Bottom)
            .items(legend_items())
            .symbol_size(12.0)
            .item_spacing(6.0)
            .padding(4.0)
            .max_width(80.0);

        let layout = legend_layout(&config, 240.0).unwrap();

        assert_eq!(layout.columns, 1);
        assert_eq!(layout.rows, 3);
        assert_eq!(layout.items[2].row, 2);
        assert_eq!(layout.items[2].column, 0);
    }

    #[test]
    fn horizontal_legend_prefers_one_row_when_it_fits() {
        let config = LegendConfig::new()
            .orientation(LegendOrientation::Horizontal)
            .items(legend_items())
            .symbol_size(12.0)
            .item_spacing(6.0)
            .padding(4.0);

        let layout = legend_layout(&config, 400.0).unwrap();

        assert_eq!(layout.columns, 3);
        assert_eq!(layout.rows, 1);
    }

    #[test]
    fn legend_layout_reports_empty_legend() {
        let layout = legend_layout(&LegendConfig::new(), 200.0).unwrap();

        assert!(layout.is_empty());
        assert_eq!(layout.width, 0.0);
        assert_eq!(layout.height, 0.0);
    }

    #[test]
    fn legend_layout_rejects_invalid_inputs() {
        let mut config = LegendConfig::new().items(legend_items());
        config.symbol_size = f64::NAN;
        assert_eq!(
            legend_layout(&config, 200.0),
            Err(LegendLayoutError::NonFiniteConfig {
                field: "symbol_size"
            })
        );

        config.symbol_size = 12.0;
        config.max_width = Some(-1.0);
        assert_eq!(
            legend_layout(&config, 200.0),
            Err(LegendLayoutError::NegativeConfig { field: "max_width" })
        );

        config.max_width = None;
        assert_eq!(
            LegendLayout::try_from_config_with_char_width(&config, 200.0, 0.0),
            Err(LegendLayoutError::NonPositiveAverageCharWidth { value: 0.0 })
        );

        assert_eq!(
            legend_layout(&config, f64::INFINITY),
            Err(LegendLayoutError::NonFiniteSize {
                field: "available_width"
            })
        );
    }
}
