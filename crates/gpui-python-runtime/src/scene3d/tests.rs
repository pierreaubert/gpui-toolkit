use super::camera_spec::CameraSpec;
use super::color_rgba::ColorRgba;
use super::line_segment_spec::LineSegmentSpec;
use super::line_strip_spec::LineStripSpec;
use super::lines_spec::LinesSpec;
use super::material_spec::MaterialSpec;
use super::mesh_spec::MeshSpec;
use super::orbit_camera_spec::OrbitCameraSpec;
use super::point3::Point3;
use super::scene_node::SceneNode;
use super::scene_spec::SceneSpec;
use super::surface_spec::SurfaceSpec;
use crate::error::Scene3DError;
use std::borrow::Cow;

#[test]
fn surface_x_values_returns_borrowed_for_explicit_axis() {
    let mut spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    spec.x = Some(vec![10.0, 20.0]);
    match spec.x_values() {
        Cow::Borrowed(values) => assert_eq!(values, &[10.0, 20.0]),
        Cow::Owned(_) => panic!("expected borrowed explicit x values"),
    }
}

#[test]
fn surface_default_axis_values_are_cached() {
    let spec_a = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let spec_b = SurfaceSpec::from_flat("other", vec![5.0, 6.0, 7.0, 8.0], 2, 2);

    let x_a: Vec<f64> = spec_a.x_values().into();
    let x_b: Vec<f64> = spec_b.x_values().into();
    assert_eq!(x_a, x_b);
    assert_eq!(x_a, vec![0.0, 1.0]);

    let y_a: Vec<f64> = spec_a.y_values().into();
    let y_b: Vec<f64> = spec_b.y_values().into();
    assert_eq!(y_a, y_b);
    assert_eq!(y_a, vec![0.0, 1.0]);
}

#[test]
fn surface_validates_grid_dimensions() {
    let spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0], 2, 2);
    assert!(matches!(
        spec.validate(),
        Err(Scene3DError::GridDimensionMismatch {
            z_len: 3,
            width: 2,
            height: 2,
            expected: 4
        })
    ));
}

#[test]
fn surface_validates_log_axis_positivity() {
    let mut spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    spec.x = Some(vec![0.0, 10.0]);
    spec.x_log = true;

    assert!(matches!(
        spec.validate(),
        Err(Scene3DError::InvalidData {
            field: "x",
            reason: "contains non-positive values for log scale"
        })
    ));
}

#[test]
fn surface_requires_monotonic_axes() {
    let mut spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    spec.y = Some(vec![2.0, 1.0]);

    assert!(matches!(
        spec.validate(),
        Err(Scene3DError::InvalidData {
            field: "y",
            reason: "must be strictly monotonically increasing"
        })
    ));
}

#[test]
fn surface_from_rows_flattens_row_major_data() {
    let spec = SurfaceSpec::from_rows("surface", vec![vec![1.0, 2.0], vec![3.0, 4.0]])
        .expect("valid surface");

    assert_eq!(spec.z.width, 2);
    assert_eq!(spec.z.height, 2);
    assert_eq!(spec.z.values, vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(
        spec.z.rows().collect::<Vec<_>>(),
        vec![&[1.0, 2.0], &[3.0, 4.0]]
    );
}

#[test]
fn mesh_rejects_invalid_indices() {
    let spec = MeshSpec {
        id: "mesh".to_string(),
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        indices: vec![0, 1, 3],
        material: MaterialSpec::default(),
        scalar_field: None,
    };

    assert!(matches!(
        spec.validate(),
        Err(Scene3DError::InvalidMeshIndex {
            position: 2,
            index: 3,
            vertex_count: 3
        })
    ));
}

#[test]
fn line_strip_expands_to_segments() {
    let strip = LineStripSpec {
        id: "path".to_string(),
        points: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ],
        color: ColorRgba::from_rgb_u8(255, 255, 255),
        width: 2.0,
    };

    let segments: Vec<_> = strip.to_segments().collect();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].from, strip.points[0]);
    assert_eq!(segments[1].to, strip.points[2]);
}

#[test]
fn serde_uses_snake_case_tags() {
    let camera = CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 60.0, 25.0));
    let value = serde_json::to_value(camera).expect("camera json");

    assert_eq!(value["kind"], "orbit");
    assert_eq!(value["distance"], 3.5);
}

