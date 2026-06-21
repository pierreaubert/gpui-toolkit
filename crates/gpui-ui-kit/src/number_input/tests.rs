use super::{NumberEditState, NumberInput};
use gpui::SharedString;

#[test]
#[allow(clippy::approx_constant)]
fn format_value_str_without_unit() {
    assert_eq!(NumberInput::format_value_str(3.14159, 2, None), "3.14");
    assert_eq!(NumberInput::format_value_str(42.0, 0, None), "42");
}

#[test]
fn format_value_str_with_unit() {
    let unit: SharedString = "Hz".into();
    assert_eq!(
        NumberInput::format_value_str(440.0, 1, Some(&unit)),
        "440.0 Hz"
    );
}

#[test]
fn parse_value_str_ignores_unit() {
    let unit: SharedString = "Hz".into();
    assert_eq!(
        NumberInput::parse_value_str("440.0 Hz", Some(&unit), 0.0, 1000.0),
        Some(440.0)
    );
    assert_eq!(
        NumberInput::parse_value_str("  440  ", Some(&unit), 0.0, 1000.0),
        Some(440.0)
    );
}

#[test]
fn format_value_str_caches_result() {
    let mut state = NumberEditState::default();
    let unit: SharedString = "Hz".into();

    let first = state.format_value_str(440.0, 1, Some(&unit));
    let second = state.format_value_str(440.0, 1, Some(&unit));

    assert_eq!(first, "440.0 Hz");
    // The cached SharedString should be the exact same allocation.
    assert!(
        std::ptr::eq(first.as_ref(), second.as_ref()),
        "format_value_str should return the cached SharedString for identical params"
    );

    // Changing any key field invalidates the cache.
    let third = state.format_value_str(440.0, 2, Some(&unit));
    assert_ne!(first.as_ref(), third.as_ref());
    assert_eq!(third, "440.00 Hz");
}
