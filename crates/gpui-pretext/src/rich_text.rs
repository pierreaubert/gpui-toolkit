//! Rich text, variable font, accessibility, and benchmark helpers.

use std::borrow::Cow;
use std::ops::Range;

/// One OpenType variable-font axis.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableFontAxis {
    pub tag: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

impl VariableFontAxis {
    pub fn new(tag: impl Into<String>, min: f32, default: f32, max: f32) -> Result<Self, String> {
        let axis = Self {
            tag: tag.into(),
            min,
            default,
            max,
        };
        axis.validate()?;
        Ok(axis)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.tag.len() != 4 || !self.tag.bytes().all(|byte| byte.is_ascii()) {
            return Err("variable font axis tag must be four ASCII bytes".to_string());
        }
        if !(self.min.is_finite() && self.default.is_finite() && self.max.is_finite()) {
            return Err("variable font axis values must be finite".to_string());
        }
        if self.min > self.default || self.default > self.max {
            return Err("variable font axis default must be inside min/max range".to_string());
        }
        Ok(())
    }
}

/// Concrete axis values for a font face.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FontVariationSettings {
    pub axes: Vec<(String, f32)>,
}

impl FontVariationSettings {
    pub fn set(mut self, tag: impl Into<String>, value: f32) -> Self {
        let tag = tag.into();
        if let Some((_, existing)) = self.axes.iter_mut().find(|(axis, _)| axis == &tag) {
            *existing = value;
        } else {
            self.axes.push((tag, value));
        }
        self
    }

    pub fn css_settings(&self) -> String {
        self.axes
            .iter()
            .map(|(tag, value)| format!("\"{tag}\" {value}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Inline style used by rich text spans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichTextStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub link: Option<String>,
}

/// A styled text span.
///
/// The text is stored as a `Cow<'_, str>` so that spans produced by
/// [`parse_inline_markdown`] can borrow directly from the input string instead
/// of allocating a new `String` for every span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichTextSpan<'a> {
    pub text: Cow<'a, str>,
    pub style: RichTextStyle,
}

impl<'a> RichTextSpan<'a> {
    pub fn plain(text: impl Into<Cow<'a, str>>) -> Self {
        Self {
            text: text.into(),
            style: RichTextStyle::default(),
        }
    }
}

/// Parse a small Markdown inline subset: `code`, **bold**, _italic_, and links.
pub fn parse_inline_markdown(input: &str) -> Vec<RichTextSpan<'_>> {
    let mut spans = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        if let Some(stripped) = remaining.strip_prefix("**")
            && let Some(end) = stripped.find("**")
        {
            spans.push(RichTextSpan {
                text: Cow::Borrowed(&stripped[..end]),
                style: RichTextStyle {
                    bold: true,
                    ..RichTextStyle::default()
                },
            });
            remaining = &stripped[end + 2..];
            continue;
        }

        if let Some(stripped) = remaining.strip_prefix('`')
            && let Some(end) = stripped.find('`')
        {
            spans.push(RichTextSpan {
                text: Cow::Borrowed(&stripped[..end]),
                style: RichTextStyle {
                    code: true,
                    ..RichTextStyle::default()
                },
            });
            remaining = &stripped[end + 1..];
            continue;
        }

        if let Some(stripped) = remaining.strip_prefix('_')
            && let Some(end) = stripped.find('_')
        {
            spans.push(RichTextSpan {
                text: Cow::Borrowed(&stripped[..end]),
                style: RichTextStyle {
                    italic: true,
                    ..RichTextStyle::default()
                },
            });
            remaining = &stripped[end + 1..];
            continue;
        }

        if let Some(stripped) = remaining.strip_prefix('[')
            && let Some(label_end) = stripped.find("](")
            && let Some(url_end) = stripped[label_end + 2..].find(')')
        {
            let url_start = label_end + 2;
            spans.push(RichTextSpan {
                text: Cow::Borrowed(&stripped[..label_end]),
                style: RichTextStyle {
                    link: Some(stripped[url_start..url_start + url_end].to_string()),
                    ..RichTextStyle::default()
                },
            });
            remaining = &stripped[url_start + url_end + 1..];
            continue;
        }

        let next_marker = remaining
            .char_indices()
            .skip(1)
            .find(|(_, ch)| matches!(ch, '*' | '`' | '_' | '['))
            .map(|(idx, _)| idx)
            .unwrap_or(remaining.len());
        spans.push(RichTextSpan::plain(&remaining[..next_marker]));
        remaining = &remaining[next_marker..];
    }

