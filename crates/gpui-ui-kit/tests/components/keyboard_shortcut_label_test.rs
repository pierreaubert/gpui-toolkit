//! KeyboardShortcutLabel component tests

use gpui_ui_kit::keyboard_shortcut_label::{KeyboardShortcutLabel, KeyboardShortcutSize};

#[test]
fn test_keyboard_shortcut_label_creation() {
    let label = KeyboardShortcutLabel::new("Cmd+K");
    let _ = label;
}

#[test]
fn test_keyboard_shortcut_label_all_sizes() {
    let sizes = [
        KeyboardShortcutSize::Sm,
        KeyboardShortcutSize::Md,
        KeyboardShortcutSize::Lg,
    ];

    for size in &sizes {
        let label = KeyboardShortcutLabel::new("Ctrl+Shift+P").size(*size);
        let _ = label;
    }
}

#[test]
fn test_keyboard_shortcut_label_custom_separator() {
    let label = KeyboardShortcutLabel::new("Ctrl-Shift-P").separator("-");
    let _ = label;
}

#[test]
fn test_keyboard_shortcut_label_single_key() {
    let label = KeyboardShortcutLabel::new("Esc");
    let _ = label;
}

#[test]
fn test_keyboard_shortcut_label_multi_key() {
    let label = KeyboardShortcutLabel::new("Ctrl+Alt+Delete");
    let _ = label;
}

#[test]
fn test_keyboard_shortcut_label_full_configuration() {
    let label = KeyboardShortcutLabel::new("Cmd+Shift+F")
        .size(KeyboardShortcutSize::Lg)
        .separator("+");
    let _ = label;
}
