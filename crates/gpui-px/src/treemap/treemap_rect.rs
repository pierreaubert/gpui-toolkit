/// A rectangle in the computed treemap layout.
#[derive(Debug, Clone)]
pub(super) struct TreemapRect {
    pub(super) x0: f64,
    pub(super) y0: f64,
    pub(super) x1: f64,
    pub(super) y1: f64,
    pub(super) name: String,
    pub(super) value: f64,
    pub(super) _depth: usize,
    pub(super) category_index: usize,
}
