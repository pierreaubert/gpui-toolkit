//! Shared utility helpers for the layout solver.

/// Format a finite `f32` with up to two decimal places, trimming trailing zeros.
///
/// Non-finite values are returned via their default `Display` representation.
pub fn format_number(value: f32) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let mut text = format!("{value:.2}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" { "0".to_string() } else { text }
}

#[cfg(test)]
mod tests {
    use super::format_number;

    #[test]
    fn trims_trailing_zeros_and_decimal_point() {
        assert_eq!(format_number(1.0), "1");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(1.50), "1.5");
        assert_eq!(format_number(1.234), "1.23");
        assert_eq!(format_number(1.239), "1.24");
    }

    #[test]
    fn normalises_negative_zero() {
        assert_eq!(format_number(-0.0), "0");
    }

    #[test]
    fn preserves_non_finite_values() {
        assert_eq!(format_number(f32::NAN), "NaN");
        assert_eq!(format_number(f32::INFINITY), "inf");
        assert_eq!(format_number(f32::NEG_INFINITY), "-inf");
    }
}
