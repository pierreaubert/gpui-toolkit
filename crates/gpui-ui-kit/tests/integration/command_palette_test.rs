//! Integration tests for CommandPalette component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::command_palette::{CommandItem, CommandPalette};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct CommandPaletteTestView;

impl Render for CommandPaletteTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(CommandPalette::new(
            "cp-1",
            vec![
                CommandItem::new("open", "Open File").shortcut("Cmd+O"),
                CommandItem::new("save", "Save").shortcut("Cmd+S"),
            ],
        ))
    }
}

#[gpui::test]
async fn test_command_palette_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| CommandPaletteTestView);
}

// ============================================================================
// Query and Filter Tests
// ============================================================================

#[gpui::test]
async fn test_command_palette_with_query(cx: &mut TestAppContext) {
    struct QueryView;

    impl Render for QueryView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                CommandPalette::new(
                    "cp-query",
                    vec![
                        CommandItem::new("open", "Open File"),
                        CommandItem::new("save", "Save"),
                        CommandItem::new("settings", "Open Settings"),
                    ],
                )
                .query("open"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| QueryView);
}

// ============================================================================
// Category and Shortcut Tests
// ============================================================================

#[gpui::test]
async fn test_command_palette_categories(cx: &mut TestAppContext) {
    struct CategoriesView;

    impl Render for CategoriesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                CommandPalette::new(
                    "cp-cats",
                    vec![
                        CommandItem::new("open", "Open File")
                            .category("File")
                            .shortcut("Cmd+O"),
                        CommandItem::new("save", "Save")
                            .category("File")
                            .shortcut("Cmd+S"),
                        CommandItem::new("settings", "Open Settings")
                            .category("Preferences")
                            .icon("⚙"),
                    ],
                )
                .max_visible(5),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CategoriesView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_command_palette_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                CommandPalette::new(
                    "cp-full",
                    vec![
                        CommandItem::new("open", "Open File")
                            .shortcut("Cmd+O")
                            .category("File")
                            .icon("📂"),
                        CommandItem::new("save", "Save")
                            .shortcut("Cmd+S")
                            .category("File"),
                        CommandItem::new("redo", "Redo")
                            .shortcut("Cmd+Shift+Z")
                            .disabled(true),
                    ],
                )
                .placeholder("What do you need?")
                .query("")
                .selected_index(0)
                .max_visible(8)
                .on_select(|_id, _window, _cx| {})
                .on_dismiss(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
