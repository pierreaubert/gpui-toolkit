//! UI Kit - A local component library for GPUI
//!
//! Inspired by adabraka-ui patterns but compatible with zed's gpui version.
//! Provides reusable, composable UI components with consistent styling.

// Allow complex callback types - common in UI code
#![allow(clippy::type_complexity)]
// Allow to_* methods that take self by reference - matches GPUI patterns
#![allow(clippy::wrong_self_convention)]
#![recursion_limit = "8192"]

// Theme, animation, i18n, and accessibility
pub mod accessibility;
pub mod adaptive_overflow;
pub mod animation;
pub(crate) mod arc;
mod behavior_matrix;
pub mod color_tokens;
pub mod design;
pub mod i18n;
pub mod interaction;
pub mod mobile;
pub mod scale;
pub mod security_surface;
pub mod theme;
pub mod visual_regression;

// Core components
pub mod button;
pub mod button_set;
pub mod card;
pub mod confirm_dialog;
pub mod context_menu;
pub mod dialog;
pub mod focus;
pub mod icon_button;
pub mod menu;
pub mod popover;
pub mod tabs;
pub mod toast;

pub mod size;

// Form components
pub mod checkbox;
pub mod color;
pub mod color_picker;
pub mod input;
pub mod number_input;
pub mod select;
pub mod slider;
pub mod toggle;

// Data display
pub mod avatar;
pub mod badge;
pub mod collection_diff;
pub mod data_navigation;
pub mod empty_state;
pub mod image_view;
pub mod keyboard_shortcut_label;
pub mod progress;
pub mod qr;
pub mod spinner;
pub mod step_indicator;
pub mod table;
pub mod text;

// Feedback
pub mod alert;
pub mod search_bar;
pub mod tooltip;

// Navigation
pub mod accordion;
pub mod breadcrumbs;
pub mod wizard;

// Layout
pub mod loading_overlay;
pub mod pane_divider;
pub mod settings_form;
pub mod sidebar;
pub mod split_pane;
pub mod stack;
pub mod status_bar;
pub mod swipe_panel;

// Tier 3 components
pub mod command_palette;
pub mod drag_list;
pub mod notification;
pub mod tag;
pub mod toolbar;
pub mod tree_view;

// Workflow canvas
pub mod workflow;

// Re-export commonly used types

