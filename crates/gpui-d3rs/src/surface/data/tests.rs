use super::surface_data::SurfaceData;
use super::surface_point3_d::SurfacePoint3D;

#[test]
fn test_surface_point_creation() {
    let p = SurfacePoint3D::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(p.x, 1.0);
    assert_eq!(p.y, 2.0);
    assert_eq!(p.z, 3.0);
    assert_eq!(p.t, 4.0);
}

#[test]
fn test_surface_point_from_xyz() {
    let p = SurfacePoint3D::from_xyz(1.0, 2.0, 3.0);
    assert_eq!(p.t, 3.0);
}

#[test]
fn test_from_function() {
    let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 5, |x, y| (x + y, x * y));

    assert_eq!(data.rows(), 5);
    assert_eq!(data.cols(), 5);
    assert_eq!(data.x_range, (-1.0, 1.0));
    assert_eq!(data.y_range, (-1.0, 1.0));
}

#[test]
fn test_from_z_function() {
    let data = SurfaceData::from_z_function((-1.0, 1.0), (-1.0, 1.0), 3, |x, y| x * y);

    let p = data.get(1, 1).unwrap();
    assert_eq!(p.z, p.t);
}

#[test]
fn test_normalize() {
    let data = SurfaceData::from_function((0.0, 10.0), (0.0, 10.0), 3, |x, y| (x + y, x - y));

    let normalized = data.normalize();

    assert_eq!(normalized.x_range, (0.0, 1.0));
    assert_eq!(normalized.y_range, (0.0, 1.0));
    assert_eq!(normalized.z_range, (0.0, 1.0));
    assert_eq!(normalized.t_range, (0.0, 1.0));
}

#[test]
fn test_normalized_t() {
    let _data = SurfaceData::from_function((0.0, 1.0), (0.0, 1.0), 3, |_, _| (0.0, 50.0));
    // t_range should be (50.0, 50.0) -> adjusted to (50.0, 51.0)

    let data2 = SurfaceData::from_function((0.0, 1.0), (0.0, 1.0), 3, |x, y| (0.0, x + y));
    // t should range from 0 to 2

    assert!((data2.normalized_t(0.0) - 0.0).abs() < 1e-10);
    assert!((data2.normalized_t(2.0) - 1.0).abs() < 1e-10);
    assert!((data2.normalized_t(1.0) - 0.5).abs() < 1e-10);
}

#[test]
fn test_from_function_logx() {
    // Create surface with log X-axis (frequency-like)
    let data =
        SurfaceData::from_function_logx((10.0, 1000.0), (0.0, 1.0), 5, |x, y| (x.ln(), x + y));

    assert_eq!(data.rows(), 5);
    assert_eq!(data.cols(), 5);

    // Check that X values are logarithmically spaced
    let x_values: Vec<f64> = (0..5).filter_map(|i| data.get(0, i).map(|p| p.x)).collect();

    // First and last should match range
    assert!((x_values[0] - 10.0).abs() < 1e-6);
    assert!((x_values[4] - 1000.0).abs() < 1e-6);

    // Check logarithmic spacing: log(x[i+1]) - log(x[i]) should be constant
    let log_diffs: Vec<f64> = x_values.windows(2).map(|w| w[1].ln() - w[0].ln()).collect();

    let expected_log_diff = log_diffs[0];
    for diff in &log_diffs {
        assert!((diff - expected_log_diff).abs() < 1e-6);
    }
}

#[test]
fn test_from_function_logy() {
    // Create surface with log Y-axis
    let data =
        SurfaceData::from_function_logy((0.0, 1.0), (10.0, 1000.0), 5, |x, y| (y.ln(), x + y));

    assert_eq!(data.rows(), 5);
    assert_eq!(data.cols(), 5);

    // Check that Y values are logarithmically spaced
    let y_values: Vec<f64> = (0..5).filter_map(|j| data.get(j, 0).map(|p| p.y)).collect();

    assert!((y_values[0] - 10.0).abs() < 1e-6);
    assert!((y_values[4] - 1000.0).abs() < 1e-6);

    // Check logarithmic spacing
    let log_diffs: Vec<f64> = y_values.windows(2).map(|w| w[1].ln() - w[0].ln()).collect();

    let expected_log_diff = log_diffs[0];
    for diff in &log_diffs {
        assert!((diff - expected_log_diff).abs() < 1e-6);
    }
}

