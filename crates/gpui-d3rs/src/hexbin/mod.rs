//! Hexagonal binning
//!
//! Provides functions for binning two-dimensional points into hexagonal bins.

use std::collections::HashMap;

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
            x: Box::new(|d| d.as_ref()[0]),
            y: Box::new(|d| d.as_ref()[1]),
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
}
