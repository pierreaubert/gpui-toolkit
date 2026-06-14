use super::bar_chart::bar;
use crate::ScaleType;

#[test]
fn build_does_not_clone_category_strings_for_grouped_bars() {
    let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    let values_2023 = vec![100.0, 120.0, 90.0, 150.0];
    let values_2024 = vec![110.0, 140.0, 100.0, 170.0];

    let result = bar(&categories, &values_2023)
        .label("2023")
        .color(0x3b82f6)
        .add_series(&values_2024, Some("2024"), 0xff7f0e, 0.8)
        .size(600.0, 400.0)
        .build();

    assert!(
        result.is_ok(),
        "grouped bar chart should build without per-datum string clones: {:?}",
        result.err()
    );
}

#[test]
fn build_grouped_bars_linear_and_log_scale() {
    let categories = vec!["A", "B", "C"];
    let values = vec![10.0, 20.0, 30.0];
    let extra = vec![5.0, 15.0, 25.0];

    let linear = bar(&categories, &values)
        .add_series(&extra, Some("Extra"), 0x2ca02c, 0.8)
        .y_scale(ScaleType::Linear)
        .build();
    assert!(linear.is_ok());

    let log = bar(&categories, &values)
        .add_series(&extra, Some("Extra"), 0x2ca02c, 0.8)
        .y_scale(ScaleType::Log)
        .build();
    assert!(log.is_ok());
}
