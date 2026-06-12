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
    // Handle chord key specs (space-separated sequences like "g g", "ctrl-k ctrl-t")
    let chords: Vec<&str> = key_spec.split_whitespace().collect();
    if chords.len() > 1 {
        return chords
            .iter()
            .map(|chord| format_single_key(chord))
            .collect::<Vec<_>>()
            .join(" ");
    }

    format_single_key(key_spec)
}

fn format_single_key(key_spec: &str) -> String {
    let parts: Vec<&str> = key_spec.split('-').collect();
    let mut modifiers = Vec::new();
    let mut key = String::new();

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if part.is_empty() {
                // Trailing dash — treat the whole spec as the key name.
                return key_spec.to_string();
            }
            key = format_key_name(part);
        } else {
            match *part {
                "secondary" => modifiers.push(platform_modifier_symbol().to_string()),
                "ctrl" => modifiers.push("Ctrl".to_string()),
                "alt" => modifiers.push("Alt".to_string()),
                "shift" => modifiers.push("Shift".to_string()),
                "cmd" => modifiers.push("⌘".to_string()),
                other => modifiers.push(capitalize(other)),
            }
        }
    }

    if modifiers.is_empty() {
        key
    } else {
        format!("{}+{}", modifiers.join("+"), key)
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
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
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
