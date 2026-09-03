//! Interaction tests for the showcase shell: keyboard editing paths
//! (`Showcase::handle_key_down`) and per-section rendering
//! (`render_section_content` via the window).

use gpui::{KeyDownEvent, TestAppContext, VisualTestContext};
use gpui_design::DesignSystemState;
use gpui_showcase::showcase::{Showcase, ShowcaseSection};
use gpui_ui_kit::{
    accessibility::AccessibilityTree,
    theme::{ThemeState, ThemeVariant},
};

fn key_event(key: &str) -> KeyDownEvent {
    KeyDownEvent {
        keystroke: gpui::Keystroke::parse(key).expect("valid test keystroke"),
        is_held: false,
        prefer_character_input: false,
    }
}

fn test_window(cx: &mut TestAppContext) -> gpui::WindowHandle<Showcase> {
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Light));
        app.set_global(DesignSystemState::new());
        app.set_global(AccessibilityTree::new());
    });
    cx.add_window(|_window, entity_cx| Showcase::new(entity_cx))
}

#[gpui::test]
async fn text_input_typing_backspace_and_commit(cx: &mut TestAppContext) {
    let window = test_window(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();

    window
        .update(&mut visual, |showcase, _window, _cx| {
            showcase.input_editing = true;
        })
        .expect("enter text input editing");

    window
        .update(&mut visual, |showcase, window, cx| {
            showcase.handle_key_down(&key_event("a"), window, cx);
            assert_eq!(showcase.input_edit_text, "a");
            showcase.handle_key_down(&key_event("b"), window, cx);
            assert_eq!(showcase.input_edit_text, "ab");
            showcase.handle_key_down(&key_event("backspace"), window, cx);
            assert_eq!(showcase.input_edit_text, "a");
            showcase.handle_key_down(&key_event("enter"), window, cx);
            assert_eq!(showcase.input_value, "a");
            assert!(!showcase.input_editing);
        })
        .expect("typing, backspace, and commit work");

    window
        .update(&mut visual, |showcase, window, cx| {
            showcase.input_editing = true;
            showcase.input_edit_text = String::from("stale");
            showcase.handle_key_down(&key_event("escape"), window, cx);
            assert!(!showcase.input_editing);
            assert!(showcase.input_edit_text.is_empty());
        })
        .expect("escape discards the pending edit");
}

#[gpui::test]
async fn number_input_accepts_only_numeric_characters(cx: &mut TestAppContext) {
    let window = test_window(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();

    window
        .update(&mut visual, |showcase, window, cx| {
            showcase.editing_number = Some("basic");
            showcase.handle_key_down(&key_event("5"), window, cx);
            assert_eq!(showcase.edit_text, "5");
            showcase.handle_key_down(&key_event("x"), window, cx);
            assert_eq!(
                showcase.edit_text, "5",
                "non-numeric keys are ignored while editing a number"
            );
            showcase.handle_key_down(&key_event("backspace"), window, cx);
            assert!(showcase.edit_text.is_empty());
            showcase.handle_key_down(&key_event("escape"), window, cx);
            assert_eq!(showcase.editing_number, None);
        })
        .expect("number editing filters input");
}

#[gpui::test]
async fn every_section_renders_without_panic(cx: &mut TestAppContext) {
    let window = test_window(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();

    // NOTE: QrCode is excluded from the parked render loop on purpose.
    // Rendering it spawns AnimatedQrCode's real 33ms smol timer, which the
    // deterministic `#[gpui::test]` scheduler rejects. Every other section
    // renders here.
    for section in ShowcaseSection::all()
        .iter()
        .copied()
        .filter(|section| *section != ShowcaseSection::QrCode)
    {
        window
            .update(&mut visual, |showcase, _window, cx| {
                showcase.current_section = section;
                cx.notify();
            })
            .expect("switch showcase section");
        visual.run_until_parked();
    }

    // Only the active section is built: the form's radio group is present
    // while FormControls is selected and gone after leaving it.
    window
        .update(&mut visual, |showcase, _window, cx| {
            showcase.current_section = ShowcaseSection::FormControls;
            cx.notify();
        })
        .expect("switch to form section");
    visual.run_until_parked();
    assert!(
        visual.debug_bounds("Name(\"rg-demo\")").is_some(),
        "form section should render its radio group"
    );
    window
        .update(&mut visual, |showcase, _window, cx| {
            showcase.current_section = ShowcaseSection::Buttons;
            cx.notify();
        })
        .expect("switch to buttons section");
    visual.run_until_parked();
    assert!(
        visual.debug_bounds("Name(\"rg-demo\")").is_none(),
        "leaving the form section should drop its content"
    );

    // QR entities are created lazily: a fresh showcase holds none, so
    // AnimatedQrCode's 33ms timer only exists while that section is live.
    window
        .update(&mut visual, |showcase, _window, _cx| {
            assert!(
                showcase.animated_qr_tiny.is_none() && showcase.animated_qr_small.is_none(),
                "animated QR entities should start out uninitialized"
            );
        })
        .expect("QR entities start uninitialized");
}