#[test]
fn color_hex_parses_alpha() {
    let color = ColorRgba::from_hex("#33669980").expect("color");

    assert!((color.r - 0x33 as f32 / 255.0).abs() < f32::EPSILON);
    assert!((color.a - 0x80 as f32 / 255.0).abs() < f32::EPSILON);
}

#[test]
fn lines_flatten_mixed_segments_and_strips() {
    let lines = LinesSpec {
        id: "lines".to_string(),
        segments: vec![LineSegmentSpec {
            from: Point3::new(0.0, 0.0, 0.0),
            to: Point3::new(1.0, 0.0, 0.0),
            color: ColorRgba::from_rgb_u8(255, 0, 0),
            width: 1.0,
        }],
        strips: vec![LineStripSpec {
            id: "strip".to_string(),
            points: vec![
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(3.0, 0.0, 0.0),
                Point3::new(4.0, 0.0, 0.0),
            ],
            color: ColorRgba::from_rgb_u8(0, 255, 0),
            width: 2.0,
        }],
        ..LinesSpec::default()
    };

    let flattened = lines.flattened_segments();
    assert_eq!(flattened.len(), 3);
    assert_eq!(flattened[0].from, Point3::new(0.0, 0.0, 0.0));
    assert_eq!(flattened[1].from, Point3::new(2.0, 0.0, 0.0));
    assert_eq!(flattened[2].from, Point3::new(3.0, 0.0, 0.0));
}

#[test]
fn scene_fingerprints_are_stable_for_unchanged_spec() {
    let scene = SceneSpec {
        id: "scene".to_string(),
        camera: CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 60.0, 25.0)),
        children: vec![SceneNode::Surface(SurfaceSpec::from_flat(
            "surface",
            vec![1.0, 2.0, 3.0, 4.0],
            2,
            2,
        ))],
        interactions: vec![],
        background: None,
        size: None,
    };

    let first = scene.fingerprints();
    let second = scene.fingerprints();

    assert_eq!(first.geometry, second.geometry);
    assert_eq!(first.material, second.material);
    assert_eq!(first.camera, second.camera);
}

#[test]
fn scene_fingerprints_differ_when_child_changes() {
    let mut scene = SceneSpec {
        id: "scene".to_string(),
        camera: CameraSpec::default(),
        children: vec![SceneNode::Surface(SurfaceSpec::from_flat(
            "surface",
            vec![1.0, 2.0, 3.0, 4.0],
            2,
            2,
        ))],
        interactions: vec![],
        background: None,
        size: None,
    };

    let original = scene.fingerprints();
    if let SceneNode::Surface(surface) = &mut scene.children[0] {
        surface.colormap = super::colormap_spec::ColormapSpec::Turbo;
    }
    let changed = scene.fingerprints();

    assert_ne!(original.material, changed.material);
}

#[test]
fn validate_helpers_reject_invalid_data() {
    use super::validate::*;

    assert!(validate_id("", "id").is_err());
    assert!(validate_finite_f32(f32::NAN, "v").is_err());
    assert!(validate_positive_f32(-1.0, "v").is_err());
    assert!(validate_positive_f32(f32::NAN, "v").is_err());
    assert!(validate_finite_f64_slice(&[], "v").is_err());
    assert!(validate_finite_f64_slice(&[1.0, f64::NAN], "v").is_err());
    assert!(validate_positive_f64_slice(&[1.0, 0.0], "v").is_err());
    assert!(validate_monotonic(&[1.0, 1.0], "v").is_err());
    assert!(validate_axis(&[1.0, 2.0], 3, "x", "y", false).is_err());
    assert!(validate_axis(&[2.0, 1.0], 2, "x", "y", false).is_err());
    assert!(validate_axis(&[0.0, 1.0], 2, "x", "y", true).is_err());
}

