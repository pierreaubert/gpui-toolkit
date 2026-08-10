//! Spec parsing cache for the Python IR showcase.
//!
//! Keeps `SurfaceSpec`/`LinesSpec`/`MeshSpec`/`SceneSpec` keyed by node id so
//! that rendering helpers do not re-parse the same `serde_json::Value` on every
//! frame.

use crate::{LinesSpec, MeshSpec, SceneSpec, SurfaceSpec};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

/// Current JSON schema version for Python-authored Scene3D spec payloads.
pub const SCENE3D_SPEC_SCHEMA_VERSION: u32 = 1;

/// Default maximum number of typed Scene3D specs retained by [`TypedSpecCache`].
pub const DEFAULT_TYPED_SPEC_CACHE_MAX_ENTRIES: usize = 128;

/// Deserialize `value` into `T` without taking ownership of the JSON value.
pub fn parse_spec<T>(value: &Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    validate_scene3d_spec_schema_version(value)?;
    T::deserialize(value).map_err(|error| error.to_string())
}

/// Return the schema version for a Scene3D spec payload.
///
/// Early v1 payloads omitted `schema_version`; those are treated as v1 for
/// compatibility. Unsupported future versions are rejected before deserializing
/// or reusing cached specs.
pub fn scene3d_spec_schema_version(value: &Value) -> Result<u32, String> {
    let Some(version) = value.get("schema_version") else {
        return Ok(SCENE3D_SPEC_SCHEMA_VERSION);
    };

    let Some(version) = version.as_u64() else {
        return Err("scene3d schema_version must be an integer".to_string());
    };

    u32::try_from(version).map_err(|_| "scene3d schema_version is too large".to_string())
}

