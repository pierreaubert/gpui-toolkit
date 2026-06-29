use std::cmp::Ordering;

/// Wrapper for f64 that implements Ord for use with array functions
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OrdF64(pub(super) f64);

impl Eq for OrdF64 {}

impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}
