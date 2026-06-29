use crate::theme::Theme;
use gpui::Rgba;

/// Get the code text color for a given theme.
pub fn code_text_color(theme: &Theme) -> Rgba {
    theme.code_text
}
