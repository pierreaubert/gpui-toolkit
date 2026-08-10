use crate::error::Scene3DError;
use crate::meshplot::MeshPlotSpec;
use crate::scene3d::{LinesSpec, MeshSpec, SceneFingerprints, SceneSpec, SurfaceSpec};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyResources {
    pub is_new: bool,
    pub geometry: bool,
    pub material: bool,
    pub camera: bool,
}

impl DirtyResources {
    #[must_use]
    pub const fn unchanged() -> Self {
        Self {
            is_new: false,
            geometry: false,
            material: false,
            camera: false,
        }
    }

    #[must_use]
    pub const fn new_scene() -> Self {
        Self {
            is_new: true,
            geometry: true,
            material: true,
            camera: true,
        }
    }

    #[must_use]
    pub const fn updates_geometry(self) -> bool {
        self.geometry
    }

    #[must_use]
    pub const fn updates_material(self) -> bool {
        self.material
    }

    #[must_use]
    pub const fn updates_camera(self) -> bool {
        self.camera
    }

    #[must_use]
    pub const fn is_unchanged(self) -> bool {
        !self.is_new && !self.geometry && !self.material && !self.camera
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheUpdate {
    pub id: String,
    pub dirty: DirtyResources,
}

/// Dirty domains for a retained mesh plot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MeshPlotDirtyResources {
    pub is_new: bool,
    pub geometry: bool,
    pub field: bool,
    pub style: bool,
    pub camera: bool,
}

impl MeshPlotDirtyResources {
    #[must_use]
    pub const fn is_unchanged(self) -> bool {
        !self.is_new && !self.geometry && !self.field && !self.style && !self.camera
    }
}

/// Result of inserting a mesh-plot spec into the retained host cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPlotCacheUpdate {
    pub id: String,
    pub dirty: MeshPlotDirtyResources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MeshPlotFingerprints {
    geometry: u64,
    field: u64,
    style: u64,
    camera: u64,
}

#[derive(Debug, Clone)]
struct RetainedEntry {
    fingerprints: SceneFingerprints,
}

#[derive(Debug, Default)]
pub struct RetainedSceneCache {
    entries: HashMap<String, RetainedEntry>,
    mesh_plots: HashMap<String, MeshPlotFingerprints>,
}

impl RetainedSceneCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.mesh_plots.clear();
    }

    pub fn retain_only<I, S>(&mut self, ids: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str> + Into<String>,
    {
        let live: std::collections::HashSet<String> = ids.into_iter().map(Into::into).collect();
        self.entries.retain(|id, _| live.contains(id));
        self.mesh_plots.retain(|id, _| live.contains(id));
    }

    pub fn upsert_scene(&mut self, spec: &SceneSpec) -> Result<CacheUpdate, Scene3DError> {
        spec.validate()?;
        Ok(self.upsert_fingerprints(&spec.id, spec.fingerprints()))
    }

    pub fn upsert_surface(&mut self, spec: &SurfaceSpec) -> Result<CacheUpdate, Scene3DError> {
        spec.validate()?;
        Ok(self.upsert_fingerprints(&spec.id, spec.fingerprints()))
    }

    pub fn upsert_lines(&mut self, spec: &LinesSpec) -> Result<CacheUpdate, Scene3DError> {
        spec.validate()?;
        Ok(self.upsert_fingerprints(&spec.id, spec.fingerprints()))
    }

    pub fn upsert_mesh(&mut self, spec: &MeshSpec) -> Result<CacheUpdate, Scene3DError> {
        spec.validate()?;
        Ok(self.upsert_fingerprints(&spec.id, spec.fingerprints()))
    }

    /// Insert or update a declarative mesh plot without conflating field and
    /// geometry changes. Validation happens before the retained entry changes.
    pub fn upsert_meshplot(&mut self, spec: &MeshPlotSpec) -> Result<MeshPlotCacheUpdate, String> {
        spec.validate()?;
        let fingerprints = mesh_plot_fingerprints(spec);
        let dirty = if let Some(previous) = self.mesh_plots.insert(spec_id(spec), fingerprints) {
            MeshPlotDirtyResources {
                is_new: false,
                geometry: previous.geometry != fingerprints.geometry,
                field: previous.field != fingerprints.field,
                style: previous.style != fingerprints.style,
                camera: previous.camera != fingerprints.camera,
            }
        } else {
            MeshPlotDirtyResources {
                is_new: true,
                geometry: true,
                field: spec.field.is_some(),
                style: true,
                camera: true,
            }
        };
        Ok(MeshPlotCacheUpdate {
            id: spec_id(spec),
            dirty,
        })
    }

    /// Number of retained mesh plot entries.
    #[must_use]
    pub fn meshplot_len(&self) -> usize {
        self.mesh_plots.len()
    }

    fn upsert_fingerprints(&mut self, id: &str, fingerprints: SceneFingerprints) -> CacheUpdate {
        let dirty = if let Some(entry) = self.entries.get_mut(id) {
            let dirty = classify(entry.fingerprints, fingerprints);
            entry.fingerprints = fingerprints;
            dirty
        } else {
            self.entries
                .insert(id.to_string(), RetainedEntry { fingerprints });
            DirtyResources::new_scene()
        };

        CacheUpdate {
            id: id.to_string(),
            dirty,
        }
    }
}

fn classify(previous: SceneFingerprints, next: SceneFingerprints) -> DirtyResources {
    DirtyResources {
        is_new: false,
        geometry: previous.geometry != next.geometry,
        material: previous.material != next.material,
        camera: previous.camera != next.camera,
    }
}

