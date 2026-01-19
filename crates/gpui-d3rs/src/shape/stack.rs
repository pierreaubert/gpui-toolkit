//! Stack layout generator
//!
//! Computes stacked layouts for stacked bar charts and stacked area charts.

/// A stacked series of data.
#[derive(Debug, Clone)]
pub struct StackSeries {
    /// The key for this series
    pub key: String,
    /// The original data values for this series
    pub data: Vec<f64>,
    /// The stacked values [y0, y1] for each data point
    pub values: Vec<[f64; 2]>,
    /// Index in the stack
    pub index: usize,
}

impl StackSeries {
    /// Get the stacked bounds for a specific index.
    pub fn get(&self, index: usize) -> Option<[f64; 2]> {
        self.values.get(index).copied()
    }
}

/// Stack order strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackOrder {
    /// Use the given order (no reordering)
    #[default]
    None,
    /// Sort by sum of values (smallest first)
    Ascending,
    /// Sort by sum of values (largest first)
    Descending,
    /// Place positive values above zero, negative below
    Appearance,
    /// Interleave largest and smallest series
    InsideOut,
    /// Reverse the given order
    Reverse,
}

/// Stack offset strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackOffset {
    /// Zero baseline (standard stacked chart)
    #[default]
    None,
    /// Expand to fill [0, 1] (100% stacked chart)
    Expand,
    /// Diverging stack around zero
    Diverging,
    /// Silhouette (centered around zero)
    Silhouette,
    /// Wiggle (minimizes weighted wiggle)
    Wiggle,
}

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
                if n == 0 || series.is_empty() {
                    return;
                }

                let m = series.len() as f64;

                for j in 0..n {
                    let mut sum = 0.0;
                    let mut s0 = 0.0;

                    for (i, s) in series.iter().enumerate() {
                        let value = s.values[j][1] - s.values[j][0];
                        if j > 0 {
                            let prev_value = s.values[j - 1][1] - s.values[j - 1][0];
                            let dy = value - prev_value;
                            s0 += dy * (m - i as f64 - 0.5);
                        }
                        sum += value;
                    }

                    let offset = if j == 0 {
                        -sum / 2.0
                    } else {
                        series[0].values[j - 1][0] - s0 / m - (sum / 2.0)
                    };

                    for s in series.iter_mut() {
                        s.values[j][0] += offset;
                        s.values[j][1] += offset;
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

/// Create a 100% stacked layout.
pub fn stack_expand(data: &[Vec<f64>]) -> Vec<StackSeries> {
    let num_series = data.first().map(|row| row.len()).unwrap_or(0);
    let keys: Vec<String> = (0..num_series).map(|i| i.to_string()).collect();

    Stack::new()
        .keys(keys)
        .offset(StackOffset::Expand)
        .generate(data)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_basic() {
        let data = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];

        let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let result = Stack::new().keys(keys).generate(&data);

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_stack_values() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];

        let keys = vec!["A".to_string(), "B".to_string()];
        let result = Stack::new().keys(keys).generate(&data);

        // First series: [0, 1], [0, 3]
        // Second series: [1, 3], [3, 7]
        assert_eq!(result[0].values[0], [0.0, 1.0]);
        assert_eq!(result[0].values[1], [0.0, 3.0]);
    }

    #[test]
    fn test_stack_expand() {
        let data = vec![vec![1.0, 1.0], vec![1.0, 1.0]];

        let result = stack_expand(&data);

        // Should normalize to [0, 1]
        let sum: f64 = result.iter().map(|s| s.values[0][1] - s.values[0][0]).sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_stack_silhouette() {
        let data = vec![vec![1.0, 1.0]];

        let keys = vec!["A".to_string(), "B".to_string()];
        let result = Stack::new()
            .keys(keys)
            .offset(StackOffset::Silhouette)
            .generate(&data);

        // Should be centered around zero
        let mid = (result[0].values[0][0] + result.last().unwrap().values[0][1]) / 2.0;
        assert!(mid.abs() < 0.001);
    }

    #[test]
    fn test_stack_order_descending() {
        let data = vec![vec![1.0, 3.0, 2.0]];

        let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let result = Stack::new()
            .keys(keys)
            .order(StackOrder::Descending)
            .generate(&data);

        // Largest sum should be first
        assert!(result[0].key == "B");
    }

    #[test]
    fn test_stack_order_preserves_data_values() {
        // Test that reordering uses correct data values, not reordered indices
        let data = vec![vec![10.0, 100.0, 1.0]]; // A=10, B=100, C=1

        let keys = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let result = Stack::new()
            .keys(keys)
            .order(StackOrder::Descending) // Order: B(100), A(10), C(1)
            .generate(&data);

        // After descending order: B first, then A, then C
        assert_eq!(result[0].key, "B");
        assert_eq!(result[1].key, "A");
        assert_eq!(result[2].key, "C");

        // Verify the stacked values use correct data
        // B: [0, 100]
        assert_eq!(result[0].values[0], [0.0, 100.0]);
        // A: [100, 110]
        assert_eq!(result[1].values[0], [100.0, 110.0]);
        // C: [110, 111]
        assert_eq!(result[2].values[0], [110.0, 111.0]);
    }

    #[test]
    fn test_streamgraph() {
        let data = vec![
            vec![1.0, 2.0, 1.0],
            vec![2.0, 3.0, 2.0],
            vec![1.0, 2.0, 1.0],
        ];

        let result = streamgraph(&data);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_stack_empty() {
        let data: Vec<Vec<f64>> = vec![];
        let result = Stack::new().keys(vec!["A".to_string()]).generate(&data);
        assert!(result.is_empty());
    }
}
