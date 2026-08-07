use crate::scale::Scale;

use super::*;
use crate::scale::LinearScale;

#[test]
fn test_line_y_rel_inverted_range_with_zero() {
    // Inverted range where y_min == 0.0 used to divide by zero
    let y_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, -100.0);
    let (y_min, y_max) = Scale::range(&y_scale);
    assert_eq!(y_min, 0.0);
    assert_eq!(y_max, -100.0);

    // Simulate the y_rel calculation logic inline
    let y_range = y_scale.scale(50.0); // should be -50.0
    let y_range_span = y_max - y_min;
    assert!(y_range_span != 0.0);
    let y_rel = if y_min > y_max {
        ((y_range - y_max) / (y_min - y_max)) as f32
    } else {
        ((y_range - y_min) / y_range_span) as f32
    };
    // 50% of domain should map to middle of relative coords
    assert!(
        (y_rel - 0.5).abs() < 1e-5,
        "y_rel should be 0.5, got {}",
        y_rel
    );
}

#[test]
fn test_line_flat_range() {
    let y_scale = LinearScale::new().domain(0.0, 100.0).range(50.0, 50.0);
    let (y_min, y_max) = Scale::range(&y_scale);
    let y_range = y_scale.scale(25.0);
    let y_range_span = y_max - y_min;
    assert_eq!(y_range_span, 0.0);
    let y_rel = if y_range_span == 0.0 {
        0.5_f32
    } else if y_min > y_max {
        ((y_range - y_max) / (y_min - y_max)) as f32
    } else {
        ((y_range - y_min) / y_range_span) as f32
    };
    assert_eq!(y_rel, 0.5);
}

#[test]
fn test_compute_line_segments_linear() {
    use super::CurveType;
    use super::validation::compute_line_segments;

    let points = vec![(-0.2, 0.5), (0.5, 0.5), (1.2, 0.5)];
    let segments = compute_line_segments(&points, CurveType::Linear);

    // Segments crossing [0,1] should be clipped to visible spans.
    assert_eq!(segments.len(), 2);
    let (x0, y0, x1, y1) = segments[0];
    assert_eq!(y0, 0.5);
    assert_eq!(y1, 0.5);
    assert!(x0 >= 0.0);
    assert!(x1 <= 1.0);
    let (x0, y0, x1, y1) = segments[1];
    assert_eq!(y0, 0.5);
    assert_eq!(y1, 0.5);
    assert!(x0 >= 0.0);
    assert!(x1 <= 1.0);
}

#[test]
fn test_compute_line_segments_supports_smooth_curves() {
    let points = [(0.0, 0.0), (0.5, 1.0), (1.0, 0.25), (1.5, 0.75)];

    for curve in [
        CurveType::Basis,
        CurveType::Cardinal,
        CurveType::CatmullRom,
        CurveType::MonotoneX,
        CurveType::Natural,
    ] {
        let segments = compute_line_segments(&points, curve);
        assert!(
            segments.len() > points.len() - 1,
            "{curve:?} should generate interpolated segments"
        );
        assert!(
            segments
                .iter()
                .all(|segment| [segment.0, segment.1, segment.2, segment.3]
                    .into_iter()
                    .all(f32::is_finite)),
            "{curve:?} should generate finite geometry"
        );
    }
}

#[test]
fn validate_line_inputs_accepts_valid_line() {
    let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 100.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(100.0, 0.0);
    let data = vec![LinePoint::new(0.0, 2.0), LinePoint::new(10.0, 8.0)];
    let config = LineConfig::new()
        .curve(CurveType::Step)
        .dash_array(StrokeDashArray::Custom(vec![4.0, 2.0]));

    validate_line_inputs(&x_scale, &y_scale, &data, &config).unwrap();
}

#[test]
fn validate_line_inputs_rejects_non_finite_data_coordinates() {
    let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 100.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(100.0, 0.0);
    let data = vec![LinePoint::new(0.0, 2.0), LinePoint::new(f64::NAN, 8.0)];
    let config = LineConfig::new();

    let error = validate_line_inputs(&x_scale, &y_scale, &data, &config).unwrap_err();
    match error {
        LineRenderError::NonFiniteDataCoordinate {
            index,
            coordinate,
            value,
        } => {
            assert_eq!(index, 1);
            assert_eq!(coordinate, "x");
            assert!(value.is_nan());
        }
        error => panic!("unexpected error: {error:?}"),
    }
}

#[test]
fn validate_line_inputs_rejects_non_finite_scale_range() {
    let x_scale = LinearScale::new()
        .domain(0.0, 10.0)
        .range(0.0, f64::INFINITY);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(100.0, 0.0);
    let data = vec![LinePoint::new(0.0, 2.0), LinePoint::new(10.0, 8.0)];
    let config = LineConfig::new();

    assert_eq!(
        validate_line_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
        LineRenderError::NonFiniteScaleRange {
            axis: "x",
            endpoint: "max",
            value: f64::INFINITY,
        }
    );
}

#[test]
fn validate_line_inputs_rejects_invalid_config() {
    let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 100.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(100.0, 0.0);
    let data = vec![LinePoint::new(0.0, 2.0), LinePoint::new(10.0, 8.0)];

    let mut config = LineConfig::new();
    config.stroke_width = -1.0;
    assert_eq!(
        validate_line_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
        LineRenderError::NegativeConfigField {
            field: "stroke_width",
            value: -1.0,
        }
    );

    let config = LineConfig::new().dash_array(StrokeDashArray::Custom(Vec::new()));
    assert_eq!(
        validate_line_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
        LineRenderError::EmptyDashPattern
    );

    let config = LineConfig::new().dash_array(StrokeDashArray::Custom(vec![2.0, f32::NAN]));
    let error = validate_line_inputs(&x_scale, &y_scale, &data, &config).unwrap_err();
    match error {
        LineRenderError::InvalidDashLength { index, value } => {
            assert_eq!(index, 1);
            assert!(value.is_nan());
        }
        error => panic!("unexpected error: {error:?}"),
    }
}
