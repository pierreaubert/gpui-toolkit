/// Tiling algorithm for treemap layout.
///
/// Different algorithms create different visual patterns:
/// - **Squarify**: Optimizes for square-like rectangles (best for size comparison)
/// - **Binary**: Recursive binary subdivision (balanced tree structure)
/// - **Slice**: Horizontal strips (simple, can create thin rectangles)
/// - **Dice**: Vertical strips (simple, can create thin rectangles)
/// - **SliceDice**: Alternates slice/dice by depth (clear hierarchy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TilingMethod {
    /// Create rectangles with aspect ratios close to 1 (most readable)
    #[default]
    Squarify,
    /// Recursively subdivide into two balanced halves
    Binary,
    /// Horizontal strips (can create thin rectangles)
    Slice,
    /// Vertical strips (can create thin rectangles)
    Dice,
    /// Alternates between slice and dice at each depth level
    SliceDice,
}