#[test]
fn color_rgba_parses_and_validates() {
    use super::color_rgba::ColorRgba;

    let white = ColorRgba::from_rgb_u8(255, 255, 255);
    assert_eq!(white.r, 1.0);
    assert_eq!(white.a, 1.0);

    let transparent = ColorRgba::from_hex("#33669980").unwrap();
    assert!((transparent.a - 0x80 as f32 / 255.0).abs() < f32::EPSILON);

    assert!(ColorRgba::from_hex("336699").is_err());
    assert!(ColorRgba::from_hex("#xyz").is_err());
    assert!(ColorRgba::from_hex("#12345").is_err());

    let mut invalid = ColorRgba::new(2.0, 0.0, 0.0, 1.0);
    assert!(invalid.validate("c").is_err());
    invalid = ColorRgba::new(f32::NAN, 0.0, 0.0, 1.0);
    assert!(invalid.validate("c").is_err());
}

#[test]
fn point3_scalar_range_viewport_size_validate() {
    use super::point3::Point3;
    use super::scalar_range::ScalarRange;
    use super::viewport_size::ViewportSize;

    assert!(Point3::new(0.0, 0.0, 0.0).validate("p").is_ok());
    assert!(Point3::new(f32::NAN, 0.0, 0.0).validate("p").is_err());

    assert!(ScalarRange::new(0.0, 1.0).validate("r").is_ok());
    assert!(ScalarRange::new(1.0, 1.0).validate("r").is_err());
    assert!(ScalarRange::new(f64::NAN, 1.0).validate("r").is_err());
    assert!(ScalarRange::new(0.0, 1.0).validate_positive("r").is_err());
    assert!(ScalarRange::new(1.0, 2.0).validate_positive("r").is_ok());

    assert!(ViewportSize::new(100.0, 50.0).validate().is_ok());
    assert!(ViewportSize::new(f32::NAN, 50.0).validate().is_err());
    assert!(ViewportSize::new(100.0, 0.0).validate().is_err());
}

#[test]
fn interaction_and_colormap_parse() {
    use super::colormap_spec::ColormapSpec;
    use super::interaction_mode::InteractionMode;

    assert_eq!(
        InteractionMode::parse("orbit").unwrap(),
        InteractionMode::Orbit
    );
    assert_eq!(
        InteractionMode::parse("hit-test").unwrap(),
        InteractionMode::HitTest
    );
    assert_eq!(
        InteractionMode::parse("  HitTest  ").unwrap(),
        InteractionMode::HitTest
    );
    assert!(InteractionMode::parse("unknown").is_err());

    assert_eq!(
        ColormapSpec::parse("viridis").unwrap(),
        ColormapSpec::Viridis
    );
    assert_eq!(
        ColormapSpec::parse("cool-warm").unwrap(),
        ColormapSpec::CoolWarm
    );
    assert_eq!(
        ColormapSpec::parse("cool_warm").unwrap(),
        ColormapSpec::CoolWarm
    );
    assert!(ColormapSpec::parse("magma").is_err());
}

#[test]
fn light_spec_validates() {
    use super::color_rgba::ColorRgba;
    use super::light_spec::LightSpec;
    use super::point3::Point3;

    let valid = LightSpec {
        id: "light".to_string(),
        direction: Point3::new(0.0, -1.0, 0.0),
        intensity: 1.0,
        color: ColorRgba::default(),
    };
    assert!(valid.validate().is_ok());

    let mut invalid = valid.clone();
    invalid.id = "".to_string();
    assert!(invalid.validate().is_err());

    invalid = valid.clone();
    invalid.direction = Point3::new(f32::NAN, 0.0, 0.0);
    assert!(invalid.validate().is_err());

    invalid = valid.clone();
    invalid.intensity = 0.0;
    assert!(invalid.validate().is_err());

    let _ = valid.fingerprints();
}

#[test]
fn line_specs_validate_and_flatten() {
    use super::color_rgba::ColorRgba;
    use super::line_segment_spec::LineSegmentSpec;
    use super::line_strip_spec::LineStripSpec;
    use super::lines_spec::LinesSpec;
    use super::point3::Point3;

    let segment = LineSegmentSpec {
        from: Point3::new(0.0, 0.0, 0.0),
        to: Point3::new(1.0, 0.0, 0.0),
        color: ColorRgba::default(),
        width: 1.0,
    };
    assert!(segment.validate().is_ok());

    let mut bad_segment = segment.clone();
    bad_segment.width = -1.0;
    assert!(bad_segment.validate().is_err());

    let strip = LineStripSpec {
        id: "strip".to_string(),
        points: vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
        color: ColorRgba::default(),
        width: 1.0,
    };
    assert!(strip.validate().is_ok());
    assert_eq!(strip.to_segments().count(), 1);

    let mut bad_strip = strip.clone();
    bad_strip.points.pop();
    assert!(bad_strip.validate().is_err());

    let lines = LinesSpec {
        id: "lines".to_string(),
        segments: vec![segment],
        strips: vec![strip],
        background: None,
        camera: None,
        interactions: vec![],
        size: None,
    };
    assert!(lines.validate().is_ok());
    assert_eq!(lines.flattened_segments().len(), 2);

    let empty_lines = LinesSpec {
        id: "empty".to_string(),
        segments: vec![],
        strips: vec![],
        ..LinesSpec::default()
    };
    assert!(empty_lines.validate().is_err());
}

