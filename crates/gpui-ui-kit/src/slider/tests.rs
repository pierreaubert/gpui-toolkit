use super::Slider;

#[test]
fn show_value_label_caches_formatted_string() {
    let label1 = Slider::format_value_label(42.5);
    let label2 = Slider::format_value_label(42.5);
    assert_eq!(label1, "42.5");
    assert!(
        std::ptr::eq(label1.as_ref(), label2.as_ref()),
        "repeated formatting for the same value should return the cached SharedString"
    );

    let label3 = Slider::format_value_label(43.0);
    assert_eq!(label3, "43.0");
    assert_ne!(label1.as_ref(), label3.as_ref());
}
