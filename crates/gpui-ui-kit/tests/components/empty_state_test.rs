//! EmptyState component tests

use gpui::div;
use gpui::prelude::ParentElement;
use gpui_ui_kit::empty_state::EmptyState;

#[test]
fn test_empty_state_creation() {
    let state = EmptyState::new("No albums found");
    let _ = state;
}

#[test]
fn test_empty_state_description() {
    let state = EmptyState::new("No results").description("Try adjusting your search filters");
    let _ = state;
}

#[test]
fn test_empty_state_icon() {
    let state = EmptyState::new("Empty library").icon("library");
    let _ = state;
}

#[test]
fn test_empty_state_action() {
    let state = EmptyState::new("No albums found").action(div().child("Add Album"));
    let _ = state;
}

#[test]
fn test_empty_state_full_configuration() {
    let state = EmptyState::new("No albums found")
        .description("Try adjusting your search filters")
        .icon("search")
        .action(div().child("Clear Filters"));
    let _ = state;
}
