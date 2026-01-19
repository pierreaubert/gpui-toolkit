//! Legend module for chart legends
//!
//! This module provides types and utilities for creating chart legends
//! that explain the meaning of colors, symbols, or other visual encodings.
//!
//! # Example
//!
//! ```
//! use d3rs::legend::{LegendConfig, LegendItem, LegendPosition, LegendOrientation};
//! use d3rs::color::D3Color;
//!
//! let legend = LegendConfig::new()
//!     .position(LegendPosition::TopRight)
//!     .orientation(LegendOrientation::Vertical)
//!     .title("Categories")
//!     .items(vec![
//!         LegendItem::color("Group A", D3Color::rgb(31, 119, 180)),
//!         LegendItem::color("Group B", D3Color::rgb(255, 127, 14)),
//!         LegendItem::color("Group C", D3Color::rgb(44, 160, 44)),
//!     ]);
//!
//! assert_eq!(legend.items.len(), 3);
//! ```

use crate::color::D3Color;

/// Position of the legend relative to the chart
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendPosition {
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    #[default]
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    BottomRight,
    /// Centered above the chart
    Top,
    /// Centered below the chart
    Bottom,
    /// Centered to the left of the chart
    Left,
    /// Centered to the right of the chart
    Right,
}

/// Orientation of legend items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendOrientation {
    /// Items arranged horizontally
    Horizontal,
    /// Items arranged vertically
    #[default]
    Vertical,
}

/// Symbol type for legend items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendSymbol {
    /// Filled circle
    #[default]
    Circle,
    /// Filled square
    Square,
    /// Horizontal line
    Line,
    /// Line with marker
    LineWithMarker,
    /// Dashed line
    DashedLine,
    /// No symbol (color swatch only)
    None,
}

/// A single item in the legend
#[derive(Debug, Clone)]
pub struct LegendItem {
    /// Label text
    pub label: String,
    /// Color for this item
    pub color: D3Color,
    /// Symbol type
    pub symbol: LegendSymbol,
    /// Optional custom data
    pub data: Option<String>,
}

impl LegendItem {
    /// Create a new legend item with a color
    pub fn color(label: impl Into<String>, color: impl Into<D3Color>) -> Self {
        Self {
            label: label.into(),
            color: color.into(),
            symbol: LegendSymbol::Circle,
            data: None,
        }
    }

    /// Create a legend item for a line series
    pub fn line(label: impl Into<String>, color: impl Into<D3Color>) -> Self {
        Self {
            label: label.into(),
            color: color.into(),
            symbol: LegendSymbol::Line,
            data: None,
        }
    }

    /// Create a legend item with custom symbol
    pub fn with_symbol(
        label: impl Into<String>,
        color: impl Into<D3Color>,
        symbol: LegendSymbol,
    ) -> Self {
        Self {
            label: label.into(),
            color: color.into(),
            symbol,
            data: None,
        }
    }

    /// Set the symbol type
    pub fn symbol(mut self, symbol: LegendSymbol) -> Self {
        self.symbol = symbol;
        self
    }

    /// Attach custom data
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }
}

