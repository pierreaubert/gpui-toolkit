//! UI Kit - A local component library for GPUI
//!
//! Inspired by adabraka-ui patterns but compatible with zed's gpui version.
//! Provides reusable, composable UI components with consistent styling.

// Allow complex callback types - common in UI code
#![allow(clippy::type_complexity)]
// Allow to_* methods that take self by reference - matches GPUI patterns
#![allow(clippy::wrong_self_convention)]

// Application templates
#[path = "../src-app/mod.rs"]
pub mod app;

// Theme, animation, and i18n
pub mod animation;
pub mod color_tokens;
pub mod i18n;
pub mod theme;

// Core components
pub mod button;
pub mod button_set;
pub mod card;
pub mod dialog;
pub mod focus;
pub mod icon_button;
pub mod menu;
pub mod tabs;
pub mod toast;

// Shared utilities
pub mod scale;
pub mod size;

// Form components
pub mod autoeq;
pub mod checkbox;
pub mod color;
pub mod color_picker;
pub mod input;
pub mod number_input;
pub mod select;
pub mod slider;
pub mod toggle;

// audio
pub mod audio;

// Data display
pub mod avatar;
pub mod badge;
pub mod progress;
pub mod spinner;
pub mod text;

// Feedback
pub mod alert;
pub mod tooltip;

// Navigation
pub mod accordion;
pub mod breadcrumbs;
pub mod wizard;

// Layout
pub mod pane_divider;
pub mod stack;

// Workflow canvas
pub mod workflow;

// Re-export commonly used types

// Buttons
pub use button::{Button, ButtonSize, ButtonTheme, ButtonVariant};
pub use button_set::{ButtonSet, ButtonSetOption, ButtonSetSize, ButtonSetTheme};
pub use icon_button::{IconButton, IconButtonSize, IconButtonTheme, IconButtonVariant};

// Containers
pub use card::{Card, SlotFactory};
pub use dialog::{Dialog, DialogSize, DialogSlotFactory, DialogTheme};

// Navigation
pub use accordion::{Accordion, AccordionItem, AccordionMode, AccordionTheme};
pub use breadcrumbs::{BreadcrumbItem, BreadcrumbSeparator, Breadcrumbs};
pub use menu::{Menu, MenuBar, MenuBarItem, MenuItem, MenuTheme, menu_bar_button};
pub use tabs::{TabItem, TabVariant, Tabs, TabsTheme};
pub use wizard::{
    StepStatus, Wizard, WizardHeader, WizardNavigation, WizardStep, WizardTheme, WizardVariant,
};

// Focus management
pub use focus::{FocusDirection, FocusGroup};

// Notifications
pub use alert::{Alert, AlertVariant, InlineAlert};
pub use toast::{Toast, ToastContainer, ToastPosition, ToastVariant};

// Form
pub use audio::potentiometer::{
    Potentiometer, PotentiometerScale, PotentiometerSize, PotentiometerTheme,
};
pub use audio::vertical_slider::{
    VerticalSlider, VerticalSliderScale, VerticalSliderSize, VerticalSliderTheme,
};
pub use audio::volume_knob::{VolumeKnob, VolumeKnobTheme};
pub use autoeq::{
    ALGORITHM_OPTIONS, AutoEqConfig, AutoEqForm, AutoEqFormTheme, AutoEqFormUiState,
    DE_STRATEGY_OPTIONS, HEADPHONE_TARGET_CURVE_OPTIONS, LOCAL_ALGO_OPTIONS, OptimizationType,
    PEQ_MODEL_OPTIONS, ParamLimits, SPEAKER_TARGET_CURVE_OPTIONS, SPINORAMA_CURVE_OPTIONS,
};
pub use checkbox::{Checkbox, CheckboxSize};
pub use color::Color;
pub use color_picker::{ColorPickerMode, ColorPickerView};
pub use input::{
    Input, InputSize, InputVariant, cleanup_input_state, cleanup_stale_input_states,
    clear_all_input_states, input_state_count,
};
pub use number_input::{
    NumberInput, NumberInputSize, NumberInputTheme, cleanup_number_input_state,
};
pub use select::{Select, SelectOption, SelectSize, SelectTheme};
pub use slider::{Slider, SliderSize, SliderTheme};
pub use toggle::{Toggle, ToggleSize, ToggleStyle, ToggleTheme};

// Data display
pub use avatar::{Avatar, AvatarGroup, AvatarShape, AvatarSize, AvatarStatus};
pub use badge::{Badge, BadgeDot, BadgeSize, BadgeVariant};
pub use progress::{CircularProgress, Progress, ProgressSize, ProgressVariant};
pub use spinner::{LoadingDots, Spinner, SpinnerSize};
pub use text::{Code, Heading, Link, Text, TextSize, TextWeight};

// Feedback
pub use tooltip::{Tooltip, TooltipPlacement, WithTooltip};

// Layout
pub use pane_divider::{CollapseDirection, PaneDivider, PaneDividerTheme};
pub use stack::{
    Divider, HStack, Spacer, StackAlign, StackJustify, StackOverflow, StackSize, StackSpacing,
    VStack,
};

// Application templates
pub use app::{MiniApp, MiniAppConfig};

// Animation
pub use animation::{
    Animation, Easing, Keyframe, KeyframeAnimation, Spring, ease, evaluate_keyframes, interpolate,
    interpolate_color,
};

// Theme and i18n
pub use color_tokens::{
    BackgroundColors, BorderColors, ColorPalette, ColorToken, SemanticColors, TextColors, darken,
    desaturate, lighten, saturate, with_alpha,
};
pub use i18n::{I18nExt, I18nState, Language, TranslationKey, Translations};
pub use theme::{Theme, ThemeExt, ThemeState, ThemeVariant};

// Workflow canvas
pub use workflow::{
    CanvasState, Command, Connection, ConnectionId, HistoryManager, HitTestResult, HitTester,
    NodeContent, NodeId, Port, PortDirection, Position, SelectionState, ViewportState,
    WorkflowCanvas, WorkflowGraph, WorkflowNode, WorkflowNodeData, WorkflowTheme,
};

// Shared size definitions
pub use size::ComponentSize;

// Derive macros for theme generation
pub use gpui_ui_kit_macros::ComponentTheme;
