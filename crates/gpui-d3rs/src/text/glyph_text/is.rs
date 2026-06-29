pub(super) fn heuristic_text_width(text: &str, font_size: f32, letter_spacing: f32) -> f32 {
    let mut width = 0.0;
    let mut visible_count = 0usize;
    for ch in text.chars() {
        if is_combining_mark(ch) {
            continue;
        }
        visible_count += 1;
        width += if ch.is_ascii() {
            0.0
        } else if is_wide_char(ch) || is_emoji_hint(ch) {
            font_size
        } else {
            font_size * 0.65
        };
    }
    if visible_count > 1 {
        width += letter_spacing * (visible_count - 1) as f32;
    }
    width
}

pub(super) fn is_combining_mark(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036f
            | 0x1ab0..=0x1aff
            | 0x1dc0..=0x1dff
            | 0x20d0..=0x20ff
            | 0xfe20..=0xfe2f
    )
}

pub(super) fn is_wide_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7af
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe19
            | 0xfe30..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
            | 0x1f300..=0x1faff
    )
}

pub(super) fn is_emoji_hint(ch: char) -> bool {
    matches!(ch as u32, 0x2600..=0x27bf | 0x1f000..=0x1faff)
}