fn spec_id(spec: &MeshPlotSpec) -> String {
    spec.cache_id()
}

fn mesh_plot_fingerprints(spec: &MeshPlotSpec) -> MeshPlotFingerprints {
    MeshPlotFingerprints {
        geometry: json_fingerprint(&spec.geometry),
        field: spec.field.as_ref().map_or(0, json_fingerprint),
        style: json_fingerprint(&serde_json::json!({
            "mode": spec.mode,
            "color_scale": spec.color_scale,
            "color_range": spec.color_range,
            "wireframe": spec.wireframe,
            "title": spec.title,
            "width": spec.width,
            "height": spec.height,
            "selection": spec.selection,
        })),
        camera: json_fingerprint(&serde_json::json!({
            "view": spec.view,
            "camera": spec.camera,
            "viewport": spec.viewport,
        })),
    }
}

fn json_fingerprint(value: &serde_json::Value) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene3d::{CameraSpec, ColormapSpec, OrbitCameraSpec, SurfaceSpec};

    #[test]
    fn unchanged_surface_is_clean_after_first_insert() {
        let spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let mut cache = RetainedSceneCache::new();

        let first = cache.upsert_surface(&spec).expect("first insert");
        let second = cache.upsert_surface(&spec).expect("second insert");

        assert!(first.dirty.is_new);
        assert!(second.dirty.is_unchanged());
    }

    #[test]
    fn camera_change_is_uniform_only() {
        let spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let mut changed = spec.clone();
        changed.camera = Some(CameraSpec::Orbit(OrbitCameraSpec::new(4.0, 60.0, 25.0)));
        let mut cache = RetainedSceneCache::new();

        cache.upsert_surface(&spec).expect("insert");
        let update = cache.upsert_surface(&changed).expect("camera update");

        assert!(update.dirty.updates_camera());
        assert!(!update.dirty.updates_geometry());
        assert!(!update.dirty.updates_material());
    }

    #[test]
    fn color_change_is_material_only() {
        let spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let mut changed = spec.clone();
        changed.colormap = ColormapSpec::Turbo;
        let mut cache = RetainedSceneCache::new();

        cache.upsert_surface(&spec).expect("insert");
        let update = cache.upsert_surface(&changed).expect("material update");

        assert!(update.dirty.updates_material());
        assert!(!update.dirty.updates_geometry());
        assert!(!update.dirty.updates_camera());
    }

    #[test]
    fn data_change_reuploads_geometry() {
        let spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let changed = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 5.0], 2, 2);
        let mut cache = RetainedSceneCache::new();

        cache.upsert_surface(&spec).expect("insert");
        let update = cache.upsert_surface(&changed).expect("geometry update");

        assert!(update.dirty.updates_geometry());
        assert!(!update.dirty.updates_camera());
    }

    #[test]
    fn retain_only_removes_missing_ids() {
        let spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let mut cache = RetainedSceneCache::new();

        cache.upsert_surface(&spec).expect("insert");
        assert_eq!(cache.len(), 1);

        cache.retain_only(["other"]);
        assert!(cache.is_empty());

        cache.upsert_surface(&spec).expect("reinsert");
        cache.retain_only(std::iter::empty::<&str>());
        assert!(cache.is_empty());
    }

    #[test]
    fn upsert_rejects_invalid_specs() {
        let mut cache = RetainedSceneCache::new();

        let bad_surface = SurfaceSpec::from_flat("bad", vec![1.0, 2.0, 3.0], 2, 2);
        assert!(cache.upsert_surface(&bad_surface).is_err());

        let bad_scene = SceneSpec {
            id: "bad".to_string(),
            camera: CameraSpec::default(),
            children: vec![],
            interactions: vec![],
            background: None,
            size: None,
        };
        assert!(cache.upsert_scene(&bad_scene).is_err());

        let bad_lines = LinesSpec {
            id: "bad".to_string(),
            segments: vec![],
            strips: vec![],
            ..LinesSpec::default()
        };
        assert!(cache.upsert_lines(&bad_lines).is_err());

        let bad_mesh = MeshSpec {
            id: "bad".to_string(),
            vertices: vec![],
            indices: vec![],
            material: crate::scene3d::MaterialSpec::default(),
            scalar_field: None,
        };
        assert!(cache.upsert_mesh(&bad_mesh).is_err());
    }

    #[test]
    fn meshplot_field_update_does_not_dirty_geometry_or_camera() {
        let geometry = serde_json::json!({
            "id":"plot-mesh",
            "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            "triangles": [[0, 1, 2]]
        });
        let first = MeshPlotSpec {
            schema_version: 1,
            id: "plot".into(),
            revision: 0,
            geometry: geometry.clone(),
            field: Some(serde_json::json!({"values":[1.0, 1.0, 1.0]})),
            view: "planar".into(),
            mode: "scalar_fill".into(),
            color_scale: "viridis".into(),
            color_range: serde_json::json!("auto"),
            missing_value_policy: "reject".into(),
            wireframe: true,
            title: None,
            width: None,
            height: None,
            selection: None,
            camera: None,
            viewport: None,
            contour_levels: None,
            equal_aspect: false,
            interactions: Vec::new(),
        };
        let mut second = first.clone();
        second.field = Some(serde_json::json!({"values":[2.0, 2.0, 2.0]}));
        let mut cache = RetainedSceneCache::new();
        cache.upsert_meshplot(&first).unwrap();
        let update = cache.upsert_meshplot(&second).unwrap();
        assert!(update.dirty.field);
        assert!(!update.dirty.geometry);
        assert!(!update.dirty.camera);
    }
}
