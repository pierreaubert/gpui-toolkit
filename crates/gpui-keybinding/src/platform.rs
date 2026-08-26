/// Returns the platform-specific modifier name.
///
/// - macOS: "Cmd"
/// - Linux/Windows: "Ctrl"
pub fn platform_modifier() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Cmd"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl"
    }
}

/// Returns the platform-specific modifier symbol.
///
/// - macOS: "⌘"
/// - Linux/Windows: "Ctrl"
pub fn platform_modifier_symbol() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "⌘"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl"
    }
}

use std::borrow::Cow;

/// Format a GPUI key spec string into a human-readable label.
///
/// Converts internal key spec format (e.g., "secondary-s", "ctrl-shift-k")
/// into display format (e.g., "⌘S", "Ctrl+Shift+K").
pub fn format_key_label(key_spec: &str) -> Cow<'static, str> {
    // Fast path: no whitespace means a single key/chord part.
    if !key_spec.bytes().any(|b: u8| b.is_ascii_whitespace()) {
        return format_single_key(key_spec);
    }

    let mut out = String::new();
    for (i, chord) in key_spec.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format_single_key(chord));
    }
    Cow::Owned(out)
}

fn format_single_key(key_spec: &str) -> Cow<'static, str> {
    // Fast path: no modifiers means we can return the formatted key name
    // without allocating a String.
    if !key_spec.contains('-') {
        return format_key_name(key_spec);
    }

    // Track whether we are still in the modifier portion. The final part after
    // the last '-' is the key name; a trailing '-' means the whole spec is the
    // key.
    if let Some((head, tail)) = key_spec.rsplit_once('-') {
        let (head, tail) = if tail.is_empty() {
            if head.is_empty() || head.ends_with('-') {
                return Cow::Owned(key_spec.to_string());
            }
            (head, "-")
        } else {
            (head, tail)
        };

        let mut out = String::new();
        for part in head.split('-') {
            if !out.is_empty() {
                out.push('+');
            }
            match modifier_label(part) {
                Some(label) => out.push_str(label),
                None => out.push_str(&capitalize(part)),
            }
        }

        if !out.is_empty() {
            out.push('+');
        }
        out.push_str(&format_key_name(tail));
        Cow::Owned(out)
    } else {
        format_key_name(key_spec)
    }
}

fn modifier_label(part: &str) -> Option<&'static str> {
    match part {
        "secondary" => Some(platform_modifier_symbol()),
        "ctrl" => Some("Ctrl"),
        "alt" => Some("Alt"),
        "shift" => Some("Shift"),
        "cmd" => Some("⌘"),
        _ => None,
    }
}

fn format_key_name(key: &str) -> Cow<'static, str> {
    match key {
        "space" => Cow::Borrowed("Space"),
        "enter" => Cow::Borrowed("Enter"),
        "escape" => Cow::Borrowed("Esc"),
        "tab" => Cow::Borrowed("Tab"),
        "backspace" => Cow::Borrowed("Backspace"),
        "delete" => Cow::Borrowed("Del"),
        "up" => Cow::Borrowed("↑"),
        "down" => Cow::Borrowed("↓"),
        "left" => Cow::Borrowed("←"),
        "right" => Cow::Borrowed("→"),
        "pageup" => Cow::Borrowed("PgUp"),
        "pagedown" => Cow::Borrowed("PgDn"),
        "home" => Cow::Borrowed("Home"),
        "end" => Cow::Borrowed("End"),
        other => Cow::Owned(capitalize(other)),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let mut out = String::with_capacity(s.len());
            for upper in c.to_uppercase() {
                out.push(upper);
            }
            out.push_str(chars.as_str());
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_key_label() {
        assert_eq!(format_key_label("up"), "↑");
        assert_eq!(format_key_label("ctrl-s"), "Ctrl+S");
        assert_eq!(format_key_label("ctrl-shift-k"), "Ctrl+Shift+K");
        assert_eq!(format_key_label("alt-left"), "Alt+←");
        assert_eq!(format_key_label("space"), "Space");
    }

    #[test]
    fn test_format_key_label_chords() {
        assert_eq!(format_key_label("g g"), "G G");
        assert_eq!(format_key_label("ctrl-k ctrl-t"), "Ctrl+K Ctrl+T");
        assert_eq!(format_key_label("z o"), "Z O");
    }

    #[test]
    fn test_format_key_label_secondary() {
        let label = format_key_label("secondary-s");
        #[cfg(target_os = "macos")]
        assert_eq!(label, "⌘+S");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(label, "Ctrl+S");
    }

    #[test]
    fn test_bare_minus_returns_original() {
        assert_eq!(format_key_label("-"), "-");
    }

    #[test]
    fn test_minus_key_with_modifiers() {
        assert!(gpui::Keystroke::parse("ctrl-").is_ok());
        assert_eq!(format_key_label("ctrl-"), "Ctrl+-");
        assert_eq!(format_key_label("shift-"), "Shift+-");
    }

    #[test]
    fn test_platform_modifier() {
        #[cfg(target_os = "macos")]
        assert_eq!(platform_modifier(), "Cmd");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(platform_modifier(), "Ctrl");
    }

    #[test]
    fn test_platform_modifier_symbol() {
        #[cfg(target_os = "macos")]
        assert_eq!(platform_modifier_symbol(), "⌘");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(platform_modifier_symbol(), "Ctrl");
    }

    #[test]
    fn test_format_key_label_empty() {
        assert_eq!(format_key_label(""), "");
    }

    #[test]
    fn test_format_key_label_unknown_modifier() {
        assert_eq!(format_key_label("foo-bar"), "Foo+Bar");
        assert_eq!(format_key_label("meta-s"), "Meta+S");
    }

    #[test]
    fn test_format_key_label_cmd_modifier() {
        assert_eq!(format_key_label("cmd-s"), "⌘+S");
    }
}
