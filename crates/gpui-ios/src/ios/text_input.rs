//! iOS text input handling — key code mapping for external keyboards.

use gpui::{KeyDownEvent, Keystroke, Modifiers, PlatformInput};

pub fn key_code_to_string(code: u32) -> String {
    match code {
        0x04..=0x1D => {
            let letter = (b'a' + (code - 0x04) as u8) as char;
            letter.to_string()
        }
        0x1E..=0x26 => {
            let num = ((code - 0x1E + 1) % 10) as u8 + b'0';
            (num as char).to_string()
        }
        0x27 => "0".to_string(),
        0x28 => "enter".to_string(),
        0x29 => "escape".to_string(),
        0x2A => "backspace".to_string(),
        0x2B => "tab".to_string(),
        0x2C => " ".to_string(),
        0x2D => "-".to_string(),
        0x2E => "=".to_string(),
        0x2F => "[".to_string(),
        0x30 => "]".to_string(),
        0x31 => "\\".to_string(),
        0x33 => ";".to_string(),
        0x34 => "'".to_string(),
        0x35 => "`".to_string(),
        0x36 => ",".to_string(),
        0x37 => ".".to_string(),
        0x38 => "/".to_string(),
        0x4F => "right".to_string(),
        0x50 => "left".to_string(),
        0x51 => "down".to_string(),
        0x52 => "up".to_string(),
        0x3A => "f1".to_string(),
        0x3B => "f2".to_string(),
        0x3C => "f3".to_string(),
        0x3D => "f4".to_string(),
        0x3E => "f5".to_string(),
        0x3F => "f6".to_string(),
        0x40 => "f7".to_string(),
        0x41 => "f8".to_string(),
        0x42 => "f9".to_string(),
        0x43 => "f10".to_string(),
        0x44 => "f11".to_string(),
        0x45 => "f12".to_string(),
        0x49 => "insert".to_string(),
        0x4A => "home".to_string(),
        0x4B => "pageup".to_string(),
        0x4C => "delete".to_string(),
        0x4D => "end".to_string(),
        0x4E => "pagedown".to_string(),
        _ => format!("unknown-{:02x}", code),
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
    let key = key_code_to_string(key_code);
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
    let key = key_code_to_string(key_code);
    let keystroke = Keystroke {
        modifiers,
        key: key.clone(),
        key_char: if key.len() == 1 { Some(key) } else { None },
    };
    PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke })
}