#[test]
fn mesh_spec_validates() {
    use super::material_spec::MaterialSpec;
    use super::mesh_spec::MeshSpec;
    use super::point3::Point3;

    let valid = MeshSpec {
        id: "mesh".to_string(),
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        indices: vec![0, 1, 2],
        material: MaterialSpec::default(),
        scalar_field: None,
    };
    assert!(valid.validate().is_ok());
    let _ = valid.fingerprints();

    let mut scalar_mesh = valid.clone();
    scalar_mesh.scalar_field = Some(super::mesh_spec::MeshScalarField {
        values: vec![0.0, 0.5, 1.0],
        association: super::mesh_spec::ScalarAssociation::Vertex,
        colormap: super::colormap_spec::ColormapSpec::Turbo,
        range: Some(super::scalar_range::ScalarRange::new(0.0, 1.0)),
        label: Some("Pressure (Pa)".into()),
    });
    assert!(scalar_mesh.validate().is_ok());
    scalar_mesh.scalar_field.as_mut().unwrap().values.pop();
    assert!(scalar_mesh.validate().is_err());

    let mut bad = valid.clone();
    bad.vertices.clear();
    assert!(bad.validate().is_err());

    bad = valid.clone();
    bad.indices = vec![0, 1];
    assert!(bad.validate().is_err());

    bad = valid.clone();
    bad.indices = vec![0, 1, 3];
    assert!(bad.validate().is_err());

    bad = valid.clone();
    bad.vertices[0] = Point3::new(f32::NAN, 0.0, 0.0);
    assert!(bad.validate().is_err());
}

#[test]
fn surface_spec_validates_and_reads_axes() {
    use super::camera_spec::CameraSpec;
    use super::orbit_camera_spec::OrbitCameraSpec;
    use super::scalar_range::ScalarRange;
    use super::surface_spec::SurfaceSpec;
    use super::viewport_size::ViewportSize;

    let mut spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    assert!(spec.validate().is_ok());
    assert_eq!(spec.x_values().as_ref(), &[0.0, 1.0]);
    assert_eq!(spec.y_values().as_ref(), &[0.0, 1.0]);

    spec.x = Some(vec![10.0, 20.0]);
    assert_eq!(spec.x_values().as_ref(), &[10.0, 20.0]);

    let mut bad = spec.clone();
    bad.id = "".to_string();
    assert!(bad.validate().is_err());

    bad = spec.clone();
    bad.x = None;
    bad.x_log = true;
    assert!(bad.validate().is_err());

    bad = spec.clone();
    bad.x = Some(vec![10.0, 20.0, 30.0]);
    assert!(bad.validate().is_err());

    bad = spec.clone();
    bad.z_range = Some(ScalarRange::new(1.0, 0.0));
    assert!(bad.validate().is_err());

    bad = spec.clone();
    bad.camera = Some(CameraSpec::Orbit(OrbitCameraSpec {
        distance: -1.0,
        ..OrbitCameraSpec::default()
    }));
    assert!(bad.validate().is_err());

    bad = spec.clone();
    bad.size = Some(ViewportSize::new(0.0, 100.0));
    assert!(bad.validate().is_err());

    let _ = spec.fingerprints();
}

#[test]
fn surface_from_rows_rejects_bad_shapes() {
    use super::surface_spec::SurfaceSpec;

    assert!(SurfaceSpec::from_rows("bad", vec![]).is_err());
    assert!(SurfaceSpec::from_rows("bad", vec![vec![]]).is_err());
    assert!(SurfaceSpec::from_rows("bad", vec![vec![1.0, 2.0], vec![3.0]]).is_err());
}

