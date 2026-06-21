//! Spec parsing cache for the Python IR showcase.
//!
//! Keeps `SurfaceSpec`/`LinesSpec`/`MeshSpec`/`SceneSpec` keyed by node id so
//! that rendering helpers do not re-parse the same `serde_json::Value` on every
//! frame.

use crate::{LinesSpec, MeshSpec, SceneSpec, SurfaceSpec};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

/// Deserialize `value` into `T` without taking ownership of the JSON value.
pub fn parse_spec<T>(value: &Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    T::deserialize(value).map_err(|error| error.to_string())
}

/// A typed 3D scene spec parsed from a JSON IR node.
#[derive(Debug, Clone)]
pub enum TypedSceneSpec {
    Surface(SurfaceSpec),
    Lines(LinesSpec),
    Mesh(MeshSpec),
    Scene(SceneSpec),
}

/// Cache for parsed 3D scene specs keyed by node id.
///
/// `SurfaceSpec`/`LinesSpec`/`MeshSpec`/`SceneSpec` are deserialized and
/// validated once per node; subsequent lookups return the cached value.
#[derive(Debug, Default, Clone)]
pub struct TypedSpecCache {
    specs: HashMap<String, TypedSceneSpec>,
}

impl TypedSpecCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    pub fn clear(&mut self) {
        self.specs.clear();
    }

    /// Parse or return a cached `SurfaceSpec` for the given node id.
    pub fn parse_surface(
        &mut self,
        id: impl Into<String>,
        value: &Value,
    ) -> Result<&SurfaceSpec, String> {
        let id = id.into();
        if !self.specs.contains_key(&id) {
            let spec = parse_spec::<SurfaceSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.specs.insert(id.clone(), TypedSceneSpec::Surface(spec));
        }
        match self.specs.get(&id) {
            Some(TypedSceneSpec::Surface(spec)) => Ok(spec),
            Some(_) => Err(format!("node {id} is not a surface spec")),
            None => unreachable!("just inserted or checked a SurfaceSpec"),
        }
    }

    /// Parse or return a cached `LinesSpec` for the given node id.
    pub fn parse_lines(
        &mut self,
        id: impl Into<String>,
        value: &Value,
    ) -> Result<&LinesSpec, String> {
        let id = id.into();
        if !self.specs.contains_key(&id) {
            let spec = parse_spec::<LinesSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.specs.insert(id.clone(), TypedSceneSpec::Lines(spec));
        }
        match self.specs.get(&id) {
            Some(TypedSceneSpec::Lines(spec)) => Ok(spec),
            Some(_) => Err(format!("node {id} is not a lines spec")),
            None => unreachable!("just inserted or checked a LinesSpec"),
        }
    }

    /// Parse or return a cached `MeshSpec` for the given node id.
    pub fn parse_mesh(
        &mut self,
        id: impl Into<String>,
        value: &Value,
    ) -> Result<&MeshSpec, String> {
        let id = id.into();
        if !self.specs.contains_key(&id) {
            let spec = parse_spec::<MeshSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.specs.insert(id.clone(), TypedSceneSpec::Mesh(spec));
        }
        match self.specs.get(&id) {
            Some(TypedSceneSpec::Mesh(spec)) => Ok(spec),
            Some(_) => Err(format!("node {id} is not a mesh spec")),
            None => unreachable!("just inserted or checked a MeshSpec"),
        }
    }

    /// Parse or return a cached `SceneSpec` for the given node id.
    pub fn parse_scene(
        &mut self,
        id: impl Into<String>,
        value: &Value,
    ) -> Result<&SceneSpec, String> {
        let id = id.into();
        if !self.specs.contains_key(&id) {
            let spec = parse_spec::<SceneSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.specs.insert(id.clone(), TypedSceneSpec::Scene(spec));
        }
        match self.specs.get(&id) {
            Some(TypedSceneSpec::Scene(spec)) => Ok(spec),
            Some(_) => Err(format!("node {id} is not a scene spec")),
            None => unreachable!("just inserted or checked a SceneSpec"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parse_surface_reports_validation_errors() {
        let mut cache = TypedSpecCache::new();
        let bad = serde_json::json!({
            "id": "bad",
            "z": { "values": [1.0, 2.0], "width": 2, "height": 2 }
        });
        assert!(cache.parse_surface("bad", &bad).is_err());
    }

    #[test]
    fn parse_methods_report_type_mismatch() {
        let mut cache = TypedSpecCache::new();
        let surface = serde_json::json!({
            "id": "shared",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });
        cache.parse_surface("shared", &surface).unwrap();
        assert!(cache.parse_lines("shared", &surface).is_err());
    }
}
