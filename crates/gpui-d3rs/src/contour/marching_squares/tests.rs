use super::contour::contours;
use super::contour_generator::ContourGenerator;
use super::contour_ring::ContourRing;
use super::contour_ring::ContourRingError;
use super::misc::upsample_axis_values;
use crate::shape::path::Point;

#[test]
fn test_contour_generator() {
    let values = vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];

    let generator = ContourGenerator::new(4, 4);
    let contour = generator.contour(&values, 0.5);
    assert_eq!(contour.value, 0.5);
}

#[test]
fn test_multiple_contours() {
    let values = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];

    let thresholds = vec![0.25, 0.5, 0.75];
    let result = contours(&values, 3, 3, &thresholds);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_contour_segments_keeps_saddle_segments_independent() {
    let values = vec![1.0, 0.0, 0.0, 1.0];
    let generator = ContourGenerator::new(2, 2).x(0.0, 1.0).y(0.0, 1.0);

    let segments = generator.contour_segments(&values, &[0.5]);

    assert_eq!(segments.len(), 2);
    assert!((segments[0].start.x - 0.5).abs() < 1e-9);
    assert!((segments[0].start.y - 0.0).abs() < 1e-9);
    assert!((segments[0].end.x - 1.0).abs() < 1e-9);
    assert!((segments[0].end.y - 0.5).abs() < 1e-9);
    assert!((segments[1].start.x - 0.5).abs() < 1e-9);
    assert!((segments[1].start.y - 1.0).abs() < 1e-9);
    assert!((segments[1].end.x - 0.0).abs() < 1e-9);
    assert!((segments[1].end.y - 0.5).abs() < 1e-9);
}

#[test]
fn test_upsampled_contours() {
    let values = vec![0.0, 1.0, 1.0, 0.0];

    let generator = ContourGenerator::new(2, 2).upsample_factor(3);
    let contour = generator.contour(&values, 0.5);

    assert_eq!(contour.value, 0.5);
}

#[test]
fn test_log_axis_upsampling_preserves_midpoints() {
    let values = upsample_axis_values(&[10.0, 1000.0], 2, 3, true);

    assert!((values[1] - 100.0).abs() < 1e-6);
}

#[test]
fn test_log_axis_crossing_interpolates_in_log_space() {
    let generator = ContourGenerator::new(2, 2)
        .x_values(vec![10.0, 1000.0])
        .x_log_interpolation(true);

    assert!((generator.transform_x(0.25) - 31.622776601683793).abs() < 1e-9);
}

#[test]
fn test_upsampled_log_axis_crossing_interpolates_in_log_space() {
    let values = vec![0.0, 1.0, 0.0, 1.0];
    let generator = ContourGenerator::new(2, 2)
        .x_values(vec![10.0, 1000.0])
        .x_log_interpolation(true)
        .upsample_factor(2);
    let (upsampled_generator, _) = generator.upsampled(&values).expect("upsampled grid");

    assert!((upsampled_generator.transform_x(0.5) - 31.622776601683793).abs() < 1e-9);
}

#[test]
fn test_contour_ring_area() {
    let ring = ContourRing::new(vec![
        Point::new(0.0, 0.0),
        Point::new(1.0, 0.0),
        Point::new(1.0, 1.0),
        Point::new(0.0, 1.0),
        Point::new(0.0, 0.0),
    ]);

    // Clockwise ring should have positive area
    assert!(ring.area().abs() > 0.0);
    assert_eq!(ring.area(), ring.try_area().unwrap());
}

#[test]
fn test_contour_ring_try_area_rejects_non_finite_points() {
    let ring = ContourRing::new(vec![
        Point::new(0.0, 0.0),
        Point::new(1.0, f64::NAN),
        Point::new(0.0, 0.0),
    ]);

    let error = ring.try_area().unwrap_err();
    match error {
        ContourRingError::NonFinitePoint {
            index,
            coordinate,
            value,
        } => {
            assert_eq!(index, 1);
            assert_eq!(coordinate, "y");
            assert!(value.is_nan());
        }
    }

    let ring = ContourRing::new(vec![
        Point::new(0.0, 0.0),
        Point::new(f64::INFINITY, 1.0),
        Point::new(0.0, 0.0),
    ]);
    assert_eq!(
        ring.try_area().unwrap_err(),
        ContourRingError::NonFinitePoint {
            index: 1,
            coordinate: "x",
            value: f64::INFINITY,
        }
    );
}

#[test]
fn test_contour_ring_closed() {
    let ring = ContourRing::new(vec![
        Point::new(0.0, 0.0),
        Point::new(1.0, 0.0),
        Point::new(1.0, 1.0),
        Point::new(0.0, 0.0),
    ]);

    assert!(ring.is_closed());
}

#[test]
fn test_transform_single_dimension() {
    let generator = ContourGenerator::new(1, 1);
    assert_eq!(generator.contour(&[1.0], 0.5).coordinates.len(), 0);
}

#[test]
fn test_contour_zero_dimension_returns_empty() {
    let generator = ContourGenerator::new(0, 0);
    assert_eq!(generator.contour(&[], 0.5).coordinates.len(), 0);
    assert!(generator.contour_bands(&[], &[0.0, 1.0]).is_empty());
}
