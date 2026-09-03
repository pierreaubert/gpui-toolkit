use d3rs::gpu3d::{
    Colormap, Surface3DConfig, Surface3DElement, SurfaceData, clamp_render_dimensions,
    overlay_color_for_background, upright_rotation_angle,
};
use gpui::{Bounds, Point, Size, px};

fn test_element() -> Surface3DElement {
    let data = SurfaceData::from_function((0.0, 100.0), (0.0, 100.0), 5, 5, |x, y| x + y);
    Surface3DElement::from_data(data)
}

fn test_bounds() -> Bounds<gpui::Pixels> {
    Bounds::new(
        Point::new(px(0.0), px(0.0)),
        Size::new(px(100.0), px(100.0)),
    )
}

#[test]
fn surface_cache_key_is_stable() {
    let element = test_element();
    let bounds = test_bounds();
    let camera = element.state().borrow().camera.clone();
    let key1 = element.compute_surface_cache_key(&bounds, 1.0, &camera);
    let key2 = element.compute_surface_cache_key(&bounds, 1.0, &camera);
    assert_eq!(key1, key2);
}

#[test]
fn surface_cache_key_changes_with_camera() {
    let element = test_element();
    let bounds = test_bounds();
    let camera1 = element.state().borrow().camera.clone();
    let mut camera2 = camera1.clone();
    camera2.position.x += 0.1;
    let key1 = element.compute_surface_cache_key(&bounds, 1.0, &camera1);
    let key2 = element.compute_surface_cache_key(&bounds, 1.0, &camera2);
    assert_ne!(key1, key2);
}

#[test]
fn surface_cache_key_changes_with_data() {
    let element = test_element();
    let bounds = test_bounds();
    let camera = element.state().borrow().camera.clone();
    let key1 = element.compute_surface_cache_key(&bounds, 1.0, &camera);

    let mut element2 = test_element();
    element2.set_data(SurfaceData::from_function(
        (0.0, 100.0),
        (0.0, 100.0),
        5,
        5,
        |x, y| x * y,
    ));
    let key2 = element2.compute_surface_cache_key(&bounds, 1.0, &camera);
    assert_ne!(key1, key2);
}

#[test]
fn surface_cache_key_changes_with_render_config() {
    let mut element = test_element();
    let bounds = test_bounds();
    let camera = element.state().borrow().camera.clone();
    let key1 = element.compute_surface_cache_key(&bounds, 1.0, &camera);

    element.set_config(Surface3DConfig::default().colormap(Colormap::Plasma));
    let key2 = element.compute_surface_cache_key(&bounds, 1.0, &camera);
    assert_ne!(key1, key2);
}

#[test]
fn surface_cache_key_stable_for_non_render_config() {
    let mut element = test_element();
    let bounds = test_bounds();
    let camera = element.state().borrow().camera.clone();
    let key1 = element.compute_surface_cache_key(&bounds, 1.0, &camera);

    element.set_config(Surface3DConfig::default().show_axes(false));
    let key2 = element.compute_surface_cache_key(&bounds, 1.0, &camera);
    assert_eq!(key1, key2);
}

#[test]
fn surface_cache_key_changes_with_bounds() {
    let element = test_element();
    let bounds1 = test_bounds();
    let bounds2 = Bounds::new(
        Point::new(px(0.0), px(0.0)),
        Size::new(px(200.0), px(200.0)),
    );
    let camera = element.state().borrow().camera.clone();
    let key1 = element.compute_surface_cache_key(&bounds1, 1.0, &camera);
    let key2 = element.compute_surface_cache_key(&bounds2, 1.0, &camera);
    assert_ne!(key1, key2);
}

#[test]
fn geometry_cache_key_is_stable() {
    let element = test_element();
    let bounds = test_bounds();
    let camera = element.state().borrow().camera.clone();
    let key1 = element.compute_geometry_cache_key(&bounds, 1.0, &camera);
    let key2 = element.compute_geometry_cache_key(&bounds, 1.0, &camera);
    assert_eq!(key1, key2);
}

#[test]
fn geometry_cache_key_changes_with_camera() {
    let element = test_element();
    let bounds = test_bounds();
    let camera1 = element.state().borrow().camera.clone();
    let mut camera2 = camera1.clone();
    camera2.position.x += 0.1;
    let key1 = element.compute_geometry_cache_key(&bounds, 1.0, &camera1);
    let key2 = element.compute_geometry_cache_key(&bounds, 1.0, &camera2);
    assert_ne!(key1, key2);
}

#[test]
fn paint_overlay_color_contrasts_background() {
    // Dark text on light backgrounds, white text on dark ones (paint stage 1/4).
    let dark = overlay_color_for_background([1.0, 1.0, 1.0]);
    for (got, want) in [(dark.r, 0.0), (dark.g, 0.0), (dark.b, 0.0), (dark.a, 1.0)] {
        assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
    }
    let light = overlay_color_for_background([0.0, 0.0, 0.0]);
    for (got, want) in [
        (light.r, 1.0),
        (light.g, 1.0),
        (light.b, 1.0),
        (light.a, 1.0),
    ] {
        assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
    }
}

#[test]
fn paint_render_dimensions_scale_and_floor() {
    // Paint stage 2 sizing: plain scale, degenerate floor, huge-input budgets.
    assert_eq!(clamp_render_dimensions(800.0, 600.0, 2.0), (1600, 1200));
    assert_eq!(clamp_render_dimensions(0.0, 0.0, 1.0), (1, 1));
    assert_eq!(clamp_render_dimensions(8192.0, 8192.0, 1.0), (2048, 2048));
}

#[test]
fn paint_upright_rotation_keeps_labels_readable() {
    use std::f32::consts::FRAC_PI_2;
    assert!(upright_rotation_angle(0.0).abs() < 1e-6);
    assert!(upright_rotation_angle(std::f32::consts::PI).abs() < 1e-6);
    for angle in [-3.0_f32, -2.0, -1.0, 1.0, 2.0, 3.0] {
        let upright = upright_rotation_angle(angle);
        assert!(
            (-FRAC_PI_2..=FRAC_PI_2).contains(&upright),
            "angle {angle} produced {upright}"
        );
    }
}
