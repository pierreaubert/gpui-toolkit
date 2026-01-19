//! Binning/histogram functions
//!
//! Provides functions for binning continuous data into discrete intervals.

/// A bin containing values within a range.
#[derive(Debug, Clone, PartialEq)]
pub struct Bin<T> {
    /// Lower bound of the bin (inclusive).
    pub x0: f64,
    /// Upper bound of the bin (exclusive, except for last bin).
    pub x1: f64,
    /// Values that fall within this bin.
    pub values: Vec<T>,
}

impl<T> Bin<T> {
    /// Returns the number of values in this bin.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if this bin is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Configuration for generating bins/histograms.
pub struct BinGenerator<T> {
    value: Option<Box<dyn Fn(&T) -> f64 + Send + Sync>>,
    domain: Option<(f64, f64)>,
    thresholds: ThresholdStrategy,
}

impl<T> std::fmt::Debug for BinGenerator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinGenerator")
            .field("value", &self.value.is_some())
            .field("domain", &self.domain)
            .field("thresholds", &self.thresholds)
            .finish()
    }
}

/// Strategy for determining bin thresholds.
#[derive(Debug, Clone, Default)]
pub enum ThresholdStrategy {
    /// Use a fixed number of bins.
    Count(usize),
    /// Use explicit threshold values.
    Values(Vec<f64>),
    /// Use Sturges' formula (default).
    #[default]
    Sturges,
    /// Use Freedman-Diaconis rule.
    FreedmanDiaconis,
    /// Use Scott's normal reference rule.
    Scott,
}

impl<T: Clone> Default for BinGenerator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> BinGenerator<T> {
    /// Creates a new bin generator.
    pub fn new() -> Self {
        Self {
            value: None,
            domain: None,
            thresholds: ThresholdStrategy::Sturges,
        }
    }

    /// Sets the value accessor function.
    pub fn value<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + Send + Sync + 'static,
    {
        self.value = Some(Box::new(f));
        self
    }

    /// Sets the domain (min, max) for binning.
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.domain = Some((min, max));
        self
    }

    /// Sets the number of bins.
    pub fn thresholds_count(mut self, count: usize) -> Self {
        self.thresholds = ThresholdStrategy::Count(count);
        self
    }

    /// Sets explicit threshold values.
    pub fn thresholds(mut self, values: Vec<f64>) -> Self {
        self.thresholds = ThresholdStrategy::Values(values);
        self
    }

    /// Uses Sturges' formula for threshold count.
    pub fn thresholds_sturges(mut self) -> Self {
        self.thresholds = ThresholdStrategy::Sturges;
        self
    }

    /// Uses Freedman-Diaconis rule for threshold count.
    pub fn thresholds_freedman_diaconis(mut self) -> Self {
        self.thresholds = ThresholdStrategy::FreedmanDiaconis;
        self
    }

    /// Uses Scott's normal reference rule for threshold count.
    pub fn thresholds_scott(mut self) -> Self {
        self.thresholds = ThresholdStrategy::Scott;
        self
    }

    /// Generates bins from the input data.
    pub fn generate(&self, data: &[T]) -> Vec<Bin<T>> {
        if data.is_empty() {
            return vec![];
        }

        // Extract values using accessor or default identity for f64
        let values: Vec<f64> = data
            .iter()
            .map(|x| {
                if let Some(ref accessor) = self.value {
                    accessor(x)
                } else {
                    // This will fail for non-f64 types without accessor
                    // but that's expected behavior
                    0.0
                }
            })
            .collect();

        // Determine domain
        let (min, max) = self.domain.unwrap_or_else(|| {
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        });

        if min == max {
            // All values are the same, return single bin
            return vec![Bin {
                x0: min,
                x1: max,
                values: data.to_vec(),
            }];
        }

        // Determine thresholds
        let thresholds = self.compute_thresholds(&values, min, max);

        // Create bins
        let mut bins: Vec<Bin<T>> = Vec::with_capacity(thresholds.len() - 1);
        for i in 0..thresholds.len() - 1 {
            bins.push(Bin {
                x0: thresholds[i],
                x1: thresholds[i + 1],
                values: Vec::new(),
            });
        }

        // Assign values to bins
        for (value, item) in values.iter().zip(data.iter()) {
            let idx = self.find_bin_index(&thresholds, *value);
            if idx < bins.len() {
                bins[idx].values.push(item.clone());
            }
        }

        bins
    }

    fn compute_thresholds(&self, values: &[f64], min: f64, max: f64) -> Vec<f64> {
        let n = values.len();
        let range = max - min;

        let count = match &self.thresholds {
            ThresholdStrategy::Count(c) => *c,
            ThresholdStrategy::Values(v) => return v.clone(),
            ThresholdStrategy::Sturges => {
                // Sturges' formula: ceil(log2(n) + 1)
                ((n as f64).log2() + 1.0).ceil() as usize
            }
            ThresholdStrategy::FreedmanDiaconis => {
                // Need IQR for Freedman-Diaconis
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let q1 = sorted[n / 4];
                let q3 = sorted[3 * n / 4];
                let iqr = q3 - q1;
                if iqr > 0.0 {
                    let bin_width = 2.0 * iqr / (n as f64).powf(1.0 / 3.0);
                    (range / bin_width).ceil() as usize
                } else {
                    ((n as f64).log2() + 1.0).ceil() as usize
                }
            }
            ThresholdStrategy::Scott => {
                // Scott's rule: 3.5 * std / n^(1/3)
                let mean: f64 = values.iter().sum::<f64>() / n as f64;
                let variance: f64 =
                    values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
                let std = variance.sqrt();
                if std > 0.0 {
                    let bin_width = 3.5 * std / (n as f64).powf(1.0 / 3.0);
                    (range / bin_width).ceil() as usize
                } else {
                    ((n as f64).log2() + 1.0).ceil() as usize
                }
            }
        };

        // Generate evenly spaced thresholds
        let step = range / count as f64;
        (0..=count).map(|i| min + i as f64 * step).collect()
    }

    fn find_bin_index(&self, thresholds: &[f64], value: f64) -> usize {
        // Binary search for the appropriate bin
        let mut lo = 0;
        let mut hi = thresholds.len() - 1;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if thresholds[mid + 1] <= value {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        // Clamp to valid bin range
        lo.min(thresholds.len().saturating_sub(2))
    }
}

