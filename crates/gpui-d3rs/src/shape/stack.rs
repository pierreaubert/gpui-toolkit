//! Stack layout generator
//!
//! Computes stacked layouts for stacked bar charts and stacked area charts.

use std::fmt;

mod stack_series;
#[cfg(test)]
mod tests;
mod types;

pub use stack_series::*;
pub use types::*;

/// Recoverable errors for checked stack layout input validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackLayoutError {
    /// Checked stack data must have one value per configured key in every row.
    RowLengthMismatch {
        row_index: usize,
        expected: usize,
        actual: usize,
    },
    /// Checked stack values must be finite.
    NonFiniteValue {
        row_index: usize,
        series_index: usize,
    },
}

impl fmt::Display for StackLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowLengthMismatch {
                row_index,
                expected,
                actual,
            } => write!(
                f,
                "stack row {row_index} has {actual} values, expected {expected}"
            ),
            Self::NonFiniteValue {
                row_index,
                series_index,
            } => write!(
                f,
                "stack value at row {row_index}, series {series_index} is not finite"
            ),
        }
    }
}

impl std::error::Error for StackLayoutError {}

/// Stack layout generator.
///
/// # Example
///
/// ```
/// use d3rs::shape::stack::{Stack, StackOrder, StackOffset};
///
/// // Data: each row is a time point, columns are different series
/// let data = vec![
///     vec![1.0, 2.0, 3.0],  // time 0
///     vec![2.0, 3.0, 4.0],  // time 1
///     vec![3.0, 4.0, 5.0],  // time 2
/// ];
///
/// let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
/// let stack = Stack::new()
///     .keys(keys)
///     .order(StackOrder::None)
///     .offset(StackOffset::None);
///
/// let result = stack.generate(&data);
/// assert_eq!(result.len(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct Stack {
    /// Keys for each series
    keys: Vec<String>,
    /// Ordering strategy
    order: StackOrder,
    /// Offset strategy
    offset: StackOffset,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            order: StackOrder::None,
            offset: StackOffset::None,
        }
    }
}

impl Stack {
    /// Create a new stack generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the keys for the series.
    pub fn keys(mut self, keys: Vec<String>) -> Self {
        self.keys = keys;
        self
    }

    /// Set the order strategy.
    pub fn order(mut self, order: StackOrder) -> Self {
        self.order = order;
        self
    }

    /// Set the offset strategy.
    pub fn offset(mut self, offset: StackOffset) -> Self {
        self.offset = offset;
        self
    }

    /// Generate stacked series from data.
    ///
    /// Data is expected to be a 2D array where each row is a data point
    /// and each column corresponds to a key.
    pub fn generate(&self, data: &[Vec<f64>]) -> Vec<StackSeries> {
        if data.is_empty() || self.keys.is_empty() {
            return Vec::new();
        }

        self.generate_validated(data)
    }

    /// Generate stacked series from data after validating the table shape and values.
    ///
    /// Unlike [`Self::generate`], this checked path rejects ragged rows and
    /// non-finite values instead of filling missing cells with zero or allowing
    /// NaN/infinity to flow into ordering and offset math.
    pub fn try_generate(&self, data: &[Vec<f64>]) -> Result<Vec<StackSeries>, StackLayoutError> {
        if data.is_empty() || self.keys.is_empty() {
            return Ok(Vec::new());
        }

        self.validate_data(data)?;
        Ok(self.generate_validated(data))
    }