#[test]
fn scene_spec_validates() {
    use super::scene_node::SceneNode;
    use super::scene_spec::SceneSpec;

    let valid = SceneSpec {
        id: "scene".to_string(),
        camera: super::camera_spec::CameraSpec::default(),
        children: vec![SceneNode::Surface(SurfaceSpec::from_flat(
            "surface",
            vec![1.0, 2.0, 3.0, 4.0],
            2,
            2,
        ))],
        interactions: vec![],
        background: None,
        size: None,
    };
    assert!(valid.validate().is_ok());
    let _ = valid.fingerprints();

    let mut bad = valid.clone();
    bad.id = "".to_string();
    assert!(bad.validate().is_err());

    bad = valid.clone();
    bad.children.clear();
    assert!(bad.validate().is_err());
}

#[test]
fn scene_node_dispatches() {
    use super::scene_node::SceneNode;

    let surface = SceneNode::Surface(SurfaceSpec::from_flat(
        "surface",
        vec![1.0, 2.0, 3.0, 4.0],
        2,
        2,
    ));
    assert_eq!(surface.id(), "surface");
    assert!(surface.validate().is_ok());
    let _ = surface.fingerprints();
}

#[test]
fn camera_specs_validate() {
    use super::camera_spec::CameraSpec;
    use super::orbit_camera_spec::OrbitCameraSpec;
    use super::perspective_camera_spec::PerspectiveCameraSpec;

    let orbit = OrbitCameraSpec::new(3.5, 60.0, 25.0);
    assert!(orbit.validate().is_ok());

    let mut bad = orbit.clone();
    bad.distance = 0.0;
    assert!(bad.validate().is_err());

    bad = orbit.clone();
    bad.near = 10.0;
    bad.far = 1.0;
    assert!(bad.validate().is_err());

    let perspective = PerspectiveCameraSpec::default();
    assert!(perspective.validate().is_ok());

    let mut bad_perspective = perspective.clone();
    bad_perspective.fov_y_deg = -45.0;
    assert!(bad_perspective.validate().is_err());

    let camera = CameraSpec::Orbit(orbit.clone());
    assert!(camera.validate().is_ok());
    assert!(camera.as_orbit().is_some());
}

