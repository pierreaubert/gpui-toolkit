use dwrote::FontStyle as DWriteFontStyle;
use crate :: properties :: { Style } ;

pub(super) const ERROR_BOUND: f32 = 0.0001;

pub(super) const OPENTYPE_TABLE_TAG_HEAD: u32 = 0x68656164;

pub(super) fn convert_len_utf16_to_utf8(text: &str, len_utf16: usize) -> usize {
    let mut l_utf8 = 0;
    let mut l_utf16 = 0;
    let mut chars = text.chars();
    while l_utf16 < len_utf16 {
        if let Some(c) = chars.next() {
            l_utf8 += c.len_utf8();
            l_utf16 += c.len_utf16();
        } else {
            break;
        }
    }
    l_utf8
}

pub(super) fn style_for_dwrite_style(style: DWriteFontStyle) -> Style {
    match style {
        DWriteFontStyle::Normal => Style::Normal,
        DWriteFontStyle::Oblique => Style::Oblique,
        DWriteFontStyle::Italic => Style::Italic,
    }
}

