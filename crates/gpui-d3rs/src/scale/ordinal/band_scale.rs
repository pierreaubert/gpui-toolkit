use std::collections::HashMap;
use std::hash::Hash;

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
    pub(super) domain: Vec<D>,
    pub(super) range_start: f64,
    pub(super) range_end: f64,
    pub(super) padding_inner: f64,
    pub(super) padding_outer: f64,
    pub(super) align: f64,
    pub(super) round: bool,
    pub(super) index_map: HashMap<D, usize>,
    // Computed values
    pub(super) bandwidth: f64,
    pub(super) step: f64,
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

    pub(super) fn rescale(&mut self) {
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
