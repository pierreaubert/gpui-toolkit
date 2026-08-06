//! Screen-bounded level-of-detail primitives for large 2D charts.
//!
//! The canonical input remains owned by the caller. These types only build
//! derived, disposable views: M4 keeps extrema for lines, while
//! [`DensityPyramid`] aggregates scatter points into a multiresolution grid.

use std::fmt;

/// Return source indices for a sorted line, retaining the first, minimum-y,
/// maximum-y, and last point in every screen column.
///
/// `x` must be sorted ascending. If it is not, this deliberately returns all
/// rows: preserving a possibly non-temporal line is preferable to silently
/// changing its shape. Non-finite y values are skipped.
pub fn m4_indices(x: &[f64], y: &[f64], x0: f64, x1: f64, columns: usize) -> Vec<usize> {
    if x.len() != y.len() || columns == 0 || !x0.is_finite() || !x1.is_finite() || x1 <= x0 {
        return Vec::new();
    }
    if x.windows(2).any(|pair| pair[0] > pair[1]) {
        return (0..x.len()).collect();
    }

    let start = x.partition_point(|value| *value < x0);
    let end = x.partition_point(|value| *value <= x1);
    let mut output = Vec::with_capacity((columns * 4).min(end.saturating_sub(start)));
    let bucket_width = (x1 - x0) / columns as f64;
    let mut bucket = None;
    let mut first = 0;
    let mut last = 0;
    let mut minimum = 0;
    let mut maximum = 0;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    let flush = |output: &mut Vec<usize>, first, minimum, maximum, last| {
        let mut indices = [first, minimum, maximum, last];
        indices.sort_unstable();
        let mut previous = None;
        for index in indices {
            if previous != Some(index) {
                output.push(index);
                previous = Some(index);
            }
        }
    };

    for index in start..end {
        let value = y[index];
        if !value.is_finite() {
            continue;
        }
        let next_bucket = (((x[index] - x0) / bucket_width) as usize).min(columns - 1);
        if bucket != Some(next_bucket) {
            if bucket.is_some() {
                flush(&mut output, first, minimum, maximum, last);
            }
            bucket = Some(next_bucket);
            first = index;
            minimum = index;
            maximum = index;
            min_y = value;
            max_y = value;
        } else {
            if value < min_y {
                min_y = value;
                minimum = index;
            }
            if value > max_y {
                max_y = value;
                maximum = index;
            }
        }
        last = index;
    }
    if bucket.is_some() {
        flush(&mut output, first, minimum, maximum, last);
    }
    output
}

/// M4 decimation for already transformed chart coordinates. Coordinates are
/// expected to be normalized to the horizontal 0..1 plot range.
pub fn m4_point_indices(points: &[(f32, f32)], columns: usize) -> Vec<usize> {
    if columns == 0 || points.iter().any(|&(x, _)| !x.is_finite()) {
        return Vec::new();
    }
    if points.windows(2).any(|pair| pair[0].0 > pair[1].0) {
        return (0..points.len()).collect();
    }
    let mut output = Vec::with_capacity((columns * 4).min(points.len()));
    let mut bucket = None;
    let mut first = 0;
    let mut last = 0;
    let mut minimum = 0;
    let mut maximum = 0;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let flush = |output: &mut Vec<usize>, first, minimum, maximum, last| {
        let mut indices = [first, minimum, maximum, last];
        indices.sort_unstable();
        let mut previous = None;
        for index in indices {
            if previous != Some(index) {
                output.push(index);
                previous = Some(index);
            }
        }
    };
    for (index, &(x, y)) in points.iter().enumerate() {
        if !y.is_finite() || !(0.0..=1.0).contains(&x) {
            continue;
        }
        let next_bucket = (x * columns as f32) as usize;
        let next_bucket = next_bucket.min(columns - 1);
        if bucket != Some(next_bucket) {
            if bucket.is_some() {
                flush(&mut output, first, minimum, maximum, last);
            }
            bucket = Some(next_bucket);
            first = index;
            minimum = index;
            maximum = index;
            min_y = y;
            max_y = y;
        } else {
            if y < min_y {
                min_y = y;
                minimum = index;
            }
            if y > max_y {
                max_y = y;
                maximum = index;
            }
        }
        last = index;
    }
    if bucket.is_some() {
        flush(&mut output, first, minimum, maximum, last);
    }
    output
}

/// A finite data-space rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodBounds {
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

impl LodBounds {
    pub fn new(x0: f64, x1: f64, y0: f64, y1: f64) -> Result<Self, LodError> {
        let bounds = Self { x0, x1, y0, y1 };
        if bounds.is_valid() {
            Ok(bounds)
        } else {
            Err(LodError::InvalidBounds)
        }
    }

