#![allow(dead_code)]
use serde_json::Value;

pub(super) fn layout_string(layout: &Value, key: &str) -> Option<String> {
    layout.get(key).and_then(Value::as_str).map(str::to_string)
}

pub(super) fn number_field(raw: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    raw.get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}
