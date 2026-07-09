//! Hexagonal binning
//!
//! Provides functions for binning two-dimensional points into hexagonal bins.

use std::collections::HashMap;
use std::fmt;

/// A hexagonal bin containing points.
#[derive(Debug, Clone)]
pub struct HexbinBin<T> {
    /// X-coordinate of the hexagon center.
    pub x: f64,
    /// Y-coordinate of the hexagon center.
    pub y: f64,
    /// Points that fall within this bin.
    pub points: Vec<T>,
}

impl<T> HexbinBin<T> {
    /// Returns the number of points in this bin.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns true if this bin is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// Recoverable errors for checked hexbin generation.
#[derive(Debug, Clone, PartialEq)]
pub enum HexbinError {
    /// Hex radius must be finite before grid normalization.
    NonFiniteRadius { radius: f64 },
    /// Hex radius must be greater than zero.
    NonPositiveRadius { radius: f64 },
    /// Extent coordinates must be finite.
    NonFiniteExtentCoordinate {
        corner: &'static str,
        coordinate: &'static str,
        value: f64,
    },
    /// Extent minimums must not exceed maximums.
    ReversedExtent {
        axis: &'static str,
        min: f64,
        max: f64,
    },
    /// Checked point coordinates must be finite.
    NonFinitePointCoordinate {
        index: usize,
        coordinate: &'static str,
        value: f64,
    },
}

impl fmt::Display for HexbinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteRadius { radius } => {
                write!(f, "hexbin radius is not finite: {radius}")
            }
            Self::NonPositiveRadius { radius } => {
                write!(f, "hexbin radius must be greater than zero: {radius}")
            }
            Self::NonFiniteExtentCoordinate {
                corner,
                coordinate,
                value,
            } => write!(
                f,
                "hexbin extent {corner} coordinate {coordinate} is not finite: {value}"
            ),
            Self::ReversedExtent { axis, min, max } => {
                write!(f, "hexbin {axis} extent is reversed: {min} > {max}")
            }
            Self::NonFinitePointCoordinate {
                index,
                coordinate,
                value,
            } => write!(
                f,
                "hexbin point coordinate {coordinate} at index {index} is not finite: {value}"
            ),
        }
    }
}

impl std::error::Error for HexbinError {}

/// Configuration for hexagonal binning.
pub struct Hexbin<T> {
    x: Box<dyn Fn(&T) -> f64 + Send + Sync>,
    y: Box<dyn Fn(&T) -> f64 + Send + Sync>,
    radius: f64,
    extent: [[f64; 2]; 2],
}