    fn is_valid(self) -> bool {
        self.x0.is_finite()
            && self.x1.is_finite()
            && self.y0.is_finite()
            && self.y1.is_finite()
            && self.x1 > self.x0
            && self.y1 > self.y0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodError {
    InvalidBounds,
    InvalidBaseDimension,
    UnequalCoordinates,
}

impl fmt::Display for LodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => f.write_str("LOD bounds must be finite and non-empty"),
            Self::InvalidBaseDimension => {
                f.write_str("LOD base dimension must be a power of two >= 2")
            }
            Self::UnequalCoordinates => {
                f.write_str("x and y coordinate columns must have equal length")
            }
        }
    }
}

impl std::error::Error for LodError {}

/// A screen-ready density raster composed from a [`DensityPyramid`].
#[derive(Debug, Clone, PartialEq)]
pub struct DensityGrid {
    pub width: usize,
    pub height: usize,
    /// Bottom-to-top rows, each stored left-to-right.
    pub values: Vec<f32>,
    /// Zero is the finest pyramid level.
    pub level: usize,
}

/// A count pyramid over one trace's fixed data bounds.
///
/// Each coarser level is an exact 4→1 reduction of the previous level. It is
/// intentionally count-only; colormapping is a rendering concern, so changing
/// colour, opacity, or the transfer curve never invalidates the pyramid.
#[derive(Debug, Clone)]
pub struct DensityPyramid {
    bounds: LodBounds,
    dimensions: Vec<usize>,
    levels: Vec<Vec<u32>>,
}

impl DensityPyramid {
    pub fn build(
        x: &[f64],
        y: &[f64],
        bounds: LodBounds,
        base_dimension: usize,
    ) -> Result<Self, LodError> {
        if x.len() != y.len() {
            return Err(LodError::UnequalCoordinates);
        }
        if !base_dimension.is_power_of_two() || base_dimension < 2 {
            return Err(LodError::InvalidBaseDimension);
        }
        let mut base = vec![0_u32; base_dimension * base_dimension];
        for (&px, &py) in x.iter().zip(y) {
            if !(px.is_finite() && py.is_finite())
                || px < bounds.x0
                || px > bounds.x1
                || py < bounds.y0
                || py > bounds.y1
            {
                continue;
            }
            let ix = (((px - bounds.x0) / (bounds.x1 - bounds.x0) * base_dimension as f64)
                as usize)
                .min(base_dimension - 1);
            let iy = (((py - bounds.y0) / (bounds.y1 - bounds.y0) * base_dimension as f64)
                as usize)
                .min(base_dimension - 1);
            let cell = &mut base[iy * base_dimension + ix];
            *cell = cell.saturating_add(1);
        }

        let mut dimensions = vec![base_dimension];
        let mut levels = vec![base];
        while *dimensions.last().expect("base dimension exists") > 1 {
            let dimension = *dimensions.last().expect("dimension exists");
            let previous = levels.last().expect("base level exists");
            let next_dimension = dimension / 2;
            let mut next = vec![0_u32; next_dimension * next_dimension];
            for row in 0..next_dimension {
                for column in 0..next_dimension {
                    let source = (row * 2) * dimension + column * 2;
                    let sum = previous[source] as u64
                        + previous[source + 1] as u64
                        + previous[source + dimension] as u64
                        + previous[source + dimension + 1] as u64;
                    next[row * next_dimension + column] = sum.min(u32::MAX as u64) as u32;
                }
            }
            dimensions.push(next_dimension);
            levels.push(next);
        }
        Ok(Self {
            bounds,
            dimensions,
            levels,
        })
    }

