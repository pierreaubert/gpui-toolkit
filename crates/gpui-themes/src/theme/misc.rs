pub use gpui_ui_kit::Color;

/// Current version for shareable community theme bundles.
pub const COMMUNITY_THEME_SCHEMA_VERSION: u32 = 1;

pub(super) fn normalize_theme_id(id: &str) -> String {
    id.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == '-' || c == '_' || c.is_ascii_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn slugify_theme_name(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "custom-theme".to_string()
    } else {
        slug
    }
}

pub(super) fn shift_lightness(color: Color, delta: f32) -> Color {
    let (h, s, l) = color.to_hsl();
    Color::from_hsl(h, s, (l + delta).clamp(0.0, 1.0))
}

pub(super) fn readable_text_color(background: Color) -> Color {
    let black = Color::from_hex(0x000000);
    let white = Color::from_hex(0xffffff);

    if contrast_ratio(black, background) >= contrast_ratio(white, background) {
        black
    } else {
        white
    }
}

pub(super) fn contrast_ratio(foreground: Color, background: Color) -> f32 {
    let foreground_luminance = relative_luminance(foreground);
    let background_luminance = relative_luminance(background);
    let lighter = foreground_luminance.max(background_luminance);
    let darker = foreground_luminance.min(background_luminance);
    (lighter + 0.05) / (darker + 0.05)
}

pub(super) fn relative_luminance(color: Color) -> f32 {
    fn channel(value: u8) -> f32 {
        let value = value as f32 / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}
