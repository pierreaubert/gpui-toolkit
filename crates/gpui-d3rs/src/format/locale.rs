//! Locale-aware number formatting

use super::specifier::{Align, FormatSpecifier, FormatType, Sign};

/// SI prefixes from yocto to yotta
const SI_PREFIXES: &[&str] = &[
    "y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
];

/// Locale configuration for number formatting
#[derive(Debug, Clone)]
pub struct Locale<'a> {
    /// Decimal separator (e.g., "." or ",")
    pub decimal: &'a str,
    /// Thousands grouping separator (e.g., "," or " ")
    pub thousands: &'a str,
    /// Currency symbol prefix
    pub currency_prefix: Option<&'a str>,
    /// Currency symbol suffix
    pub currency_suffix: Option<&'a str>,
    /// Grouping pattern (e.g., `[3]` for 1,234,567)
    pub grouping: &'a [usize],
    /// Numerals (for non-ASCII number systems)
    pub numerals: Option<&'a [&'a str]>,
    /// Minus sign
    pub minus: &'a str,
    /// Percent sign
    pub percent: &'a str,
}

/// Default US English locale
pub const DEFAULT_LOCALE: Locale<'static> = Locale {
    decimal: ".",
    thousands: ",",
    currency_prefix: Some("$"),
    currency_suffix: None,
    grouping: &[3],
    numerals: None,
    minus: "-",
    percent: "%",
};

