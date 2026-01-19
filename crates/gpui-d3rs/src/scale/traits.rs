//! Core scale trait definitions

/// Core trait for all scales that map from domain to range
///
/// Scales provide bidirectional mapping between data space (domain) and
/// visual space (range), along with utility functions for tick generation.
pub trait Scale<Domain, Range> {
    /// Map a domain value to a range value
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::{LinearScale, Scale};
    ///
    /// let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);
    /// assert_eq!(scale.scale(50.0), 250.0);
    /// ```
    fn scale(&self, value: Domain) -> Range;

    /// Inverse mapping from range to domain
    ///
    /// Returns `None` if the scale doesn't support inversion or if the
    /// value is outside the valid range.
    fn invert(&self, value: Range) -> Option<Domain>;

    /// Generate approximately `count` tick values for the domain
    ///
    /// The actual number of ticks may differ to ensure nice, round numbers.
    fn ticks(&self, count: usize) -> Vec<Domain>;

    /// Get the domain extent
    fn domain(&self) -> (Domain, Domain);

    /// Get the range extent
    fn range(&self) -> (Range, Range);
}
