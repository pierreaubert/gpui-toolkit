use super::{NavigationAction, NavigationMapping};

pub static VIM_NAVIGATION: &[NavigationMapping] = &[
    NavigationMapping {
        action: NavigationAction::Up,
        primary: "k",
        alt: Some("up"),
        display: "k / ↑",
    },
    NavigationMapping {
        action: NavigationAction::Down,
        primary: "j",
        alt: Some("down"),
        display: "j / ↓",
    },
    NavigationMapping {
        action: NavigationAction::Left,
        primary: "h",
        alt: Some("left"),
        display: "h / ←",
    },
    NavigationMapping {
        action: NavigationAction::Right,
        primary: "l",
        alt: Some("right"),
        display: "l / →",
    },
    NavigationMapping {
        action: NavigationAction::PageUp,
        primary: "ctrl-u",
        alt: Some("pageup"),
        display: "Ctrl+U",
    },
    NavigationMapping {
        action: NavigationAction::PageDown,
        primary: "ctrl-d",
        alt: Some("pagedown"),
        display: "Ctrl+D",
    },
    NavigationMapping {
        action: NavigationAction::First,
        primary: "g g",
        alt: None,
        display: "gg",
    },
    NavigationMapping {
        action: NavigationAction::Last,
        primary: "G",
        alt: None,
        display: "G",
    },
    NavigationMapping {
        action: NavigationAction::Enter,
        primary: "enter",
        alt: Some("o"),
        display: "Enter / o",
    },
    NavigationMapping {
        action: NavigationAction::Cancel,
        primary: "escape",
        alt: None,
        display: "Esc",
    },
    NavigationMapping {
        action: NavigationAction::Expand,
        primary: "l",
        alt: Some("z o"),
        display: "l / zo",
    },
    NavigationMapping {
        action: NavigationAction::Collapse,
        primary: "h",
        alt: Some("z c"),
        display: "h / zc",
    },
    NavigationMapping {
        action: NavigationAction::NextTab,
        primary: "g t",
        alt: None,
        display: "gt",
    },
    NavigationMapping {
        action: NavigationAction::PrevTab,
        primary: "g T",
        alt: None,
        display: "gT",
    },
    NavigationMapping {
        action: NavigationAction::Search,
        primary: "/",
        alt: None,
        display: "/",
    },
    NavigationMapping {
        action: NavigationAction::Delete,
        primary: "d d",
        alt: Some("x"),
        display: "dd / x",
    },
    NavigationMapping {
        action: NavigationAction::Undo,
        primary: "u",
        alt: None,
        display: "u",
    },
    NavigationMapping {
        action: NavigationAction::Redo,
        primary: "ctrl-r",
        alt: None,
        display: "Ctrl+R",
    },
];
