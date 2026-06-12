use d3rs::gpu3d::{
    Camera3D, CartesianGridLineDebugKind, Surface3DConfig, SurfaceData,
    cartesian_grid_lines_for_testing, projected_surface_depth_visibility_for_testing,
    transparent_surface_clear_color_for_testing, unpremultiply_rgba_for_testing,
};
use glam::Vec3;

#[test]
fn billboard_axes_are_orthonormal() {
    let camera = Camera3D::default();
    let (right, up) = camera.billboard_axes();
    let forward = camera.forward();

    assert!((right.length() - 1.0).abs() < 1e-5);
    assert!((up.length() - 1.0).abs() < 1e-5);
    assert!(right.dot(up).abs() < 1e-5);
    assert!(right.dot(forward).abs() < 1e-5);
    assert!(up.dot(forward).abs() < 1e-5);
}

#[test]
fn project_to_screen_rejects_points_behind_camera() {
    let camera = Camera3D::new()
        .with_position(Vec3::new(0.0, 0.0, 2.0))
        .with_target(Vec3::ZERO);

    assert!(camera.project_to_screen(Vec3::ZERO, 400.0, 300.0).is_some());
    assert!(
        camera
            .project_to_screen(Vec3::new(0.0, 0.0, 3.0), 400.0, 300.0)
            .is_none()
    );
}

#[test]
fn surface_config_clamps_projected_isoline_upsampling() {
    let low = Surface3DConfig::new().isoline_upsample_factor(0);
    let high = Surface3DConfig::new().isoline_upsample_factor(99);

    assert_eq!(low.isoline_upsample_factor, 1);
    assert_eq!(high.isoline_upsample_factor, 8);
}

#[test]
fn projected_isolines_are_occluded_by_nearer_surface_depth() {
    let triangle = [[2.0, 2.0, 0.40], [18.0, 2.0, 0.40], [2.0, 18.0, 0.40]];

    assert!(
        projected_surface_depth_visibility_for_testing(triangle, [8.0, 8.0, 0.402], 24, 24),
        "an isoline sample near the surface depth should remain visible"
    );
    assert!(
        !projected_surface_depth_visibility_for_testing(triangle, [8.0, 8.0, 0.45], 24, 24),
        "an isoline sample behind a nearer surface triangle should be hidden"
    );
    assert!(
        projected_surface_depth_visibility_for_testing(triangle, [22.0, 22.0, 0.45], 24, 24),
        "samples without projected surface coverage should not be clipped"
    );
}

fn grid_test_data() -> SurfaceData {
    SurfaceData::from_grid(
        vec![20.0, 200.0, 2000.0, 20000.0],
        vec![-180.0, -90.0, 0.0, 90.0, 180.0],
        vec![vec![-40.0; 4]; 5],
    )
    .with_log_x(true)
    .with_z_range(-40.0, 10.0)
    .with_x_ticks(vec![20.0, 200.0, 2000.0, 20000.0])
    .with_y_ticks(vec![-180.0, -90.0, 0.0, 90.0, 180.0])
    .with_z_ticks(vec![-40.0, -20.0, 0.0, 10.0])
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn cartesian_grid_lines_skip_boundary_tick_duplicates() {
    let data = grid_test_data();
    let camera = Camera3D::new()
        .with_position(Vec3::new(2.0, 2.0, 2.0))
        .with_target(Vec3::ZERO);
    let lines = cartesian_grid_lines_for_testing(&data, &camera);
    let x_200 = data.normalize_x(200.0);

    assert!(lines.iter().any(|line| {
        line.kind == CartesianGridLineDebugKind::Major
            && near(line.start[0], x_200)
            && near(line.end[0], x_200)
            && near(line.start[1], -0.5)
            && near(line.end[1], -0.5)
            && near(line.start[2], -1.0)
            && near(line.end[2], 1.0)
    }));
    assert!(!lines.iter().any(|line| {
        line.kind == CartesianGridLineDebugKind::Major
            && near(line.start[0], line.end[0])
            && (near(line.start[0], -1.0) || near(line.start[0], 1.0))
            && near(line.start[1], -0.5)
            && near(line.end[1], -0.5)
            && near(line.start[2], -1.0)
            && near(line.end[2], 1.0)
    }));
}

#[test]
fn cartesian_grid_lines_use_floor_and_far_walls() {
    let data = grid_test_data();
    let camera = Camera3D::new()
        .with_position(Vec3::new(2.0, 2.0, 2.0))
        .with_target(Vec3::ZERO);
    let lines = cartesian_grid_lines_for_testing(&data, &camera);

    assert_eq!(
        lines
            .iter()
            .filter(|line| line.kind == CartesianGridLineDebugKind::Border)
            .count(),
        9
    );
    assert!(lines.iter().any(|line| {
        line.kind != CartesianGridLineDebugKind::Border
            && near(line.start[0], -1.0)
            && near(line.end[0], -1.0)
            && !near(line.start[1], line.end[1])
    }));
    assert!(lines.iter().any(|line| {
        line.kind != CartesianGridLineDebugKind::Border
            && near(line.start[2], -1.0)
            && near(line.end[2], -1.0)
            && !near(line.start[1], line.end[1])
    }));
}

#[test]
fn surface_clear_is_transparent_for_projected_grid_compositing() {
    assert_eq!(
        transparent_surface_clear_color_for_testing(),
        [0.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn transparent_surface_pixels_are_unpremultiplied_for_gpui_image() {
    let mut pixels = vec![32, 64, 96, 128, 10, 20, 30, 255, 8, 9, 10, 0];

    unpremultiply_rgba_for_testing(&mut pixels);

    assert_eq!(
        pixels,
        vec![64, 128, 191, 128, 10, 20, 30, 255, 8, 9, 10, 0]
    );
}