impl Locale<'_> {
    /// Create a new locale
    pub const fn new(
        decimal: &'static str,
        thousands: &'static str,
        currency_prefix: Option<&'static str>,
        currency_suffix: Option<&'static str>,
    ) -> Self {
        Self {
            decimal,
            thousands,
            currency_prefix,
            currency_suffix,
            grouping: &[3],
            numerals: None,
            minus: "-",
            percent: "%",
        }
    }

    /// Format a number according to the given specifier
    pub fn format(&self, spec: &FormatSpecifier, value: f64) -> String {
        if value.is_nan() {
            return "NaN".to_string();
        }
        if value.is_infinite() {
            return if value > 0.0 {
                "Infinity".to_string()
            } else {
                "-Infinity".to_string()
            };
        }

        let mut value = value;
        let mut prefix = String::new();
        let mut suffix = String::new();

        // Handle percentage types
        if spec.format_type == FormatType::Percent || spec.format_type == FormatType::PercentRounded
        {
            value *= 100.0;
            suffix.push_str(self.percent);
        }

        // Handle sign
        let negative = value < 0.0;
        value = value.abs();

        match spec.sign {
            Sign::Plus if !negative => prefix.push('+'),
            Sign::Space if !negative => prefix.push(' '),
            Sign::Parens if negative => {
                prefix.push('(');
                suffix.push(')');
            }
            _ if negative => prefix.push_str(self.minus),
            _ => {}
        }

        // Handle currency symbol
        if spec.symbol == Some('$') {
            if let Some(cp) = self.currency_prefix {
                prefix.push_str(cp);
            }
            if let Some(cs) = self.currency_suffix {
                let mut new_suffix = String::with_capacity(cs.len() + suffix.len());
                new_suffix.push_str(cs);
                new_suffix.push_str(&suffix);
                suffix = new_suffix;
            }
        }

        // Format the number based on type
        let mut body = self.format_number(spec, value);

        // Handle alternate form for hex/octal/binary
        if spec.symbol == Some('#') {
            match spec.format_type {
                FormatType::Binary => {
                    let mut prefixed = String::with_capacity(body.len() + 2);
                    prefixed.push_str("0b");
                    prefixed.push_str(&body);
                    body = prefixed;
                }
                FormatType::Octal => {
                    let mut prefixed = String::with_capacity(body.len() + 2);
                    prefixed.push_str("0o");
                    prefixed.push_str(&body);
                    body = prefixed;
                }
                FormatType::HexLower => {
                    let mut prefixed = String::with_capacity(body.len() + 2);
                    prefixed.push_str("0x");
                    prefixed.push_str(&body);
                    body = prefixed;
                }
                FormatType::HexUpper => {
                    let mut prefixed = String::with_capacity(body.len() + 2);
                    prefixed.push_str("0x");
                    prefixed.push_str(&body);
                    body = prefixed;
                }
                _ => {}
            }
        }

        // Apply grouping
        if spec.comma {
            body = self.apply_grouping(&body);
        }

        // Assemble content
        let mut content = String::with_capacity(prefix.len() + body.len() + suffix.len());
        content.push_str(&prefix);
        content.push_str(&body);
        content.push_str(&suffix);

        // Apply padding
        self.apply_padding(spec, content, &prefix, &body, &suffix)
    }

    /// Format the numeric part of the value
    fn format_number(&self, spec: &FormatSpecifier, value: f64) -> String {
        let precision = spec.precision.unwrap_or(6);

        let mut result = match spec.format_type {
            FormatType::None => {
                if spec.precision.is_some() {
                    format!("{:.prec$}", value, prec = precision)
                } else {
                    format!("{}", value)
                }
            }
            FormatType::Exponent => {
                format!("{:.prec$e}", value, prec = precision)
            }
            FormatType::Fixed => {
                format!("{:.prec$}", value, prec = precision)
            }
            FormatType::General => {
                // Use shorter of exponential or fixed
                let exp = format!("{:.prec$e}", value, prec = precision);
                let fixed = format!("{:.prec$}", value, prec = precision);
                if exp.len() < fixed.len() { exp } else { fixed }
            }
            FormatType::Round => {
                // Round to significant digits
                if value == 0.0 {
                    "0".to_string()
                } else {
                    let digits = precision.max(1);
                    let magnitude = value.abs().log10().floor() as i32;
                    let scale = 10_f64.powi(digits as i32 - 1 - magnitude);
                    let rounded = (value * scale).round() / scale;
                    format!("{}", rounded)
                }
            }
            FormatType::Si => self.format_si(value, precision),
            FormatType::Percent | FormatType::PercentRounded => {
                format!("{:.prec$}", value, prec = precision)
            }
            FormatType::Decimal => {
                format!("{:.0}", value)
            }
            FormatType::Binary => {
                format!("{:b}", value as i64)
            }
            FormatType::Octal => {
                format!("{:o}", value as i64)
            }
            FormatType::HexLower => {
                format!("{:x}", value as i64)
            }
            FormatType::HexUpper => {
                format!("{:X}", value as i64)
            }
            FormatType::Character => {
                if let Some(c) = char::from_u32(value as u32) {
                    c.to_string()
                } else {
                    String::new()
                }
            }
        };

        // Replace decimal point with locale-specific one
        if self.decimal != "." {
            result = result.replace('.', self.decimal);
        }

        // Trim trailing zeros if requested
        if spec.trim {
            self.trim_trailing_zeros(&mut result);
        }

        result
    }

    /// Format with SI prefix
    fn format_si(&self, value: f64, precision: usize) -> String {
        if value == 0.0 {
            return format!("{:.prec$}", 0.0, prec = precision);
        }

        let exp = (value.abs().log10() / 3.0).floor() as i32;
        let exp = exp.clamp(-8, 8);
        let si_index = (exp + 8) as usize;
        let prefix = SI_PREFIXES[si_index];

        let scaled = value / 10_f64.powi(exp * 3);
        format!("{:.prec$}{}", scaled, prefix, prec = precision)
    }

    /// Apply thousands grouping
    fn apply_grouping(&self, s: &str) -> String {
        let mut grouped = String::with_capacity(s.len() + s.len() / 3);
        // Split on decimal point
        let decimal_idx = s.find(self.decimal);
        let integer_part = &s[..decimal_idx.unwrap_or(s.len())];
        let decimal_part = decimal_idx.map(|i| &s[i + self.decimal.len()..]);

        // Group integer part from right without allocating a char Vec
        let len = integer_part.chars().count();
        for (i, c) in integer_part.chars().enumerate() {
            if i > 0 && (len - i).is_multiple_of(3) {
                grouped.push_str(self.thousands);
            }
            grouped.push(c);
        }

        if let Some(dec) = decimal_part {
            grouped.push_str(self.decimal);
            grouped.push_str(dec);
        }

        grouped
    }

    /// Apply padding to reach desired width
    fn apply_padding(
        &self,
        spec: &FormatSpecifier,
        content: String,
        prefix: &str,
        body: &str,
        suffix: &str,
    ) -> String {
        let width = spec.width.unwrap_or(0);
        let content_len = content.chars().count();

        if content_len >= width {
            return content;
        }

        let padding_len = width - content_len;
        let mut result = String::with_capacity(width);

        match spec.align {
            Align::Left => {
                result.push_str(&content);
                result.extend(std::iter::repeat_n(spec.fill, padding_len));
            }
            Align::Right => {
                result.extend(std::iter::repeat_n(spec.fill, padding_len));
                result.push_str(&content);
            }
            Align::Center => {
                let left = padding_len / 2;
                let right = padding_len - left;
                result.extend(std::iter::repeat_n(spec.fill, left));
                result.push_str(&content);
                result.extend(std::iter::repeat_n(spec.fill, right));
            }
            Align::AfterSign => {
                // Pad after sign/symbol but before number
                result.push_str(prefix);
                result.extend(std::iter::repeat_n(spec.fill, padding_len));
                result.push_str(body);
                result.push_str(suffix);
            }
        }

        result
    }

    /// Trim trailing zeros after decimal point
    fn trim_trailing_zeros(&self, s: &mut String) {
        if !s.contains(self.decimal) {
            return;
        }
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with(self.decimal) {
            s.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::specifier::parse_specifier;

    #[test]
    fn test_format_decimal() {
        let spec = parse_specifier("d");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 42.0), "42");
        assert_eq!(DEFAULT_LOCALE.format(&spec, -42.0), "-42");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_format_fixed() {
        let spec = parse_specifier(".2f");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 3.14159), "3.14");
    }

    #[test]
    fn test_format_grouping() {
        let spec = parse_specifier(",d");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 1234567.0), "1,234,567");
    }

    #[test]
    fn test_format_sign() {
        let spec = parse_specifier("+d");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 42.0), "+42");
        assert_eq!(DEFAULT_LOCALE.format(&spec, -42.0), "-42");
    }

    #[test]
    fn test_format_padding() {
        let spec = parse_specifier("8d");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 42.0), "      42");

        let spec = parse_specifier("08d");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 42.0), "00000042");
    }

    #[test]
    fn test_format_si() {
        let spec = parse_specifier(".2s");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 1234.0), "1.23k");
    }

    #[test]
    fn test_format_percent() {
        let spec = parse_specifier(".0%");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 0.5), "50%");
    }

    #[test]
    fn test_format_hex() {
        let spec = parse_specifier("x");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 255.0), "ff");

        let spec = parse_specifier("#x");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 255.0), "0xff");
    }

    #[test]
    fn test_format_special_values() {
        let spec = parse_specifier(".2f");
        assert_eq!(DEFAULT_LOCALE.format(&spec, f64::NAN), "NaN");
        assert_eq!(DEFAULT_LOCALE.format(&spec, f64::INFINITY), "Infinity");
        assert_eq!(DEFAULT_LOCALE.format(&spec, f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn test_format_sign_variants() {
        let plus = parse_specifier("+d");
        assert_eq!(DEFAULT_LOCALE.format(&plus, 5.0), "+5");

        let space = parse_specifier(" d");
        assert_eq!(DEFAULT_LOCALE.format(&space, 5.0), " 5");

        let parens = parse_specifier("(d");
        assert_eq!(DEFAULT_LOCALE.format(&parens, -5.0), "(5)");
    }

    #[test]
    fn test_format_currency() {
        let spec = parse_specifier("$.2f");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 1234.5), "$1234.50");
    }

    #[test]
    fn test_format_types() {
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier(".3e"), 1234.6),
            "1.235e3"
        );
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier(".4g"), 1.5),
            "1.5000"
        );
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier(".2r"), 1234.0),
            "1200"
        );
        assert_eq!(DEFAULT_LOCALE.format(&parse_specifier("b"), 13.0), "1101");
        assert_eq!(DEFAULT_LOCALE.format(&parse_specifier("#o"), 9.0), "0o11");
        assert_eq!(DEFAULT_LOCALE.format(&parse_specifier("X"), 255.0), "FF");
        assert_eq!(DEFAULT_LOCALE.format(&parse_specifier("c"), 65.0), "A");
    }

    #[test]
    fn test_format_trim() {
        let spec = parse_specifier("#.4f");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 1.5), "1.5000");
        let spec_trim = parse_specifier("#.4~f");
        assert_eq!(DEFAULT_LOCALE.format(&spec_trim, 1.5), "1.5");
    }

    #[test]
    fn test_format_si_negative() {
        let spec = parse_specifier(".2s");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 0.00123), "1.23m");
        assert_eq!(DEFAULT_LOCALE.format(&spec, -1234.0), "-1.23k");
    }

    #[test]
    fn test_format_locale_new_and_decimal() {
        let locale = Locale::new(",", " ", Some("€"), None);
        let spec = parse_specifier(",.2f");
        assert_eq!(locale.format(&spec, 1234.5), "1 234,50");
    }

    #[test]
    fn test_format_padding_alignments() {
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier("<8d"), 42.0),
            "42      "
        );
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier(">8d"), 42.0),
            "      42"
        );
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier("^8d"), 42.0),
            "   42   "
        );
        assert_eq!(
            DEFAULT_LOCALE.format(&parse_specifier("=+8d"), 42.0),
            "+     42"
        );
    }

    #[test]
    fn test_format_no_padding_needed() {
        let spec = parse_specifier("2d");
        assert_eq!(DEFAULT_LOCALE.format(&spec, 1234.0), "1234");
    }
}