/// Validate the schema version for a Scene3D spec payload.
pub fn validate_scene3d_spec_schema_version(value: &Value) -> Result<(), String> {
    let version = scene3d_spec_schema_version(value)?;
    if version != SCENE3D_SPEC_SCHEMA_VERSION {
        return Err(format!(
            "unsupported scene3d schema version {version}; supported version is {SCENE3D_SPEC_SCHEMA_VERSION}"
        ));
    }
    Ok(())
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
#[derive(Debug, Clone)]
pub struct TypedSpecCache {
    specs: HashMap<String, TypedSceneSpec>,
    /// Fingerprints are kept separately from typed values so a node id can be
    /// reused for a new payload without returning stale retained geometry.
    fingerprints: HashMap<String, u64>,
    lru: VecDeque<String>,
    max_entries: usize,
}

impl Default for TypedSpecCache {
    fn default() -> Self {
        Self {
            specs: HashMap::new(),
            fingerprints: HashMap::new(),
            lru: VecDeque::new(),
            max_entries: DEFAULT_TYPED_SPEC_CACHE_MAX_ENTRIES,
        }
    }
}

impl TypedSpecCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    #[must_use]
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    #[must_use]
    pub fn contains_id(&self, id: &str) -> bool {
        self.specs.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.specs.clear();
        self.fingerprints.clear();
        self.lru.clear();
    }

    fn touch(&mut self, id: &str) {
        self.lru.retain(|entry| entry != id);
        self.lru.push_back(id.to_string());
    }

    fn insert_spec(&mut self, id: String, spec: TypedSceneSpec, fingerprint: u64) {
        while self.specs.len() >= self.max_entries {
            let Some(evicted) = self.lru.pop_front() else {
                break;
            };
            self.specs.remove(&evicted);
            self.fingerprints.remove(&evicted);
        }

        self.specs.insert(id.clone(), spec);
        self.fingerprints.insert(id.clone(), fingerprint);
        self.touch(&id);
    }

    fn invalidate_if_changed(&mut self, id: &str, value: &Value) -> u64 {
        let fingerprint = content_fingerprint(value);
        if self
            .fingerprints
            .get(id)
            .is_some_and(|cached| *cached != fingerprint)
        {
            self.specs.remove(id);
            self.fingerprints.remove(id);
            self.lru.retain(|entry| entry != id);
        }
        fingerprint
    }

    /// Parse or return a cached `SurfaceSpec` for the given node id.
    pub fn parse_surface(
        &mut self,
        id: impl Into<String>,
        value: &Value,
    ) -> Result<&SurfaceSpec, String> {
        validate_scene3d_spec_schema_version(value)?;
        let id = id.into();
        let fingerprint = self.invalidate_if_changed(&id, value);
        if self.specs.contains_key(&id) {
            self.touch(&id);
        } else {
            let spec = parse_spec::<SurfaceSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.insert_spec(id.clone(), TypedSceneSpec::Surface(spec), fingerprint);
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
        validate_scene3d_spec_schema_version(value)?;
        let id = id.into();
        let fingerprint = self.invalidate_if_changed(&id, value);
        if self.specs.contains_key(&id) {
            self.touch(&id);
        } else {
            let spec = parse_spec::<LinesSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.insert_spec(id.clone(), TypedSceneSpec::Lines(spec), fingerprint);
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
        validate_scene3d_spec_schema_version(value)?;
        let id = id.into();
        let fingerprint = self.invalidate_if_changed(&id, value);
        if self.specs.contains_key(&id) {
            self.touch(&id);
        } else {
            let spec = parse_spec::<MeshSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.insert_spec(id.clone(), TypedSceneSpec::Mesh(spec), fingerprint);
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
        validate_scene3d_spec_schema_version(value)?;
        let id = id.into();
        let fingerprint = self.invalidate_if_changed(&id, value);
        if self.specs.contains_key(&id) {
            self.touch(&id);
        } else {
            let spec = parse_spec::<SceneSpec>(value)?;
            spec.validate().map_err(|error| error.to_string())?;
            self.insert_spec(id.clone(), TypedSceneSpec::Scene(spec), fingerprint);
        }
        match self.specs.get(&id) {
            Some(TypedSceneSpec::Scene(spec)) => Ok(spec),
            Some(_) => Err(format!("node {id} is not a scene spec")),
            None => unreachable!("just inserted or checked a SceneSpec"),
        }
    }
}

/// Compute a deterministic content fingerprint for a JSON payload.
///
/// Object keys are sorted before hashing, so equivalent payloads with a
/// different insertion order still hit the cache. Arrays remain ordered.
fn content_fingerprint(value: &Value) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_value(value, &mut hasher);
    hasher.finish()
}

fn hash_value(value: &Value, hasher: &mut impl Hasher) {
    std::mem::discriminant(value).hash(hasher);
    match value {
        Value::Null => {}
        Value::Bool(value) => value.hash(hasher),
        Value::Number(value) => value.to_string().hash(hasher),
        Value::String(value) => value.hash(hasher),
        Value::Array(values) => {
            values.len().hash(hasher);
            for value in values {
                hash_value(value, hasher);
            }
        }
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for key in keys {
                key.hash(hasher);
                hash_value(&values[key], hasher);
            }
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
            "schema_version": SCENE3D_SPEC_SCHEMA_VERSION,
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
    fn scene3d_schema_version_defaults_to_v1_for_legacy_specs() {
        let value = serde_json::json!({
            "id": "surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });

        assert_eq!(
            scene3d_spec_schema_version(&value).unwrap(),
            SCENE3D_SPEC_SCHEMA_VERSION
        );
        assert!(validate_scene3d_spec_schema_version(&value).is_ok());
    }

    #[test]
    fn scene3d_schema_version_rejects_future_specs_before_cache_hit() {
        let mut cache = TypedSpecCache::new();
        let value = serde_json::json!({
            "id": "surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });
        cache.parse_surface("surface", &value).unwrap();

        let future = serde_json::json!({
            "schema_version": SCENE3D_SPEC_SCHEMA_VERSION + 1,
            "id": "surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });

        let error = cache.parse_surface("surface", &future).unwrap_err();
        assert!(error.contains("unsupported scene3d schema version"));
    }

    #[test]
    fn content_change_replaces_cached_value_for_same_node_id() {
        let mut cache = TypedSpecCache::new();
        let first = serde_json::json!({
            "id": "surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });
        let second = serde_json::json!({
            "id": "surface",
            "z": { "values": [9.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });
        assert_eq!(
            cache.parse_surface("surface", &first).unwrap().z.values[0],
            1.0
        );
        assert_eq!(
            cache.parse_surface("surface", &second).unwrap().z.values[0],
            9.0
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn equivalent_object_order_reuses_content_cache() {
        let a = serde_json::json!({"id":"surface", "z":{"values":[1.0,2.0,3.0,4.0],"width":2,"height":2}});
        let b = serde_json::from_str::<Value>(
            r#"{"z":{"height":2,"width":2,"values":[1.0,2.0,3.0,4.0]},"id":"surface"}"#,
        )
        .unwrap();
        assert_eq!(content_fingerprint(&a), content_fingerprint(&b));
    }

    #[test]
    fn scene3d_schema_version_requires_integer() {
        let value = serde_json::json!({
            "schema_version": "1",
            "id": "surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });

        let error = validate_scene3d_spec_schema_version(&value).unwrap_err();
        assert!(error.contains("schema_version must be an integer"));
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
    fn spec_cache_exposes_default_capacity_policy() {
        let cache = TypedSpecCache::new();

        assert_eq!(cache.max_entries(), DEFAULT_TYPED_SPEC_CACHE_MAX_ENTRIES);
        assert!(cache.max_entries() > 1);
    }

    #[test]
    fn spec_cache_evictions_keep_large_scene_streams_bounded() {
        let mut cache = TypedSpecCache::with_max_entries(3);

        for index in 0..12 {
            let value = serde_json::json!({
                "id": format!("surface-{index}"),
                "z": {
                    "values": [index as f64, 1.0, 2.0, 3.0],
                    "width": 2,
                    "height": 2
                }
            });
            cache
                .parse_surface(format!("surface-{index}"), &value)
                .expect("parse streamed surface");
            assert!(
                cache.len() <= cache.max_entries(),
                "cache exceeded configured max entries"
            );
        }

        assert_eq!(cache.len(), 3);
        assert!(!cache.contains_id("surface-0"));
        assert!(!cache.contains_id("surface-8"));
        assert!(cache.contains_id("surface-9"));
        assert!(cache.contains_id("surface-10"));
        assert!(cache.contains_id("surface-11"));
    }

    #[test]
    fn spec_cache_eviction_preserves_recently_used_entry() {
        let mut cache = TypedSpecCache::with_max_entries(2);
        let surface_a = serde_json::json!({
            "id": "a",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });
        let surface_b = serde_json::json!({
            "id": "b",
            "z": { "values": [5.0, 6.0, 7.0, 8.0], "width": 2, "height": 2 }
        });
        let surface_c = serde_json::json!({
            "id": "c",
            "z": { "values": [9.0, 10.0, 11.0, 12.0], "width": 2, "height": 2 }
        });

        cache.parse_surface("a", &surface_a).unwrap();
        cache.parse_surface("b", &surface_b).unwrap();
        cache.parse_surface("a", &surface_a).unwrap();
        cache.parse_surface("c", &surface_c).unwrap();

        assert_eq!(cache.len(), 2);
        assert!(cache.contains_id("a"));
        assert!(!cache.contains_id("b"));
        assert!(cache.contains_id("c"));
    }

    #[test]
    fn spec_cache_clamps_zero_capacity_to_one_entry() {
        let mut cache = TypedSpecCache::with_max_entries(0);
        let surface_a = serde_json::json!({
            "id": "a",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });
        let surface_b = serde_json::json!({
            "id": "b",
            "z": { "values": [5.0, 6.0, 7.0, 8.0], "width": 2, "height": 2 }
        });

        assert_eq!(cache.max_entries(), 1);
        cache.parse_surface("a", &surface_a).unwrap();
        cache.parse_surface("b", &surface_b).unwrap();

        assert_eq!(cache.len(), 1);
        assert!(!cache.contains_id("a"));
        assert!(cache.contains_id("b"));
    }

    #[test]
    fn spec_cache_clear_resets_entries_and_eviction_order() {
        let mut cache = TypedSpecCache::with_max_entries(2);
        let surface = serde_json::json!({
            "id": "surface",
            "z": { "values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2 }
        });

        cache.parse_surface("surface", &surface).unwrap();
        cache.clear();

        assert!(cache.is_empty());
        assert!(!cache.contains_id("surface"));
        cache.parse_surface("surface", &surface).unwrap();
        assert_eq!(cache.len(), 1);
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
