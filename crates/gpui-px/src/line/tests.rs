use crate::ScaleType;
use crate::error::ChartError;
use crate::line::line;

#[test]
fn build_uses_generic_scale_helper_all_combinations_build() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];

    let combos = [
        (ScaleType::Linear, ScaleType::Linear),
        (ScaleType::Log, ScaleType::Linear),
        (ScaleType::Linear, ScaleType::Log),
        (ScaleType::Log, ScaleType::Log),
    ];

    for (x_scale, y_scale) in combos {
        let chart = line(&x, &y)
            .size(400.0, 300.0)
            .x_scale(x_scale)
            .y_scale(y_scale)
            .build();
        assert!(
            chart.is_ok(),
            "failed for x={:?}, y={:?}: {:?}",
            x_scale,
            y_scale,
            chart.err()
        );
    }
}

#[test]
fn build_generic_helper_with_secondary_axis_all_combinations_build() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y1 = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let y2 = vec![5.0, 15.0, 25.0, 35.0, 45.0];

    let combos = [
        (ScaleType::Linear, ScaleType::Linear),
        (ScaleType::Log, ScaleType::Linear),
        (ScaleType::Linear, ScaleType::Log),
        (ScaleType::Log, ScaleType::Log),
    ];

    for (x_scale, y_scale) in combos {
        let chart = line(&x, &y1)
            .size(400.0, 300.0)
            .x_scale(x_scale)
            .y_scale(y_scale)
            .add_series_y2(&y2, Some("Secondary"), 0xff7f0e, 2.0, 1.0)
            .build();
        assert!(
            chart.is_ok(),
            "failed with secondary axis for x={:?}, y={:?}: {:?}",
            x_scale,
            y_scale,
            chart.err()
        );
    }
}

#[test]
fn test_line_empty_x() {
    let result = line(&[], &[1.0, 2.0]).build();
    assert!(matches!(result, Err(ChartError::EmptyData { field: "x" })));
}

#[test]
fn test_line_empty_y() {
    let result = line(&[1.0, 2.0], &[]).build();
    assert!(matches!(result, Err(ChartError::EmptyData { field: "y" })));
}

#[test]
fn test_line_length_mismatch() {
    let result = line(&[1.0, 2.0, 3.0], &[1.0, 2.0]).build();
    assert!(matches!(
        result,
        Err(ChartError::DataLengthMismatch {
            x_field: "x",
            y_field: "y",
            x_len: 3,
            y_len: 2,
        })
    ));
}

#[test]
fn test_line_nan_in_data() {
    let result = line(&[1.0, f64::NAN], &[1.0, 2.0]).build();
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "contains NaN or Infinity"
        })
    ));
}

#[test]
fn test_line_invalid_dimensions() {
    let result = line(&[1.0, 2.0], &[1.0, 2.0]).size(0.0, 400.0).build();
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "width",
            value: 0.0
        })
    ));
}

#[test]
fn test_line_log_x_negative() {
    let result = line(&[-1.0, 1.0], &[1.0, 2.0])
        .x_scale(ScaleType::Log)
        .build();
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "contains non-positive values for log scale"
        })
    ));
}

#[test]
fn test_line_log_y_zero() {
    let result = line(&[1.0, 2.0], &[0.0, 1.0])
        .y_scale(ScaleType::Log)
        .build();
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "y",
            reason: "contains non-positive values for log scale"
        })
    ));
}

#[test]
fn test_line_series_length_mismatch() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0];
    let y2 = vec![1.0, 2.0];
    let result = line(&x, &y).add_series(&y2, Some("Short"), 0xff0000, 2.0, 1.0).build();
    assert!(matches!(
        result,
        Err(ChartError::DataLengthMismatch {
            x_field: "x",
            y_field: "series.y",
            ..
        })
    ));
}

#[test]
fn test_line_series_custom_x_mismatch() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0];
    let x2 = vec![1.0, 2.0];
    let y2 = vec![1.0, 2.0, 3.0];
    let result = line(&x, &y)
        .add_series_with_x(&x2, &y2, Some("Custom"), 0xff0000, 2.0, 1.0)
        .build();
    assert!(matches!(
        result,
        Err(ChartError::DataLengthMismatch {
            x_field: "series.x",
            y_field: "series.y",
            ..
        })
    ));
}

#[test]
fn test_line_range_reversal() {
    let result = line(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])
        .x_range(10.0, 0.0)
        .build();
    assert!(matches!(result, Err(ChartError::InvalidData { .. })));
}

#[test]
fn test_line_log_range_negative() {
    let result = line(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])
        .x_scale(ScaleType::Log)
        .x_range(-1.0, 10.0)
        .build();
    assert!(matches!(result, Err(ChartError::InvalidData { .. })));
}

#[test]
fn test_line_secondary_axis_empty() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0];
    let y2: Vec<f64> = vec![];
    let result = line(&x, &y)
        .add_series_y2(&y2, Some("Empty"), 0xff0000, 2.0, 1.0)
        .build();
    assert!(matches!(
        result,
        Err(ChartError::EmptyData { field: "series.y" })
    ));
}

#[test]
fn test_line_hidden_series_builds() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0];
    let y2 = vec![3.0, 2.0, 1.0];
    let result = line(&x, &y)
        .add_series(&y2, Some("Second"), 0xff0000, 2.0, 1.0)
        .hidden_series(&[1])
        .build();
    assert!(result.is_ok());
}

#[test]
fn test_line_builder_chain() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let result = line(&x, &y)
        .title("My Line")
        .x_label("X")
        .y_label("Y")
        .color(0xff0000)
        .stroke_width(2.0)
        .opacity(0.8)
        .curve(d3rs::shape::CurveType::Step)
        .show_points(true)
        .size(800.0, 600.0)
        .build();
    assert!(result.is_ok());
}

#[test]
fn test_line_opacity_clamping() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0];
    let chart = line(&x, &y).opacity(2.0);
    assert_eq!(chart.opacity, 1.0);
    let chart = line(&x, &y).opacity(-1.0);
    assert_eq!(chart.opacity, 0.0);
}