    pub fn bounds(&self) -> LodBounds {
        self.bounds
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Compose a viewport-sized density grid from the coarsest suitable level.
    /// Returns `None` when the viewport needs more than `max_upsample` output
    /// pixels for one finest-level source cell; callers should then exact-bin
    /// only their visible source rows.
    pub fn compose(
        &self,
        view: LodBounds,
        width: usize,
        height: usize,
        max_upsample: usize,
    ) -> Option<DensityGrid> {
        if !view.is_valid() || width == 0 || height == 0 || max_upsample == 0 {
            return None;
        }
        let view = LodBounds {
            x0: view.x0.max(self.bounds.x0),
            x1: view.x1.min(self.bounds.x1),
            y0: view.y0.max(self.bounds.y0),
            y1: view.y1.min(self.bounds.y1),
        };
        if !view.is_valid() {
            return Some(DensityGrid {
                width,
                height,
                values: vec![0.0; width * height],
                level: 0,
            });
        }

        let level = self.dimensions.iter().rposition(|&dimension| {
            let cells_x =
                (view.x1 - view.x0) / (self.bounds.x1 - self.bounds.x0) * dimension as f64;
            let cells_y =
                (view.y1 - view.y0) / (self.bounds.y1 - self.bounds.y0) * dimension as f64;
            cells_x >= width as f64 / max_upsample as f64
                && cells_y >= height as f64 / max_upsample as f64
        })?;
        let dimension = self.dimensions[level];
        let mut values = vec![0.0_f32; width * height];
        let cell_w = (self.bounds.x1 - self.bounds.x0) / dimension as f64;
        let cell_h = (self.bounds.y1 - self.bounds.y0) / dimension as f64;
        let min_x = ((view.x0 - self.bounds.x0) / cell_w).floor().max(0.0) as usize;
        let max_x = ((view.x1 - self.bounds.x0) / cell_w)
            .ceil()
            .min(dimension as f64) as usize;
        let min_y = ((view.y0 - self.bounds.y0) / cell_h).floor().max(0.0) as usize;
        let max_y = ((view.y1 - self.bounds.y0) / cell_h)
            .ceil()
            .min(dimension as f64) as usize;
        let output_cell_w = (view.x1 - view.x0) / width as f64;
        let output_cell_h = (view.y1 - view.y0) / height as f64;

        for source_y in min_y..max_y {
            let y0 = (self.bounds.y0 + source_y as f64 * cell_h).max(view.y0);
            let y1 = (self.bounds.y0 + (source_y + 1) as f64 * cell_h).min(view.y1);
            for source_x in min_x..max_x {
                let count = self.levels[level][source_y * dimension + source_x];
                if count == 0 {
                    continue;
                }
                let x0 = (self.bounds.x0 + source_x as f64 * cell_w).max(view.x0);
                let x1 = (self.bounds.x0 + (source_x + 1) as f64 * cell_w).min(view.x1);
                let out_x0 = ((x0 - view.x0) / output_cell_w).floor().max(0.0) as usize;
                let out_x1 = ((x1 - view.x0) / output_cell_w).ceil().min(width as f64) as usize;
                let out_y0 = ((y0 - view.y0) / output_cell_h).floor().max(0.0) as usize;
                let out_y1 = ((y1 - view.y0) / output_cell_h).ceil().min(height as f64) as usize;
                for output_y in out_y0..out_y1 {
                    let oy0 = view.y0 + output_y as f64 * output_cell_h;
                    let oy1 = oy0 + output_cell_h;
                    let y_weight = ((y1.min(oy1) - y0.max(oy0)) / cell_h).max(0.0);
                    for output_x in out_x0..out_x1 {
                        let ox0 = view.x0 + output_x as f64 * output_cell_w;
                        let ox1 = ox0 + output_cell_w;
                        let x_weight = ((x1.min(ox1) - x0.max(ox0)) / cell_w).max(0.0);
                        values[output_y * width + output_x] +=
                            count as f32 * (x_weight * y_weight) as f32;
                    }
                }
            }
        }
        Some(DensityGrid {
            width,
            height,
            values,
            level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m4_keeps_a_spike_inside_a_screen_column() {
        let x = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
        let y = [0.0, 1.0, 99.0, 2.0, -5.0, 0.0];
        let indices = m4_indices(&x, &y, 0.0, 1.0, 2);
        assert!(indices.contains(&2));
        assert!(indices.contains(&4));
        assert_eq!(
            indices,
            indices
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn pyramid_levels_preserve_total_count() {
        let x = [0.1, 0.2, 0.9, 0.9, 1.0];
        let y = [0.1, 0.2, 0.9, 0.1, 1.0];
        let pyramid =
            DensityPyramid::build(&x, &y, LodBounds::new(0.0, 1.0, 0.0, 1.0).unwrap(), 8).unwrap();
        assert_eq!(
            pyramid.levels[0]
                .iter()
                .map(|&value| value as u64)
                .sum::<u64>(),
            5
        );
        assert_eq!(pyramid.levels.last().unwrap(), &vec![5]);
    }

    #[test]
    fn compose_conserves_a_full_aligned_view() {
        let x = [0.125, 0.375, 0.625, 0.875];
        let y = [0.125, 0.375, 0.625, 0.875];
        let bounds = LodBounds::new(0.0, 1.0, 0.0, 1.0).unwrap();
        let pyramid = DensityPyramid::build(&x, &y, bounds, 4).unwrap();
        let grid = pyramid.compose(bounds, 4, 4, 1).unwrap();
        assert_eq!(grid.level, 0);
        assert!((grid.values.iter().map(|&value| value as f64).sum::<f64>() - 4.0).abs() < 1e-6);
    }
}
