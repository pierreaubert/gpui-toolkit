use crate::error::Scene3DError;
use crate::scene3d::{LinesSpec, MeshSpec, SceneFingerprints, SceneSpec, SurfaceSpec};
use std::collections::HashMap;

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

#[derive(Debug, Clone)]
struct RetainedEntry {
    fingerprints: SceneFingerprints,
}

#[derive(Debug, Default)]
pub struct RetainedSceneCache {
    entries: HashMap<String, RetainedEntry>,
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
    }

    pub fn retain_only<I, S>(&mut self, ids: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str> + Into<String>,
    {
        let live: std::collections::HashSet<String> = ids.into_iter().map(Into::into).collect();
        self.entries.retain(|id, _| live.contains(id));
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
}
