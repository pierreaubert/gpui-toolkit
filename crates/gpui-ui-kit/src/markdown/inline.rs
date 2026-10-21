//! Native inline text shaping and keyboard-accessible Markdown links.

// Rust guideline compliant 2026-02-21
use super::MarkdownTheme;
use crate::accessibility::{AriaProps, AriaRole, apply_native_accessibility};
use gpui::prelude::*;
use gpui::{
    App, ElementId, FontStyle, FontWeight, HighlightStyle, InteractiveText, SharedString,
    StyledText, UnderlineStyle, Window, div, px,
};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::ops::Range;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Style {
    bold: bool,
    italic: bool,
    code: bool,
    strike: bool,
    link: Option<String>,
}

#[derive(Default, Debug)]
struct Inline {
    text: String,
    spans: Vec<(Range<usize>, Style)>,
}

impl Inline {
    fn append(&mut self, text: &str, style: &Style) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        if let Some((range, previous)) = self.spans.last_mut()
            && previous == style
        {
            range.end = self.text.len();
        } else {
            self.spans.push((start..self.text.len(), style.clone()));
        }
    }

    fn parse(source: &str) -> Self {
        let mut result = Self::default();
        let mut style = Style::default();
        let mut stack = Vec::new();
        for event in Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH) {
            match event {
                Event::Start(tag) => {
                    stack.push(style.clone());
                    match tag {
                        Tag::Strong => style.bold = true,
                        Tag::Emphasis => style.italic = true,
                        Tag::Strikethrough => style.strike = true,
                        Tag::Link { dest_url, .. } => style.link = Some(dest_url.to_string()),
                        _ => {}
                    }
                }
                Event::End(_) => {
                    style = stack.pop().unwrap_or_default();
                }
                Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                    result.append(&text, &style);
                }
                Event::Code(text) => {
                    let mut code = style.clone();
                    code.code = true;
                    result.append(&text, &code);
                }
                Event::SoftBreak => result.append(" ", &style),
                Event::HardBreak => result.append("\n", &style),
                _ => {}
            }
        }
        result
    }
}

fn openable(url: &str) -> bool {
    // Agent output is untrusted; never dispatch arbitrary OS/application schemes.
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|prefix| url.starts_with(prefix))
        && !url.chars().any(char::is_control)
}

#[derive(IntoElement)]
pub(super) struct InlineMarkdown {
    id: ElementId,
    source: String,
    theme: MarkdownTheme,
}

impl InlineMarkdown {
    pub(super) fn new(id: ElementId, source: String, theme: MarkdownTheme) -> Self {
        Self { id, source, theme }
    }
}

