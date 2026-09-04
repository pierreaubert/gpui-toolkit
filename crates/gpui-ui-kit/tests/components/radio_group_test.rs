//! RadioGroup component tests

use gpui_ui_kit::ComponentSize;
use gpui_ui_kit::radio_group::{RadioGroup, RadioGroupOrientation, RadioGroupSize, RadioOption};

fn options() -> Vec<RadioOption> {
    vec![
        RadioOption::new("a", "Alpha"),
        RadioOption::new("b", "Beta"),
        RadioOption::new("c", "Gamma").disabled(true),
    ]
}

#[test]
fn test_radio_group_creation() {
    let group = RadioGroup::new("test").options(options());
    drop(group);
}

#[test]
fn test_radio_group_selected() {
    let group = RadioGroup::new("test")
        .options(options())
        .selected(Some("b".into()));
    assert_eq!(group.selected_index(), Some(1));

    let group = RadioGroup::new("test").options(options());
    assert_eq!(group.selected_index(), None);

    // Unknown value selects nothing.
    let group = RadioGroup::new("test")
        .options(options())
        .selected(Some("zzz".into()));
    assert_eq!(group.selected_index(), None);
}

#[test]
fn test_radio_group_on_change() {
    let group = RadioGroup::new("test")
        .options(options())
        .on_change(|_value, _window, _cx| {});
    drop(group);
}

#[test]
fn test_radio_group_orientation() {
    let vertical = RadioGroup::new("v")
        .options(options())
        .orientation(RadioGroupOrientation::Vertical);
    drop(vertical);
    let horizontal = RadioGroup::new("h")
        .options(options())
        .orientation(RadioGroupOrientation::Horizontal);
    drop(horizontal);
}

#[test]
fn test_radio_group_all_sizes() {
    for size in [RadioGroupSize::Sm, RadioGroupSize::Md, RadioGroupSize::Lg] {
        let group = RadioGroup::new("test").options(options()).size(size);
        drop(group);
    }
}

#[test]
fn test_radio_group_size_from_component_size() {
    let conversions: Vec<(ComponentSize, RadioGroupSize)> = vec![
        (ComponentSize::Xs, RadioGroupSize::Sm),
        (ComponentSize::Sm, RadioGroupSize::Sm),
        (ComponentSize::Md, RadioGroupSize::Md),
        (ComponentSize::Lg, RadioGroupSize::Lg),
        (ComponentSize::Xl, RadioGroupSize::Lg),
    ];
    for (component_size, expected) in conversions {
        let size: RadioGroupSize = component_size.into();
        assert_eq!(size, expected);
    }
}

#[test]
fn test_radio_group_disabled() {
    let group = RadioGroup::new("test")
        .options(options())
        .disabled(true)
        .on_change(|_value, _window, _cx| {});
    drop(group);
}

#[test]
fn test_radio_group_variant_matchers() {
    use std::str::FromStr;

    assert_eq!(RadioGroupOrientation::all().len(), 2);
    assert_eq!(RadioGroupOrientation::Vertical.as_str(), "vertical");
    assert_eq!(
        RadioGroupOrientation::from_str("horizontal"),
        Ok(RadioGroupOrientation::Horizontal)
    );
    assert_eq!(RadioGroupSize::all().len(), 3);
    assert_eq!(RadioGroupSize::Md.as_str(), "md");
    assert!(RadioGroupSize::Md.is_default_variant());
}

#[test]
fn test_radio_option_disabled_flag() {
    let option = RadioOption::new("a", "Alpha").disabled(true);
    assert!(option.disabled);
    let option = RadioOption::new("a", "Alpha");
    assert!(!option.disabled);
}
