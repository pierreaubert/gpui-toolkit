//! Ordinal and band scales for categorical data
//!
//! Ordinal scales map discrete domain values to discrete range values.
//! Band scales are a variant that divide a continuous range into uniform bands.

use std::collections::HashMap;
use std::hash::Hash;

/// Ordinal scale that maps discrete domain values to discrete range values.
///
/// Unlike linear scales, ordinal scales have a discrete domain and range.
/// Each unique input value maps to a specific output value.
///
/// # Example
///
/// ```
/// use d3rs::scale::OrdinalScale;
///
/// let scale = OrdinalScale::new()
///     .domain(vec!["a", "b", "c"])
///     .range(vec![0.0, 50.0, 100.0]);
///
/// assert_eq!(scale.scale(&"a"), Some(0.0));
/// assert_eq!(scale.scale(&"b"), Some(50.0));
/// assert_eq!(scale.scale(&"c"), Some(100.0));
/// ```
#[derive(Debug, Clone)]
pub struct OrdinalScale<D, R>
where
    D: Eq + Hash + Clone,
    R: Clone,
{
    domain: Vec<D>,
    range: Vec<R>,
    unknown: Option<R>,
    index_map: HashMap<D, usize>,
}

impl<D, R> Default for OrdinalScale<D, R>
where
    D: Eq + Hash + Clone,
    R: Clone,
{
    fn default() -> Self {
        Self {
            domain: Vec::new(),
            range: Vec::new(),
            unknown: None,
            index_map: HashMap::new(),
        }
    }
}

impl<D, R> OrdinalScale<D, R>
where
    D: Eq + Hash + Clone,
    R: Clone,
{
    /// Create a new ordinal scale.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the domain (input values).
    pub fn domain(mut self, domain: Vec<D>) -> Self {
        self.index_map.clear();
        for (i, d) in domain.iter().enumerate() {
            self.index_map.insert(d.clone(), i);
        }
        self.domain = domain;
        self
    }

    /// Set the range (output values).
    pub fn range(mut self, range: Vec<R>) -> Self {
        self.range = range;
        self
    }

    /// Set the value to return for unknown domain values.
    pub fn unknown(mut self, unknown: R) -> Self {
        self.unknown = Some(unknown);
        self
    }

    /// Map a domain value to its corresponding range value.
    pub fn scale(&self, value: &D) -> Option<R> {
        if let Some(&index) = self.index_map.get(value) {
            if index < self.range.len() {
                return Some(self.range[index].clone());
            }
            // Cycle through range if domain is larger
            if !self.range.is_empty() {
                return Some(self.range[index % self.range.len()].clone());
            }
        }
        self.unknown.clone()
    }

    /// Get the domain values.
    pub fn get_domain(&self) -> &[D] {
        &self.domain
    }

    /// Get the range values.
    pub fn get_range(&self) -> &[R] {
        &self.range
    }
}

/// Band scale for positioning categorical data in bands.
///
/// Band scales divide a continuous range into uniform bands,
/// one for each value in the domain. Useful for bar charts.
///
/// # Example
///
/// ```
/// use d3rs::scale::BandScale;
///
/// let scale = BandScale::new()
///     .domain(vec!["a", "b", "c", "d"])
///     .range(0.0, 400.0);
///
/// // Each band is 100 pixels wide (400/4)
/// assert_eq!(scale.scale(&"a"), Some(0.0));
/// assert_eq!(scale.scale(&"b"), Some(100.0));
/// assert_eq!(scale.bandwidth(), 100.0);
/// ```
#[derive(Debug, Clone)]
pub struct BandScale<D>
where
    D: Eq + Hash + Clone,
{
    domain: Vec<D>,
    range_start: f64,
    range_end: f64,
    padding_inner: f64,
    padding_outer: f64,
    align: f64,
    round: bool,
    index_map: HashMap<D, usize>,
    // Computed values
    bandwidth: f64,
    step: f64,
}

impl<D> Default for BandScale<D>
where
    D: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self {
            domain: Vec::new(),
            range_start: 0.0,
            range_end: 1.0,
            padding_inner: 0.0,
            padding_outer: 0.0,
            align: 0.5,
            round: false,
            index_map: HashMap::new(),
            bandwidth: 0.0,
            step: 0.0,
        }
    }
}

