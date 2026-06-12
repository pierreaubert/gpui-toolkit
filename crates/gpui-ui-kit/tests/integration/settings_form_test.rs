//! Integration tests for SettingsForm component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use gpui_ui_kit::settings_form::{SettingsForm, SettingsRow};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SettingsFormTestView;

impl Render for SettingsFormTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(SettingsForm::new("test-form"))
    }
}

#[gpui::test]
async fn test_settings_form_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SettingsFormTestView);
}

// ============================================================================
// Section Tests
// ============================================================================

#[gpui::test]
async fn test_settings_form_with_section(cx: &mut TestAppContext) {
    struct SectionView;

    impl Render for SectionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SettingsForm::new("section-form")
                    .section("Audio Settings")
                    .row(SettingsRow::new("Volume")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SectionView);
}

// ============================================================================
// Row Tests
// ============================================================================

#[gpui::test]
async fn test_settings_form_row_with_description(cx: &mut TestAppContext) {
    struct DescView;

    impl Render for DescView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SettingsForm::new("desc-form")
                    .row(SettingsRow::new("Volume").description("Master output volume level")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DescView);
}

#[gpui::test]
async fn test_settings_form_row_with_control(cx: &mut TestAppContext) {
    struct ControlView;

    impl Render for ControlView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SettingsForm::new("control-form")
                    .row(SettingsRow::new("Enable").control(div().child("toggle"))),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ControlView);
}

// ============================================================================
// Multiple Sections Tests
// ============================================================================

#[gpui::test]
async fn test_settings_form_multiple_sections(cx: &mut TestAppContext) {
    struct MultiSectionView;

    impl Render for MultiSectionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SettingsForm::new("multi-form")
                    .section("Playback")
                    .row(SettingsRow::new("Volume"))
                    .row(SettingsRow::new("Mute"))
                    .section("Display")
                    .row(SettingsRow::new("Theme")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| MultiSectionView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_settings_form_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SettingsForm::new("audio-settings")
                    .label_width(px(220.0))
                    .section("Playback")
                    .row(
                        SettingsRow::new("Volume")
                            .description("Master output volume")
                            .control(div().child("slider"))
                            .label_width(px(180.0)),
                    )
                    .row(
                        SettingsRow::new("Mute")
                            .description("Mute all output")
                            .control(div().child("toggle")),
                    )
                    .section("Output")
                    .row(
                        SettingsRow::new("Device")
                            .description("Select audio output device")
                            .control(div().child("select")),
                    ),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
