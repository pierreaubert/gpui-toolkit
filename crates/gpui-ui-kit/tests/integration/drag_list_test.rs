//! Integration tests for DragList component

use gpui::{Context, IntoElement, ParentElement, Render, TestAppContext, Window, div};
use gpui_ui_kit::drag_list::{DragItem, DragList, DragListOrientation};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct DragListTestView;

impl Render for DragListTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(DragList::new(
            "dl-1",
            vec![
                DragItem::new("eq", div().child("EQ")),
                DragItem::new("comp", div().child("Compressor")),
            ],
        ))
    }
}

#[gpui::test]
async fn test_drag_list_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| DragListTestView);
}

// ============================================================================
// Orientation Tests
// ============================================================================

#[gpui::test]
async fn test_drag_list_horizontal(cx: &mut TestAppContext) {
    struct HorizontalView;

    impl Render for HorizontalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                DragList::new(
                    "dl-horiz",
                    vec![
                        DragItem::new("a", div().child("A")),
                        DragItem::new("b", div().child("B")),
                    ],
                )
                .orientation(DragListOrientation::Horizontal),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HorizontalView);
}

// ============================================================================
// Handle Visibility Tests
// ============================================================================

#[gpui::test]
async fn test_drag_list_no_handles(cx: &mut TestAppContext) {
    struct NoHandlesView;

    impl Render for NoHandlesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                DragList::new("dl-no-handles", vec![DragItem::new("a", div().child("A"))])
                    .show_handles(false),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NoHandlesView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_drag_list_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                DragList::new(
                    "dl-full",
                    vec![
                        DragItem::new("eq", div().child("EQ")),
                        DragItem::new("comp", div().child("Compressor")),
                        DragItem::new("limiter", div().child("Limiter")),
                    ],
                )
                .orientation(DragListOrientation::Vertical)
                .show_handles(true)
                .gap(gpui::px(4.0))
                .on_reorder(|_from, _to, _window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