impl<D> BandScale<D>
where
    D: Eq + Hash + Clone,
{
    /// Create a new band scale.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the domain (categorical values).
    pub fn domain(mut self, domain: Vec<D>) -> Self {
        self.index_map.clear();
        for (i, d) in domain.iter().enumerate() {
            self.index_map.insert(d.clone(), i);
        }
        self.domain = domain;
        self.rescale();
        self
    }

    /// Set the range (output interval).
    pub fn range(mut self, start: f64, end: f64) -> Self {
        self.range_start = start;
        self.range_end = end;
        self.rescale();
        self
    }

    /// Set inner padding (between bands) as a proportion of the step.
    ///
    /// Value should be between 0 and 1. Default is 0 (no padding).
    pub fn padding_inner(mut self, padding: f64) -> Self {
        self.padding_inner = padding.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// Set outer padding (before first and after last band) as a proportion of the step.
    ///
    /// Value should be between 0 and 1. Default is 0 (no padding).
    pub fn padding_outer(mut self, padding: f64) -> Self {
        self.padding_outer = padding.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// Set both inner and outer padding to the same value.
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding_inner = padding.clamp(0.0, 1.0);
        self.padding_outer = padding.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// Set the alignment of bands within the range.
    ///
    /// Value between 0 and 1:
    /// - 0: bands start at range start
    /// - 0.5: bands centered in range (default)
    /// - 1: bands end at range end
    pub fn align(mut self, align: f64) -> Self {
        self.align = align.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// Enable or disable rounding of output values.
    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self.rescale();
        self
    }

    fn rescale(&mut self) {
        let n = self.domain.len();
        if n == 0 {
            self.bandwidth = 0.0;
            self.step = 0.0;
            return;
        }

        let reverse = self.range_end < self.range_start;
        let (start, end) = if reverse {
            (self.range_end, self.range_start)
        } else {
            (self.range_start, self.range_end)
        };

        // Calculate step and bandwidth
        // step = (range_end - range_start) / (n - padding_inner + 2 * padding_outer)
        // bandwidth = step * (1 - padding_inner)
        let n_f = n as f64;
        self.step = (end - start) / (n_f - self.padding_inner + 2.0 * self.padding_outer).max(1.0);
        self.bandwidth = self.step * (1.0 - self.padding_inner);

        if self.round {
            self.step = self.step.floor();
            self.bandwidth = self.bandwidth.floor();
        }
    }

    /// Map a domain value to the start position of its band.
    pub fn scale(&self, value: &D) -> Option<f64> {
        if let Some(&index) = self.index_map.get(value) {
            let pos = self.padding_outer * self.step + index as f64 * self.step;
            let result = self.range_start + pos;
            return Some(if self.round { result.round() } else { result });
        }
        None
    }

    /// Get the bandwidth (width of each band).
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }

    /// Get the step (distance between band starts).
    pub fn step(&self) -> f64 {
        self.step
    }

    /// Get the domain values.
    pub fn get_domain(&self) -> &[D] {
        &self.domain
    }

    /// Get the range as (start, end).
    pub fn get_range(&self) -> (f64, f64) {
        (self.range_start, self.range_end)
    }
}

/// Point scale - a band scale with zero bandwidth.
///
/// Useful for scatter plots with categorical axes.
///
/// # Example
///
/// ```
/// use d3rs::scale::PointScale;
///
/// let scale = PointScale::new()
///     .domain(vec!["a", "b", "c"])
///     .range(0.0, 100.0);
///
/// assert_eq!(scale.scale(&"a"), Some(0.0));
/// assert_eq!(scale.scale(&"b"), Some(50.0));
/// assert_eq!(scale.scale(&"c"), Some(100.0));
/// ```
#[derive(Debug, Clone)]
pub struct PointScale<D>
where
    D: Eq + Hash + Clone,
{
    domain: Vec<D>,
    range_start: f64,
    range_end: f64,
    padding: f64,
    align: f64,
    round: bool,
    index_map: HashMap<D, usize>,
    step: f64,
}

impl<D> Default for PointScale<D>
where
    D: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self {
            domain: Vec::new(),
            range_start: 0.0,
            range_end: 1.0,
            padding: 0.0,
            align: 0.5,
            round: false,
            index_map: HashMap::new(),
            step: 0.0,
        }
    }
}

