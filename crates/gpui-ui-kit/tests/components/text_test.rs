//! Text/Typography component tests

use gpui::rems;
use gpui_ui_kit::text::{Code, Heading, Link, Text, TextSize, TextWeight, code_text_color};
use gpui_ui_kit::theme::Theme;

#[test]
fn test_text_size_to_rems_matches_gpui_tailwind_scale() {
    // These values must match GPUI's `text_xs`/`text_sm`/`text_base`/...
    // exactly so font-zoom (via `window.set_rem_size`) scales Text uniformly
    // with everything else. `Md` must resolve to `text_base` (1.0 rem); a prior
    // bug aliased it to `text_sm` (0.875 rem), making default-sized text one
    // step smaller than intended. If you change these values, audit every
    // caller — they're load-bearing for cross-component consistency.
    assert_eq!(TextSize::Xs.to_rems(), rems(0.75));
    assert_eq!(TextSize::Sm.to_rems(), rems(0.875));
    assert_eq!(TextSize::Md.to_rems(), rems(1.0));
    assert_eq!(TextSize::Lg.to_rems(), rems(1.125));
    assert_eq!(TextSize::Xl.to_rems(), rems(1.25));
    assert_eq!(TextSize::Xxl.to_rems(), rems(1.5));
}

#[test]
fn test_text_size_default_is_medium() {
    // Default-sized text should be `Md` → `rems(1.0)` — the base, not one step
    // smaller. This pairs with the mapping test above.
    assert_eq!(TextSize::default(), TextSize::Md);
    assert_eq!(TextSize::default().to_rems(), rems(1.0));
}

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

#[test]
fn test_code_uses_theme_code_text_color() {
    let mut theme = Theme::dark();
    theme.code_text = gpui::rgb(0xaabbcc);
    assert_eq!(
        code_text_color(&theme),
        theme.code_text,
        "Code text color should come from theme.code_text, not be hardcoded"
    );
}

// ----------------------------------------------------------------------------
// Semantic `Text::*` constructor state tests
//
// Pin the role → size/weight/muted mapping so the typography conventions
// documented in app-gpui/CLAUDE.md can't drift silently. If one of these
// values changes, every caller migrating to the semantic constructor will
// shift with it — audit usage before updating the expectations here.
// ----------------------------------------------------------------------------

#[test]
fn test_text_eyebrow_is_xs_bold() {
    let t = Text::eyebrow("RECORDING NAME");
    assert_eq!(t.preset_style(), (TextSize::Xs, TextWeight::Bold, false));
}

#[test]
fn test_text_section_header_is_md_semibold() {
    let t = Text::section_header("Playback");
    assert_eq!(
        t.preset_style(),
        (TextSize::Md, TextWeight::Semibold, false)
    );
}

#[test]
fn test_text_body_is_md_normal() {
    let t = Text::body("Description paragraph.");
    assert_eq!(t.preset_style(), (TextSize::Md, TextWeight::Normal, false));
}

#[test]
fn test_text_label_is_sm_medium() {
    let t = Text::label("Speaker");
    assert_eq!(t.preset_style(), (TextSize::Sm, TextWeight::Medium, false));
}

#[test]
fn test_text_caption_is_xs_normal_muted() {
    let t = Text::caption("seconds");
    assert_eq!(t.preset_style(), (TextSize::Xs, TextWeight::Normal, true));
}

#[test]
fn test_text_selectable_when_selected_is_xs_semibold() {
    let t = Text::selectable("Near-field", true);
    assert_eq!(
        t.preset_style(),
        (TextSize::Xs, TextWeight::Semibold, false)
    );
}

#[test]
fn test_text_selectable_when_not_selected_is_xs_normal() {
    let t = Text::selectable("Mid-field", false);
    assert_eq!(t.preset_style(), (TextSize::Xs, TextWeight::Normal, false));
}
