//! Format specifier parsing

use regex::Regex;
use std::sync::LazyLock;

/// Format type determines how the number is formatted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    /// No type (general)
    None,
    /// Exponent notation (e.g., 1.23e+4)
    Exponent,
    /// Fixed-point notation (e.g., 1234.56)
    Fixed,
    /// General format (shorter of e or f)
    General,
    /// Round to significant digits
    Round,
    /// SI prefix (e.g., 1.2k, 3.4M)
    Si,
    /// Percentage (multiply by 100, add %)
    Percent,
    /// Percentage (rounded)
    PercentRounded,
    /// Binary
    Binary,
    /// Octal
    Octal,
    /// Decimal integer
    Decimal,
    /// Hexadecimal lowercase
    HexLower,
    /// Hexadecimal uppercase
    HexUpper,
    /// Character (Unicode code point)
    Character,
}

impl FormatType {
    fn from_char(c: char) -> Self {
        match c {
            'e' => FormatType::Exponent,
            'f' => FormatType::Fixed,
            'g' => FormatType::General,
            'r' => FormatType::Round,
            's' => FormatType::Si,
            '%' => FormatType::Percent,
            'p' => FormatType::PercentRounded,
            'b' => FormatType::Binary,
            'o' => FormatType::Octal,
            'd' => FormatType::Decimal,
            'x' => FormatType::HexLower,
            'X' => FormatType::HexUpper,
            'c' => FormatType::Character,
            _ => FormatType::None,
        }
    }
}

/// Alignment for padding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Left align
    Left,
    /// Right align (default)
    Right,
    /// Center align
    Center,
    /// Pad after sign/symbol
    AfterSign,
}

/// Sign handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// Show minus for negative (default)
    Minus,
    /// Show plus for positive, minus for negative
    Plus,
    /// Show space for positive, minus for negative
    Space,
    /// Use parentheses for negative
    Parens,
}

/// Parsed format specifier
#[derive(Debug, Clone)]
pub struct FormatSpecifier {
    pub fill: char,
    pub align: Align,
    pub sign: Sign,
    pub symbol: Option<char>,
    pub zero: bool,
    pub width: Option<usize>,
    pub comma: bool,
    pub precision: Option<usize>,
    pub trim: bool,
    pub format_type: FormatType,
}

impl Default for FormatSpecifier {
    fn default() -> Self {
        Self {
            fill: ' ',
            align: Align::Right,
            sign: Sign::Minus,
            symbol: None,
            zero: false,
            width: None,
            comma: false,
            precision: None,
            trim: false,
            format_type: FormatType::None,
        }
    }
}

static FORMAT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(.)?([<>=^]))?([+\- (])?([#$])?(0)?(\d+)?(,)?(?:\.(\d+))?(~)?([a-zA-Z%])?$")
        .unwrap()
});

/// Parse a format specifier string
pub fn parse_specifier(spec: &str) -> FormatSpecifier {
    let mut result = FormatSpecifier::default();

    if spec.is_empty() {
        return result;
    }

    if let Some(caps) = FORMAT_RE.captures(spec) {
        // Fill character
        if let Some(m) = caps.get(1) {
            result.fill = m.as_str().chars().next().unwrap();
        }

        // Align
        if let Some(m) = caps.get(2) {
            result.align = match m.as_str() {
                "<" => Align::Left,
                ">" => Align::Right,
                "^" => Align::Center,
                "=" => Align::AfterSign,
                _ => Align::Right,
            };
        }

        // Sign
        if let Some(m) = caps.get(3) {
            result.sign = match m.as_str() {
                "+" => Sign::Plus,
                "-" => Sign::Minus,
                " " => Sign::Space,
                "(" => Sign::Parens,
                _ => Sign::Minus,
            };
        }

        // Symbol (# or $)
        if let Some(m) = caps.get(4) {
            result.symbol = Some(m.as_str().chars().next().unwrap());
        }

        // Zero fill
        if caps.get(5).is_some() {
            result.zero = true;
            result.fill = '0';
            result.align = Align::AfterSign;
        }

        // Width
        if let Some(m) = caps.get(6) {
            result.width = m.as_str().parse().ok();
        }

        // Comma grouping
        if caps.get(7).is_some() {
            result.comma = true;
        }

        // Precision
        if let Some(m) = caps.get(8) {
            result.precision = m.as_str().parse().ok();
        }

        // Trim trailing zeros
        if caps.get(9).is_some() {
            result.trim = true;
        }

        // Type
        if let Some(m) = caps.get(10) {
            result.format_type = FormatType::from_char(m.as_str().chars().next().unwrap());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let spec = parse_specifier("d");
        assert_eq!(spec.format_type, FormatType::Decimal);
    }

    #[test]
    fn test_parse_precision() {
        let spec = parse_specifier(".2f");
        assert_eq!(spec.precision, Some(2));
        assert_eq!(spec.format_type, FormatType::Fixed);
    }

    #[test]
    fn test_parse_width() {
        let spec = parse_specifier("8d");
        assert_eq!(spec.width, Some(8));
        assert_eq!(spec.format_type, FormatType::Decimal);
    }

    #[test]
    fn test_parse_zero_fill() {
        let spec = parse_specifier("08d");
        assert!(spec.zero);
        assert_eq!(spec.width, Some(8));
        assert_eq!(spec.fill, '0');
        assert_eq!(spec.align, Align::AfterSign);
    }

    #[test]
    fn test_parse_sign() {
        assert_eq!(parse_specifier("+d").sign, Sign::Plus);
        assert_eq!(parse_specifier("-d").sign, Sign::Minus);
        assert_eq!(parse_specifier(" d").sign, Sign::Space);
    }

    #[test]
    fn test_parse_align() {
        assert_eq!(parse_specifier("<8d").align, Align::Left);
        assert_eq!(parse_specifier(">8d").align, Align::Right);
        assert_eq!(parse_specifier("^8d").align, Align::Center);
        assert_eq!(parse_specifier("=8d").align, Align::AfterSign);
    }

    #[test]
    fn test_parse_fill_align() {
        let spec = parse_specifier("_<8d");
        assert_eq!(spec.fill, '_');
        assert_eq!(spec.align, Align::Left);
        assert_eq!(spec.width, Some(8));
    }

    #[test]
    fn test_parse_comma() {
        let spec = parse_specifier(",d");
        assert!(spec.comma);
    }

    #[test]
    fn test_parse_symbol() {
        assert_eq!(parse_specifier("$d").symbol, Some('$'));
        assert_eq!(parse_specifier("#x").symbol, Some('#'));
    }

    #[test]
    fn test_parse_trim() {
        let spec = parse_specifier("~f");
        assert!(spec.trim);
    }

    #[test]
    fn test_parse_complex() {
        let spec = parse_specifier("_>+$12,.2f");
        assert_eq!(spec.fill, '_');
        assert_eq!(spec.align, Align::Right);
        assert_eq!(spec.sign, Sign::Plus);
        assert_eq!(spec.symbol, Some('$'));
        assert_eq!(spec.width, Some(12));
        assert!(spec.comma);
        assert_eq!(spec.precision, Some(2));
        assert_eq!(spec.format_type, FormatType::Fixed);
    }
}
