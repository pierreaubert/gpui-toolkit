//! NumberInput component tests

use gpui_ui_kit::number_input::{NumberInput, NumberInputSize};

#[test]
fn test_number_input_configuration() {
    let input = NumberInput::new("num-input")
        .value(42.0)
        .min(0.0)
        .max(100.0)
        .step(0.5)
        .decimals(2)
        .unit("Hz")
        .label("Frequency")
        .size(NumberInputSize::Md)
        .width(100.0)
        .disabled(false)
        .on_change(|val, _window, _cx| {
            println!("Value: {}", val);
        });

    drop(input);
}

#[test]
fn test_number_input_range_validation() {
    let input = NumberInput::new("range-test").range(10.0, 50.0);
    drop(input);

    let _ = NumberInput::new("test").min(0.0).max(10.0).value(5.0);
}
