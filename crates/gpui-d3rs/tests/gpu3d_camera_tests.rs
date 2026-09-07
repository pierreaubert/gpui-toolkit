use d3rs::gpu3d::{
    Camera3D, CartesianGridLineDebugKind, OrbitControls, Projection, StandardView, Surface3DConfig,
    SurfaceData, cartesian_grid_lines_for_testing, projected_surface_depth_visibility_for_testing,
    transparent_surface_clear_color_for_testing, unpremultiply_rgba_for_testing,
};
use d3rs::mesh::MeshBounds;
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

#[test]
fn orthographic_projection_preserves_parallel_lines() {
    let cam = Camera3D::with_projection(Projection::Orthographic { half_height: 5.0 })
        .with_position(Vec3::new(0.0, 0.0, 10.0))
        .with_target(Vec3::ZERO);
    let m = cam.view_projection_matrix();
    let a = m * glam::Vec4::new(1.0, 0.0, 0.0, 1.0);
    let b = m * glam::Vec4::new(1.0, 0.0, -100.0, 1.0);
    assert!((a.x / a.w - b.x / b.w).abs() < 1e-5);
}

#[test]
fn standard_views_are_axis_aligned() {
    let mut controls = OrbitControls::default();
    controls.set_standard_view(StandardView::Top);
    let dir = controls.view_direction();
    assert!((dir.z.abs() - 1.0).abs() < 1e-5);
    controls.set_standard_view(StandardView::Front);
    let dir = controls.view_direction();
    assert!((dir.y.abs() - 1.0).abs() < 1e-5);
}

#[test]
fn fit_to_bounds_contains_all_corners() {
    let mut controls = OrbitControls::default();
    let bounds = MeshBounds {
        min: [-1.0, -2.0, -0.5],
        max: [3.0, 2.0, 0.5],
    };
    controls.fit_to_bounds(bounds, 16.0 / 9.0);

    let camera = controls.to_camera().with_aspect(16.0 / 9.0);
    let view_projection = camera.view_projection_matrix();
    for x in [bounds.min[0], bounds.max[0]] {
        for y in [bounds.min[1], bounds.max[1]] {
            for z in [bounds.min[2], bounds.max[2]] {
                let clip = view_projection * Vec3::new(x as f32, y as f32, z as f32).extend(1.0);
                assert!(clip.w > 0.0, "corner should be in front of camera");
                let ndc = clip.truncate() / clip.w;
                assert!(ndc.x.abs() <= 1.0 + 1e-5, "x out of bounds: {ndc:?}");
                assert!(ndc.y.abs() <= 1.0 + 1e-5, "y out of bounds: {ndc:?}");
                assert!(ndc.z >= 0.0 && ndc.z <= 1.0, "z out of bounds: {ndc:?}");
            }
        }
    }
}

#[test]
fn world_per_screen_px_grows_with_distance() {
    let near = Camera3D::new()
        .with_position(Vec3::new(0.0, 0.0, 2.0))
        .with_target(Vec3::ZERO);
    let far = Camera3D::new()
        .with_position(Vec3::new(0.0, 0.0, 8.0))
        .with_target(Vec3::ZERO);
    let near_scale = near
        .world_per_screen_px(Vec3::ZERO, 400.0, 300.0)
        .expect("origin projects");
    let far_scale = far
        .world_per_screen_px(Vec3::ZERO, 400.0, 300.0)
        .expect("origin projects");
    assert!(near_scale > 0.0 && far_scale > 0.0);
    // Four times the distance spans roughly four times the world per pixel.
    assert!(
        (far_scale / near_scale - 4.0).abs() < 0.5,
        "near {near_scale} far {far_scale}"
    );
}

#[test]
fn world_per_screen_px_rejects_unprojectable_anchors() {
    let camera = Camera3D::new()
        .with_position(Vec3::new(0.0, 0.0, 2.0))
        .with_target(Vec3::ZERO);
    assert!(
        camera
            .world_per_screen_px(Vec3::new(0.0, 0.0, 3.0), 400.0, 300.0)
            .is_none()
    );
}

