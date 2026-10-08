//! Recursive-descent parser for layout expressions.
//!
//! The parser is intentionally dependency-free and total: every input
//! either yields a node forest or a [`ParseError`] with a byte offset
//! and a 1-based column. Nesting depth is capped so adversarial input
//! cannot overflow the stack.

// Rust guideline compliant 2026-02-21

use super::{LayoutAttr, LayoutNode};
use std::fmt::{Display, Formatter};

/// Maximum nesting depth for chains and groups.
///
/// Recursive descent recurses per `>` and per parenthesis level; the
/// cap keeps hostile input (a 200KB run of `A>`) from overflowing the
/// stack. Legitimate layouts never approach it.
const MAX_DEPTH: usize = 256;

/// Syntax failure with source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset of the offense.
    pub offset: usize,
    /// 1-based column of the offense.
    pub column: usize,
    /// What went wrong.
    pub message: String,
}

impl Display for ParseError {
    /// Renders `column N: message`.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "column {}: {}", self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Computes the 1-based column for a byte offset.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::column_for_offset;
///
/// assert_eq!(column_for_offset("V > B", 4), 5);
/// ```
pub fn column_for_offset(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].chars().count() + 1
}

/// Parses a full expression into a sibling forest.
///
/// Returns [`ParseError`] on any syntax failure; use
/// [`super::validate_layout`] afterwards for registry checks.
///
/// # Errors
///
/// Returns [`ParseError`] for empty input, malformed nodes, bad
/// repeats, unterminated groups, or trailing input.
///
/// # Examples
///
/// ```rust
/// use gpui_layout_expr::parse_layout;
///
/// let nodes = parse_layout("V > B\"Hi\"").unwrap();
/// assert_eq!(nodes.len(), 1);
/// assert_eq!(nodes[0].children.len(), 1);
/// ```
pub fn parse_layout(text: &str) -> Result<Vec<LayoutNode>, ParseError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        offset: 0,
        depth: 0,
        text,
    };
    parser.skip_ws();
    if parser.at_end() {
        return Err(parser.error("empty expression"));
    }
    let nodes = parser.parse_seq()?;
    parser.skip_ws();
    if !parser.at_end() {
        return Err(parser.error("unexpected trailing input"));
    }
    Ok(nodes)
}

/// Byte-walking parser state.
struct Parser<'a> {
    bytes: &'a [u8],
    offset: usize,
    depth: usize,
    text: &'a str,
}

/// One parsed atom: spliced nodes plus singularity.
struct Atom {
    nodes: Vec<LayoutNode>,
    single: bool,
}

