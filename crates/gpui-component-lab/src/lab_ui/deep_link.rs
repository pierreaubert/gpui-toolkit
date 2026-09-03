//! Shareable `?story=` deep links for the component lab.
//!
//! Storybook-equivalent URL routing: the selected story id plus overridden
//! prop values encode into a query string (`?story=<id>&prop.<name>=<value>`)
//! that can be pasted, bookmarked, or asserted in visual-regression harnesses.
//! [`parse_lab_deep_link`] accepts the query with or without a leading `?`.

use crate::StoryPropValue;
use gpui::SharedString;

/// Selected story plus overridden prop values decoded from a deep link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabDeepLink {
    pub story_id: String,
    pub props: Vec<(String, String)>,
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if is_unreserved(byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = value.get(index + 1..index + 3)?;
                let byte = u8::from_str_radix(hex, 16).ok()?;
                out.push(byte);
                index += 3;
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Encodes the selected story plus prop overrides as a shareable query
/// string, e.g. `?story=ui-kit.button&prop.variant=primary`.
pub fn encode_lab_deep_link(story_id: &str, props: &[(&str, &str)]) -> String {
    let mut link = format!("?story={}", percent_encode(story_id));
    for (name, value) in props {
        link.push_str("&prop.");
        link.push_str(&percent_encode(name));
        link.push('=');
        link.push_str(&percent_encode(value));
    }
    link
}

/// Parses a [`encode_lab_deep_link`] query back into a story selection.
/// Returns `None` when no usable `story=` parameter is present or any
/// percent-encoding is malformed.
pub fn parse_lab_deep_link(query: &str) -> Option<LabDeepLink> {
    let query = query.strip_prefix(['?', '#']).unwrap_or(query);
    let mut story_id = None;
    let mut props = Vec::new();
    for pair in query.split('&') {
        let (key, raw) = pair.split_once('=')?;
        if key == "story" {
            let decoded = percent_decode(raw)?;
            if !decoded.is_empty() {
                story_id = Some(decoded);
            }
        } else if let Some(name) = key.strip_prefix("prop.") {
            let name = percent_decode(name)?;
            let value = percent_decode(raw)?;
            if !name.is_empty() {
                props.push((name, value));
            }
        }
    }
    Some(LabDeepLink {
        story_id: story_id?,
        props,
    })
}

/// Renders a prop value in its deep-link string form.
pub fn prop_value_to_query_string(value: &StoryPropValue) -> String {
    match value {
        StoryPropValue::Bool(value) => value.to_string(),
        StoryPropValue::Number(value) => value.to_string(),
        StoryPropValue::Text(value)
        | StoryPropValue::Choice(value)
        | StoryPropValue::Color(value) => value.to_string(),
    }
}

/// Coerces a deep-link string back to the story's declared prop type,
/// keeping the current value when the string does not parse.
pub fn coerce_prop_value(current: &StoryPropValue, raw: &str) -> StoryPropValue {
    match current {
        StoryPropValue::Bool(_) => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "on" | "yes" => StoryPropValue::Bool(true),
            "false" | "0" | "off" | "no" => StoryPropValue::Bool(false),
            _ => current.clone(),
        },
        StoryPropValue::Number(_) => raw
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(StoryPropValue::Number)
            .unwrap_or_else(|| current.clone()),
        StoryPropValue::Text(_) => StoryPropValue::Text(SharedString::from(raw)),
        StoryPropValue::Choice(_) => StoryPropValue::Choice(SharedString::from(raw)),
        StoryPropValue::Color(_) => StoryPropValue::Color(SharedString::from(raw)),
    }
}
