//! Integration tests for Menu component
//!
//! Tests the Menu, MenuItem, and MenuBar components including:
//! - Basic rendering
//! - Menu items with shortcuts, icons
//! - Separators and checkboxes
//! - Disabled and danger items
//! - Selection callbacks
//! - MenuBar with multiple menus

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::menu::{Menu, MenuBar, MenuBarItem, MenuItem};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct MenuTestView;

impl Render for MenuTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Menu::new(
            "test-menu",
            vec![
                MenuItem::new("item1", "Menu Item 1"),
                MenuItem::new("item2", "Menu Item 2"),
            ],
        ))
    }
}

#[gpui::test]
async fn test_menu_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| MenuTestView);
}

// ============================================================================
// MenuItem Tests
// ============================================================================

#[gpui::test]
async fn test_menu_item_with_shortcut(cx: &mut TestAppContext) {
    struct ShortcutView;

    impl Render for ShortcutView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "shortcut-menu",
                vec![
                    MenuItem::new("copy", "Copy").with_shortcut("⌘C"),
                    MenuItem::new("paste", "Paste").with_shortcut("⌘V"),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| ShortcutView);
}

#[gpui::test]
async fn test_menu_item_with_icon(cx: &mut TestAppContext) {
    struct IconView;

    impl Render for IconView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "icon-menu",
                vec![
                    MenuItem::new("edit", "Edit").with_icon("✏️"),
                    MenuItem::new("delete", "Delete").with_icon("🗑️"),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| IconView);
}

#[gpui::test]
async fn test_menu_item_disabled(cx: &mut TestAppContext) {
    struct DisabledView;

    impl Render for DisabledView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "disabled-menu",
                vec![
                    MenuItem::new("enabled", "Enabled"),
                    MenuItem::new("disabled", "Disabled").disabled(true),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledView);
}

#[gpui::test]
async fn test_menu_separator(cx: &mut TestAppContext) {
    struct SeparatorView;

    impl Render for SeparatorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "separator-menu",
                vec![
                    MenuItem::new("item1", "Item 1"),
                    MenuItem::separator(),
                    MenuItem::new("item2", "Item 2"),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| SeparatorView);
}

#[gpui::test]
async fn test_menu_checkbox(cx: &mut TestAppContext) {
    struct CheckboxView;

    impl Render for CheckboxView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "checkbox-menu",
                vec![
                    MenuItem::checkbox("show-toolbar", "Show Toolbar", true),
                    MenuItem::checkbox("show-sidebar", "Show Sidebar", false),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| CheckboxView);
}

#[gpui::test]
async fn test_menu_danger_item(cx: &mut TestAppContext) {
    struct DangerView;

    impl Render for DangerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "danger-menu",
                vec![
                    MenuItem::new("normal", "Normal Action"),
                    MenuItem::separator(),
                    MenuItem::new("quit", "Quit").danger(),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| DangerView);
}

// ============================================================================
// Selection Callback Tests
// ============================================================================

struct SelectableMenuView {
    select_count: Arc<AtomicUsize>,
    last_selected: Rc<RefCell<Option<String>>>,
}

impl Render for SelectableMenuView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let select_count = self.select_count.clone();
        let last_selected = self.last_selected.clone();

        div().size_full().child(
            Menu::new(
                "selectable-menu",
                vec![
                    MenuItem::new("action1", "Action 1"),
                    MenuItem::new("action2", "Action 2"),
                ],
            )
            .on_select(move |id, _window, _cx| {
                select_count.fetch_add(1, Ordering::SeqCst);
                *last_selected.borrow_mut() = Some(id.to_string());
            }),
        )
    }
}

#[gpui::test]
async fn test_menu_selection_callback(cx: &mut TestAppContext) {
    let select_count = Arc::new(AtomicUsize::new(0));
    let last_selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let select_count_clone = select_count.clone();
    let last_selected_clone = last_selected.clone();

    let window = cx.add_window(move |_window, _cx| SelectableMenuView {
        select_count: select_count_clone,
        last_selected: last_selected_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click on menu-item-action1
    if let Some(bounds) = cx.debug_bounds("menu-item-action1") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            select_count.load(Ordering::SeqCst),
            1,
            "on_select should have been called"
        );
        assert_eq!(
            *last_selected.borrow(),
            Some("action1".to_string()),
            "Selected item should be action1"
        );
    }
}

// ============================================================================
// MenuBar Tests
// ============================================================================

#[gpui::test]
async fn test_menu_bar_renders(cx: &mut TestAppContext) {
    struct MenuBarView;

    impl Render for MenuBarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(MenuBar::new(vec![
                MenuBarItem::new("file", "File").with_items(vec![
                    MenuItem::new("new", "New").with_shortcut("⌘N"),
                    MenuItem::new("open", "Open").with_shortcut("⌘O"),
                    MenuItem::separator(),
                    MenuItem::new("save", "Save").with_shortcut("⌘S"),
                ]),
                MenuBarItem::new("edit", "Edit").with_items(vec![
                    MenuItem::new("cut", "Cut").with_shortcut("⌘X"),
                    MenuItem::new("copy", "Copy").with_shortcut("⌘C"),
                    MenuItem::new("paste", "Paste").with_shortcut("⌘V"),
                ]),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| MenuBarView);
}

#[gpui::test]
async fn test_menu_bar_with_active_menu(cx: &mut TestAppContext) {
    struct ActiveMenuView;

    impl Render for ActiveMenuView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                MenuBar::new(vec![
                    MenuBarItem::new("file", "File"),
                    MenuBarItem::new("edit", "Edit"),
                ])
                .active_menu(Some("file".into())),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ActiveMenuView);
}

// ============================================================================
// Complex Menu Tests
// ============================================================================

#[gpui::test]
async fn test_menu_complex(cx: &mut TestAppContext) {
    struct ComplexMenuView;

    impl Render for ComplexMenuView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "complex-menu",
                vec![
                    MenuItem::new("new", "New File")
                        .with_shortcut("⌘N")
                        .with_icon("📄"),
                    MenuItem::new("open", "Open File")
                        .with_shortcut("⌘O")
                        .with_icon("📂"),
                    MenuItem::separator(),
                    MenuItem::checkbox("autosave", "Auto Save", true),
                    MenuItem::separator(),
                    MenuItem::new("close", "Close")
                        .with_shortcut("⌘W")
                        .disabled(true),
                    MenuItem::separator(),
                    MenuItem::new("quit", "Quit").with_shortcut("⌘Q").danger(),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| ComplexMenuView);
}

// ============================================================================
// Min Width Tests
// ============================================================================

#[gpui::test]
async fn test_menu_min_width(cx: &mut TestAppContext) {
    struct MinWidthView;

    impl Render for MinWidthView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Menu::new("min-width-menu", vec![MenuItem::new("short", "S")])
                    .min_width(gpui::px(250.0)),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| MinWidthView);
}

// ============================================================================
// Submenu Tests
// ============================================================================

#[gpui::test]
async fn test_menu_item_with_children(cx: &mut TestAppContext) {
    struct SubmenuView;

    impl Render for SubmenuView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Menu::new(
                "submenu-menu",
                vec![MenuItem::new("view", "View").with_children(vec![
                    MenuItem::new("zoom-in", "Zoom In"),
                    MenuItem::new("zoom-out", "Zoom Out"),
                ])],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| SubmenuView);
}