    coalesce_plain_spans(spans)
}

/// One platform-accessible run mapped back to UTF-8 byte ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleTextRun<'a> {
    pub byte_range: Range<usize>,
    pub label: Cow<'a, str>,
    pub role: AccessibleTextRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibleTextRole {
    Text,
    Link,
    Code,
}

pub fn accessibility_runs_for_spans<'a>(spans: &[RichTextSpan<'a>]) -> Vec<AccessibleTextRun<'a>> {
    let mut offset = 0;
    let mut runs = Vec::with_capacity(spans.len());
    for span in spans {
        let len = span.text.len();
        let role = if span.style.link.is_some() {
            AccessibleTextRole::Link
        } else if span.style.code {
            AccessibleTextRole::Code
        } else {
            AccessibleTextRole::Text
        };
        runs.push(AccessibleTextRun {
            byte_range: offset..offset + len,
            label: span.text.clone(),
            role,
        });
        offset += len;
    }
    runs
}

/// Benchmark case descriptor for layout throughput comparisons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBenchmarkCase {
    pub id: String,
    pub text: String,
    pub width_px: u32,
    pub expected_lines: Option<usize>,
}

impl TextBenchmarkCase {
    pub fn estimated_work_units(&self) -> usize {
        self.text
            .chars()
            .count()
            .saturating_mul(self.width_px as usize)
    }
}

fn coalesce_plain_spans<'a>(spans: Vec<RichTextSpan<'a>>) -> Vec<RichTextSpan<'a>> {
    let mut merged: Vec<RichTextSpan<'a>> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        if span.style == RichTextStyle::default()
            && let Some(last) = merged.last_mut()
            && last.style == RichTextStyle::default()
        {
            last.text.to_mut().push_str(&span.text);
            continue;
        }
        merged.push(span);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_font_axes_validate_tags_and_ranges() {
        assert!(VariableFontAxis::new("wght", 100.0, 400.0, 900.0).is_ok());
        assert!(VariableFontAxis::new("weight", 100.0, 400.0, 900.0).is_err());
        assert!(VariableFontAxis::new("wdth", 100.0, 50.0, 900.0).is_err());
    }

    #[test]
    fn inline_markdown_parser_marks_core_styles() {
        let spans = parse_inline_markdown("Play **loud** _now_ `ok` [site](https://x)");

        assert!(
            spans
                .iter()
                .any(|span| span.text == "loud" && span.style.bold)
        );
        assert!(
            spans
                .iter()
                .any(|span| span.text == "now" && span.style.italic)
        );
        assert!(
            spans
                .iter()
                .any(|span| span.text == "ok" && span.style.code)
        );
        assert!(
            spans
                .iter()
                .any(|span| span.text == "site" && span.style.link.is_some())
        );
    }

    #[test]
    fn inline_markdown_borrows_from_input_when_parsing() {
        let input = "plain **bold** _italic_".to_string();
        let spans = parse_inline_markdown(&input);

        // Markdown spans should borrow from the input string.
        assert!(matches!(spans[0].text, Cow::Borrowed(_)));
        assert!(matches!(spans[1].text, Cow::Borrowed(_)));
        assert!(matches!(spans[2].text, Cow::Borrowed(_)));
    }

    #[test]
    fn rich_text_span_plain_owns_string_input() {
        let span = RichTextSpan::plain("owned text".to_string());
        assert!(matches!(span.text, Cow::Owned(_)));
        assert_eq!(span.text, "owned text");
    }

    #[test]
    fn accessibility_runs_preserve_byte_offsets() {
        let spans = parse_inline_markdown("A [link](https://example.com)");
        let runs = accessibility_runs_for_spans(&spans);

        assert_eq!(runs[0].byte_range, 0..2);
        assert_eq!(runs[1].role, AccessibleTextRole::Link);
    }

    #[test]
    fn benchmark_case_estimates_work() {
        let case = TextBenchmarkCase {
            id: "latin".to_string(),
            text: "abc".to_string(),
            width_px: 200,
            expected_lines: Some(1),
        };
        assert_eq!(case.estimated_work_units(), 600);
    }
}
