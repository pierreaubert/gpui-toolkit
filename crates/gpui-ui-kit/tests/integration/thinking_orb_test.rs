//! Integration tests for the ThinkingOrb component.
//!
//! Tests entity creation and rendering in actual GPUI windows:
//! - All 9 states render
//! - Builder configuration (speed, count_scale, paused, aria_label)
//! - Runtime updates via `entity.update` (set_count_scale, set_paused)

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, TestAppContext, Window, div,
    px,
};
use gpui_ui_kit::{OrbState, ThinkingOrb};

struct OrbTestView {
    orb: Entity<ThinkingOrb>,
}

impl Render for OrbTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(self.orb.clone())
    }
}

// ============================================================================
// Rendering — all 9 states
// ============================================================================

#[gpui::test]
async fn test_thinking_orb_all_states_render(cx: &mut TestAppContext) {
    for state in OrbState::ALL {
        let _window = cx.add_window(|_window, cx| OrbTestView {
            orb: cx.new(|cx| ThinkingOrb::new(state, px(96.0), cx)),
        });
    }
}

// ============================================================================
// Builder configuration
// ============================================================================

#[gpui::test]
async fn test_thinking_orb_builder_configuration(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, cx| OrbTestView {
        orb: cx.new(|cx| {
            ThinkingOrb::new(OrbState::Working, px(96.0), cx)
                .speed(2.0)
                .count_scale(1.5)
                .paused(true)
                .aria_label("Thinking hard")
        }),
    });
}

#[gpui::test]
async fn test_thinking_orb_builder_all_states(cx: &mut TestAppContext) {
    for state in OrbState::ALL {
        let _window = cx.add_window(|_window, cx| OrbTestView {
            orb: cx.new(|cx| {
                ThinkingOrb::new(state, px(96.0), cx)
                    .speed(0.5)
                    .count_scale(0.5)
                    .aria_label(state.label())
            }),
        });
    }
}

// ============================================================================
// Runtime updates
// ============================================================================

#[gpui::test]
async fn test_thinking_orb_runtime_updates(cx: &mut TestAppContext) {
    let window = cx.add_window(|_window, cx| OrbTestView {
        orb: cx.new(|cx| ThinkingOrb::new(OrbState::Working, px(96.0), cx)),
    });

    window
        .update(cx, |view, _window, cx| {
            view.orb.update(cx, |orb, cx| {
                orb.set_count_scale(2.0, cx);
                orb.set_paused(true, cx);
                orb.set_paused(false, cx);
                orb.set_count_scale(0.25, cx);
            });
        })
        .unwrap();
}

#[gpui::test]
async fn test_thinking_orb_frame_stats_available(cx: &mut TestAppContext) {
    let window = cx.add_window(|_window, cx| OrbTestView {
        orb: cx.new(|cx| ThinkingOrb::new(OrbState::Searching, px(96.0), cx)),
    });

    window
        .update(cx, |view, _window, cx| {
            let _stats = view.orb.read(cx).frame_stats();
        })
        .unwrap();
}