    fn generate_validated(&self, data: &[Vec<f64>]) -> Vec<StackSeries> {
        let n = data.len(); // Number of data points

        // Create initial series with raw values
        let mut series: Vec<StackSeries> = self
            .keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let series_data: Vec<f64> = data
                    .iter()
                    .map(|row| row.get(i).copied().unwrap_or(0.0))
                    .collect();
                StackSeries {
                    key: key.clone(),
                    data: series_data,
                    values: vec![[0.0, 0.0]; n],
                    index: i,
                }
            })
            .collect();

        // Reorder series based on order strategy
        let order = self.compute_order(&series, data);

        // Apply ordering
        for (new_index, &old_index) in order.iter().enumerate() {
            series[old_index].index = new_index;
        }
        series.sort_by_key(|s| s.index);

        // Compute stacked values
        // Use series.data which was populated before reordering with the correct column values
        for j in 0..n {
            let mut y0 = 0.0;
            for series in &mut series {
                let value = series.data.get(j).copied().unwrap_or(0.0);
                series.values[j] = [y0, y0 + value];
                y0 += value;
            }
        }

        // Apply offset
        self.apply_offset(&mut series, n);

        series
    }

    fn validate_data(&self, data: &[Vec<f64>]) -> Result<(), StackLayoutError> {
        let expected = self.keys.len();
        for (row_index, row) in data.iter().enumerate() {
            if row.len() != expected {
                return Err(StackLayoutError::RowLengthMismatch {
                    row_index,
                    expected,
                    actual: row.len(),
                });
            }

            for (series_index, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    return Err(StackLayoutError::NonFiniteValue {
                        row_index,
                        series_index,
                    });
                }
            }
        }

        Ok(())
    }

    /// Compute series order based on strategy.
    fn compute_order(&self, series: &[StackSeries], data: &[Vec<f64>]) -> Vec<usize> {
        let m = series.len();
        let mut order: Vec<usize> = (0..m).collect();

        match self.order {
            StackOrder::None => {}
            StackOrder::Ascending => {
                let sums: Vec<f64> = (0..m)
                    .map(|i| {
                        data.iter()
                            .map(|row| row.get(i).copied().unwrap_or(0.0))
                            .sum()
                    })
                    .collect();
                order.sort_by(|&a, &b| {
                    sums[a]
                        .partial_cmp(&sums[b])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            StackOrder::Descending => {
                let sums: Vec<f64> = (0..m)
                    .map(|i| {
                        data.iter()
                            .map(|row| row.get(i).copied().unwrap_or(0.0))
                            .sum()
                    })
                    .collect();
                order.sort_by(|&a, &b| {
                    sums[b]
                        .partial_cmp(&sums[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            StackOrder::Appearance => {
                // Find first non-zero appearance for each series
                let first_appearance: Vec<usize> = (0..m)
                    .map(|i| {
                        data.iter()
                            .position(|row| row.get(i).copied().unwrap_or(0.0) != 0.0)
                            .unwrap_or(usize::MAX)
                    })
                    .collect();
                order.sort_by_key(|&i| first_appearance[i]);
            }
            StackOrder::InsideOut => {
                let sums: Vec<f64> = (0..m)
                    .map(|i| {
                        data.iter()
                            .map(|row| row.get(i).copied().unwrap_or(0.0))
                            .sum()
                    })
                    .collect();
                order.sort_by(|&a, &b| {
                    sums[b]
                        .partial_cmp(&sums[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                // Interleave: place largest in middle, then alternate sides
                let mut result = Vec::with_capacity(m);
                let mut top = Vec::new();
                let mut bottom = Vec::new();
                let mut use_top = true;

                for &i in &order {
                    if use_top {
                        top.push(i);
                    } else {
                        bottom.push(i);
                    }
                    use_top = !use_top;
                }

                bottom.reverse();
                result.extend(bottom);
                result.extend(top);
                order = result;
            }
            StackOrder::Reverse => {
                order.reverse();
            }
        }

        order
    }

    /// Apply offset to stacked values.
    fn apply_offset(&self, series: &mut [StackSeries], n: usize) {
        match self.offset {
            StackOffset::None => {}
            StackOffset::Expand => {
                // Normalize each column to [0, 1]
                for j in 0..n {
                    let total: f64 = series.iter().map(|s| s.values[j][1] - s.values[j][0]).sum();
                    if total > 0.0 {
                        let mut y0 = 0.0;
                        for s in series.iter_mut() {
                            let value = (s.values[j][1] - s.values[j][0]) / total;
                            s.values[j] = [y0, y0 + value];
                            y0 += value;
                        }
                    }
                }
            }
            StackOffset::Diverging => {
                // Separate positive and negative values
                for j in 0..n {
                    let mut y_pos = 0.0;
                    let mut y_neg = 0.0;
                    for s in series.iter_mut() {
                        let value = s.values[j][1] - s.values[j][0];
                        if value >= 0.0 {
                            s.values[j] = [y_pos, y_pos + value];
                            y_pos += value;
                        } else {
                            s.values[j] = [y_neg + value, y_neg];
                            y_neg += value;
                        }
                    }
                }
            }
            StackOffset::Silhouette => {
                // Center around zero
                for j in 0..n {
                    let total: f64 = series.iter().map(|s| s.values[j][1] - s.values[j][0]).sum();
                    let offset = -total / 2.0;
                    for s in series.iter_mut() {
                        s.values[j][0] += offset;
                        s.values[j][1] += offset;
                    }
                }
            }
            StackOffset::Wiggle => {
                // Minimize weighted wiggle (streamgraph offset)
                // Matches D3.js stackOffsetWiggle exactly:
                // Track a running baseline offset y for series[0], then restack.
                if n == 0 || series.is_empty() {
                    return;
                }

                let num_series = series.len();
                let mut y = 0.0_f64;

                for j in 1..n {
                    let mut s1 = 0.0; // sum of (sij - si(j-1)) across all series
                    let mut s2 = 0.0; // weighted sum

                    for i in 0..num_series {
                        let sij0 = series[i].values[j][1] - series[i].values[j][0];
                        let sij1 = series[i].values[j - 1][1] - series[i].values[j - 1][0];
                        let mut s3 = (sij0 - sij1) / 2.0;

                        for sk in &series[..i] {
                            let skj0 = sk.values[j][1] - sk.values[j][0];
                            let skj1 = sk.values[j - 1][1] - sk.values[j - 1][0];
                            s3 += skj0 - skj1;
                        }

                        s1 += sij0 - sij1;
                        s2 += s3 * (sij0 - sij1);
                    }

                    series[0].values[j - 1][1] += y;
                    series[0].values[j - 1][0] += y;
                    if s1 != 0.0 {
                        y -= s2 / s1;
                    }
                }
                // Apply to last column
                let last = n - 1;
                series[0].values[last][1] += y;
                series[0].values[last][0] += y;

                // Restack: apply StackOffset::None (cumulative from series[0] baseline)
                for j in 0..n {
                    let mut y0 = series[0].values[j][0];
                    for s in series.iter_mut() {
                        let width = s.values[j][1] - s.values[j][0];
                        s.values[j][0] = y0;
                        s.values[j][1] = y0 + width;
                        y0 += width;
                    }
                }
            }
        }
    }
}

/// Simple stack function for basic use cases.
///
/// # Example
///
/// ```
/// use d3rs::shape::stack::stack;
///
/// let data = vec![
///     vec![1.0, 2.0],
///     vec![3.0, 4.0],
/// ];
///
/// let result = stack(&data);
/// assert_eq!(result.len(), 2);
/// ```
pub fn stack(data: &[Vec<f64>]) -> Vec<StackSeries> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new().keys(keys).generate(data)
}

/// Checked simple stack function for basic use cases.
pub fn try_stack(data: &[Vec<f64>]) -> Result<Vec<StackSeries>, StackLayoutError> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new().keys(keys).try_generate(data)
}

/// Create a 100% stacked layout.
pub fn stack_expand(data: &[Vec<f64>]) -> Vec<StackSeries> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new()
        .keys(keys)
        .offset(StackOffset::Expand)
        .generate(data)
}

/// Create a checked 100% stacked layout.
pub fn try_stack_expand(data: &[Vec<f64>]) -> Result<Vec<StackSeries>, StackLayoutError> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new()
        .keys(keys)
        .offset(StackOffset::Expand)
        .try_generate(data)
}

/// Create a streamgraph layout (wiggle offset with inside-out ordering).
pub fn streamgraph(data: &[Vec<f64>]) -> Vec<StackSeries> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new()
        .keys(keys)
        .order(StackOrder::InsideOut)
        .offset(StackOffset::Wiggle)
        .generate(data)
}

/// Create a checked streamgraph layout.
pub fn try_streamgraph(data: &[Vec<f64>]) -> Result<Vec<StackSeries>, StackLayoutError> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new()
        .keys(keys)
        .order(StackOrder::InsideOut)
        .offset(StackOffset::Wiggle)
        .try_generate(data)
}
