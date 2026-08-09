//! Display ranges for scalar fields rendered with a [`ColorScale`](crate::ColorScale).

use crate::ChartError;

/// How a color range should be resolved from the data extent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorRange {
    /// Use the data extent, expanding a constant field to a useful display range.
    Auto,
    /// Use an explicitly validated minimum and maximum.
    Fixed { min: f64, max: f64 },
    /// Center the range around `center` with an automatic or explicit extent.
    Symmetric { center: f64, extent: AutoOrFixed },
}

/// Whether a symmetric color range should infer its extent from the data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AutoOrFixed {
    Auto,
    Fixed(f64),
}

impl ColorRange {
    /// Resolve this display range from the finite data extent.
    pub fn resolve(&self, data_min: f64, data_max: f64) -> Result<[f64; 2], ChartError> {
        if !data_min.is_finite() || !data_max.is_finite() || data_min > data_max {
            return Err(ChartError::InvalidColorRange {
                reason: "data extent must be finite and ordered",
            });
        }

        let range = match *self {
            Self::Auto => {
                if data_min == data_max {
                    let padding = 1.0_f64.max(data_min.abs() * 0.05);
                    [data_min - padding, data_max + padding]
                } else {
                    [data_min, data_max]
                }
            }
            Self::Fixed { min, max } => [min, max],
            Self::Symmetric { center, extent } => {
                if !center.is_finite() {
                    return Err(ChartError::InvalidColorRange {
                        reason: "symmetric center must be finite",
                    });
                }

                let extent = match extent {
                    AutoOrFixed::Auto => (data_min - center).abs().max((data_max - center).abs()),
                    AutoOrFixed::Fixed(extent) => extent,
                };
                [center - extent, center + extent]
            }
        };

        if range[0].is_finite() && range[1].is_finite() && range[0] < range[1] {
            Ok(range)
        } else {
            Err(ChartError::InvalidColorRange {
                reason: "range bounds must be finite and min must be less than max",
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_range_uses_data_extent() {
        assert_eq!(ColorRange::Auto.resolve(-2.0, 8.0).unwrap(), [-2.0, 8.0]);
    }

    #[test]
    fn symmetric_range_centers_zero() {
        let r = ColorRange::Symmetric {
            center: 0.0,
            extent: AutoOrFixed::Auto,
        }
        .resolve(-3.0, 8.0)
        .unwrap();
        assert_eq!(r, [-8.0, 8.0]);
    }

    #[test]
    fn fixed_range_validated() {
        assert!(
            ColorRange::Fixed { min: 5.0, max: 5.0 }
                .resolve(0.0, 1.0)
                .is_err()
        );
    }

    #[test]
    fn constant_field_yields_valid_constant_range() {
        // spec §5.2: one color, a valid constant range
        let r = ColorRange::Auto.resolve(3.0, 3.0).unwrap();
        assert!(
            r[0] < r[1],
            "constant data must expand to a valid display range"
        );
        assert!(r[0] <= 3.0 && r[1] >= 3.0);
    }
}
