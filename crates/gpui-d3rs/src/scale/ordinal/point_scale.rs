use std::collections::HashMap;
use std::hash::Hash;

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
    pub(super) domain: Vec<D>,
    pub(super) range_start: f64,
    pub(super) range_end: f64,
    pub(super) padding: f64,
    pub(super) align: f64,
    pub(super) round: bool,
    pub(super) index_map: HashMap<D, usize>,
    pub(super) step: f64,
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

    pub(super) fn rescale(&mut self) {
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
