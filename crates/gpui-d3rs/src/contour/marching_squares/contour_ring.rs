use crate::shape::path::Point;

/// Recoverable errors for checked contour ring operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ContourRingError {
    /// Contour ring points must be finite for checked area computation.
    NonFinitePoint {
        index: usize,
        coordinate: &'static str,
        value: f64,
    },
}

impl std::fmt::Display for ContourRingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinitePoint {
                index,
                coordinate,
                value,
            } => write!(
                f,
                "contour ring point {coordinate} at index {index} is not finite: {value}"
            ),
        }
    }
}

impl std::error::Error for ContourRingError {}

/// A contour ring (polygon) representing a closed contour line.
#[derive(Debug, Clone, Default)]
pub struct ContourRing {
    /// The points forming this ring
    pub points: Vec<Point>,
}

impl ContourRing {
    /// Create a new contour ring.
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    /// Check if the ring is closed (first and last points are the same).
    pub fn is_closed(&self) -> bool {
        if self.points.len() < 2 {
            return false;
        }
        let first = &self.points[0];
        let last = &self.points[self.points.len() - 1];
        (first.x - last.x).abs() < 1e-10 && (first.y - last.y).abs() < 1e-10
    }

    /// Get the area of this ring (positive for counter-clockwise, negative for clockwise).
    pub fn area(&self) -> f64 {
        if self.points.len() < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..self.points.len() - 1 {
            let p0 = &self.points[i];
            let p1 = &self.points[i + 1];
            sum += (p1.x - p0.x) * (p1.y + p0.y);
        }
        sum / 2.0
    }

    /// Get the checked area of this ring.
    pub fn try_area(&self) -> Result<f64, ContourRingError> {
        self.validate_finite_points()?;
        Ok(self.area())
    }

    fn validate_finite_points(&self) -> Result<(), ContourRingError> {
        for (index, point) in self.points.iter().enumerate() {
            if !point.x.is_finite() {
                return Err(ContourRingError::NonFinitePoint {
                    index,
                    coordinate: "x",
                    value: point.x,
                });
            }
            if !point.y.is_finite() {
                return Err(ContourRingError::NonFinitePoint {
                    index,
                    coordinate: "y",
                    value: point.y,
                });
            }
        }
        Ok(())
    }
}