// Buttons
pub use button::{Button, ButtonSize, ButtonTheme, ButtonVariant};
pub use button_set::{ButtonSet, ButtonSetOption, ButtonSetSize, ButtonSetTheme};
pub use icon_button::{IconButton, IconButtonSize, IconButtonTheme, IconButtonVariant};
// Containers
pub use adaptive_overflow::{AdaptiveOverflow, AdaptiveOverflowPresentation};
pub use card::{Card, SlotFactory};
pub use confirm_dialog::{ConfirmDialog, ConfirmDialogTheme, ConfirmDialogVariant};
pub use context_menu::{ContextMenu, ContextMenuTheme};
pub use dialog::{Dialog, DialogSize, DialogSlotFactory, DialogTheme};
pub use popover::{Popover, PopoverPlacement, PopoverSlotFactory, PopoverTheme};
// Navigation
pub use accordion::{
    Accordion, AccordionItem, AccordionMode, AccordionOrientation, AccordionTheme,
};
pub use breadcrumbs::{BreadcrumbItem, BreadcrumbSeparator, Breadcrumbs};
pub use menu::{Menu, MenuBar, MenuBarItem, MenuItem, MenuTheme, menu_bar_button};
pub use tabs::{IconFactory, TabItem, TabVariant, Tabs, TabsTheme};
pub use wizard::{
    StepStatus, Wizard, WizardHeader, WizardNavigation, WizardStep, WizardTheme, WizardVariant,
};
// Focus management
pub use focus::{
    FOCUS_INTEGRATION_REPORT_TYPE, FOCUS_INTEGRATION_SCHEMA_VERSION, FocusDirection, FocusGroup,
    FocusGroupExt, FocusIntegrationEntry, FocusIntegrationReport, FocusIntegrationStatus,
    focus_integration_entries, focus_integration_report,
};
pub use security_surface::{
    SECURITY_SURFACE_REPORT_TYPE, SECURITY_SURFACE_SCHEMA_VERSION, SecuritySurfaceEntry,
    SecuritySurfaceReport, SecuritySurfaceStatus, security_surface_entries,
    security_surface_report,
};
pub use visual_regression::{
    UI_KIT_VISUAL_COLOR_SCHEMES, UI_KIT_VISUAL_REGRESSION_REPORT_TYPE,
    UI_KIT_VISUAL_REGRESSION_SCHEMA_VERSION, UI_KIT_VISUAL_STORIES, UI_KIT_VISUAL_VIEWPORTS,
    UiKitVisualCapture, UiKitVisualColorScheme, UiKitVisualRegressionManifest, UiKitVisualStory,
    UiKitVisualViewport, ui_kit_visual_regression_manifest, ui_kit_visual_stories,
};
// Notifications
pub use alert::{Alert, AlertVariant, InlineAlert};
pub use data_navigation::{DataNavigationAction, DataNavigationState, DataVirtualWindow, move_key};
pub use toast::{Toast, ToastContainer, ToastPosition, ToastVariant};
// Form
pub use checkbox::{Checkbox, CheckboxSize, CheckboxTheme};
pub use color::Color;
pub use color_picker::{ColorPickerMode, ColorPickerView};
pub use input::{
    Input, InputSize, InputTheme, InputVariant, cleanup_input_state, cleanup_stale_input_states,
    clear_all_input_states, input_state_count, is_input_editing,
};
pub use number_input::{
    NumberInput, NumberInputSize, NumberInputTheme, cleanup_number_input_state,
    is_number_input_editing,
};
pub use select::{Select, SelectOption, SelectSize, SelectTheme};
pub use slider::{Slider, SliderSize, SliderTheme};
pub use toggle::{Toggle, ToggleSize, ToggleStyle, ToggleTheme};
// Data display
pub use avatar::{Avatar, AvatarGroup, AvatarShape, AvatarSize, AvatarStatus};
pub use badge::{Badge, BadgeDot, BadgeSize, BadgeVariant};
pub use empty_state::EmptyState;
pub use image_view::{ImageFit, ImageView, ImageViewTheme};
pub use keyboard_shortcut_label::{KeyboardShortcutLabel, KeyboardShortcutSize};
pub use progress::{CircularProgress, Progress, ProgressSize, ProgressVariant};
pub use qr::{AnimatedQrCode, QrCode, QrCodeError, QrCodeLimits};
pub use spinner::{LoadingDots, Spinner, SpinnerSize};
pub use step_indicator::{
    StepIndicator, StepIndicatorSize, StepIndicatorTheme, StepItem, StepItemStatus, StepOrientation,
};
pub use table::{
    Column, PaginationState, SelectionMode, SortDirection, SortState, Table, TableTheme,
};
pub use text::{Code, Heading, Link, Text, TextSize, TextWeight, code_text_color};
// Feedback
pub use search_bar::{SearchBar, SearchBarSize, SearchBarTheme};
pub use tooltip::{Tooltip, TooltipPlacement, WithTooltip};
// Layout
pub use loading_overlay::{LoadingOverlay, LoadingOverlayTheme};
pub use pane_divider::{CollapseDirection, PaneDivider, PaneDividerTheme};
pub use settings_form::{SettingsForm, SettingsFormTheme, SettingsRow};
pub use sidebar::{Sidebar, SidebarSide, SidebarSlotFactory, SidebarTheme};
pub use split_pane::{SplitDirection, SplitPane, SplitPaneTheme};
pub use stack::{
    Divider, HStack, Spacer, StackAlign, StackJustify, StackOverflow, StackSize, StackSpacing,
    VStack,
};
pub use swipe_panel::{SwipePanel, SwipePanelAnchor, SwipePanelState};
// Status bar
pub use status_bar::{StatusBar, StatusBarPosition, StatusBarTheme};
// Animation
pub use animation::{
    Animation, Easing, Keyframe, KeyframeAnimation, Spring, ease, evaluate_keyframes, interpolate,
    interpolate_color,
};
// Accessibility
pub use accessibility::{
    ACCESSIBILITY_ADAPTER_REPORT_TYPE, ACCESSIBILITY_ADAPTER_SCHEMA_VERSION,
    ACCESSIBILITY_BRIDGE_REPORT_TYPE, ACCESSIBILITY_BRIDGE_SCHEMA_VERSION,
    ACCESSIBILITY_READINESS_REPORT_TYPE, ACCESSIBILITY_READINESS_SCHEMA_VERSION,
    AccessibilityBridgeNode, AccessibilityBridgeSnapshot, AccessibilityBridgeState,
    AccessibilityBridgeValue, AccessibilityExt, AccessibilityNode, AccessibilityReadinessEntry,
    AccessibilityReadinessReport, AccessibilityReadinessStatus, AccessibilityTree, AriaLive,
    AriaProps, AriaRole, AriaState, NativeAccessibilityAction, NativeAccessibilityAdapterError,
    NativeAccessibilityAdapterNode, NativeAccessibilityAdapterPayload, NativeAccessibilityTarget,
    accessibility_readiness_entries, accessibility_readiness_report, apply_native_accessibility,
};
pub use behavior_matrix::{
    BehaviorStatus, COMPONENT_BEHAVIOR_REPORT_TYPE, COMPONENT_BEHAVIOR_SCHEMA_VERSION,
    ComponentBehaviorEntry, ComponentBehaviorReport, component_behavior_entries,
    component_behavior_report,
};
pub use collection_diff::{CollectionPatch, diff_by_key, is_content_only_update};
pub use mobile::{
    ContextPreview, DynamicTypePolicy, EdgeInsets, MomentumDelta, MomentumScroller,
    PullToRefreshState, SwipeAction, SwipeDirection, VelocityTracker, WaveformScrubber, is_mobile,
};
// Theme and i18n
pub use color_tokens::{
    BackgroundColors, BorderColors, ColorPalette, ColorToken, SemanticColors, TextColors, darken,
    desaturate, lighten, saturate, with_alpha,
};
pub use gpui_design::{DesignExt, DesignSystem, DesignSystemState};
pub use i18n::{I18nExt, I18nState, Language, TranslationKey, Translations};
pub use theme::{Theme, ThemeExt, ThemeState, ThemeVariant, glow_shadow};
// Builder/layout solver integration
pub mod layout_builder {
    pub use gpui_builder::{
        LayoutDebugReport, LayoutDebugWarning, LayoutDebugWarningKind, LayoutSnapshot,
        LayoutSnapshotMatrix, LayoutViewport, SolvedNode, solve, solve_snapshot_matrix,
    };
}