impl<D> PointScale<D>
where
    D: Eq + Hash + Clone,
{
    /// Create a new point scale.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the domain (categorical values).
    pub fn domain(mut self, domain: Vec<D>) -> Self {
        self.index_map.clear();
        for (i, d) in domain.iter().enumerate() {
            self.index_map.insert(d.clone(), i);
        }
        self.domain = domain;
        self.rescale();
        self
    }

    /// Set the range (output interval).
    pub fn range(mut self, start: f64, end: f64) -> Self {
        self.range_start = start;
        self.range_end = end;
        self.rescale();
        self
    }

    /// Set padding as a proportion of the step.
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// Set the alignment.
    pub fn align(mut self, align: f64) -> Self {
        self.align = align.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// Enable or disable rounding.
    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self.rescale();
        self
    }

    fn rescale(&mut self) {
        let n = self.domain.len();
        if n == 0 {
            self.step = 0.0;
            return;
        }
        if n == 1 {
            self.step = 0.0;
            return;
        }

        let range_size = (self.range_end - self.range_start).abs();
        self.step = range_size / (n as f64 - 1.0 + 2.0 * self.padding).max(1.0);

        if self.round {
            self.step = self.step.floor();
        }
    }

    /// Map a domain value to its position.
    pub fn scale(&self, value: &D) -> Option<f64> {
        if let Some(&index) = self.index_map.get(value) {
            let n = self.domain.len();
            if n == 1 {
                // Single point: center in range
                let center = (self.range_start + self.range_end) / 2.0;
                return Some(if self.round { center.round() } else { center });
            }
            let pos = self.padding * self.step + index as f64 * self.step;
            let result = self.range_start + pos;
            return Some(if self.round { result.round() } else { result });
        }
        None
    }

    /// Get the step (distance between points).
    pub fn step(&self) -> f64 {
        self.step
    }

    /// Get the domain values.
    pub fn get_domain(&self) -> &[D] {
        &self.domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordinal_scale_basic() {
        let scale = OrdinalScale::new()
            .domain(vec!["a", "b", "c"])
            .range(vec![10.0, 20.0, 30.0]);

        assert_eq!(scale.scale(&"a"), Some(10.0));
        assert_eq!(scale.scale(&"b"), Some(20.0));
        assert_eq!(scale.scale(&"c"), Some(30.0));
        assert_eq!(scale.scale(&"d"), None);
    }

    #[test]
    fn test_ordinal_scale_unknown() {
        let scale = OrdinalScale::new()
            .domain(vec!["a", "b"])
            .range(vec![10.0, 20.0])
            .unknown(-1.0);

        assert_eq!(scale.scale(&"a"), Some(10.0));
        assert_eq!(scale.scale(&"unknown"), Some(-1.0));
    }

    #[test]
    fn test_ordinal_scale_cycling() {
        let scale = OrdinalScale::new()
            .domain(vec!["a", "b", "c", "d", "e"])
            .range(vec![1.0, 2.0, 3.0]); // Only 3 range values

        assert_eq!(scale.scale(&"a"), Some(1.0));
        assert_eq!(scale.scale(&"b"), Some(2.0));
        assert_eq!(scale.scale(&"c"), Some(3.0));
        assert_eq!(scale.scale(&"d"), Some(1.0)); // Cycles back
        assert_eq!(scale.scale(&"e"), Some(2.0));
    }

    #[test]
    fn test_band_scale_basic() {
        let scale = BandScale::new()
            .domain(vec!["a", "b", "c", "d"])
            .range(0.0, 400.0);

        assert_eq!(scale.scale(&"a"), Some(0.0));
        assert_eq!(scale.scale(&"b"), Some(100.0));
        assert_eq!(scale.scale(&"c"), Some(200.0));
        assert_eq!(scale.scale(&"d"), Some(300.0));
        assert_eq!(scale.bandwidth(), 100.0);
    }

    #[test]
    fn test_band_scale_padding() {
        let scale = BandScale::new()
            .domain(vec!["a", "b"])
            .range(0.0, 100.0)
            .padding_inner(0.2);

        // With 2 bands and 20% inner padding:
        // step = 100 / (2 - 0.2) = 55.56
        // bandwidth = 55.56 * 0.8 = 44.44
        assert!(scale.bandwidth() < 50.0); // Less than without padding
        assert!(scale.step() > scale.bandwidth()); // Step includes padding
    }

    #[test]
    fn test_band_scale_unknown() {
        let scale = BandScale::new().domain(vec!["a", "b"]).range(0.0, 100.0);

        assert_eq!(scale.scale(&"unknown"), None);
    }

    #[test]
    fn test_point_scale_basic() {
        let scale = PointScale::new()
            .domain(vec!["a", "b", "c"])
            .range(0.0, 100.0);

        assert_eq!(scale.scale(&"a"), Some(0.0));
        assert_eq!(scale.scale(&"b"), Some(50.0));
        assert_eq!(scale.scale(&"c"), Some(100.0));
    }

    #[test]
    fn test_point_scale_single() {
        let scale = PointScale::new().domain(vec!["only"]).range(0.0, 100.0);

        // Single point should be centered
        assert_eq!(scale.scale(&"only"), Some(50.0));
    }

    #[test]
    fn test_point_scale_padding() {
        let scale = PointScale::new()
            .domain(vec!["a", "b", "c"])
            .range(0.0, 100.0)
            .padding(0.5);

        // With padding, first and last points won't be at range boundaries
        let a = scale.scale(&"a").unwrap();
        let c = scale.scale(&"c").unwrap();
        assert!(a > 0.0);
        assert!(c < 100.0);
    }
}
