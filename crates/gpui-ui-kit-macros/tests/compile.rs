use gpui_ui_kit_macros::{ComponentBuilder, ComponentTheme, ComponentVariant, FormField};

mod gpui {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Rgba(pub u32, pub bool);

    pub fn rgb(val: u32) -> Rgba {
        Rgba(val, false)
    }

    pub fn rgba(val: u32) -> Rgba {
        Rgba(val, true)
    }
}

mod theme {
    use super::gpui::Rgba;

    #[derive(Debug, Clone)]
    pub struct Theme {
        pub accent: Rgba,
        pub surface: Rgba,
        pub transparent: Rgba,
    }
}

#[derive(Debug, Clone, ComponentTheme)]
pub struct BasicTheme {
    #[theme(default = 0x007acc, from = accent)]
    pub primary: gpui::Rgba,
    #[theme(default = 0x3c3c3c, from = surface)]
    pub surface: gpui::Rgba,
}

#[derive(Debug, Clone, ComponentTheme)]
pub struct TransparentTheme {
    #[theme(default = 0x00000000, from = transparent)]
    pub transparent: gpui::Rgba,
}

#[derive(Debug, Clone, ComponentTheme)]
pub struct LiteralTheme {
    #[theme(default = 16777215, from = accent)]
    pub decimal_white: gpui::Rgba,
    #[theme(default = 0x007accffu32, from = accent)]
    pub suffixed_rgba: gpui::Rgba,
    #[theme(default = 0x00000000u32, from = accent)]
    pub suffixed_transparent: gpui::Rgba,
    #[theme(default = 0x00_7a_cc_ff, from = accent)]
    pub underscored_rgba: gpui::Rgba,
}

#[derive(Debug, Clone, ComponentTheme)]
pub struct FloatTheme {
    #[theme(default_f32 = 1.0, from_expr = "1.0")]
    pub opacity: f32,
}

#[derive(Debug, Clone, ComponentTheme)]
pub struct GenericTheme<T>
where
    T: Default,
{
    #[theme(default_expr = "T::default()", from_expr = "T::default()")]
    pub value: T,
}

#[derive(Debug, Clone, ComponentTheme)]
#[theme_path = "theme::Theme"]
#[gpui_path = "gpui"]
pub struct ExplicitPathTheme {
    #[theme(default = 0xff0000, from = accent)]
    pub red: gpui::Rgba,
}

#[test]
fn test_rgb_uses_rgb() {
    let theme = BasicTheme::default();
    assert_eq!(theme.primary, gpui::rgb(0x007acc));
    assert_eq!(theme.surface, gpui::rgb(0x3c3c3c));
}

#[test]
fn test_transparent_uses_rgba() {
    let theme = TransparentTheme::default();
    assert_eq!(theme.transparent, gpui::rgba(0x00000000));
}

#[test]
fn test_integer_literal_color_defaults_preserve_rgb_and_rgba() {
    let theme = LiteralTheme::default();
    assert_eq!(theme.decimal_white, gpui::rgb(16777215));
    assert_eq!(theme.suffixed_rgba, gpui::rgba(0x007accff));
    assert_eq!(theme.suffixed_transparent, gpui::rgba(0x00000000));
    assert_eq!(theme.underscored_rgba, gpui::rgba(0x00_7a_cc_ff));
}

#[test]
fn test_float_default_matches_documented_attribute() {
    assert_eq!(FloatTheme::default().opacity, 1.0);
}

#[test]
fn test_component_theme_preserves_type_generics() {
    assert_eq!(GenericTheme::<u8>::default().value, 0);

    let global = theme::Theme {
        accent: gpui::rgb(0),
        surface: gpui::rgb(0),
        transparent: gpui::rgba(0),
    };
    assert_eq!(GenericTheme::<u8>::from(&global).value, 0);
}

#[test]
fn test_from_theme() {
    let global = theme::Theme {
        accent: gpui::rgba(0x12345678),
        surface: gpui::rgb(0xabcdef),
        transparent: gpui::rgba(0x00000000),
    };
    let t = BasicTheme::from(&global);
    assert_eq!(t.primary, global.accent);
    assert_eq!(t.surface, global.surface);
}

