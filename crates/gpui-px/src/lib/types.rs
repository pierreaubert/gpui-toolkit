/// Scale type for axis transformations.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ScaleType {
    /// Linear scale (default).
    #[default]
    Linear,
    /// Logarithmic scale (base 10).
    Log,
}
