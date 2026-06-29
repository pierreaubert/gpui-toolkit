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