#[test]
fn test_explicit_path_theme() {
    let global = theme::Theme {
        accent: gpui::rgb(0xff0000),
        surface: gpui::rgb(0x00ff00),
        transparent: gpui::rgba(0x00000000),
    };
    let t = ExplicitPathTheme::from(&global);
    assert_eq!(t.red, global.accent);
}

#[derive(Debug, Clone, ComponentBuilder)]
pub struct BuilderComponent {
    #[field(required, into)]
    pub id: String,
    #[field(optional, into)]
    pub label: Option<String>,
    #[field(default = "true")]
    pub enabled: bool,
    #[field(default = "4")]
    pub count: usize,
    #[field(default = "String::from(\"md\")", rename = "variant", into)]
    pub kind: String,
    #[field(skip, default = "99")]
    pub skipped: usize,
}

#[derive(Debug, Clone, FormField)]
pub struct FormComponent {
    #[field(required, into)]
    pub id: String,
    #[field(optional, into)]
    pub value: Option<String>,
    #[field(default = "false")]
    pub disabled: bool,
}

#[derive(Debug, Clone, FormField)]
pub struct DocumentedFormComponent {
    #[field(required)]
    pub id: String,
    #[field(optional, into)]
    pub value: Option<String>,
}

#[test]
fn test_component_builder_required_optional_into_defaults_and_rename() {
    let component = BuilderComponent::new("field")
        .label("Name")
        .enabled(false)
        .count(7)
        .variant("lg");

    assert_eq!(component.id, "field");
    assert_eq!(component.label.as_deref(), Some("Name"));
    assert!(!component.enabled);
    assert_eq!(component.count, 7);
    assert_eq!(component.kind, "lg");
    assert_eq!(component.skipped, 99);
}

#[test]
fn test_form_field_alias_generates_builder() {
    let component = FormComponent::new("input").value("hello").disabled(true);

    assert_eq!(component.id, "input");
    assert_eq!(component.value.as_deref(), Some("hello"));
    assert!(component.disabled);
}

#[test]
fn test_form_field_required_fields_accept_into_like_readme() {
    let component = DocumentedFormComponent::new("input").value("hello");

    assert_eq!(component.id, "input");
    assert_eq!(component.value.as_deref(), Some("hello"));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ComponentVariant)]
pub enum SampleVariant {
    #[default]
    Primary,
    Secondary,
    #[variant(name = "danger")]
    Destructive,
}

#[test]
fn test_component_variant_matchers_round_trip() {
    use std::str::FromStr;

    assert_eq!(SampleVariant::all().len(), 3);
    assert_eq!(SampleVariant::variant_count(), 3);
    assert_eq!(SampleVariant::Primary.as_str(), "primary");
    assert_eq!(SampleVariant::Secondary.as_str(), "secondary");
    assert_eq!(SampleVariant::Destructive.as_str(), "danger");
    assert!(SampleVariant::Primary.is_default_variant());
    assert!(!SampleVariant::Secondary.is_default_variant());
    assert_eq!(SampleVariant::Primary.to_string(), "primary");
    assert_eq!(SampleVariant::from_str("primary"), Ok(SampleVariant::Primary));
    assert_eq!(
        SampleVariant::from_str("danger"),
        Ok(SampleVariant::Destructive)
    );
    assert!(SampleVariant::from_str("unknown").is_err());
}

#[test]
fn test_prop_docs_json_parses_with_expected_entries() {
    let value: serde_json::Value =
        serde_json::from_str(BuilderComponent::__PROP_DOCS_JSON).expect("valid JSON");
    let entries = value.as_array().expect("top-level array");
    assert_eq!(entries.len(), 6);

    let by_name = |name: &str| {
        entries
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("missing prop entry `{name}`"))
    };
    assert_eq!(by_name("id")["required"], true);
    assert_eq!(by_name("id")["type"], "String");
    assert_eq!(by_name("label")["optional"], true);
    assert_eq!(by_name("label")["default"], "None");
    assert_eq!(by_name("kind")["setter"], "variant");
    assert_eq!(by_name("skipped")["has_setter"], false);
    assert_eq!(by_name("skipped")["default"], "99");
}
