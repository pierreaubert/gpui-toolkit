#![cfg(feature = "showcase")]

//! Showcase helpers shared with the `gpui-python-showcase` binary.

use gpui_px::ColorScale;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn normalized_eq(value: &str, target: &str) -> bool {
    let mut value = value.trim().chars().filter(|&c| c != '-' && c != '_');
    let mut target = target.chars();
    loop {
        match (value.next(), target.next()) {
            (Some(v), Some(t)) if !v.eq_ignore_ascii_case(&t) => return false,
            (Some(_), Some(_)) => continue,
            (None, None) => return true,
            _ => return false,
        }
    }
}

pub fn color_scale(value: &str) -> ColorScale {
    if normalized_eq(value, "plasma") {
        ColorScale::Plasma
    } else if normalized_eq(value, "inferno") {
        ColorScale::Inferno
    } else if normalized_eq(value, "magma") {
        ColorScale::Magma
    } else if normalized_eq(value, "heat") {
        ColorScale::Heat
    } else if normalized_eq(value, "coolwarm") {
        ColorScale::Coolwarm
    } else if normalized_eq(value, "greys") || normalized_eq(value, "grays") {
        ColorScale::Greys
    } else {
        ColorScale::Viridis
    }
}

pub fn parse_spec<T>(value: &Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    T::deserialize(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Sample {
        value: i32,
    }

    #[test]
    fn color_scale_matches_case_and_separators() {
        assert!(matches!(color_scale("Plasma"), ColorScale::Plasma));
        assert!(matches!(color_scale("COOL-WARM"), ColorScale::Coolwarm));
        assert!(matches!(color_scale("cool_warm"), ColorScale::Coolwarm));
        assert!(matches!(color_scale("Greys"), ColorScale::Greys));
        assert!(matches!(color_scale("Grays"), ColorScale::Greys));
        assert!(matches!(color_scale("unknown"), ColorScale::Viridis));
    }

    #[test]
    fn parse_spec_deserializes_without_cloning_value() {
        let value = serde_json::json!({ "value": 42 });
        let parsed: Sample = parse_spec(&value).expect("parse");
        assert_eq!(parsed, Sample { value: 42 });
        // value is still usable after parsing
        assert_eq!(value["value"], 42);
    }
}
