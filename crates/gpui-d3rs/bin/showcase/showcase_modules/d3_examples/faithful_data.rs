//! Old Faithful Geyser Dataset
//!
//! This dataset contains waiting times (in minutes) between eruptions
//! of the Old Faithful geyser in Yellowstone National Park.
//!
//! # Data Source
//! - Original source: Härdle, W. (1991) "Smoothing Techniques with Implementation in S"
//! - R datasets package: https://stat.ethz.ch/R-manual/R-devel/library/datasets/html/faithful.html
//! - JSON version: https://gist.github.com/curran/4b59d1046d9e66f2787780ad51a1cd87
//!
//! # License
//! The underlying observational data is in the public domain (factual measurements
//! of a natural phenomenon at a U.S. National Park). The R package documentation
//! is licensed under GPL, but the data values themselves are not copyrightable.
//!
//! # References
//! - Azzalini, A. and Bowman, A. W. (1990). A look at some data on the Old Faithful geyser.
//!   Applied Statistics, 39, 357-365.
//! - Härdle, W. (1991). Smoothing Techniques with Implementation in S. New York: Springer.

/// Old Faithful waiting times between eruptions (in minutes)
/// 272 observations
pub const FAITHFUL_WAITING: &[f64] = &[
    79.0, 54.0, 74.0, 62.0, 85.0, 55.0, 88.0, 85.0, 51.0, 85.0, 54.0, 84.0, 78.0, 47.0, 83.0, 52.0,
    62.0, 84.0, 52.0, 79.0, 51.0, 47.0, 78.0, 69.0, 74.0, 83.0, 55.0, 76.0, 78.0, 79.0, 73.0, 77.0,
    66.0, 80.0, 74.0, 52.0, 48.0, 80.0, 59.0, 90.0, 80.0, 58.0, 84.0, 58.0, 73.0, 83.0, 64.0, 53.0,
    82.0, 59.0, 75.0, 90.0, 54.0, 80.0, 54.0, 83.0, 71.0, 64.0, 77.0, 81.0, 59.0, 84.0, 48.0, 82.0,
    60.0, 92.0, 78.0, 78.0, 65.0, 73.0, 82.0, 56.0, 79.0, 71.0, 62.0, 76.0, 60.0, 78.0, 76.0, 83.0,
    75.0, 82.0, 70.0, 65.0, 73.0, 88.0, 76.0, 80.0, 48.0, 86.0, 60.0, 90.0, 50.0, 78.0, 63.0, 72.0,
    84.0, 75.0, 51.0, 82.0, 62.0, 88.0, 49.0, 83.0, 81.0, 47.0, 84.0, 52.0, 86.0, 81.0, 75.0, 59.0,
    89.0, 79.0, 59.0, 81.0, 50.0, 85.0, 59.0, 87.0, 53.0, 69.0, 77.0, 56.0, 88.0, 81.0, 45.0, 82.0,
    55.0, 90.0, 45.0, 83.0, 56.0, 89.0, 46.0, 82.0, 51.0, 86.0, 53.0, 79.0, 81.0, 60.0, 82.0, 77.0,
    76.0, 59.0, 80.0, 49.0, 96.0, 53.0, 77.0, 77.0, 65.0, 81.0, 71.0, 70.0, 81.0, 93.0, 53.0, 89.0,
    45.0, 86.0, 58.0, 78.0, 66.0, 76.0, 63.0, 88.0, 52.0, 93.0, 49.0, 57.0, 77.0, 68.0, 81.0, 81.0,
    73.0, 50.0, 85.0, 74.0, 55.0, 77.0, 83.0, 83.0, 51.0, 78.0, 84.0, 46.0, 83.0, 55.0, 81.0, 57.0,
    76.0, 84.0, 77.0, 81.0, 87.0, 77.0, 51.0, 78.0, 60.0, 82.0, 91.0, 53.0, 78.0, 46.0, 77.0, 84.0,
    49.0, 83.0, 71.0, 80.0, 49.0, 75.0, 64.0, 76.0, 53.0, 94.0, 55.0, 76.0, 50.0, 82.0, 54.0, 75.0,
    78.0, 79.0, 78.0, 78.0, 70.0, 79.0, 70.0, 54.0, 86.0, 50.0, 90.0, 54.0, 54.0, 77.0, 79.0, 64.0,
    75.0, 47.0, 86.0, 63.0, 85.0, 82.0, 57.0, 82.0, 67.0, 74.0, 54.0, 83.0, 73.0, 73.0, 88.0, 80.0,
    71.0, 83.0, 56.0, 79.0, 78.0, 84.0, 58.0, 83.0, 43.0, 60.0, 75.0, 81.0, 46.0, 90.0, 46.0, 74.0,
];

/// Get the data extent (min, max)
pub fn faithful_extent() -> (f64, f64) {
    let min = FAITHFUL_WAITING
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max = FAITHFUL_WAITING
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    (min, max)
}

/// Get basic statistics
pub fn faithful_stats() -> FaithfulStats {
    let n = FAITHFUL_WAITING.len();
    let sum: f64 = FAITHFUL_WAITING.iter().sum();
    let mean = sum / n as f64;

    let variance: f64 = FAITHFUL_WAITING
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / n as f64;
    let std_dev = variance.sqrt();

    let (min, max) = faithful_extent();

    FaithfulStats {
        count: n,
        min,
        max,
        mean,
        std_dev,
    }
}

/// Statistics for the faithful dataset
#[derive(Debug, Clone, Copy)]
pub struct FaithfulStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std_dev: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_count() {
        assert_eq!(FAITHFUL_WAITING.len(), 272);
    }

    #[test]
    fn test_extent() {
        let (min, max) = faithful_extent();
        assert_eq!(min, 43.0);
        assert_eq!(max, 96.0);
    }

    #[test]
    fn test_stats() {
        let stats = faithful_stats();
        assert_eq!(stats.count, 272);
        assert!(stats.mean > 70.0 && stats.mean < 75.0);
    }
}
