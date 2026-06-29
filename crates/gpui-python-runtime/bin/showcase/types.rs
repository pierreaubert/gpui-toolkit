use gpui::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum StackDirection {
    Vertical,
    Horizontal,
    Wrap,
}
