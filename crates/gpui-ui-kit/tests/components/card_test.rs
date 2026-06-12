//! Card component tests

use gpui::div;
use gpui::prelude::{IntoElement, ParentElement, Styled};
use gpui_ui_kit::card::Card;

#[test]
fn test_card_composition() {
    let card = Card::new()
        .header(div().child("Header"))
        .content(div().child("Content"))
        .footer(div().child("Footer"))
        .style(|div| div.p_4())
        .background(gpui::rgb(0xFF0000))
        .border(gpui::rgb(0x00FF00));

    drop(card);
}

#[test]
fn test_card_header_with_factory() {
    let card = Card::new().header_with(|_theme| div().child("Themed Header").into_any_element());
    drop(card);
}

#[test]
fn test_card_content_with_factory() {
    let card = Card::new().content_with(|_theme| div().child("Themed Content").into_any_element());
    drop(card);
}

#[test]
fn test_card_footer_with_factory() {
    let card = Card::new().footer_with(|_theme| div().child("Themed Footer").into_any_element());
    drop(card);
}

#[test]
fn test_card_header_background() {
    let card = Card::new()
        .header(div().child("Header"))
        .header_background(gpui::rgb(0x333333));
    drop(card);
}

#[test]
fn test_empty_card() {
    let card = Card::new();
    drop(card);
}
