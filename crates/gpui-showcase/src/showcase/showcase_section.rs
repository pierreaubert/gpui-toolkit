use super::showcase_group::ShowcaseGroup;

/// Section identifiers for navigation
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowcaseSection {
    #[default]
    Buttons,
    Text,
    Badges,
    Avatars,
    FormControls,
    Progress,
    Alerts,
    Tabs,
    Cards,
    Breadcrumbs,
    Spinners,
    Layout,
    IconButtons,
    Toasts,
    Dialog,
    Menu,
    Table,
    Tooltips,
    Accordion,
    Wizard,
    Workflow,
    QrCode,
    ContextMenu,
    Popover,
    Sidebar,
    StatusBar,
    SearchBar,
    KeyboardShortcut,
    EmptyState,
    ConfirmDialog,
    SplitPane,
    ImageView,
    SettingsForm,
    StepIndicator,
    LoadingOverlay,
    Tag,
    Toolbar,
    Notification,
    TreeView,
    DragList,
    CommandPalette,
    Accessibility,
    AudioVisuals,
    ThinkingOrbs,
    Scene2d,
    Zip,
    Queens,
    Sudoku,
    Tetris,
    Chess,
    Othello,
    AppShell,
    TopNav,
    MobileNav,
    DateRangeInput,
    TimeInput,
    DateTimeInput,
    Chat,
    Markdown,
    Blockquote,
    Carousel,
    SelectableCard,
    Citation,
    Timestamp,
    MetadataList,
    FileInput,
    Tokenizer,
    Field,
    FieldStatus,
    Skeleton,
    Lightbox,
    HoverCard,
    Layer,
    Calendar,
    Pagination,
    List,
}

impl ShowcaseSection {
    /// Stable position in [`Self::all`], used by allocation-free navigation caches.
    pub const fn index(self) -> usize {
        self as usize
    }

