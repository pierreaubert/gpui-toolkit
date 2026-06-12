//! SettingsForm component tests

use gpui::div;
use gpui::prelude::ParentElement;
use gpui_ui_kit::settings_form::{SettingsForm, SettingsRow};

#[test]
fn test_settings_form_creation() {
    let form = SettingsForm::new("test");
    drop(form);
}

#[test]
fn test_settings_form_section() {
    let form = SettingsForm::new("test").section("Audio");
    drop(form);
}

#[test]
fn test_settings_form_row() {
    let form = SettingsForm::new("test").row(SettingsRow::new("Volume"));
    drop(form);
}

#[test]
fn test_settings_form_row_with_description() {
    let form = SettingsForm::new("test")
        .row(SettingsRow::new("Volume").description("Master output volume"));
    drop(form);
}

#[test]
fn test_settings_form_row_with_control() {
    let form = SettingsForm::new("test")
        .row(SettingsRow::new("Enable").control(div().child("toggle placeholder")));
    drop(form);
}

#[test]
fn test_settings_form_label_width() {
    let form = SettingsForm::new("test").label_width(gpui::px(250.0));
    drop(form);
}

#[test]
fn test_settings_row_label_width() {
    let row = SettingsRow::new("Volume").label_width(gpui::px(180.0));
    drop(row);
}

#[test]
fn test_settings_form_multiple_sections() {
    let form = SettingsForm::new("test")
        .section("Playback")
        .row(SettingsRow::new("Volume"))
        .row(SettingsRow::new("Mute"))
        .section("Display")
        .row(SettingsRow::new("Theme"));
    drop(form);
}

#[test]
fn test_settings_form_full_configuration() {
    let form = SettingsForm::new("audio-settings")
        .label_width(gpui::px(220.0))
        .section("Playback")
        .row(
            SettingsRow::new("Volume")
                .description("Master output volume")
                .control(div().child("slider"))
                .label_width(gpui::px(180.0)),
        )
        .row(
            SettingsRow::new("Mute")
                .description("Mute all audio output")
                .control(div().child("toggle")),
        )
        .section("Output")
        .row(
            SettingsRow::new("Device")
                .description("Select audio output device")
                .control(div().child("select")),
        );
    drop(form);
}
