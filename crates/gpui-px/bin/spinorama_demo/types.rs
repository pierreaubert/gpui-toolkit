use gpui::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum LoadState {
    Idle,
    Loading,
    Loaded,
    Error(String),
}
