//! `Markdown` component
//!
//! A small Markdown renderer for headings, paragraphs, fenced code
//! blocks, unordered list items, and quotes. The source is a
//! parent-owned string; parsing is deterministic and dependency-free so
//! previews render identically on every platform.
//!
//! # Usage
//!
//! ```ignore
//! Markdown::new("release-notes", "# Title\n\nShipped today.")
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, FontWeight, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// One parsed Markdown block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownBlock {
    /// `#` heading with level 1-6 and text.
    Heading {
        /// Heading level from the `#` count.
        level: u8,
        /// Heading text without markers.
        text: String,
    },
    /// Plain paragraph text.
    Paragraph(String),
    /// Fenced code with optional language and code.
    CodeBlock {
        /// Info string after the fence, if any.
        language: Option<String>,
        /// Code lines without fences.
        code: String,
    },
    /// Unordered `-` list item text.
    ListItem(String),
    /// `>` quote line text.
    Quote(String),
}

/// Parse Markdown source into blocks.
///
/// Blank lines separate paragraphs. A line starting with `#` becomes a
/// heading, with ```` ``` ```` fences toggling code blocks, with `- `
/// starting list items, and with `> ` starting quotes. Everything else
/// joins the current paragraph.
///
/// # Examples
///
/// ```ignore
/// let blocks = parse_markdown("# Title\n\nBody");
/// assert_eq!(blocks.len(), 2);
/// ```
pub fn parse_markdown(source: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = String::new();
    let mut code: Option<(Option<String>, Vec<String>)> = None;

    let flush_paragraph = |paragraph: &mut String, blocks: &mut Vec<MarkdownBlock>| {
        if !paragraph.is_empty() {
            blocks.push(MarkdownBlock::Paragraph(std::mem::take(paragraph)));
        }
    };

    for line in source.lines() {
        if code.is_some() {
            if line.trim_start().starts_with("```") {
                let (language, lines) = code.take().expect("code fence tracked above");
                flush_paragraph(&mut paragraph, &mut blocks);
                blocks.push(MarkdownBlock::CodeBlock {
                    language,
                    code: lines.join("\n"),
                });
            } else if let Some((_, lines)) = code.as_mut() {
                lines.push(line.to_string());
            }
            continue;
        }
        let trimmed = line.trim();
        if let Some(fence) = trimmed.strip_prefix("```") {
            flush_paragraph(&mut paragraph, &mut blocks);
            let language = fence.trim();
            code = Some((
                (!language.is_empty()).then(|| language.to_string()),
                Vec::new(),
            ));
        } else if trimmed.is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks);
        } else if let Some(heading) = parse_heading(trimmed) {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(heading);
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(MarkdownBlock::ListItem(item.trim().to_string()));
        } else if let Some(quote) = trimmed
            .strip_prefix('>')
            .map(|rest| rest.strip_prefix(' ').unwrap_or(rest))
        {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(MarkdownBlock::Quote(quote.trim().to_string()));
        } else if paragraph.is_empty() {
            paragraph.push_str(trimmed);
        } else {
            paragraph.push(' ');
            paragraph.push_str(trimmed);
        }
    }
    if let Some((language, lines)) = code.take() {
        flush_paragraph(&mut paragraph, &mut blocks);
        blocks.push(MarkdownBlock::CodeBlock {
            language,
            code: lines.join("\n"),
        });
    } else {
        flush_paragraph(&mut paragraph, &mut blocks);
    }
    blocks
}

/// Parse a `#` heading line into a block.
fn parse_heading(trimmed: &str) -> Option<MarkdownBlock> {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let text = trimmed[hashes..].strip_prefix(' ')?.trim();
    if text.is_empty() {
        return None;
    }
    Some(MarkdownBlock::Heading {
        level: hashes as u8,
        text: text.to_string(),
    })
}

/// Theme colors for rendered Markdown.
#[derive(Debug, Clone, ComponentTheme)]
pub struct MarkdownTheme {
    /// Heading color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub heading: Rgba,
    /// Paragraph and list color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub body: Rgba,
    /// Code block background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub code_background: Rgba,
    /// Code block text.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub code_text: Rgba,
    /// Quote bar and quote text.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub quote: Rgba,
}

/// Markdown renderer size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkdownSize {
    /// Compact prose.
    Sm,
    /// Standard prose (default).
    #[default]
    Md,
    /// Large prose.
    Lg,
}

