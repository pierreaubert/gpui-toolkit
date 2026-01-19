//! Card component tests

use gpui::div;
use gpui::prelude::*;
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