#[test]
fn cartesian_tick_plan_labels_linear_data_plainly() {
    use d3rs::gpu3d::{SurfaceData, cartesian_tick_plan_for_testing};

    // Sinc-demo-shaped linear data: no explicit ticks anywhere.
    let data = SurfaceData::from_function(
        (-3.0 * std::f64::consts::PI, 3.0 * std::f64::consts::PI),
        (-3.0 * std::f64::consts::PI, 3.0 * std::f64::consts::PI),
        5,
        5,
        |x, y| x + y,
    );
    let plan = cartesian_tick_plan_for_testing(&data);

    // X and Y span ±3π: nice step 2 gives nine plain integer labels each.
    for (ticks, labels) in [&plan.x_ticks, &plan.y_ticks]
        .into_iter()
        .zip([&plan.x_labels, &plan.y_labels])
    {
        assert_eq!(ticks.len(), 9, "ticks: {ticks:?}");
        assert_eq!(ticks[0], -8.0);
        assert_eq!(ticks[8], 8.0);
        assert_eq!(labels.len(), 9);
        assert_eq!(labels[0], "-8");
        assert_eq!(labels[8], "8");
        assert!(
            labels.iter().all(|label| {
                !label.contains('k')
                    && !label.contains('°')
                    && !label.contains('d')
                    && !label.contains('B')
            }),
            "plain labels, got {labels:?}"
        );
    }

    // Z spans ±6π: nice step 5, still plain.
    assert_eq!(plan.z_ticks[0], -15.0);
    assert!(plan.z_ticks.len() >= 5);
    assert_eq!(plan.z_labels.len(), plan.z_ticks.len());
    assert!(
        plan.z_labels.iter().all(|label| {
            !label.contains('k')
                && !label.contains('°')
                && !label.contains('d')
                && !label.contains('B')
        }),
        "plain labels, got {:?}",
        plan.z_labels
    );
}

#[test]
fn cartesian_tick_plan_keeps_audio_contract_for_explicit_ticks() {
    use d3rs::gpu3d::{SurfaceData, cartesian_tick_plan_for_testing};

    // Spinorama-style explicit ticks keep today's audio formatting exactly.
    let data = SurfaceData::from_function((20.0, 20000.0), (-180.0, 180.0), 3, 3, |_, _| 0.0)
        .with_x_ticks(vec![100.0, 1000.0, 2000.0])
        .with_y_ticks(vec![-90.0, 0.0, 90.0])
        .with_z_ticks(vec![-40.0, 0.0]);
    let plan = cartesian_tick_plan_for_testing(&data);
    assert_eq!(plan.x_ticks, vec![100.0, 1000.0, 2000.0]);
    assert_eq!(plan.x_labels, vec!["100", "1k", "2k"]);
    assert_eq!(plan.y_labels, vec!["-90°", "0°", "90°"]);
    assert_eq!(plan.z_labels, vec!["-40dB", "0dB"]);
}

#[test]
fn cartesian_tick_plan_formats_fractions_without_fp_dust() {
    use d3rs::gpu3d::{SurfaceData, cartesian_tick_plan_for_testing};

    // Unit range → 0.1 steps: 0.1 + 0.2 accumulates float dust that must
    // never reach a label.
    let data = SurfaceData::from_function((0.0, 1.0), (0.0, 1.0), 2, 2, |_, _| 0.0);
    let plan = cartesian_tick_plan_for_testing(&data);
    assert_eq!(plan.x_ticks.len(), 11);
    assert_eq!(plan.x_labels[3], "0.3");
    assert_eq!(plan.x_labels[7], "0.7");
    assert_eq!(plan.x_labels[10], "1");
}

#[test]
fn billboard_label_opts_default_off() {
    use d3rs::sphere_gallery::SphereGalleryConfig;

    let surface = Surface3DConfig::default();
    assert!(!surface.billboard_labels);
    assert!(surface.billboard_labels(true).billboard_labels);

    let gallery = SphereGalleryConfig::default();
    assert!(!gallery.billboard_labels);
    assert_eq!(gallery.label_size_px, 11.0);
    let enabled = gallery.billboard_labels(true).label_size_px(14.0);
    assert!(enabled.billboard_labels);
    assert_eq!(enabled.label_size_px, 14.0);
}