// Workflow canvas
pub use workflow::{
    AddConnectionCommand, AddNodeCommand, BoxSelection, BulkConnectDrag, CanvasState,
    ChangePortCountsCommand, Command, CompositeCommand, Connection, ConnectionDrag, ConnectionId,
    ContextMenuState, DefaultNodeContent, HistoryManager, HitTestResult, HitTester,
    InteractionMode, LinkType, MoveNodesCommand, NodeContent, NodeDragState, NodeId, Port,
    PortDirection, Position, RemoveConnectionCommand, RemoveNodeCommand, SelectionState,
    ViewportState, WorkflowCanvas, WorkflowGraph, WorkflowNode, WorkflowNodeData, WorkflowTheme,
};
// Tier 3 components
pub use command_palette::{CommandItem, CommandPalette, CommandPaletteTheme};
pub use drag_list::{DragItem, DragList, DragListOrientation, DragListTheme};
pub use notification::{Notification, NotificationTheme, NotificationVariant};
pub use tag::{Tag, TagSize, TagTheme, TagVariant};
pub use toolbar::{Toolbar, ToolbarItem, ToolbarTheme};
pub use tree_view::{TreeNode, TreeView, TreeViewTheme};
// Shared size definitions
pub use size::ComponentSize;

// Derive macros for theme and builder generation
pub use gpui_ui_kit_macros::{ComponentBuilder, ComponentTheme, FormField};
