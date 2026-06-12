use super::{NavigationAction, NavigationMapping};

pub static DEFAULT_NAVIGATION: &[NavigationMapping] = &[
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
        primary: "home",
        alt: Some("ctrl-home"),
        display: "Home",
    },
    NavigationMapping {
        action: NavigationAction::Last,
        primary: "end",
        alt: Some("ctrl-end"),
        display: "End",
    },
    NavigationMapping {
        action: NavigationAction::Enter,
        primary: "enter",
        alt: None,
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
        primary: "right",
        alt: None,
        display: "→",
    },
    NavigationMapping {
        action: NavigationAction::Collapse,
        primary: "left",
        alt: None,
        display: "←",
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
        primary: "secondary-f",
        alt: Some("/"),
        display: "Ctrl+F",
    },
    NavigationMapping {
        action: NavigationAction::Delete,
        primary: "delete",
        alt: Some("backspace"),
        display: "Del",
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
        alt: None,
        display: "Ctrl+Shift+Z",
    },
];
