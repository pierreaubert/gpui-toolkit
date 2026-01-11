//! Text/Typography component tests

use gpui_ui_kit::text::{Code, Heading, Link, Text, TextSize, TextWeight};
use gpui_ui_kit::theme::Theme;

#[test]
fn test_text_styling() {
    let text = Text::new("Hello World")
        .size(TextSize::Xl)
        .weight(TextWeight::Bold)
        .color(gpui::rgb(0x000000))
        .muted(true)
        .truncate(true)
        .with_theme(Theme::light());

    drop(text);
}

#[test]
fn test_heading_levels() {
    let h1 = Heading::h1("Title");
    let h2 = Heading::h2("Subtitle");
    let h3 = Heading::h3("Section");
    let h4 = Heading::h4("Subsection");
    let custom = Heading::new("Custom").level(5);

    drop(h1);
    drop(h2);
    drop(h3);
    drop(h4);
    drop(custom);
}

#[test]
fn test_code_blocks() {
    let inline = Code::new("let x = 1;");
    let block = Code::block("fn main() {\n  println!(\"Hello\");\n}");

    drop(inline);
    drop(block);
}

#[test]
fn test_link_component() {
    let link = Link::new("link-id", "Click here")
        .href("https://example.com")
        .external(true)
        .on_click(|_window, _cx| {});

    drop(link);
}
