use super::contour_ring::ContourRing;

/// A contour band representing the filled region between two threshold values.
#[derive(Debug, Clone)]
pub struct ContourBand {
    /// The lower threshold value
    pub lower: f64,
    /// The upper threshold value
    pub upper: f64,
    /// The polygons forming this band (outer rings and holes)
    pub polygons: Vec<ContourRing>,
}

impl ContourBand {
    /// Create a new contour band.
    pub fn new(lower: f64, upper: f64) -> Self {
        Self {
            lower,
            upper,
            polygons: Vec::new(),
        }
    }

    /// Get the midpoint value of this band (for color interpolation).
    pub fn mid_value(&self) -> f64 {
        (self.lower + self.upper) / 2.0
    }
}
