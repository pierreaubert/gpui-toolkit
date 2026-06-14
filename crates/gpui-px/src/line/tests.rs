use crate::line::line;
use crate::ScaleType;

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
