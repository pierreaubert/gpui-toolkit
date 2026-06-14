use gpui_python_runtime::spec_cache::TypedSpecCache;

#[test]
fn render_surface_spec_parses_once() {
    let mut cache = TypedSpecCache::new();
    let value = serde_json::json!({
        "id": "surface",
        "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
    });

    let first = cache
        .parse_surface("surface", &value)
        .expect("first parse")
        .clone();
    let second = cache
        .parse_surface("surface", &value)
        .expect("second parse")
        .clone();

    assert_eq!(first, second);
    assert_eq!(cache.len(), 1);
}

#[test]
fn spec_cache_keeps_distinct_nodes_separate() {
    let mut cache = TypedSpecCache::new();
    let surface_a = serde_json::json!({
        "id": "a",
        "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
    });
    let surface_b = serde_json::json!({
        "id": "b",
        "z": { "values": [5.0, 6.0, 7.0, 8.0], "width": 2, "height": 2 }
    });

    let a = cache
        .parse_surface("a", &surface_a)
        .expect("parse a")
        .clone();
    let b = cache
        .parse_surface("b", &surface_b)
        .expect("parse b")
        .clone();

    assert_ne!(a.id, b.id);
    assert_eq!(cache.len(), 2);
}

#[test]
fn lines_spec_is_cached_by_node_id() {
    let mut cache = TypedSpecCache::new();
    let value = serde_json::json!({
        "id": "lines",
        "strips": [{
            "id": "strip",
            "points": [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
            "color": { "r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0 },
            "width": 1.0
        }]
    });

    let first = cache
        .parse_lines("lines", &value)
        .expect("first parse")
        .clone();
    let second = cache
        .parse_lines("lines", &value)
        .expect("second parse")
        .clone();

    assert_eq!(first, second);
    assert_eq!(cache.len(), 1);
}

#[test]
fn mesh_and_scene_specs_are_cached_by_node_id() {
    let mut cache = TypedSpecCache::new();
    let mesh_value = serde_json::json!({
        "id": "mesh",
        "vertices": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        "indices": [0, 1, 2]
    });
    let scene_value = serde_json::json!({
        "id": "scene",
        "camera": {
            "kind": "orbit",
            "distance": 3.0,
            "azimuth_deg": 60.0,
            "elevation_deg": 25.0,
            "target": { "x": 0.0, "y": 0.0, "z": 0.0 },
            "fov_y_deg": 45.0,
            "near": 0.1,
            "far": 100.0
        },
        "children": [{
            "kind": "surface",
            "id": "child_surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        }]
    });

    let mesh_first = cache
        .parse_mesh("mesh", &mesh_value)
        .expect("mesh first")
        .clone();
    let mesh_second = cache
        .parse_mesh("mesh", &mesh_value)
        .expect("mesh second")
        .clone();
    let scene_first = cache
        .parse_scene("scene", &scene_value)
        .expect("scene first")
        .clone();
    let scene_second = cache
        .parse_scene("scene", &scene_value)
        .expect("scene second")
        .clone();

    assert_eq!(mesh_first, mesh_second);
    assert_eq!(scene_first, scene_second);
    assert_eq!(cache.len(), 2);
}