impl RenderOnce for InlineMarkdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parsed = Inline::parse(&self.source);
        let mut highlights = Vec::new();
        let mut fonts = Vec::new();
        let mut ranges: Vec<Range<usize>> = Vec::new();
        let mut urls: Vec<String> = Vec::new();
        for (range, style) in parsed.spans {
            let mut highlight = HighlightStyle {
                font_weight: style.bold.then_some(FontWeight::BOLD),
                font_style: style.italic.then_some(FontStyle::Italic),
                ..Default::default()
            };
            if style.code {
                highlight.background_color = Some(self.theme.code_background.into());
                highlight.color = Some(self.theme.code_text.into());
                fonts.push((range.clone(), SharedString::from("monospace")));
            }
            if style.strike {
                highlight.strikethrough = Some(gpui::StrikethroughStyle {
                    thickness: px(1.0),
                    color: None,
                });
            }
            if let Some(url) = style.link.filter(|url| openable(url)) {
                highlight.underline = Some(UnderlineStyle {
                    thickness: px(1.0),
                    color: None,
                    wavy: false,
                });
                if urls.last() == Some(&url)
                    && ranges.last().is_some_and(|last| last.end == range.start)
                {
                    ranges.last_mut().expect("link range exists").end = range.end;
                } else {
                    ranges.push(range.clone());
                    urls.push(url);
                }
            }
            highlights.push((range, highlight));
        }
        let mut container = div().id(self.id.clone()).min_w_0();
        if urls.is_empty() {
            return container.child(
                StyledText::new(parsed.text)
                    .with_highlights(highlights)
                    .with_font_family_overrides(fonts),
            );
        }

        // One tab stop per paragraph. Arrows cycle links; Enter opens the selected
        // destination. State is window-owned and survives streaming redraws.
        let state = window.use_keyed_state((self.id.clone(), "links"), cx, |_, cx| {
            (cx.focus_handle(), 0usize)
        });
        let focus = state.read(cx).0.clone().tab_stop(true);
        state.update(cx, |state, _| state.1 = state.1.min(urls.len() - 1));
        let selected = state.read(cx).1;
        for (range, highlight) in &mut highlights {
            if range.start < ranges[selected].end
                && range.end > ranges[selected].start
                && let Some(underline) = &mut highlight.underline
            {
                underline.thickness = px(2.0);
            }
        }
        let label = format!(
            "{}: {}. Left and right choose a link; Enter opens it.",
            &parsed.text[ranges[selected].clone()],
            urls[selected]
        );
        let accessible_url = urls[selected].clone();
        container =
            apply_native_accessibility(container, label, &AriaProps::with_role(AriaRole::Link))
                .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                    cx.open_url(&accessible_url);
                });
        let text = StyledText::new(parsed.text)
            .with_highlights(highlights)
            .with_font_family_overrides(fonts);
        let keyboard_urls = urls.clone();
        container = container
            .track_focus_element(&focus)
            .on_key_down(move |event, window, cx| {
                match event.keystroke.key.as_str() {
                    "tab" => {
                        if event.keystroke.modifiers.shift {
                            window.focus_prev(cx);
                        } else {
                            window.focus_next(cx);
                        }
                    }
                    "left" | "up" => {
                        state.update(cx, |state, _| {
                            state.1 = (state.1 + keyboard_urls.len() - 1) % keyboard_urls.len();
                        });
                    }
                    "right" | "down" => {
                        state.update(cx, |state, _| state.1 = (state.1 + 1) % keyboard_urls.len());
                    }
                    "enter" => cx.open_url(&keyboard_urls[state.read(cx).1]),
                    _ => return,
                }
                cx.stop_propagation();
                window.refresh();
            });
        container.child(
            InteractiveText::new((self.id, "text"), text)
                .on_click(ranges, move |index, _, cx| cx.open_url(&urls[index])),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    async fn keyboard_links_cycle_without_opening_until_enter(cx: &mut gpui::TestAppContext) {
        struct Document;
        impl gpui::Render for Document {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                div().size_full().child(super::super::Markdown::new("link-test", "[First](https://example.com/first) and [**second**](https://example.com/second)"))
            }
        }
        let window = cx.add_window(|_, _| Document);
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        cx.update(Window::focus_next);
        cx.simulate_keystrokes("right");
        cx.run_until_parked();
        assert_eq!(cx.opened_url(), None);
        cx.simulate_keystrokes("enter");
        assert_eq!(
            cx.opened_url().as_deref(),
            Some("https://example.com/second")
        );
        cx.simulate_keystrokes("left enter");
        assert_eq!(
            cx.opened_url().as_deref(),
            Some("https://example.com/first")
        );
    }

    #[test]
    fn nested_emphasis_code_escapes_and_unicode_keep_correct_ranges() {
        let parsed = Inline::parse(r"Café **bold _both_** and `a_b` plus \*literal\*.");
        assert_eq!(parsed.text, "Café bold both and a_b plus *literal*.");
        assert!(
            parsed
                .spans
                .iter()
                .any(|(range, style)| &parsed.text[range.clone()] == "both"
                    && style.bold
                    && style.italic)
        );
        assert!(
            parsed
                .spans
                .iter()
                .any(|(range, style)| &parsed.text[range.clone()] == "a_b"
                    && style.code
                    && !style.italic)
        );
        assert!(
            parsed
                .spans
                .iter()
                .all(|(range, _)| parsed.text.is_char_boundary(range.start)
                    && parsed.text.is_char_boundary(range.end))
        );
    }

    #[test]
    fn links_and_incomplete_streamed_markup_preserve_visible_text() {
        let parsed = Inline::parse("[**guide**](https://example.com/a_(b)) and **unfinished");
        assert_eq!(parsed.text, "guide and **unfinished");
        assert!(parsed.spans[0].1.bold);
        assert_eq!(
            parsed.spans[0].1.link.as_deref(),
            Some("https://example.com/a_(b)")
        );
        assert!(!openable("javascript:alert(1)"));
        assert!(!openable("file:///tmp/file"));
        assert!(openable("https://example.com"));
    }
}
