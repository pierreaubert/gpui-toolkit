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
    pub(super) domain: Vec<D>,
    pub(super) range: Vec<R>,
    pub(super) unknown: Option<R>,
    pub(super) index_map: HashMap<D, usize>,
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
