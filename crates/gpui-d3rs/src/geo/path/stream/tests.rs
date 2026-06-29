use super::bounds::BoundsStream;
use super::path_string::PathString;
use crate::geo::stream::Stream;

#[test]
fn test_bounds_stream_empty() {
    let bounds = BoundsStream::new();
    let ((min_x, min_y), (max_x, max_y)) = bounds.result();
    assert!(min_x.is_nan());
    assert!(min_y.is_nan());
    assert!(max_x.is_nan());
    assert!(max_y.is_nan());
}

#[test]
fn test_bounds_stream_finite_points() {
    let mut bounds = BoundsStream::new();
    bounds.point(1.0, 2.0, 0);
    bounds.point(3.0, 4.0, 0);
    bounds.point(0.0, 5.0, 0);

    let ((min_x, min_y), (max_x, max_y)) = bounds.result();
    assert_eq!(min_x, 0.0);
    assert_eq!(min_y, 2.0);
    assert_eq!(max_x, 3.0);
    assert_eq!(max_y, 5.0);
}

#[test]
fn test_bounds_stream_ignores_non_finite() {
    let mut bounds = BoundsStream::new();
    bounds.point(1.0, 2.0, 0);
    bounds.point(f64::NAN, f64::NAN, 0);
    bounds.point(f64::INFINITY, f64::NEG_INFINITY, 0);

    let ((min_x, min_y), (max_x, max_y)) = bounds.result();
    assert_eq!(min_x, 1.0);
    assert_eq!(min_y, 2.0);
    assert_eq!(max_x, 1.0);
    assert_eq!(max_y, 2.0);
}

#[test]
fn test_bounds_stream_line_and_polygon_noop() {
    let mut bounds = BoundsStream::new();
    bounds.line_start();
    bounds.point(1.0, 2.0, 0);
    bounds.line_end();
    bounds.polygon_start();
    bounds.polygon_end();
    bounds.sphere();

    let ((min_x, min_y), (max_x, max_y)) = bounds.result();
    assert_eq!(min_x, 1.0);
    assert_eq!(min_y, 2.0);
    assert_eq!(max_x, 1.0);
    assert_eq!(max_y, 2.0);
}

#[test]
fn test_path_string_point_feature() {
    let mut path = PathString::new(2, 4.5);
    path.point(10.0, 20.0, 0);
    let svg = path.result();
    assert!(svg.starts_with('M'));
    assert!(svg.contains("10,20"));
    assert!(svg.contains("m0,4.5"));
    assert!(svg.ends_with('z'));
}

#[test]
fn test_path_string_line() {
    let mut path = PathString::new(2, 1.0);
    path.line_start();
    path.point(0.0, 0.0, 0);
    path.point(10.0, 20.0, 0);
    path.point(30.0, 40.0, 0);
    path.line_end();

    let svg = path.result();
    assert!(svg.starts_with('M'));
    assert_eq!(svg.matches('L').count(), 2);
    assert!(!svg.contains('Z'));
}

#[test]
fn test_path_string_polygon() {
    let mut path = PathString::new(0, 1.0);
    path.polygon_start();
    path.line_start();
    path.point(0.0, 0.0, 0);
    path.point(10.0, 0.0, 0);
    path.point(10.0, 10.0, 0);
    path.line_end();
    path.polygon_end();

    let svg = path.result();
    assert!(svg.starts_with('M'));
    assert!(svg.ends_with('Z'));
}

#[test]
fn test_path_string_digits_rounding() {
    let mut path = PathString::new(2, 1.0);
    path.line_start();
    path.point(1.23456, 2.34567, 0);
    path.line_end();

    let svg = path.result();
    assert!(svg.contains("1.23,2.35"));
}

#[test]
fn test_path_string_zero_digits_no_rounding() {
    let mut path = PathString::new(0, 1.0);
    path.line_start();
    path.point(1.23456, 2.34567, 0);
    path.line_end();

    let svg = path.result();
    assert!(svg.contains("1.23456,2.34567"));
}

#[test]
fn test_path_string_result_clears_buffer() {
    let mut path = PathString::new(0, 1.0);
    path.line_start();
    path.point(0.0, 0.0, 0);
    path.line_end();

    let first = path.result();
    assert!(!first.is_empty());
    let second = path.result();
    assert!(second.is_empty());
}

#[test]
fn test_path_string_sphere_is_noop() {
    let mut path = PathString::new(0, 1.0);
    path.sphere();
    assert!(path.result().is_empty());
}