#[test]
fn grid_data_validates() {
    use super::grid_data::GridData;

    let grid = GridData::from_flat(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    assert!(grid.validate().is_ok());
    assert_eq!(grid.as_flat().0, &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(grid.rows().count(), 2);

    let mut bad = grid.clone();
    bad.values.push(5.0);
    assert!(bad.validate().is_err());

    assert!(GridData::from_rows(vec![]).is_err());
    assert!(GridData::from_rows(vec![vec![]]).is_err());
    assert!(GridData::from_rows(vec![vec![1.0, 2.0], vec![3.0]]).is_err());
}

#[test]
fn material_and_line_defaults_via_serde() {
    use super::light_spec::LightSpec;
    use super::line_segment_spec::LineSegmentSpec;
    use super::material_spec::MaterialSpec;

    let material: MaterialSpec =
        serde_json::from_str("{\"color\":{\"r\":1.0,\"g\":1.0,\"b\":1.0,\"a\":1.0}}").unwrap();
    assert!(material.validate().is_ok());
    assert_eq!(material.opacity, 1.0);

    let segment: LineSegmentSpec =
        serde_json::from_str(r#"{"from":[0,0,0],"to":[1,0,0],"color":{"r":1,"g":1,"b":1,"a":1}}"#)
            .unwrap();
    assert!(segment.validate().is_ok());
    assert!(segment.width > 0.0);

    let light: LightSpec = serde_json::from_str(r#"{"id":"light","direction":[0,-1,0]}"#).unwrap();
    assert_eq!(light.intensity, 1.0);
    assert!(light.validate().is_ok());
}

#[test]
fn hash_functions_run_without_panic() {
    use super::color_rgba::ColorRgba;
    use super::grid_data::GridData;
    use super::line_segment_spec::LineSegmentSpec;
    use super::line_strip_spec::LineStripSpec;
    use super::lines_spec::LinesSpec;
    use super::material_spec::MaterialSpec;
    use super::mesh_spec::MeshSpec;
    use super::point3::Point3;
    use super::scalar_range::ScalarRange;
    use super::viewport_size::ViewportSize;
    use std::collections::hash_map::DefaultHasher;

    let mut h = DefaultHasher::new();
    ColorRgba::from_rgb_u8(255, 0, 0).hash_into(&mut h);
    Point3::new(1.0, 2.0, 3.0).hash_into(&mut h);
    ScalarRange::new(0.0, 1.0).hash_into(&mut h);
    ViewportSize::new(100.0, 100.0).hash_into(&mut h);

    let segment = LineSegmentSpec {
        from: Point3::new(0.0, 0.0, 0.0),
        to: Point3::new(1.0, 0.0, 0.0),
        color: ColorRgba::default(),
        width: 1.0,
    };
    segment.hash_into(&mut h);

    let strip = LineStripSpec {
        id: "s".to_string(),
        points: vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
        color: ColorRgba::default(),
        width: 1.0,
    };
    strip.hash_into(&mut h);

    let lines = LinesSpec {
        id: "l".to_string(),
        segments: vec![segment],
        strips: vec![strip],
        background: Some(ColorRgba::default()),
        camera: None,
        interactions: vec![],
        size: Some(ViewportSize::new(100.0, 100.0)),
    };
    lines.fingerprints();

    MaterialSpec::default().hash_into(&mut h);
    GridData::from_flat(vec![1.0, 2.0, 3.0, 4.0], 2, 2).hash_into(&mut h);

    let mesh = MeshSpec {
        id: "m".to_string(),
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        indices: vec![0, 1, 2],
        material: MaterialSpec::default(),
        scalar_field: None,
    };
    mesh.fingerprints();
}

#[test]
fn scene_node_dispatches_all_kinds() {
    use super::color_rgba::ColorRgba;
    use super::light_spec::LightSpec;
    use super::line_segment_spec::LineSegmentSpec;
    use super::lines_spec::LinesSpec;
    use super::material_spec::MaterialSpec;
    use super::mesh_spec::MeshSpec;
    use super::point3::Point3;
    use super::scene_node::SceneNode;

    let surface = SceneNode::Surface(SurfaceSpec::from_flat("s", vec![1.0, 2.0, 3.0, 4.0], 2, 2));
    assert_eq!(surface.id(), "s");
    assert!(surface.validate().is_ok());
    let _ = surface.fingerprints();

    let lines = SceneNode::Lines(LinesSpec {
        id: "l".to_string(),
        segments: vec![LineSegmentSpec {
            from: Point3::new(0.0, 0.0, 0.0),
            to: Point3::new(1.0, 0.0, 0.0),
            color: ColorRgba::default(),
            width: 1.0,
        }],
        strips: vec![],
        background: None,
        camera: None,
        interactions: vec![],
        size: None,
    });
    assert_eq!(lines.id(), "l");
    assert!(lines.validate().is_ok());
    let _ = lines.fingerprints();

    let mesh = SceneNode::Mesh(MeshSpec {
        id: "m".to_string(),
        vertices: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        indices: vec![0, 1, 2],
        material: MaterialSpec::default(),
        scalar_field: None,
    });
    assert_eq!(mesh.id(), "m");
    assert!(mesh.validate().is_ok());
    let _ = mesh.fingerprints();

    let light = SceneNode::Light(LightSpec {
        id: "li".to_string(),
        direction: Point3::new(0.0, -1.0, 0.0),
        intensity: 1.0,
        color: ColorRgba::default(),
    });
    assert_eq!(light.id(), "li");
    assert!(light.validate().is_ok());
    let _ = light.fingerprints();
}

#[test]
fn perspective_camera_spec_validates() {
    use super::camera_spec::CameraSpec;
    use super::perspective_camera_spec::PerspectiveCameraSpec;
    use super::point3::Point3;

    let mut cam = PerspectiveCameraSpec::default();
    assert!(cam.validate().is_ok());

    cam.position = Point3::new(f32::NAN, 0.0, 0.0);
    assert!(cam.validate().is_err());

    cam = PerspectiveCameraSpec::default();
    cam.target = Point3::new(f32::NAN, 0.0, 0.0);
    assert!(cam.validate().is_err());

    cam = PerspectiveCameraSpec::default();
    cam.up = Point3::new(f32::NAN, 0.0, 0.0);
    assert!(cam.validate().is_err());

    cam = PerspectiveCameraSpec::default();
    cam.near = 10.0;
    cam.far = 1.0;
    assert!(cam.validate().is_err());

    let spec = CameraSpec::Perspective(PerspectiveCameraSpec::default());
    assert!(spec.validate().is_ok());
    assert!(spec.as_orbit().is_none());
    let mut h = std::collections::hash_map::DefaultHasher::new();
    spec.hash_into(&mut h);
}

#[test]
fn viewport_size_and_scalar_range_edge_cases() {
    use super::scalar_range::ScalarRange;
    use super::viewport_size::ViewportSize;

    assert!(ViewportSize::new(1.0, f32::NAN).validate().is_err());
    let mut h = std::collections::hash_map::DefaultHasher::new();
    ViewportSize::new(1.0, 1.0).hash_into(&mut h);

    assert!(
        ScalarRange::new(f64::NAN, f64::NAN)
            .validate_positive("r")
            .is_err()
    );
    ScalarRange::new(1.0, 2.0).hash_into(&mut h);
}

#[test]
fn lines_spec_validates_optional_fields() {
    use super::camera_spec::CameraSpec;
    use super::color_rgba::ColorRgba;
    use super::line_segment_spec::LineSegmentSpec;
    use super::lines_spec::LinesSpec;
    use super::orbit_camera_spec::OrbitCameraSpec;
    use super::point3::Point3;
    use super::viewport_size::ViewportSize;

    let base = LinesSpec {
        id: "l".to_string(),
        segments: vec![LineSegmentSpec {
            from: Point3::new(0.0, 0.0, 0.0),
            to: Point3::new(1.0, 0.0, 0.0),
            color: ColorRgba::default(),
            width: 1.0,
        }],
        strips: vec![],
        background: None,
        camera: None,
        interactions: vec![],
        size: None,
    };

    let mut bad = base.clone();
    bad.background = Some(ColorRgba::new(f32::NAN, 0.0, 0.0, 1.0));
    assert!(bad.validate().is_err());

    bad = base.clone();
    bad.camera = Some(CameraSpec::Orbit(OrbitCameraSpec {
        distance: -1.0,
        ..OrbitCameraSpec::default()
    }));
    assert!(bad.validate().is_err());

    bad = base.clone();
    bad.size = Some(ViewportSize::new(0.0, 1.0));
    assert!(bad.validate().is_err());
}

#[test]
fn surface_spec_y_log_and_z_range_positive() {
    use super::scalar_range::ScalarRange;
    use super::surface_spec::SurfaceSpec;

    let mut spec = SurfaceSpec::from_flat("s", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    spec.y_log = true;
    assert!(spec.validate().is_err());

    spec.y = Some(vec![1.0, 2.0]);
    spec.y_log = true;
    assert!(spec.validate().is_ok());

    spec.z_range = Some(ScalarRange::new(1.0, 2.0));
    assert!(spec.validate().is_ok());
}

#[test]
fn scene_spec_validates_optional_fields() {
    use super::camera_spec::CameraSpec;
    use super::color_rgba::ColorRgba;
    use super::orbit_camera_spec::OrbitCameraSpec;
    use super::scene_node::SceneNode;
    use super::scene_spec::SceneSpec;
    use super::viewport_size::ViewportSize;

    let base = SceneSpec {
        id: "scene".to_string(),
        camera: CameraSpec::default(),
        children: vec![SceneNode::Surface(SurfaceSpec::from_flat(
            "s",
            vec![1.0, 2.0, 3.0, 4.0],
            2,
            2,
        ))],
        interactions: vec![],
        background: None,
        size: None,
    };

    let mut bad = base.clone();
    bad.background = Some(ColorRgba::new(f32::NAN, 0.0, 0.0, 1.0));
    assert!(bad.validate().is_err());

    bad = base.clone();
    bad.camera = CameraSpec::Orbit(OrbitCameraSpec {
        distance: -1.0,
        ..OrbitCameraSpec::default()
    });
    assert!(bad.validate().is_err());

    bad = base.clone();
    bad.size = Some(ViewportSize::new(0.0, 1.0));
    assert!(bad.validate().is_err());
}
