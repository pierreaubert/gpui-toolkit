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
