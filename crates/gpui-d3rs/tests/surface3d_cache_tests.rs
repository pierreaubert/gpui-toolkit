use d3rs::gpu3d::{Colormap, Surface3DConfig, Surface3DElement, SurfaceData};
use gpui::{Bounds, Point, Size, px};

fn test_element() -> Surface3DElement {
    let data = SurfaceData::from_function((0.0, 100.0), (0.0, 100.0), 5, 5, |x, y| x + y);
    Surface3DElement::from_data(data)
}

fn test_bounds() -> Bounds<gpui::Pixels> {
    Bounds::new(Point::new(px(0.0), px(0.0)), Size::new(px(100.0), px(100.0)))
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
