/// Main view tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditorTab {
    #[default]
    Colors,
    Preview,
    Export,
}
