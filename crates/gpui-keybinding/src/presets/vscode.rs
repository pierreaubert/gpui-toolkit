use super::{NavigationAction, NavigationMapping};

pub static VSCODE_NAVIGATION: &[NavigationMapping] = &[
    NavigationMapping {
        action: NavigationAction::Up,
        primary: "up",
        alt: None,
        display: "↑",
    },
    NavigationMapping {
        action: NavigationAction::Down,
        primary: "down",
        alt: None,
        display: "↓",
    },
    NavigationMapping {
        action: NavigationAction::Left,
        primary: "left",
        alt: None,
        display: "←",
    },
    NavigationMapping {
        action: NavigationAction::Right,
        primary: "right",
        alt: None,
        display: "→",
    },
    NavigationMapping {
        action: NavigationAction::PageUp,
        primary: "pageup",
        alt: None,
        display: "PgUp",
    },
    NavigationMapping {
        action: NavigationAction::PageDown,
        primary: "pagedown",
        alt: None,
        display: "PgDn",
    },
    NavigationMapping {
        action: NavigationAction::First,
        primary: "ctrl-home",
        alt: Some("home"),
        display: "Ctrl+Home",
    },
    NavigationMapping {
        action: NavigationAction::Last,
        primary: "ctrl-end",
        alt: Some("end"),
        display: "Ctrl+End",
    },
    NavigationMapping {
        action: NavigationAction::Enter,
        primary: "enter",
        alt: Some("ctrl-enter"),
        display: "Enter",
    },
    NavigationMapping {
        action: NavigationAction::Cancel,
        primary: "escape",
        alt: None,
        display: "Esc",
    },
    NavigationMapping {
        action: NavigationAction::Expand,
        primary: "ctrl-shift-]",
        alt: Some("right"),
        display: "Ctrl+Shift+]",
    },
    NavigationMapping {
        action: NavigationAction::Collapse,
        primary: "ctrl-shift-[",
        alt: Some("left"),
        display: "Ctrl+Shift+[",
    },
    NavigationMapping {
        action: NavigationAction::NextTab,
        primary: "ctrl-tab",
        alt: None,
        display: "Ctrl+Tab",
    },
    NavigationMapping {
        action: NavigationAction::PrevTab,
        primary: "ctrl-shift-tab",
        alt: None,
        display: "Ctrl+Shift+Tab",
    },
    NavigationMapping {
        action: NavigationAction::Search,
        primary: "ctrl-f",
        alt: Some("secondary-f"),
        display: "Ctrl+F",
    },
    NavigationMapping {
        action: NavigationAction::Delete,
        primary: "delete",
        alt: Some("ctrl-shift-k"),
        display: "Del / Ctrl+Shift+K",
    },
    NavigationMapping {
        action: NavigationAction::Undo,
        primary: "secondary-z",
        alt: None,
        display: "Ctrl+Z",
    },
    NavigationMapping {
        action: NavigationAction::Redo,
        primary: "secondary-shift-z",
        alt: Some("secondary-y"),
        display: "Ctrl+Shift+Z",
    },
];