/// Convenience function to create bins from f64 data with default settings.
///
/// # Example
///
/// ```
/// use d3rs::array::bin;
///
/// let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 6.0, 7.0, 8.0, 9.0];
/// let bins = bin(&data, 4);
///
/// assert_eq!(bins.len(), 4);
/// assert!(bins[0].x0 < bins[0].x1);
/// ```
pub fn bin(data: &[f64], count: usize) -> Vec<Bin<f64>> {
    BinGenerator::new()
        .value(|x: &f64| *x)
        .thresholds_count(count)
        .generate(data)
}

/// Computes the recommended number of bins using Sturges' formula.
pub fn threshold_sturges(n: usize) -> usize {
    ((n as f64).log2() + 1.0).ceil() as usize
}

/// Computes nice bin edges that span the given extent.
///
/// # Example
///
/// ```
/// use d3rs::array::nice_bin_edges;
///
/// let edges = nice_bin_edges(0.3, 9.7, 5);
/// assert!(edges[0] <= 0.3);
/// assert!(*edges.last().unwrap() >= 9.7);
/// ```
pub fn nice_bin_edges(min: f64, max: f64, count: usize) -> Vec<f64> {
    if min == max || count == 0 {
        return vec![min, max];
    }

    let range = max - min;
    let rough_step = range / count as f64;

    // Find a nice step size
    let exp = rough_step.log10().floor();
    let fraction = rough_step / 10_f64.powf(exp);
    let nice_fraction = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = nice_fraction * 10_f64.powf(exp);

    // Compute nice bounds
    let nice_min = (min / step).floor() * step;
    let nice_max = (max / step).ceil() * step;

    // Generate edges
    let mut edges = Vec::new();
    let mut edge = nice_min;
    while edge <= nice_max + step * 0.5 {
        edges.push(edge);
        edge += step;
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_basic() {
        let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 6.0, 7.0, 8.0, 9.0];
        let bins = bin(&data, 4);

        assert_eq!(bins.len(), 4);
        assert!(bins[0].x0 < bins[0].x1);

        // All values should be assigned
        let total: usize = bins.iter().map(|b| b.values.len()).sum();
        assert_eq!(total, data.len());
    }

    #[test]
    fn test_bin_generator() {
        #[derive(Clone)]
        struct Point {
            x: f64,
        }

        let data: Vec<Point> = (0..100).map(|i| Point { x: i as f64 }).collect();

        let bins = BinGenerator::new()
            .value(|p: &Point| p.x)
            .thresholds_count(10)
            .generate(&data);

        assert_eq!(bins.len(), 10);
    }

    #[test]
    fn test_nice_bin_edges() {
        let edges = nice_bin_edges(0.3, 9.7, 5);
        assert!(edges[0] <= 0.3);
        assert!(*edges.last().unwrap() >= 9.7);
    }

    #[test]
    fn test_threshold_sturges() {
        assert_eq!(threshold_sturges(10), 5);
        assert_eq!(threshold_sturges(100), 8);
        assert_eq!(threshold_sturges(1000), 11);
    }
}