impl Parser<'_> {
    /// Builds an error at the current offset.
    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            offset: self.offset,
            column: column_for_offset(self.text, self.offset),
            message: message.into(),
        }
    }

    /// Whether input remains.
    fn at_end(&self) -> bool {
        self.offset >= self.bytes.len()
    }

    /// Peeks the current byte without consuming it.
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    /// Consumes the current byte when it matches.
    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    /// Skips ASCII whitespace.
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.offset += 1;
        }
    }

    /// Guards recursion depth against hostile input.
    fn descend(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error("nesting too deep"));
        }
        Ok(())
    }

    /// Parses `chain ("+" chain)*`.
    fn parse_seq(&mut self) -> Result<Vec<LayoutNode>, ParseError> {
        let mut nodes = self.parse_chain()?;
        loop {
            self.skip_ws();
            if !self.eat(b'+') {
                return Ok(nodes);
            }
            self.skip_ws();
            if self.at_end() || matches!(self.peek(), Some(b')')) {
                return Err(self.error("expected node after '+'"));
            }
            nodes.extend(self.parse_chain()?);
        }
    }

    /// Parses `atom (">" seq)?`, attaching the whole sibling run.
    ///
    /// The right-hand side is a full sequence, so `A > B + C` gives A
    /// the children [B, C]; right-nesting (`A > B > C` is A(B(C)))
    /// falls out of the recursion.
    fn parse_chain(&mut self) -> Result<Vec<LayoutNode>, ParseError> {
        let first = self.parse_atom()?;
        self.skip_ws();
        if !self.eat(b'>') {
            return Ok(first.nodes);
        }
        if !first.single {
            return Err(self.error("only single nodes take children with '>'"));
        }
        self.descend()?;
        let mut node = first.nodes.into_iter().next().expect("single atom");
        self.skip_ws();
        let mut children = self.parse_seq()?;
        node.children.append(&mut children);
        self.depth -= 1;
        Ok(vec![node])
    }

    /// Parses one atom: a node or a parenthesized group.
    fn parse_atom(&mut self) -> Result<Atom, ParseError> {
        self.skip_ws();
        if self.eat(b'(') {
            self.descend()?;
            self.skip_ws();
            if matches!(self.peek(), Some(b')')) {
                return Err(self.error("empty group"));
            }
            let mut nodes = self.parse_seq()?;
            self.skip_ws();
            if !self.eat(b')') {
                return Err(self.error("expected ')'"));
            }
            self.depth -= 1;
            let repeat = self.parse_repeat_suffix()?;
            if repeat > 1 {
                let base = nodes.clone();
                for _ in 1..repeat {
                    nodes.extend(base.iter().cloned());
                }
            }
            let single = nodes.len() == 1;
            return Ok(Atom { nodes, single });
        }
        let node = self.parse_node()?;
        Ok(Atom {
            nodes: vec![node],
            single: true,
        })
    }

    /// Parses an optional `*N` suffix, defaulting to 1.
    fn parse_repeat_suffix(&mut self) -> Result<usize, ParseError> {
        self.skip_ws();
        if !self.eat(b'*') {
            return Ok(1);
        }
        self.skip_ws();
        let start = self.offset;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        if start == self.offset {
            return Err(self.error("expected repeat count after '*'"));
        }
        let count: usize = self.text[start..self.offset]
            .parse()
            .map_err(|_| self.error("repeat count out of range"))?;
        if count < 1 {
            return Err(self.error("repeat count must be at least 1"));
        }
        Ok(count)
    }

    /// Parses one node with unordered at-most-once suffixes.
    fn parse_node(&mut self) -> Result<LayoutNode, ParseError> {
        self.skip_ws();
        let offset = self.offset;
        let name = self.parse_name()?;
        let mut node = LayoutNode {
            name,
            id: None,
            modifier: None,
            payload: None,
            attrs: Vec::new(),
            repeat: 1,
            children: Vec::new(),
            offset,
        };
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'#') => {
                    if node.id.is_some() {
                        return Err(self.error("duplicate '#id'"));
                    }
                    self.offset += 1;
                    node.id = Some(self.parse_id()?);
                }
                Some(b'.') => {
                    if node.modifier.is_some() {
                        return Err(self.error("duplicate '.modifier'"));
                    }
                    self.offset += 1;
                    node.modifier = Some(self.parse_modifier()?);
                }
                Some(b'"') => {
                    if node.payload.is_some() {
                        return Err(self.error("duplicate payload"));
                    }
                    node.payload = Some(self.parse_quoted()?);
                }
                Some(b'[') => {
                    if !node.attrs.is_empty() {
                        return Err(self.error("duplicate '[attrs]'"));
                    }
                    self.offset += 1;
                    node.attrs = self.parse_attrs()?;
                }
                Some(b'*') => {
                    if node.repeat != 1 {
                        return Err(self.error("duplicate '*N'"));
                    }
                    node.repeat = self.parse_repeat_suffix()?;
                }
                _ => return Ok(node),
            }
        }
    }

    /// Parses a component name.
    fn parse_name(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(b'A'..=b'Z' | b'a'..=b'z') => {}
            _ => return Err(self.error("expected node name")),
        }
        let start = self.offset;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
        ) {
            self.offset += 1;
        }
        Ok(self.text[start..self.offset].to_owned())
    }

    /// Parses an element id after `#`.
    fn parse_id(&mut self) -> Result<String, ParseError> {
        let start = self.offset;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
        ) {
            self.offset += 1;
        }
        if start == self.offset {
            return Err(self.error("expected id after '#'"));
        }
        Ok(self.text[start..self.offset].to_owned())
    }

    /// Parses a dot modifier.
    fn parse_modifier(&mut self) -> Result<String, ParseError> {
        let start = self.offset;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
        ) {
            self.offset += 1;
        }
        if start == self.offset {
            return Err(self.error("expected modifier after '.'"));
        }
        Ok(self.text[start..self.offset].to_owned())
    }

    /// Parses a double-quoted string with `\"` and `\\` escapes.
    fn parse_quoted(&mut self) -> Result<String, ParseError> {
        self.offset += 1;
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated string")),
                Some(b'"') => {
                    self.offset += 1;
                    return String::from_utf8(out)
                        .map_err(|_| self.error("invalid UTF-8 in string"));
                }
                Some(b'\\') => {
                    self.offset += 1;
                    match self.peek() {
                        Some(b'"') => out.push(b'"'),
                        Some(b'\\') => out.push(b'\\'),
                        _ => return Err(self.error("unknown escape; use \\\" or \\\\")),
                    }
                    self.offset += 1;
                }
                Some(byte) => {
                    out.push(byte);
                    self.offset += 1;
                }
            }
        }
    }

    /// Parses bracket attributes up to `]`.
    fn parse_attrs(&mut self) -> Result<Vec<LayoutAttr>, ParseError> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            if self.eat(b']') {
                if attrs.is_empty() {
                    return Err(self.error("expected attribute"));
                }
                return Ok(attrs);
            }
            if self.at_end() {
                return Err(self.error("unterminated '['"));
            }
            attrs.push(self.parse_attr()?);
        }
    }

    /// Parses one attribute: a flag or `key=value`.
    fn parse_attr(&mut self) -> Result<LayoutAttr, ParseError> {
        let start = self.offset;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.')
        ) {
            self.offset += 1;
        }
        if start == self.offset {
            return Err(self.error("expected attribute key"));
        }
        let key = self.text[start..self.offset].to_owned();
        if !self.eat(b'=') {
            return Ok(LayoutAttr { key, value: None });
        }
        self.skip_ws();
        let value = if self.peek() == Some(b'"') {
            self.parse_quoted()?
        } else {
            let start = self.offset;
            while let Some(byte) = self.peek() {
                if byte == b' ' || byte == b'\t' || byte == b']' {
                    break;
                }
                self.offset += 1;
            }
            if start == self.offset {
                return Err(self.error("expected attribute value"));
            }
            self.text[start..self.offset].to_owned()
        };
        Ok(LayoutAttr {
            key,
            value: Some(value),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_layout;

    #[test]
    fn parses_nesting_and_siblings() {
        let nodes = parse_layout("V > (H > B\"Go\" + I#name)").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "V");
        assert_eq!(nodes[0].children.len(), 1);
        assert_eq!(nodes[0].children[0].children.len(), 2);
    }

    #[test]
    fn siblings_attach_to_parent() {
        let nodes = parse_layout("V > Tx\"Hi\" + B\"Ok\"").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].children.len(), 2);
    }

    #[test]
    fn nests_right_associatively() {
        let nodes = parse_layout("V > H > B\"Go\"").unwrap();
        assert_eq!(nodes[0].children[0].name, "H");
        assert_eq!(nodes[0].children[0].children[0].name, "B");
    }

    #[test]
    fn expands_group_repeats() {
        let nodes = parse_layout("(B\"a\" + B\"b\")*2").unwrap();
        assert_eq!(nodes.len(), 4);
    }

    #[test]
    fn rejects_group_children() {
        let error = parse_layout("(V + H) > B\"x\"").unwrap_err();
        assert!(error.message.contains("only single nodes"));
    }

    #[test]
    fn reports_columns() {
        let error = parse_layout("V > *").unwrap_err();
        assert_eq!(error.column, 5);
    }
}