impl<T> Default for Hexbin<T>
where
    T: AsRef<[f64]>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Hexbin<T> {
    /// Creates a new hexbin generator with default settings.
    ///
    /// The default x-accessor is `d[0]` and the default y-accessor is `d[1]`.
    /// This constructor is only available if `T` implements `AsRef<[f64]>`.
    pub fn new() -> Self
    where
        T: AsRef<[f64]>,
    {
        Self {
            x: Box::new(|d| d.as_ref().first().copied().unwrap_or(f64::NAN)),
            y: Box::new(|d| d.as_ref().get(1).copied().unwrap_or(f64::NAN)),
            radius: 1.0,
            extent: [[0.0, 0.0], [1.0, 1.0]],
        }
    }

    /// Creates a new hexbin generator with explicit accessor functions.
    pub fn with_accessors<FX, FY>(x: FX, y: FY) -> Self
    where
        FX: Fn(&T) -> f64 + Send + Sync + 'static,
        FY: Fn(&T) -> f64 + Send + Sync + 'static,
    {
        Self {
            x: Box::new(x),
            y: Box::new(y),
            radius: 1.0,
            extent: [[0.0, 0.0], [1.0, 1.0]],
        }
    }

    /// Sets the x-accessor function.
    pub fn x<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + Send + Sync + 'static,
    {
        self.x = Box::new(f);
        self
    }

    /// Sets the y-accessor function.
    pub fn y<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + Send + Sync + 'static,
    {
        self.y = Box::new(f);
        self
    }

    /// Sets the radius of the hexagons.
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    /// Sets the extent (bounds) of the binning.
    pub fn extent(mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        self.extent = [[x0, y0], [x1, y1]];
        self
    }

    /// Bins the provided data.
    ///
    /// Algorithm matches d3-hexbin exactly: for each point, find the nearest
    /// hex center using normalized coordinates and a distance-based correction
    /// for points near hex boundaries.
    pub fn bin(&self, data: Vec<T>) -> Vec<HexbinBin<T>> {
        self.bin_unchecked(data)
    }

    /// Bins the provided data after validating configuration and point coordinates.
    ///
    /// Unlike [`Self::bin`], this checked path rejects invalid radius/extent
    /// configuration and non-finite accessor outputs instead of skipping NaN
    /// coordinates or allowing infinity through grid math.
    pub fn try_bin(&self, data: Vec<T>) -> Result<Vec<HexbinBin<T>>, HexbinError> {
        self.validate_config()?;

        for (index, d) in data.iter().enumerate() {
            let px = (self.x)(d);
            let py = (self.y)(d);

            if !px.is_finite() {
                return Err(HexbinError::NonFinitePointCoordinate {
                    index,
                    coordinate: "x",
                    value: px,
                });
            }
            if !py.is_finite() {
                return Err(HexbinError::NonFinitePointCoordinate {
                    index,
                    coordinate: "y",
                    value: py,
                });
            }
        }

        Ok(self.bin_unchecked(data))
    }

    /// Generate an SVG path for a hexagon with the configured radius.
    ///
    /// This mirrors D3's `hexbin.hexagon()` helper and keeps the permissive
    /// behavior of the rest of the builder-style API.
    pub fn hexagon(&self) -> String {
        self.hexagon_with_radius(self.radius)
    }

    /// Generate an SVG path for a hexagon with an explicit radius.
    pub fn hexagon_with_radius(&self, radius: f64) -> String {
        hexagon_path_unchecked(radius)
    }

    /// Generate a checked SVG path for a hexagon with the configured radius.
    pub fn try_hexagon(&self) -> Result<String, HexbinError> {
        validate_radius(self.radius)?;
        Ok(self.hexagon())
    }

    /// Generate a checked SVG path for a hexagon with an explicit radius.
    pub fn try_hexagon_with_radius(&self, radius: f64) -> Result<String, HexbinError> {
        validate_radius(radius)?;
        Ok(self.hexagon_with_radius(radius))
    }

    /// Return all hexagon centers that cover the configured extent.
    ///
    /// This mirrors D3's `hexbin.centers()` helper.
    pub fn centers(&self) -> Vec<(f64, f64)> {
        self.centers_unchecked()
    }

    /// Return configured hexagon centers after validating radius and extent.
    pub fn try_centers(&self) -> Result<Vec<(f64, f64)>, HexbinError> {
        self.validate_config()?;
        Ok(self.centers_unchecked())
    }

    fn validate_config(&self) -> Result<(), HexbinError> {
        validate_radius(self.radius)?;

        let [[x0, y0], [x1, y1]] = self.extent;
        validate_extent_coordinate("min", "x", x0)?;
        validate_extent_coordinate("min", "y", y0)?;
        validate_extent_coordinate("max", "x", x1)?;
        validate_extent_coordinate("max", "y", y1)?;

        if x0 > x1 {
            return Err(HexbinError::ReversedExtent {
                axis: "x",
                min: x0,
                max: x1,
            });
        }
        if y0 > y1 {
            return Err(HexbinError::ReversedExtent {
                axis: "y",
                min: y0,
                max: y1,
            });
        }

        Ok(())
    }

    fn bin_unchecked(&self, data: Vec<T>) -> Vec<HexbinBin<T>> {
        let dx = self.radius * 3.0f64.sqrt();
        let dy = self.radius * 1.5;
        let mut bins: HashMap<(i64, i64), HexbinBin<T>> = HashMap::new();

        for d in data {
            let px = (self.x)(&d);
            let py = (self.y)(&d);

            if px.is_nan() || py.is_nan() {
                continue;
            }

            // Normalize coordinates to hex grid
            let py1 = py / dy;
            let mut pj0 = py1.round();
            let px1 = px / dx - if (pj0 as i64) & 1 == 1 { 0.5 } else { 0.0 };
            let mut pi0 = px1.round();
            let py2 = py1 - pj0;

            // Correction for points near hex boundaries:
            // compare distance to current center vs adjacent center
            if py2.abs() * 3.0 > 1.0 {
                let px2 = px1 - pi0;
                let pi1 = pi0 + if px2 > 0.0 { 0.5 } else { -0.5 };
                let pj1 = pj0 + if py2 > 0.0 { 1.0 } else { -1.0 };
                let px1n = px1 - pi1;
                let py1n = py1 - pj1;
                if px2 * px2 + py2 * py2 > px1n * px1n + py1n * py1n {
                    pi0 = pi1 + if (pj0 as i64) & 1 == 1 { 0.5 } else { -0.5 };
                    pj0 = pj1;
                }
            }

            let key = (pi0 as i64, pj0 as i64);
            let odd = (pj0 as i64) & 1 == 1;
            if let Some(bin) = bins.get_mut(&key) {
                bin.points.push(d);
            } else {
                bins.insert(
                    key,
                    HexbinBin {
                        x: (pi0 + if odd { 0.5 } else { 0.0 }) * dx,
                        y: pj0 * dy,
                        points: vec![d],
                    },
                );
            }
        }

        bins.into_values().collect()
    }

    fn centers_unchecked(&self) -> Vec<(f64, f64)> {
        let [[x0, y0], [x1, y1]] = self.extent;
        let dx = self.radius * 3.0f64.sqrt();
        let dy = self.radius * 1.5;
        let mut centers = Vec::new();

        let mut j = (y0 / dy).round() as i64;
        let i0 = (x0 / dx).round() as i64;
        let mut y = j as f64 * dy;
        while y < y1 + self.radius {
            let mut x = i0 as f64 * dx + if j & 1 == 1 { dx / 2.0 } else { 0.0 };
            while x < x1 + dx / 2.0 {
                centers.push((x, y));
                x += dx;
            }
            j += 1;
            y += dy;
        }

        centers
    }
}