#[test]
fn test_from_function_logxy() {
    // Create surface with both axes logarithmic
    let data = SurfaceData::from_function_logxy((20.0, 20000.0), (10.0, 1000.0), 5, |x, y| {
        ((x * y).ln(), x + y)
    });

    assert_eq!(data.rows(), 5);
    assert_eq!(data.cols(), 5);

    // Check X range
    let first_row = 0;
    let x_min = data.get(first_row, 0).unwrap().x;
    let x_max = data.get(first_row, 4).unwrap().x;
    assert!((x_min - 20.0).abs() < 1e-6);
    assert!((x_max - 20000.0).abs() < 1e-6);

    // Check Y range
    let first_col = 0;
    let y_min = data.get(0, first_col).unwrap().y;
    let y_max = data.get(4, first_col).unwrap().y;
    assert!((y_min - 10.0).abs() < 1e-6);
    assert!((y_max - 1000.0).abs() < 1e-6);
}

#[test]
fn test_from_z_function_logx() {
    // Simple case where z = t
    let data = SurfaceData::from_z_function_logx((10.0, 100.0), (0.0, 1.0), 5, |x, _y| x.ln());

    // Check that z equals t for all points
    for row in 0..data.rows() {
        for col in 0..data.cols() {
            let p = data.get(row, col).unwrap();
            assert!((p.z - p.t).abs() < 1e-10);
        }
    }

    // Check X is logarithmic
    let x_values: Vec<f64> = (0..5).filter_map(|i| data.get(0, i).map(|p| p.x)).collect();
    assert!((x_values[0] - 10.0).abs() < 1e-6);
    assert!((x_values[4] - 100.0).abs() < 1e-6);
}

#[test]
fn test_from_z_function_logy() {
    let data = SurfaceData::from_z_function_logy((0.0, 1.0), (10.0, 100.0), 5, |_x, y| y.ln());

    // Check that z equals t
    for row in 0..data.rows() {
        for col in 0..data.cols() {
            let p = data.get(row, col).unwrap();
            assert!((p.z - p.t).abs() < 1e-10);
        }
    }
}

#[test]
fn test_from_z_function_logxy() {
    let data =
        SurfaceData::from_z_function_logxy((10.0, 100.0), (10.0, 100.0), 5, |x, y| (x * y).ln());

    // Check that z equals t
    for row in 0..data.rows() {
        for col in 0..data.cols() {
            let p = data.get(row, col).unwrap();
            assert!((p.z - p.t).abs() < 1e-10);
        }
    }

    // Check ranges are correct
    assert!((data.x_range.0 - 10.0).abs() < 1e-6);
    assert!((data.x_range.1 - 100.0).abs() < 1e-6);
    assert!((data.y_range.0 - 10.0).abs() < 1e-6);
    assert!((data.y_range.1 - 100.0).abs() < 1e-6);
}

#[test]
fn test_logx_frequency_response() {
    // Simulate a typical audio frequency response plot
    let data = SurfaceData::from_z_function_logx(
        (20.0, 20000.0), // 20 Hz to 20 kHz
        (0.0, 1.0),
        100,
        |freq, _| {
            // Simulated frequency response
            if freq < 100.0 {
                -6.0 // Low frequency rolloff
            } else if freq > 10000.0 {
                -3.0 // High frequency rolloff
            } else {
                0.0 // Flat response
            }
        },
    );

    assert_eq!(data.rows(), 100);
    assert_eq!(data.cols(), 100);
    assert!((data.x_range.0 - 20.0).abs() < 0.1);
    assert!((data.x_range.1 - 20000.0).abs() < 1.0);
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn test_logx_negative_range_panics() {
    SurfaceData::from_function_logx((-10.0, 10.0), (0.0, 1.0), 5, |x, y| (x, y));
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn test_logy_negative_range_panics() {
    SurfaceData::from_function_logy((0.0, 1.0), (-10.0, 10.0), 5, |x, y| (x, y));
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn test_logxy_negative_x_range_panics() {
    SurfaceData::from_function_logxy((-10.0, 10.0), (10.0, 100.0), 5, |x, y| (x, y));
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn test_logxy_negative_y_range_panics() {
    SurfaceData::from_function_logxy((10.0, 100.0), (-10.0, 10.0), 5, |x, y| (x, y));
}