/// Configuration for a chart legend
#[derive(Debug, Clone)]
pub struct LegendConfig {
    /// Position of the legend
    pub position: LegendPosition,
    /// Orientation of items
    pub orientation: LegendOrientation,
    /// Optional title
    pub title: Option<String>,
    /// Legend items
    pub items: Vec<LegendItem>,
    /// Symbol size in pixels
    pub symbol_size: f64,
    /// Spacing between items
    pub item_spacing: f64,
    /// Padding around the legend
    pub padding: f64,
    /// Whether the legend has a background
    pub background: bool,
    /// Background color (if enabled)
    pub background_color: D3Color,
    /// Border width (0 = no border)
    pub border_width: f64,
    /// Border color
    pub border_color: D3Color,
    /// Font size for labels
    pub font_size: f64,
    /// Maximum width (for text wrapping)
    pub max_width: Option<f64>,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl LegendConfig {
    /// Create a new legend configuration with defaults
    pub fn new() -> Self {
        Self {
            position: LegendPosition::TopRight,
            orientation: LegendOrientation::Vertical,
            title: None,
            items: Vec::new(),
            symbol_size: 12.0,
            item_spacing: 8.0,
            padding: 8.0,
            background: true,
            background_color: D3Color::rgb(255, 255, 255),
            border_width: 1.0,
            border_color: D3Color::rgb(200, 200, 200),
            font_size: 12.0,
            max_width: None,
        }
    }

    /// Set the position
    pub fn position(mut self, position: LegendPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the orientation
    pub fn orientation(mut self, orientation: LegendOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the legend items
    pub fn items(mut self, items: Vec<LegendItem>) -> Self {
        self.items = items;
        self
    }

    /// Add a single item
    pub fn add_item(mut self, item: LegendItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the symbol size
    pub fn symbol_size(mut self, size: f64) -> Self {
        self.symbol_size = size;
        self
    }

    /// Set the item spacing
    pub fn item_spacing(mut self, spacing: f64) -> Self {
        self.item_spacing = spacing;
        self
    }

    /// Set the padding
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// Enable or disable background
    pub fn background(mut self, enabled: bool) -> Self {
        self.background = enabled;
        self
    }

    /// Set background color
    pub fn background_color(mut self, color: impl Into<D3Color>) -> Self {
        self.background_color = color.into();
        self
    }

    /// Set border width
    pub fn border_width(mut self, width: f64) -> Self {
        self.border_width = width;
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: impl Into<D3Color>) -> Self {
        self.border_color = color.into();
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Set maximum width
    pub fn max_width(mut self, width: f64) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Estimate the dimensions of the legend
    ///
    /// Returns (width, height) in pixels.
    /// This is an estimate based on average character width.
    pub fn estimate_dimensions(&self, avg_char_width: f64) -> (f64, f64) {
        if self.items.is_empty() {
            return (0.0, 0.0);
        }

        let title_height = if self.title.is_some() {
            self.font_size * 1.5
        } else {
            0.0
        };

        match self.orientation {
            LegendOrientation::Vertical => {
                let max_label_width = self
                    .items
                    .iter()
                    .map(|item| item.label.len() as f64 * avg_char_width)
                    .fold(0.0, f64::max);
                let width = self.padding * 2.0 + self.symbol_size + 8.0 + max_label_width;
                let height = self.padding * 2.0
                    + title_height
                    + self.items.len() as f64 * (self.symbol_size + self.item_spacing)
                    - self.item_spacing;
                (width, height)
            }
            LegendOrientation::Horizontal => {
                let total_width: f64 = self
                    .items
                    .iter()
                    .map(|item| {
                        self.symbol_size
                            + 8.0
                            + item.label.len() as f64 * avg_char_width
                            + self.item_spacing
                    })
                    .sum();
                let width = self.padding * 2.0 + total_width - self.item_spacing;
                let height = self.padding * 2.0 + title_height + self.symbol_size;
                (width, height)
            }
        }
    }

    /// Calculate offset from chart corner for the legend
    pub fn offset_from_corner(
        &self,
        chart_width: f64,
        chart_height: f64,
        legend_width: f64,
        legend_height: f64,
        margin: f64,
    ) -> (f64, f64) {
        match self.position {
            LegendPosition::TopLeft => (margin, margin),
            LegendPosition::TopRight => (chart_width - legend_width - margin, margin),
            LegendPosition::BottomLeft => (margin, chart_height - legend_height - margin),
            LegendPosition::BottomRight => (
                chart_width - legend_width - margin,
                chart_height - legend_height - margin,
            ),
            LegendPosition::Top => ((chart_width - legend_width) / 2.0, margin),
            LegendPosition::Bottom => (
                (chart_width - legend_width) / 2.0,
                chart_height - legend_height - margin,
            ),
            LegendPosition::Left => (margin, (chart_height - legend_height) / 2.0),
            LegendPosition::Right => (
                chart_width - legend_width - margin,
                (chart_height - legend_height) / 2.0,
            ),
        }
    }
}

/// Create items from a color scale for continuous legends
pub fn legend_from_scale<F>(
    scale: F,
    ticks: &[f64],
    format: impl Fn(f64) -> String,
) -> Vec<LegendItem>
where
    F: Fn(f64) -> D3Color,
{
    ticks
        .iter()
        .map(|&t| LegendItem::color(format(t), scale(t)).symbol(LegendSymbol::Square))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legend_item_creation() {
        let item = LegendItem::color("Test", D3Color::rgb(255, 0, 0));
        assert_eq!(item.label, "Test");
        assert_eq!(item.symbol, LegendSymbol::Circle);
    }

    #[test]
    fn test_legend_item_line() {
        let item = LegendItem::line("Series", D3Color::rgb(0, 128, 255));
        assert_eq!(item.symbol, LegendSymbol::Line);
    }

    #[test]
    fn test_legend_config_builder() {
        let legend = LegendConfig::new()
            .position(LegendPosition::BottomLeft)
            .orientation(LegendOrientation::Horizontal)
            .title("My Legend")
            .symbol_size(16.0);

        assert_eq!(legend.position, LegendPosition::BottomLeft);
        assert_eq!(legend.orientation, LegendOrientation::Horizontal);
        assert_eq!(legend.title, Some("My Legend".to_string()));
        assert_eq!(legend.symbol_size, 16.0);
    }

    #[test]
    fn test_legend_add_items() {
        let legend = LegendConfig::new()
            .add_item(LegendItem::color("A", D3Color::rgb(255, 0, 0)))
            .add_item(LegendItem::color("B", D3Color::rgb(0, 255, 0)))
            .add_item(LegendItem::color("C", D3Color::rgb(0, 0, 255)));

        assert_eq!(legend.items.len(), 3);
    }

    #[test]
    fn test_legend_dimensions() {
        let legend = LegendConfig::new().items(vec![
            LegendItem::color("Short", D3Color::rgb(255, 0, 0)),
            LegendItem::color("Longer label", D3Color::rgb(0, 255, 0)),
        ]);

        let (width, height) = legend.estimate_dimensions(7.0);
        assert!(width > 0.0);
        assert!(height > 0.0);
    }

    #[test]
    fn test_legend_offset() {
        let legend = LegendConfig::new().position(LegendPosition::TopRight);
        let (x, y) = legend.offset_from_corner(800.0, 600.0, 100.0, 50.0, 10.0);
        assert_eq!(x, 690.0); // 800 - 100 - 10
        assert_eq!(y, 10.0);
    }
}
