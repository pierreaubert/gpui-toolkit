use d3rs::brush::BrushSelection;

/// Configuration for the secondary (right) Y-axis
pub struct SecondaryAxisConfig {
    /// Domain for the secondary axis (min, max)
    pub domain: (f64, f64),
    /// Title for the secondary axis
    pub title: &'static str,
    /// Tick values (only values in this list will show labels)
    pub tick_values: Vec<f64>,
}

/// Optional brush selection overlay configuration
pub struct BrushOverlay {
    /// Selection rectangle in pixels (x0, y0, x1, y1)
    pub selection: BrushSelection,
}
