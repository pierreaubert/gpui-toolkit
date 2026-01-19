//! Categorical color schemes

use super::D3Color;

/// A color scheme provides categorical colors for data visualization
///
/// # Example
///
/// ```
/// use d3rs::color::ColorScheme;
///
/// let scheme = ColorScheme::category10();
/// let color0 = scheme.color(0); // Blue
/// let color1 = scheme.color(1); // Orange
/// ```
#[derive(Debug, Clone)]
pub struct ColorScheme {
    colors: Vec<D3Color>,
}

impl ColorScheme {
    /// Create a custom color scheme from a vector of colors
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::color::{ColorScheme, D3Color};
    ///
    /// let scheme = ColorScheme::new(vec![
    ///     D3Color::rgb(255, 0, 0),
    ///     D3Color::rgb(0, 255, 0),
    ///     D3Color::rgb(0, 0, 255),
    /// ]);
    /// ```
    pub fn new(colors: Vec<D3Color>) -> Self {
        Self { colors }
    }

    /// D3 Category10 color scheme
    ///
    /// A popular 10-color categorical scheme suitable for most visualizations.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::color::ColorScheme;
    ///
    /// let scheme = ColorScheme::category10();
    /// ```
    pub fn category10() -> Self {
        Self {
            colors: vec![
                D3Color::from_hex(0x1f77b4), // Blue
                D3Color::from_hex(0xff7f0e), // Orange
                D3Color::from_hex(0x2ca02c), // Green
                D3Color::from_hex(0xd62728), // Red
                D3Color::from_hex(0x9467bd), // Purple
                D3Color::from_hex(0x8c564b), // Brown
                D3Color::from_hex(0xe377c2), // Pink
                D3Color::from_hex(0x7f7f7f), // Gray
                D3Color::from_hex(0xbcbd22), // Yellow-green
                D3Color::from_hex(0x17becf), // Cyan
            ],
        }
    }

    /// Tableau10 color scheme
    ///
    /// A perceptually distinct 10-color scheme from Tableau.
    pub fn tableau10() -> Self {
        Self {
            colors: vec![
                D3Color::from_hex(0x4e79a7), // Blue
                D3Color::from_hex(0xf28e2b), // Orange
                D3Color::from_hex(0xe15759), // Red
                D3Color::from_hex(0x76b7b2), // Teal
                D3Color::from_hex(0x59a14f), // Green
                D3Color::from_hex(0xedc948), // Yellow
                D3Color::from_hex(0xb07aa1), // Purple
                D3Color::from_hex(0xff9da7), // Pink
                D3Color::from_hex(0x9c755f), // Brown
                D3Color::from_hex(0xbab0ac), // Gray
            ],
        }
    }

    /// Pastel color scheme
    ///
    /// Soft, muted colors suitable for backgrounds or less prominent data.
    pub fn pastel() -> Self {
        Self {
            colors: vec![
                D3Color::from_hex(0xfbb4ae), // Pastel red
                D3Color::from_hex(0xb3cde3), // Pastel blue
                D3Color::from_hex(0xccebc5), // Pastel green
                D3Color::from_hex(0xdecbe4), // Pastel purple
                D3Color::from_hex(0xfed9a6), // Pastel orange
                D3Color::from_hex(0xffffcc), // Pastel yellow
                D3Color::from_hex(0xe5d8bd), // Pastel brown
                D3Color::from_hex(0xfddaec), // Pastel pink
            ],
        }
    }

    /// Get color by index (cycles through the scheme)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::color::ColorScheme;
    ///
    /// let scheme = ColorScheme::category10();
    /// let color = scheme.color(0);  // First color
    /// let color_cycle = scheme.color(10); // Cycles back to first color
    /// ```
    pub fn color(&self, index: usize) -> D3Color {
        if self.colors.is_empty() {
            D3Color::rgb(0, 0, 0)
        } else {
            self.colors[index % self.colors.len()]
        }
    }

    /// Get the number of colors in the scheme
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Check if the scheme is empty
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Get all colors in the scheme
    pub fn colors(&self) -> &[D3Color] {
        &self.colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category10_count() {
        let scheme = ColorScheme::category10();
        assert_eq!(scheme.len(), 10);
    }

    #[test]
    fn test_color_cycling() {
        let scheme = ColorScheme::category10();

        let color0 = scheme.color(0);
        let color10 = scheme.color(10);
        let color20 = scheme.color(20);

        // Should cycle back to the same color
        assert_eq!(color0, color10);
        assert_eq!(color0, color20);
    }

    #[test]
    fn test_custom_scheme() {
        let colors = vec![
            D3Color::rgb(255, 0, 0),
            D3Color::rgb(0, 255, 0),
            D3Color::rgb(0, 0, 255),
        ];

        let scheme = ColorScheme::new(colors);
        assert_eq!(scheme.len(), 3);

        let red = scheme.color(0);
        assert_eq!(red.r, 1.0);
    }

    #[test]
    fn test_tableau10() {
        let scheme = ColorScheme::tableau10();
        assert_eq!(scheme.len(), 10);
    }

    #[test]
    fn test_pastel() {
        let scheme = ColorScheme::pastel();
        assert_eq!(scheme.len(), 8);
    }
}
