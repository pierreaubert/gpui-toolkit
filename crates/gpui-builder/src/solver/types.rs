use crate::types::LayoutNode;

pub(super) struct ChildInfo<'a> {
    pub(super) node: &'a LayoutNode<'a>,
    pub(super) user_collapsed: bool,
    pub(super) solver_collapsed: bool,
    pub(super) allocated_size: f32,
    /// Pre-computed size for `Sizing::Text` nodes (None for other sizing types).
    pub(super) computed_text_size: Option<f32>,
}
