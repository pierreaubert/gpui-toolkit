use super::{NavigationAction, NavigationMapping};

pub static EMACS_NAVIGATION: &[NavigationMapping] = &[
    NavigationMapping {
        action: NavigationAction::Up,
        primary: "ctrl-p",
        alt: Some("up"),
        display: "Ctrl+P / ↑",
    },
    NavigationMapping {
        action: NavigationAction::Down,
        primary: "ctrl-n",
        alt: Some("down"),
        display: "Ctrl+N / ↓",
    },
    NavigationMapping {
        action: NavigationAction::Left,
        primary: "ctrl-b",
        alt: Some("left"),
        display: "Ctrl+B / ←",
    },
    NavigationMapping {
        action: NavigationAction::Right,
        primary: "ctrl-f",
        alt: Some("right"),
        display: "Ctrl+F / →",
    },
    NavigationMapping {
        action: NavigationAction::PageUp,
        primary: "alt-v",
        alt: Some("pageup"),
        display: "Alt+V",
    },
    NavigationMapping {
        action: NavigationAction::PageDown,
        primary: "ctrl-v",
        alt: Some("pagedown"),
        display: "Ctrl+V",
    },
    NavigationMapping {
        action: NavigationAction::First,
        primary: "alt-<",
        alt: None,
        display: "Alt+<",
    },
    NavigationMapping {
        action: NavigationAction::Last,
        primary: "alt->",
        alt: None,
        display: "Alt+>",
    },
    NavigationMapping {
        action: NavigationAction::Enter,
        primary: "enter",
        alt: Some("ctrl-m"),
        display: "Enter / Ctrl+M",
    },
    NavigationMapping {
        action: NavigationAction::Cancel,
        primary: "ctrl-g",
        alt: Some("escape"),
        display: "Ctrl+G / Esc",
    },
    NavigationMapping {
        action: NavigationAction::Expand,
        primary: "ctrl-f",
        alt: Some("right"),
        display: "Ctrl+F",
    },
    NavigationMapping {
        action: NavigationAction::Collapse,
        primary: "ctrl-b",
        alt: Some("left"),
        display: "Ctrl+B",
    },
    NavigationMapping {
        action: NavigationAction::NextTab,
        primary: "ctrl-x right",
        alt: None,
        display: "Ctrl+X →",
    },
    NavigationMapping {
        action: NavigationAction::PrevTab,
        primary: "ctrl-x left",
        alt: None,
        display: "Ctrl+X ←",
    },
    NavigationMapping {
        action: NavigationAction::Search,
        primary: "ctrl-s",
        alt: Some("/"),
        display: "Ctrl+S / /",
    },
    NavigationMapping {
        action: NavigationAction::Delete,
        primary: "ctrl-d",
        alt: Some("ctrl-k"),
        display: "Ctrl+D / Ctrl+K",
    },
    NavigationMapping {
        action: NavigationAction::Undo,
        primary: "ctrl-/",
        alt: Some("ctrl-x u"),
        display: "Ctrl+/ / Ctrl+X U",
    },
    NavigationMapping {
        action: NavigationAction::Redo,
        primary: "ctrl-shift-/",
        alt: None,
        display: "Ctrl+Shift+/",
    },
];
