use super::path_builder::PathBuilder;
use super::point::Point;

#[test]
fn test_path_builder() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(100.0, 0.0)
        .line_to(100.0, 100.0)
        .line_to(0.0, 100.0)
        .close_path()
        .build();

    assert_eq!(path.commands().len(), 5);
}

#[test]
fn test_path_bounds() {
    let path = PathBuilder::new()
        .move_to(10.0, 20.0)
        .line_to(50.0, 20.0)
        .line_to(50.0, 80.0)
        .line_to(10.0, 80.0)
        .close_path()
        .build();

    let bounds = path.bounds().unwrap();
    assert_eq!(bounds, (10.0, 20.0, 50.0, 80.0));
}

#[test]
fn test_path_flatten() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(100.0, 0.0)
        .line_to(100.0, 100.0)
        .build();

    let points = path.flatten(1.0);
    assert_eq!(points.len(), 3);
}

#[test]
fn test_path_to_svg() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(100.0, 0.0)
        .close_path()
        .build();

    let svg = path.to_svg_string();
    assert!(svg.contains("M0,0"));
    assert!(svg.contains("L100,0"));
    assert!(svg.contains('Z'));
}

#[test]
fn test_to_svg_string_is_stable_and_write_matches() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(100.0, 0.0)
        .line_to(100.0, 100.0)
        .close_path()
        .build();

    let svg1 = path.to_svg_string();
    let svg2 = path.to_svg_string();
    assert_eq!(svg1, svg2);

    let mut buf = String::new();
    path.write_svg_string(&mut buf);
    assert_eq!(buf, svg1);
}

#[test]
fn test_point_distance() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    assert!((p1.distance(&p2) - 5.0).abs() < 1e-10);
}

#[test]
fn test_point_lerp() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(10.0, 20.0);
    let mid = p1.lerp(&p2, 0.5);
    assert!((mid.x - 5.0).abs() < 1e-10);
    assert!((mid.y - 10.0).abs() < 1e-10);
}

#[test]
fn test_distance_to_line() {
    use super::point::distance_to_line;

    let p = Point::new(0.0, 1.0);
    let start = Point::new(0.0, 0.0);
    let end = Point::new(1.0, 0.0);
    assert!((distance_to_line(&p, &start, &end) - 1.0).abs() < 1e-10);
}

#[test]
fn test_distance_to_line_zero_length() {
    use super::point::distance_to_line;

    let p = Point::new(3.0, 4.0);
    let start = Point::new(0.0, 0.0);
    let end = Point::new(0.0, 0.0);
    assert!((distance_to_line(&p, &start, &end) - 5.0).abs() < 1e-10);
}

#[test]
fn test_path_builder_all_commands() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(10.0, 0.0)
        .horizontal_line_to(20.0)
        .vertical_line_to(10.0)
        .quadratic_curve_to(15.0, 5.0, 20.0, 0.0)
        .cubic_curve_to(15.0, -5.0, 10.0, -10.0, 5.0, -5.0)
        .arc(0.0, 0.0, 5.0, 0.0, std::f64::consts::PI, false)
        .elliptical_arc(5.0, 5.0, 0.0, false, false, 0.0, 0.0)
        .rect(-5.0, -5.0, 10.0, 10.0)
        .close_path()
        .build();

    assert!(!path.commands().is_empty());
}

#[test]
fn test_path_bounds_with_curves() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .quadratic_curve_to(50.0, 100.0, 100.0, 0.0)
        .build();

    let bounds = path.bounds().unwrap();
    assert!(bounds.0 <= 0.0);
    assert!(bounds.1 <= 0.0);
    assert!(bounds.2 >= 100.0);
    assert!(bounds.3 >= 100.0);
}

#[test]
fn test_path_flatten_quadratic() {
    use super::flatten::flatten_quadratic;

    let p0 = Point::new(0.0, 0.0);
    let p1 = Point::new(50.0, 100.0);
    let p2 = Point::new(100.0, 0.0);
    let mut points = vec![p0];
    flatten_quadratic(&p0, &p1, &p2, 1.0, &mut points);
    assert!(points.len() > 2);
}

#[test]
fn test_path_flatten_cubic() {
    use super::flatten::flatten_cubic;

    let p0 = Point::new(0.0, 0.0);
    let p1 = Point::new(30.0, 100.0);
    let p2 = Point::new(70.0, 100.0);
    let p3 = Point::new(100.0, 0.0);
    let mut points = vec![p0];
    flatten_cubic(&p0, &p1, &p2, &p3, 1.0, &mut points);
    assert!(points.len() > 2);
}

#[test]
fn test_path_flatten_arc() {
    use super::flatten::flatten_arc;

    let mut points = Vec::new();
    flatten_arc(
        0.0,
        0.0,
        10.0,
        0.0,
        std::f64::consts::PI,
        false,
        1.0,
        &mut points,
    );
    assert!(!points.is_empty());
}

#[test]
fn test_path_flatten_arc_anticlockwise() {
    use super::flatten::flatten_arc;

    let mut points = Vec::new();
    flatten_arc(
        0.0,
        0.0,
        10.0,
        0.0,
        std::f64::consts::PI,
        true,
        1.0,
        &mut points,
    );
    assert!(!points.is_empty());
}

#[test]
fn test_path_svg_all_commands() {
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(10.0, 0.0)
        .horizontal_line_to(20.0)
        .vertical_line_to(10.0)
        .quadratic_curve_to(15.0, 5.0, 20.0, 0.0)
        .cubic_curve_to(15.0, -5.0, 10.0, -10.0, 5.0, -5.0)
        .rect(0.0, 0.0, 10.0, 10.0)
        .close_path()
        .build();

    let svg = path.to_svg_string();
    assert!(svg.contains('M'));
    assert!(svg.contains('L'));
    assert!(svg.contains('H'));
    assert!(svg.contains('V'));
    assert!(svg.contains('Q'));
    assert!(svg.contains('C'));
    assert!(svg.contains('Z'));
}