impl From<crate::ComponentSize> for MarkdownSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Small Markdown source renderer.
#[derive(IntoElement)]
pub struct Markdown {
    id: ElementId,
    source: SharedString,
    size: MarkdownSize,
    theme: Option<MarkdownTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Markdown {
    /// Create a renderer for `source`.
    pub fn new(id: impl Into<ElementId>, source: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            size: MarkdownSize::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the Markdown source.
    pub fn source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = source.into();
        self
    }

    /// Set the renderer size.
    pub fn size(mut self, size: MarkdownSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: MarkdownTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Group).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Render one parsed block with theme and design tokens.
    fn render_block(block: MarkdownBlock, theme: &MarkdownTheme, design: &DesignSystem) -> Div {
        match block {
            MarkdownBlock::Heading { level, text } => {
                let mut heading = div()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.heading);
                heading = match level {
                    1 => heading.text_xl(),
                    2 => heading.text_lg(),
                    3 => heading.text_base(),
                    4 => heading.text_sm(),
                    _ => heading.text_xs(),
                };
                heading.child(text)
            }
            MarkdownBlock::Paragraph(text) => div().text_color(theme.body).child(text),
            MarkdownBlock::CodeBlock { code, .. } => div()
                .bg(theme.code_background)
                .rounded(px(design.corners.sm))
                .px(px(design.spacing.control_padding_x))
                .py(px(design.spacing.control_padding_y * 0.5))
                .text_color(theme.code_text)
                .child(code),
            MarkdownBlock::ListItem(text) => div()
                .flex()
                .gap(px(design.spacing.control_gap))
                .text_color(theme.body)
                .child(div().child("-"))
                .child(div().child(text)),
            MarkdownBlock::Quote(text) => div()
                .flex()
                .gap(px(design.spacing.control_gap))
                .child(
                    div()
                        .w(px(design.spacing.grid_unit * 0.375))
                        .rounded(px(design.corners.sm))
                        .bg(theme.quote),
                )
                .child(div().text_color(theme.quote).child(text)),
        }
    }

    /// Build the renderer with theme.
    pub fn build_with_theme(self, theme: &MarkdownTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the renderer with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &MarkdownTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut doc = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap));
        doc = match self.size {
            MarkdownSize::Sm => doc.text_xs(),
            MarkdownSize::Md => doc.text_sm(),
            MarkdownSize::Lg => doc.text_lg(),
        };
        for block in parse_markdown(&self.source) {
            doc = doc.child(Self::render_block(block, theme, design));
        }
        doc
    }
}

impl RenderOnce for Markdown {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| "Markdown content".into()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| MarkdownTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Markdown, MarkdownBlock, MarkdownSize, MarkdownTheme, parse_markdown};
    use gpui::rgba;

    #[test]
    fn markdown_parses_blocks_in_order() {
        let blocks = parse_markdown("# Title\n\nBody text\n\n- item\n\n> quote");
        assert_eq!(
            blocks,
            vec![
                MarkdownBlock::Heading {
                    level: 1,
                    text: "Title".to_string()
                },
                MarkdownBlock::Paragraph("Body text".to_string()),
                MarkdownBlock::ListItem("item".to_string()),
                MarkdownBlock::Quote("quote".to_string()),
            ]
        );
    }

    #[test]
    fn markdown_clamps_heading_levels_and_joins_paragraphs() {
        let blocks = parse_markdown("### Deep\nline one\nline two\n\n####### kept?");
        assert_eq!(
            blocks,
            vec![
                MarkdownBlock::Heading {
                    level: 3,
                    text: "Deep".to_string()
                },
                MarkdownBlock::Paragraph("line one line two".to_string()),
                MarkdownBlock::Paragraph("####### kept?".to_string()),
            ]
        );
    }

    #[test]
    fn markdown_parses_fenced_code_with_language() {
        let blocks = parse_markdown("```rust\nlet x = 1;\n```");
        assert_eq!(
            blocks,
            vec![MarkdownBlock::CodeBlock {
                language: Some("rust".to_string()),
                code: "let x = 1;".to_string(),
            }]
        );
    }

    #[test]
    fn markdown_builders_store_source_and_size() {
        let doc = Markdown::new("notes", "# Hi")
            .source("# Bye")
            .size(MarkdownSize::Lg);
        assert_eq!(doc.source.as_ref(), "# Bye");
        assert_eq!(doc.size, MarkdownSize::Lg);
    }

    #[test]
    fn markdown_theme_defaults_match_compiled_tokens() {
        let theme = MarkdownTheme::default();
        assert_eq!(theme.heading, rgba(0xffffffff));
        assert_eq!(theme.body, rgba(0xccccccff));
        assert_eq!(theme.code_background, rgba(0x1e1e1eff));
        assert_eq!(theme.code_text, rgba(0xccccccff));
        assert_eq!(theme.quote, rgba(0x777777ff));
    }
}