fn validate_radius(radius: f64) -> Result<(), HexbinError> {
    if !radius.is_finite() {
        return Err(HexbinError::NonFiniteRadius { radius });
    }
    if radius <= 0.0 {
        return Err(HexbinError::NonPositiveRadius { radius });
    }
    Ok(())
}

fn hexagon_path_unchecked(radius: f64) -> String {
    let dx = radius * 3.0f64.sqrt() / 2.0;
    format!(
        "m0,{}l{},{}l0,{}l{},{}l{},{}l0,{}z",
        -radius,
        dx,
        radius / 2.0,
        radius,
        -dx,
        radius / 2.0,
        -dx,
        -radius / 2.0,
        -radius
    )
}

fn validate_extent_coordinate(
    corner: &'static str,
    coordinate: &'static str,
    value: f64,
) -> Result<(), HexbinError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(HexbinError::NonFiniteExtentCoordinate {
            corner,
            coordinate,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexbin_basic() {
        let hexbin = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1).radius(1.0);
        let data = vec![(0.0, 0.0), (0.1, 0.1), (2.0, 2.0)];
        let bins = hexbin.bin(data);

        assert!(!bins.is_empty());
        let total: usize = bins.iter().map(|b| b.len()).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_hexbin_ignores_nan() {
        let hexbin = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1).radius(1.0);
        let data = vec![(0.0, 0.0), (f64::NAN, 1.0), (1.0, f64::NAN)];
        let bins = hexbin.bin(data);

        let total: usize = bins.iter().map(|b| b.len()).sum();
        assert_eq!(total, 1);
    }

    #[test]
    fn checked_hexbin_matches_permissive_for_valid_data() {
        let hexbin = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1)
            .radius(1.0)
            .extent(0.0, 0.0, 10.0, 10.0);
        let data = vec![(0.0, 0.0), (0.1, 0.1), (2.0, 2.0), (4.0, 1.0)];

        let permissive_total: usize = hexbin.bin(data.clone()).iter().map(|b| b.len()).sum();
        let checked = hexbin.try_bin(data).unwrap();
        let checked_total: usize = checked.iter().map(|b| b.len()).sum();

        assert_eq!(permissive_total, checked_total);
        assert_eq!(checked_total, 4);
    }

    #[test]
    fn checked_hexbin_rejects_invalid_radius() {
        let data = vec![(0.0, 0.0)];
        let zero_radius =
            Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1).radius(0.0);
        assert_eq!(
            zero_radius.try_bin(data.clone()).unwrap_err(),
            HexbinError::NonPositiveRadius { radius: 0.0 }
        );

        let nan_radius =
            Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1).radius(f64::NAN);
        assert!(matches!(
            nan_radius.try_bin(data).unwrap_err(),
            HexbinError::NonFiniteRadius { radius } if radius.is_nan()
        ));
    }

    #[test]
    fn checked_hexbin_rejects_invalid_extent() {
        let data = vec![(0.0, 0.0)];
        let reversed = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1)
            .radius(1.0)
            .extent(10.0, 0.0, 0.0, 10.0);

        assert_eq!(
            reversed.try_bin(data.clone()).unwrap_err(),
            HexbinError::ReversedExtent {
                axis: "x",
                min: 10.0,
                max: 0.0
            }
        );

        let non_finite = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1)
            .radius(1.0)
            .extent(0.0, f64::INFINITY, 10.0, 10.0);

        assert_eq!(
            non_finite.try_bin(data).unwrap_err(),
            HexbinError::NonFiniteExtentCoordinate {
                corner: "min",
                coordinate: "y",
                value: f64::INFINITY
            }
        );
    }

    #[test]
    fn checked_hexbin_rejects_non_finite_point_coordinates() {
        let hexbin = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1);
        let data = vec![(0.0, 0.0), (f64::INFINITY, 1.0)];

        assert_eq!(
            hexbin.try_bin(data).unwrap_err(),
            HexbinError::NonFinitePointCoordinate {
                index: 1,
                coordinate: "x",
                value: f64::INFINITY
            }
        );
    }

    #[test]
    fn hexagon_path_matches_d3_hexbin_shape() {
        let hexbin = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1).radius(2.0);

        let path = hexbin.hexagon();
        let explicit = hexbin.hexagon_with_radius(2.0);

        assert_eq!(path, explicit);
        assert!(path.starts_with("m0,-2l1.7320508075688772,1"));
        assert!(path.ends_with("l0,-2z"));
    }

    #[test]
    fn checked_hexagon_rejects_invalid_radius() {
        let hexbin =
            Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1).radius(f64::NAN);

        assert!(matches!(
            hexbin.try_hexagon(),
            Err(HexbinError::NonFiniteRadius { radius }) if radius.is_nan()
        ));
        assert_eq!(
            hexbin.try_hexagon_with_radius(0.0).unwrap_err(),
            HexbinError::NonPositiveRadius { radius: 0.0 }
        );
    }

    #[test]
    fn centers_cover_configured_extent() {
        let hexbin = Hexbin::with_accessors(|p: &(f64, f64)| p.0, |p: &(f64, f64)| p.1)
            .radius(1.0)
            .extent(0.0, 0.0, 2.0, 2.0);

        let centers = hexbin.try_centers().unwrap();

        assert_eq!(
            centers,
            vec![
                (0.0, 0.0),
                (3.0f64.sqrt(), 0.0),
                (3.0f64.sqrt() / 2.0, 1.5),
                (3.0f64.sqrt() * 1.5, 1.5),
            ]
        );
        assert_eq!(hexbin.centers(), centers);
    }

    #[test]
    fn checked_centers_rejects_invalid_extent() {
        let hexbin = Hexbin::<(f64, f64)>::with_accessors(|p| p.0, |p| p.1)
            .radius(1.0)
            .extent(0.0, 10.0, 10.0, 0.0);

        assert_eq!(
            hexbin.try_centers().unwrap_err(),
            HexbinError::ReversedExtent {
                axis: "y",
                min: 10.0,
                max: 0.0
            }
        );
    }

    #[test]
    fn default_accessors_treat_missing_coordinates_as_invalid() {
        let hexbin = Hexbin::<Vec<f64>>::new();
        let permissive_total: usize = hexbin
            .bin(vec![vec![0.0, 0.0], vec![1.0]])
            .iter()
            .map(|b| b.len())
            .sum();
        assert_eq!(permissive_total, 1);

        assert!(matches!(
            hexbin.try_bin(vec![vec![0.0, 0.0], vec![1.0]]),
            Err(HexbinError::NonFinitePointCoordinate {
                index: 1,
                coordinate: "y",
                value,
            }) if value.is_nan()
        ));
    }
}
