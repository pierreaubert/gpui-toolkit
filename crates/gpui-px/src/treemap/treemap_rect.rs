/// A rectangle in the computed treemap layout.
#[derive(Debug, Clone, PartialEq)]
pub struct TreemapRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub name: String,
    pub value: f64,
    pub depth: usize,
    pub category_index: usize,
}
