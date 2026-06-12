//! CommandPalette component tests

use gpui_ui_kit::command_palette::{CommandItem, CommandPalette};

#[test]
fn test_command_palette_creation() {
    let palette = CommandPalette::new("cp-1", vec![CommandItem::new("open", "Open File")]);
    drop(palette);
}

#[test]
fn test_command_palette_placeholder() {
    let palette = CommandPalette::new("cp-ph", vec![]).placeholder("Search commands...");
    drop(palette);
}

#[test]
fn test_command_palette_query() {
    let palette =
        CommandPalette::new("cp-query", vec![CommandItem::new("open", "Open File")]).query("open");
    drop(palette);
}

#[test]
fn test_command_palette_selected_index() {
    let palette = CommandPalette::new(
        "cp-sel",
        vec![
            CommandItem::new("open", "Open File"),
            CommandItem::new("save", "Save"),
        ],
    )
    .selected_index(1);
    drop(palette);
}

#[test]
fn test_command_palette_max_visible() {
    let palette = CommandPalette::new("cp-max", vec![]).max_visible(5);
    drop(palette);
}

#[test]
fn test_command_item_with_shortcut() {
    let item = CommandItem::new("save", "Save").shortcut("Cmd+S");
    let palette = CommandPalette::new("cp-shortcut", vec![item]);
    drop(palette);
}

#[test]
fn test_command_item_with_category() {
    let item = CommandItem::new("settings", "Open Settings").category("Preferences");
    let palette = CommandPalette::new("cp-cat", vec![item]);
    drop(palette);
}

#[test]
fn test_command_item_with_icon() {
    let item = CommandItem::new("open", "Open File").icon("📂");
    let palette = CommandPalette::new("cp-icon", vec![item]);
    drop(palette);
}

#[test]
fn test_command_item_disabled() {
    let item = CommandItem::new("redo", "Redo").disabled(true);
    let palette = CommandPalette::new("cp-disabled", vec![item]);
    drop(palette);
}

#[test]
fn test_command_palette_on_select() {
    let palette = CommandPalette::new("cp-on-sel", vec![CommandItem::new("open", "Open")])
        .on_select(|_id, _window, _cx| {});
    drop(palette);
}

#[test]
fn test_command_palette_on_dismiss() {
    let palette = CommandPalette::new("cp-on-dismiss", vec![]).on_dismiss(|_window, _cx| {});
    drop(palette);
}

#[test]
fn test_command_palette_full_configuration() {
    let palette = CommandPalette::new(
        "cp-full",
        vec![
            CommandItem::new("open", "Open File")
                .shortcut("Cmd+O")
                .category("File")
                .icon("📂"),
            CommandItem::new("save", "Save")
                .shortcut("Cmd+S")
                .category("File"),
            CommandItem::new("settings", "Open Settings")
                .category("Preferences")
                .icon("⚙"),
            CommandItem::new("redo", "Redo")
                .shortcut("Cmd+Shift+Z")
                .disabled(true),
        ],
    )
    .placeholder("What do you need?")
    .query("open")
    .selected_index(0)
    .max_visible(8)
    .on_select(|_id, _window, _cx| {})
    .on_dismiss(|_window, _cx| {});
    drop(palette);
}