    pub fn all() -> &'static [ShowcaseSection] {
        &[
            ShowcaseSection::Buttons,
            ShowcaseSection::Text,
            ShowcaseSection::Badges,
            ShowcaseSection::Avatars,
            ShowcaseSection::FormControls,
            ShowcaseSection::Progress,
            ShowcaseSection::Alerts,
            ShowcaseSection::Tabs,
            ShowcaseSection::Cards,
            ShowcaseSection::Breadcrumbs,
            ShowcaseSection::Spinners,
            ShowcaseSection::Layout,
            ShowcaseSection::IconButtons,
            ShowcaseSection::Toasts,
            ShowcaseSection::Dialog,
            ShowcaseSection::Menu,
            ShowcaseSection::Table,
            ShowcaseSection::Tooltips,
            ShowcaseSection::Accordion,
            ShowcaseSection::Wizard,
            ShowcaseSection::Workflow,
            ShowcaseSection::QrCode,
            ShowcaseSection::ContextMenu,
            ShowcaseSection::Popover,
            ShowcaseSection::Sidebar,
            ShowcaseSection::StatusBar,
            ShowcaseSection::SearchBar,
            ShowcaseSection::KeyboardShortcut,
            ShowcaseSection::EmptyState,
            ShowcaseSection::ConfirmDialog,
            ShowcaseSection::SplitPane,
            ShowcaseSection::ImageView,
            ShowcaseSection::SettingsForm,
            ShowcaseSection::StepIndicator,
            ShowcaseSection::LoadingOverlay,
            ShowcaseSection::Tag,
            ShowcaseSection::Toolbar,
            ShowcaseSection::Notification,
            ShowcaseSection::TreeView,
            ShowcaseSection::DragList,
            ShowcaseSection::CommandPalette,
            ShowcaseSection::Accessibility,
            ShowcaseSection::AudioVisuals,
            ShowcaseSection::ThinkingOrbs,
            ShowcaseSection::Scene2d,
            ShowcaseSection::Zip,
            ShowcaseSection::Queens,
            ShowcaseSection::Sudoku,
            ShowcaseSection::Tetris,
            ShowcaseSection::Chess,
            ShowcaseSection::Othello,
            ShowcaseSection::AppShell,
            ShowcaseSection::TopNav,
            ShowcaseSection::MobileNav,
            ShowcaseSection::DateRangeInput,
            ShowcaseSection::TimeInput,
            ShowcaseSection::DateTimeInput,
            ShowcaseSection::Chat,
            ShowcaseSection::Markdown,
            ShowcaseSection::Blockquote,
            ShowcaseSection::Carousel,
            ShowcaseSection::SelectableCard,
            ShowcaseSection::Citation,
            ShowcaseSection::Timestamp,
            ShowcaseSection::MetadataList,
            ShowcaseSection::FileInput,
            ShowcaseSection::Tokenizer,
            ShowcaseSection::Field,
            ShowcaseSection::FieldStatus,
            ShowcaseSection::Skeleton,
            ShowcaseSection::Lightbox,
            ShowcaseSection::HoverCard,
            ShowcaseSection::Layer,
            ShowcaseSection::Calendar,
            ShowcaseSection::Pagination,
            ShowcaseSection::List,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ShowcaseSection::Buttons => "Buttons",
            ShowcaseSection::Text => "Text",
            ShowcaseSection::Badges => "Badges",
            ShowcaseSection::Avatars => "Avatars",
            ShowcaseSection::FormControls => "Form Controls",
            ShowcaseSection::Progress => "Progress",
            ShowcaseSection::Alerts => "Alerts",
            ShowcaseSection::Tabs => "Tabs",
            ShowcaseSection::Cards => "Cards",
            ShowcaseSection::Breadcrumbs => "Breadcrumbs",
            ShowcaseSection::Spinners => "Spinners",
            ShowcaseSection::Layout => "Layout",
            ShowcaseSection::IconButtons => "Icon Buttons",
            ShowcaseSection::Toasts => "Toasts",
            ShowcaseSection::Dialog => "Dialog",
            ShowcaseSection::Menu => "Menu",
            ShowcaseSection::Table => "Table",
            ShowcaseSection::Tooltips => "Tooltips",
            ShowcaseSection::Accordion => "Accordion",
            ShowcaseSection::Wizard => "Wizard",
            ShowcaseSection::Workflow => "Workflow",
            ShowcaseSection::QrCode => "QR Code",
            ShowcaseSection::ContextMenu => "Context Menu",
            ShowcaseSection::Popover => "Popover",
            ShowcaseSection::Sidebar => "Sidebar",
            ShowcaseSection::StatusBar => "Status Bar",
            ShowcaseSection::SearchBar => "Search Bar",
            ShowcaseSection::KeyboardShortcut => "Keyboard Shortcuts",
            ShowcaseSection::EmptyState => "Empty State",
            ShowcaseSection::ConfirmDialog => "Confirm Dialog",
            ShowcaseSection::SplitPane => "Split Pane",
            ShowcaseSection::ImageView => "Image View",
            ShowcaseSection::SettingsForm => "Settings Form",
            ShowcaseSection::StepIndicator => "Step Indicator",
            ShowcaseSection::LoadingOverlay => "Loading Overlay",
            ShowcaseSection::Tag => "Tag",
            ShowcaseSection::Toolbar => "Toolbar",
            ShowcaseSection::Notification => "Notification",
            ShowcaseSection::TreeView => "Tree View",
            ShowcaseSection::DragList => "Drag List",
            ShowcaseSection::CommandPalette => "Command Palette",
            ShowcaseSection::Accessibility => "Accessibility",
            ShowcaseSection::AudioVisuals => "Audio Visuals",
            ShowcaseSection::ThinkingOrbs => "Thinking Orbs",
            ShowcaseSection::Scene2d => "Scene2D",
            ShowcaseSection::Zip => "Zip",
            ShowcaseSection::Queens => "Queens",
            ShowcaseSection::Sudoku => "Sudoku",
            ShowcaseSection::Tetris => "Tetris",
            ShowcaseSection::Chess => "Chess",
            ShowcaseSection::Othello => "Othello",
            ShowcaseSection::AppShell => "App Shell",
            ShowcaseSection::TopNav => "Top Nav",
            ShowcaseSection::MobileNav => "Mobile Nav",
            ShowcaseSection::DateRangeInput => "Date Range Input",
            ShowcaseSection::TimeInput => "Time Input",
            ShowcaseSection::DateTimeInput => "Date Time Input",
            ShowcaseSection::Chat => "Chat",
            ShowcaseSection::Markdown => "Markdown",
            ShowcaseSection::Blockquote => "Blockquote",
            ShowcaseSection::Carousel => "Carousel",
            ShowcaseSection::SelectableCard => "Selectable Card",
            ShowcaseSection::Citation => "Citation",
            ShowcaseSection::Timestamp => "Timestamp",
            ShowcaseSection::MetadataList => "Metadata List",
            ShowcaseSection::FileInput => "File Input",
            ShowcaseSection::Tokenizer => "Tokenizer",
            ShowcaseSection::Field => "Field",
            ShowcaseSection::FieldStatus => "Field Status",
            ShowcaseSection::Skeleton => "Skeleton",
            ShowcaseSection::Lightbox => "Lightbox",
            ShowcaseSection::HoverCard => "Hover Card",
            ShowcaseSection::Layer => "Layer",
            ShowcaseSection::Calendar => "Calendar",
            ShowcaseSection::Pagination => "Pagination",
            ShowcaseSection::List => "List",
        }
    }

    /// Whether this section renders one of the playable games.
    pub fn is_game(self) -> bool {
        matches!(
            self,
            ShowcaseSection::Zip
                | ShowcaseSection::Queens
                | ShowcaseSection::Sudoku
                | ShowcaseSection::Tetris
                | ShowcaseSection::Chess
                | ShowcaseSection::Othello
        )
    }

    pub fn group(&self) -> ShowcaseGroup {
        for group in ShowcaseGroup::all() {
            if group.sections().contains(self) {
                return *group;
            }
        }
        ShowcaseGroup::Actions
    }
}

#[cfg(test)]
mod tests {
    use super::ShowcaseSection;

    #[test]
    fn section_indices_match_navigation_order() {
        for (index, section) in ShowcaseSection::all().iter().enumerate() {
            assert_eq!(section.index(), index);
        }
    }
}
