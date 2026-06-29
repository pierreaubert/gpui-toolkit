use super::showcase_section::ShowcaseSection;

/// Component groups for organizing related sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShowcaseGroup {
    Actions,
    TextAndLabels,
    FormControls,
    Navigation,
    Feedback,
    DataDisplay,
    Overlays,
    LayoutAndStructure,
    Controls,
    MultiStep,
    Media,
}

impl ShowcaseGroup {
    pub fn all() -> &'static [ShowcaseGroup] {
        &[
            ShowcaseGroup::Actions,
            ShowcaseGroup::TextAndLabels,
            ShowcaseGroup::FormControls,
            ShowcaseGroup::Navigation,
            ShowcaseGroup::Feedback,
            ShowcaseGroup::DataDisplay,
            ShowcaseGroup::Overlays,
            ShowcaseGroup::LayoutAndStructure,
            ShowcaseGroup::Controls,
            ShowcaseGroup::MultiStep,
            ShowcaseGroup::Media,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ShowcaseGroup::Actions => "Actions",
            ShowcaseGroup::TextAndLabels => "Text & Labels",
            ShowcaseGroup::FormControls => "Form Controls",
            ShowcaseGroup::Navigation => "Navigation",
            ShowcaseGroup::Feedback => "Feedback",
            ShowcaseGroup::DataDisplay => "Data Display",
            ShowcaseGroup::Overlays => "Overlays & Popups",
            ShowcaseGroup::LayoutAndStructure => "Layout & Structure",
            ShowcaseGroup::Controls => "Controls",
            ShowcaseGroup::MultiStep => "Multi-Step",
            ShowcaseGroup::Media => "Media & Visuals",
        }
    }

    pub fn sections(&self) -> &'static [ShowcaseSection] {
        match self {
            ShowcaseGroup::Actions => &[ShowcaseSection::Buttons, ShowcaseSection::IconButtons],
            ShowcaseGroup::TextAndLabels => &[
                ShowcaseSection::Text,
                ShowcaseSection::Badges,
                ShowcaseSection::Tag,
                ShowcaseSection::KeyboardShortcut,
            ],
            ShowcaseGroup::FormControls => &[
                ShowcaseSection::FormControls,
                ShowcaseSection::SettingsForm,
                ShowcaseSection::Accessibility,
            ],
            ShowcaseGroup::Navigation => &[
                ShowcaseSection::Menu,
                ShowcaseSection::ContextMenu,
                ShowcaseSection::CommandPalette,
                ShowcaseSection::Tabs,
                ShowcaseSection::Breadcrumbs,
                ShowcaseSection::SearchBar,
            ],
            ShowcaseGroup::Feedback => &[
                ShowcaseSection::Alerts,
                ShowcaseSection::Toasts,
                ShowcaseSection::Notification,
                ShowcaseSection::Progress,
                ShowcaseSection::Spinners,
                ShowcaseSection::LoadingOverlay,
                ShowcaseSection::EmptyState,
            ],
            ShowcaseGroup::DataDisplay => &[
                ShowcaseSection::Table,
                ShowcaseSection::Cards,
                ShowcaseSection::Avatars,
                ShowcaseSection::TreeView,
                ShowcaseSection::DragList,
            ],
            ShowcaseGroup::Overlays => &[
                ShowcaseSection::Dialog,
                ShowcaseSection::ConfirmDialog,
                ShowcaseSection::Popover,
                ShowcaseSection::Tooltips,
            ],
            ShowcaseGroup::LayoutAndStructure => &[
                ShowcaseSection::Layout,
                ShowcaseSection::SplitPane,
                ShowcaseSection::Sidebar,
                ShowcaseSection::StatusBar,
                ShowcaseSection::Toolbar,
            ],
            ShowcaseGroup::Controls => &[ShowcaseSection::Accordion],
            ShowcaseGroup::MultiStep => &[
                ShowcaseSection::Wizard,
                ShowcaseSection::StepIndicator,
                ShowcaseSection::Workflow,
            ],
            ShowcaseGroup::Media => &[ShowcaseSection::QrCode, ShowcaseSection::ImageView],
        }
    }

    /// Description explaining the differences between components in this group
    pub fn description(&self) -> &'static str {
        match self {
            ShowcaseGroup::Actions => {
                "\
Buttons trigger actions on click. \
Icon Buttons are compact, icon-only variants for toolbars and tight spaces where a text label would be too wide."
            }

            ShowcaseGroup::TextAndLabels => {
                "\
Text renders styled inline or block text. \
Badges are small status indicators (counts, labels) typically attached to other elements. \
Tags are removable, categorical labels for filtering and tagging content. \
Keyboard Shortcuts display key combinations like Cmd+S."
            }

            ShowcaseGroup::FormControls => {
                "\
Form Controls are individual input primitives (toggles, checkboxes, sliders, selects, number inputs). \
Settings Form composes multiple form controls into a labeled, grouped settings page with sections and descriptions."
            }

            ShowcaseGroup::Navigation => {
                "\
Menu is a bare dropdown panel of items you position yourself (used inside MenuBar or custom dropdowns). \
Context Menu wraps Menu with absolute positioning at the cursor and a click-outside-to-dismiss backdrop (right-click menus). \
Command Palette is a searchable, filterable command list triggered by a keyboard shortcut. \
Tabs switch between views in a fixed set. \
Breadcrumbs show hierarchical location for drill-down navigation. \
Search Bar provides a text input with search-specific affordances (icon, clear button)."
            }

            ShowcaseGroup::Feedback => {
                "\
Alerts are persistent, inline banners for important messages (info, warning, error). \
Toasts are temporary, auto-dismissing notifications that stack at a screen edge. \
Notifications are richer, persistent messages with actions (mark read, dismiss). \
Progress shows determinate completion (0-100%). \
Spinners show indeterminate loading with no known duration. \
Loading Overlay covers a region with a spinner and optional message, blocking interaction. \
Empty State is a placeholder shown when a list or view has no data."
            }

            ShowcaseGroup::DataDisplay => {
                "\
Table displays structured rows and columns with sorting and selection. \
Cards are flexible content containers for preview or summary information. \
Avatars represent users or entities with images or initials. \
Tree View shows hierarchical, expandable data (file trees, nested lists). \
Drag List is an orderable list where items can be reordered by dragging."
            }

            ShowcaseGroup::Overlays => {
                "\
Dialog is a general-purpose modal overlay for complex content (forms, detail views). \
Confirm Dialog is a focused, pre-built dialog for yes/no confirmation prompts with variant styling (info, warning, danger). \
Popover is a non-modal floating panel anchored to a trigger element (menus, pickers). \
Tooltips are small, hover-triggered text hints that describe an element."
            }

            ShowcaseGroup::LayoutAndStructure => {
                "\
Layout provides flex/grid containers and spacing primitives (VStack, HStack, Divider). \
Split Pane divides a region into resizable panels with a draggable divider. \
Sidebar is a fixed or collapsible side panel for navigation or tools. \
Status Bar is a narrow bar at the window bottom for status information. \
Toolbar is a horizontal bar of actions and controls, typically at the top of a view."
            }

            ShowcaseGroup::Controls => {
                "\
Accordion expands and collapses content sections, either one-at-a-time or multiple simultaneously."
            }

            ShowcaseGroup::MultiStep => {
                "\
Wizard guides users through a multi-step process with prev/next navigation and step validation. \
Step Indicator is a read-only progress display showing which step the user is on, without navigation logic. \
Workflow is a node-and-edge graph editor for visual pipeline or flowchart building."
            }

            ShowcaseGroup::Media => {
                "\
QR Code generates and renders a QR code from data. \
Image View displays images with optional zoom, pan, and loading states."
            }
        }
    }
}
