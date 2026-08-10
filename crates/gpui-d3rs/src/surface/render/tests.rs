use super::super::projection::ProjectionType;
use super::color_scale_type::ColorScaleType;
use super::misc::cool_color;
use super::misc::grayscale_color;
use super::misc::heat_color;
use super::misc::normalize_vec;
use super::misc::spectral_color;
use super::misc::viridis_color;
use super::surface_config::SurfaceConfig;
use gpui::prelude::*;
use gpui::*;

use super::*;

#[::core::prelude::v1::test]
fn test_color_scales() {
    // Test that all color scales produce valid colors
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let _ = viridis_color(t);
        let _ = heat_color(t);
        let _ = cool_color(t);
        let _ = spectral_color(t);
        let _ = grayscale_color(t);
    }
}

#[::core::prelude::v1::test]
fn test_config_builder() {
    let config = SurfaceConfig::new()
        .isometric()
        .rotation(45.0, 60.0)
        .zoom(1.5)
        .color_scale(ColorScaleType::Heat)
        .opacity(0.8)
        .wireframe(true)
        .lighting(true)
        .ambient(0.3)
        .diffuse(0.7);

    assert_eq!(config.projection_type, ProjectionType::Isometric);
    assert_eq!(config.camera.rotation_x, 45.0);
    assert_eq!(config.camera.rotation_z, 60.0);
    assert_eq!(config.camera.zoom, 1.5);
    assert_eq!(config.color_scale, ColorScaleType::Heat);
    assert!((config.opacity - 0.8).abs() < 1e-6);
    assert!(config.wireframe);
    assert!(config.lighting);
    assert!((config.ambient - 0.3).abs() < 1e-6);
    assert!((config.diffuse - 0.7).abs() < 1e-6);
}

#[::core::prelude::v1::test]
fn test_normalize_vec() {
    let v = normalize_vec((3.0, 4.0, 0.0));
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    assert!((len - 1.0).abs() < 1e-10);
}

#[::core::prelude::v1::test]
fn prepaint_reuses_sorted_triangles_when_camera_unchanged() {
    let data = SurfaceData::from_z_function((0.0, 1.0), (0.0, 1.0), 5, |x, y| x + y);
    let config = SurfaceConfig::new().isometric().rotation(30.0, 45.0);
    let mut element = SurfaceElement::new(&data, config, 400.0, 400.0);
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(400.0), px(400.0)));

    element.prepare_geometry(bounds);
    let gen1 = element.cache_generation;
    let projected1 = element.projected_triangles().to_vec();
    let triangle_order1: Vec<_> = element
        .mesh
        .triangles
        .iter()
        .map(|t| t.centroid())
        .collect();

    // Second call with the same camera/bounds must reuse the cached geometry.
    element.prepare_geometry(bounds);
    let gen2 = element.cache_generation;
    let projected2 = element.projected_triangles().to_vec();
    let triangle_order2: Vec<_> = element
        .mesh
        .triangles
        .iter()
        .map(|t| t.centroid())
        .collect();

    assert_eq!(gen1, gen2, "generation key should be unchanged");
    assert_eq!(
        triangle_order1, triangle_order2,
        "sorted triangle order should be stable"
    );
    assert_eq!(
        projected1, projected2,
        "projected triangles should be reused"
    );
}

#[::core::prelude::v1::test]
fn paint_reuses_projected_vertices_after_camera_change() {
    let data = SurfaceData::from_z_function((0.0, 1.0), (0.0, 1.0), 5, |x, y| x + y);
    let config = SurfaceConfig::new().isometric().rotation(30.0, 45.0);
    let mut element = SurfaceElement::new(&data, config, 400.0, 400.0);
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(400.0), px(400.0)));

    element.prepare_geometry(bounds);
    let gen1 = element.cache_generation;
    assert!(element.has_projected_cache());

    // Changing the camera must invalidate the cache and rebuild projected vertices.
    element.config.camera.rotation_z += 5.0;
    element.prepare_geometry(bounds);
    let gen2 = element.cache_generation;
    assert_ne!(
        gen1, gen2,
        "generation key should change when camera changes"
    );
    assert!(element.has_projected_cache());
    assert!(!element.projected_triangles().is_empty());
}
