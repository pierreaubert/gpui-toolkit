//! iOS text input handling — key code mapping for external keyboards.

use gpui::{KeyDownEvent, Keystroke, Modifiers, PlatformInput};
use std::borrow::Cow;
use std::ops::Range;

pub fn clamp_utf16_selection(text: &str, location: usize, length: usize) -> Range<usize> {
    let text_len = text.encode_utf16().count();
    let start = location.min(text_len);
    let end = start.saturating_add(length).min(text_len);
    start..end
}

#[cfg_attr(not(target_os = "tvos"), allow(dead_code))]
pub fn tvos_press_key(press_type: i64) -> Option<&'static str> {
    match press_type {
        0 => Some("up"),
        1 => Some("down"),
        2 => Some("left"),
        3 => Some("right"),
        5 => Some("escape"),
        6 => Some("space"),
        _ => None,
    }
}

pub fn key_code_to_string(code: u32) -> Cow<'static, str> {
    match code {
        0x04..=0x1D => {
            let letter = (b'a' + (code - 0x04) as u8) as char;
            Cow::Owned(letter.to_string())
        }
        0x1E..=0x26 => {
            let num = ((code - 0x1E + 1) % 10) as u8 + b'0';
            Cow::Owned((num as char).to_string())
        }
        0x27 => Cow::Borrowed("0"),
        0x28 => Cow::Borrowed("enter"),
        0x29 => Cow::Borrowed("escape"),
        0x2A => Cow::Borrowed("backspace"),
        0x2B => Cow::Borrowed("tab"),
        0x2C => Cow::Borrowed(" "),
        0x2D => Cow::Borrowed("-"),
        0x2E => Cow::Borrowed("="),
        0x2F => Cow::Borrowed("["),
        0x30 => Cow::Borrowed("]"),
        0x31 => Cow::Borrowed("\\"),
        0x33 => Cow::Borrowed(";"),
        0x34 => Cow::Borrowed("'"),
        0x35 => Cow::Borrowed("`"),
        0x36 => Cow::Borrowed(","),
        0x37 => Cow::Borrowed("."),
        0x38 => Cow::Borrowed("/"),
        0x4F => Cow::Borrowed("right"),
        0x50 => Cow::Borrowed("left"),
        0x51 => Cow::Borrowed("down"),
        0x52 => Cow::Borrowed("up"),
        0x3A => Cow::Borrowed("f1"),
        0x3B => Cow::Borrowed("f2"),
        0x3C => Cow::Borrowed("f3"),
        0x3D => Cow::Borrowed("f4"),
        0x3E => Cow::Borrowed("f5"),
        0x3F => Cow::Borrowed("f6"),
        0x40 => Cow::Borrowed("f7"),
        0x41 => Cow::Borrowed("f8"),
        0x42 => Cow::Borrowed("f9"),
        0x43 => Cow::Borrowed("f10"),
        0x44 => Cow::Borrowed("f11"),
        0x45 => Cow::Borrowed("f12"),
        0x49 => Cow::Borrowed("insert"),
        0x4A => Cow::Borrowed("home"),
        0x4B => Cow::Borrowed("pageup"),
        0x4C => Cow::Borrowed("delete"),
        0x4D => Cow::Borrowed("end"),
        0x4E => Cow::Borrowed("pagedown"),
        _ => Cow::Owned(format!("unknown-{:02x}", code)),
    }
}

pub fn modifier_flags_to_modifiers(flags: u32) -> Modifiers {
    const SHIFT: u32 = 1 << 17;
    const CONTROL: u32 = 1 << 18;
    const ALT: u32 = 1 << 19;
    const COMMAND: u32 = 1 << 20;

    Modifiers {
        control: flags & CONTROL != 0,
        alt: flags & ALT != 0,
        shift: flags & SHIFT != 0,
        platform: flags & COMMAND != 0,
        function: false,
    }
}

pub fn key_code_to_key_down(key_code: u32, modifier_flags: u32) -> PlatformInput {
    key_code_to_key_down_with_characters(key_code, modifier_flags, None)
}

pub fn key_code_to_key_down_with_characters(
    key_code: u32,
    modifier_flags: u32,
    characters: Option<String>,
) -> PlatformInput {
    let modifiers = modifier_flags_to_modifiers(modifier_flags);
    let key = key_code_to_string(key_code).into_owned();
    let key_char = characters
        .filter(|characters| !characters.is_empty() && key.len() == 1)
        .or_else(|| {
            if key.len() == 1 {
                Some(key.clone())
            } else {
                None
            }
        });
    let keystroke = Keystroke {
        modifiers,
        key: key.clone(),
        key_char,
    };
    PlatformInput::KeyDown(KeyDownEvent {
        keystroke,
        is_held: false,
        prefer_character_input: false,
    })
}

pub fn key_code_to_key_up(key_code: u32, modifier_flags: u32) -> PlatformInput {
    let modifiers = modifier_flags_to_modifiers(modifier_flags);
    let key = key_code_to_string(key_code).into_owned();
    let keystroke = Keystroke {
        modifiers,
        key: key.clone(),
        key_char: if key.len() == 1 { Some(key) } else { None },
    };
    PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_to_string_returns_static_str_for_named_keys() {
        let key = key_code_to_string(0x29);
        assert_eq!(key, "escape");
        assert!(
            matches!(key, Cow::Borrowed(_)),
            "static key names should be borrowed"
        );
    }

    #[test]
    fn key_code_to_string_allocates_for_unknown_code() {
        let key = key_code_to_string(0xFF);
        assert_eq!(key, "unknown-ff");
        assert!(matches!(key, Cow::Owned(_)), "unknown codes must allocate");
    }

    #[test]
    fn key_code_to_key_down_maps_letters() {
        let input = key_code_to_key_down(0x04, 0);
        if let PlatformInput::KeyDown(event) = input {
            assert_eq!(event.keystroke.key, "a");
            assert_eq!(event.keystroke.key_char, Some("a".to_string()));
        } else {
            panic!("expected KeyDown event");
        }
    }

    #[test]
    fn composition_selection_is_clamped_in_utf16_units() {
        assert_eq!(clamp_utf16_selection("a😀b", 1, 2), 1..3);
        assert_eq!(clamp_utf16_selection("a😀b", 99, 4), 4..4);
    }

    #[test]
    fn siri_remote_presses_map_to_focus_navigation_keys() {
        assert_eq!(tvos_press_key(0), Some("up"));
        assert_eq!(tvos_press_key(1), Some("down"));
        assert_eq!(tvos_press_key(2), Some("left"));
        assert_eq!(tvos_press_key(3), Some("right"));
        assert_eq!(tvos_press_key(4), None);
        assert_eq!(tvos_press_key(5), Some("escape"));
        assert_eq!(tvos_press_key(6), Some("space"));
    }
}
