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

/// Format a GPUI key spec string into a human-readable label.
///
/// Converts internal key spec format (e.g., "secondary-s", "ctrl-shift-k")
/// into display format (e.g., "⌘S", "Ctrl+Shift+K").
pub fn format_key_label(key_spec: &str) -> String {
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
    out
}

fn format_single_key(key_spec: &str) -> String {
    let mut out = String::new();
    let key;

    // Track whether we are still in the modifier portion. The final part after
    // the last '-' is the key name; a trailing '-' means the whole spec is the
    // key.
    if let Some((head, tail)) = key_spec.rsplit_once('-') {
        if tail.is_empty() {
            return key_spec.to_string();
        }
        key = tail;

        for part in head.split('-') {
            if !out.is_empty() {
                out.push('+');
            }
            match modifier_label(part) {
                Some(label) => out.push_str(label),
                None => out.push_str(&capitalize(part)),
            }
        }
    } else {
        key = key_spec;
    }

    if !out.is_empty() {
        out.push('+');
    }
    out.push_str(&format_key_name(key));
    out
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

fn format_key_name(key: &str) -> String {
    match key {
        "space" => "Space".to_string(),
        "enter" => "Enter".to_string(),
        "escape" => "Esc".to_string(),
        "tab" => "Tab".to_string(),
        "backspace" => "Backspace".to_string(),
        "delete" => "Del".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "pageup" => "PgUp".to_string(),
        "pagedown" => "PgDn".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        other => capitalize(other),
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
    fn test_trailing_dash_returns_original() {
        // A trailing dash should not produce an empty key label.
        assert_eq!(format_key_label("ctrl-"), "ctrl-");
        assert_eq!(format_key_label("shift-"), "shift-");
    }
}
