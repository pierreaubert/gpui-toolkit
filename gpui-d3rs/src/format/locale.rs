//! Locale-aware number formatting

use super::specifier::{Align, FormatSpecifier, FormatType, Sign};

/// SI prefixes from yocto to yotta
const SI_PREFIXES: &[&str] = &[
    "y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
];

/// Locale configuration for number formatting
#[derive(Debug, Clone)]
pub struct Locale {
    /// Decimal separator (e.g., "." or ",")
    pub decimal: &'static str,
    /// Thousands grouping separator (e.g., "," or " ")
    pub thousands: &'static str,
    /// Currency symbol prefix
    pub currency_prefix: Option<&'static str>,
    /// Currency symbol suffix
    pub currency_suffix: Option<&'static str>,
    /// Grouping pattern (e.g., [3] for 1,234,567)
    pub grouping: &'static [usize],
    /// Numerals (for non-ASCII number systems)
    pub numerals: Option<&'static [&'static str]>,
    /// Minus sign
    pub minus: &'static str,
    /// Percent sign
    pub percent: &'static str,
}

/// Default US English locale
pub const DEFAULT_LOCALE: Locale = Locale {
    decimal: ".",
    thousands: ",",
    currency_prefix: Some("$"),
    currency_suffix: None,
    grouping: &[3],
    numerals: None,
    minus: "-",
    percent: "%",
};

impl Locale {
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
        let mut value = value;
        let mut prefix = String::new();
        let mut suffix = String::new();

        // Handle special values
        if value.is_nan() {
            return "NaN".to_string();
        }
        if value.is_infinite() {
            return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
        }

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
                suffix = format!("{}{}", cs, suffix);
            }
        }

        // Format the number based on type
        let mut body = self.format_number(spec, value);

        // Handle alternate form for hex/octal/binary
        if spec.symbol == Some('#') {
            match spec.format_type {
                FormatType::Binary => body = format!("0b{}", body),
                FormatType::Octal => body = format!("0o{}", body),
                FormatType::HexLower => body = format!("0x{}", body),
                FormatType::HexUpper => body = format!("0x{}", body),
                _ => {}
            }
        }

        // Apply grouping
        if spec.comma {
            body = self.apply_grouping(&body);
        }

        // Apply padding
        let content = format!("{}{}{}", prefix, body, suffix);
        self.apply_padding(spec, &content, &prefix, &body, &suffix)
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
                    "".to_string()
                }
            }
        };

        // Replace decimal point with locale-specific one
        if self.decimal != "." {
            result = result.replace('.', self.decimal);
        }

        // Trim trailing zeros if requested
        if spec.trim {
            result = self.trim_trailing_zeros(&result);
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
        // Split on decimal point
        let parts: Vec<&str> = s.split(self.decimal).collect();
        let integer_part = parts[0];
        let decimal_part = parts.get(1);

        // Group integer part from right
        let mut grouped = String::new();
        let chars: Vec<char> = integer_part.chars().collect();
        let len = chars.len();

        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (len - i).is_multiple_of(3) {
                grouped.push_str(self.thousands);
            }
            grouped.push(*c);
        }

        if let Some(dec) = decimal_part {
            format!("{}{}{}", grouped, self.decimal, dec)
        } else {
            grouped
        }
    }

    /// Apply padding to reach desired width
    fn apply_padding(
        &self,
        spec: &FormatSpecifier,
        content: &str,
        prefix: &str,
        body: &str,
        suffix: &str,
    ) -> String {
        let width = spec.width.unwrap_or(0);
        let content_len = content.chars().count();

        if content_len >= width {
            return content.to_string();
        }

        let padding_len = width - content_len;
        let padding: String = std::iter::repeat_n(spec.fill, padding_len).collect();

        match spec.align {
            Align::Left => format!("{}{}", content, padding),
            Align::Right => format!("{}{}", padding, content),
            Align::Center => {
                let left = padding_len / 2;
                let right = padding_len - left;
                let left_pad: String = std::iter::repeat_n(spec.fill, left).collect();
                let right_pad: String = std::iter::repeat_n(spec.fill, right).collect();
                format!("{}{}{}", left_pad, content, right_pad)
            }
            Align::AfterSign => {
                // Pad after sign/symbol but before number
                format!("{}{}{}{}", prefix, padding, body, suffix)
            }
        }
    }

    /// Trim trailing zeros after decimal point
    fn trim_trailing_zeros(&self, s: &str) -> String {
        if !s.contains(self.decimal) {
            return s.to_string();
        }

        let mut result = s.trim_end_matches('0').to_string();
        if result.ends_with(self.decimal) {
            result.pop();
        }
        result
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
}
