//! Scene-owned derived geometry shared by immutable dataset revisions.

// Rust guideline compliant 2026-02-21

use crate::prepare::{DEFAULT_MAX_TRIANGLES, GpuChunk, prepare_chunks_with_bounds_and_limits};
use crate::{Actor, Scene, SceneError};
use gpui_mesh::{Association, Bounds, Cells, Field, FieldData};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GeometryIdentity {
    positions: usize,
    cell_types: usize,
    offsets: usize,
    connectivity: usize,
    scalar: Option<(Association, usize, Option<usize>)>,
    wireframe: bool,
    scalar_transport: crate::scalar::ScalarIdentity,
}

fn arc_id<T>(values: &Arc<[T]>) -> usize {
    Arc::as_ptr(values).cast::<T>() as usize
}

pub(crate) fn selected_field(actor: &Actor) -> Result<Option<(Association, &Field)>, SceneError> {
    actor
        .field
        .as_ref()
        .map(|(association, name)| {
            let field = actor
                .dataset
                .field(*association, name)
                .filter(|field| matches!(field.data, FieldData::Scalar(_)))
                .ok_or_else(|| SceneError::FieldNotFound(name.clone()))?;
            Ok((*association, field))
        })
        .transpose()
}

impl GeometryIdentity {
    fn source_identity(mut self) -> Self {
        self.scalar_transport = crate::scalar::ScalarIdentity::Flat;
        self
    }

    fn matches_buffers(self, actor: &Actor) -> Result<bool, SceneError> {
        let next = Self::of(actor)?;
        if self == next {
            return Ok(true);
        }
        Ok(self.source_identity() == next.source_identity()
            && self.scalar_transport.transport().supports_display(actor)?)
    }

    fn of(actor: &Actor) -> Result<Self, SceneError> {
        let scalar = selected_field(actor)?.map(|(association, field)| {
            let FieldData::Scalar(values) = &field.data else {
                unreachable!("scalar field checked")
            };
            (
                association,
                arc_id(values),
                field.valid.as_ref().map(arc_id),
            )
        });
        Ok(Self {
            positions: arc_id(&actor.dataset.positions),
            cell_types: arc_id(&actor.dataset.cells.types),
            offsets: arc_id(&actor.dataset.cells.offsets),
            connectivity: arc_id(&actor.dataset.cells.connectivity),
            scalar,
            wireframe: actor.wireframe,
            scalar_transport: crate::scalar::ScalarTransport::of(actor)?.identity(),
        })
    }
}

/// Keeps source allocations alive while their pointer identities are cached.
/// Safe Arc mutation must then copy, so a changed revision gets a new identity.
#[derive(Clone)]
struct GeometrySource {
    _positions: Arc<[[f64; 3]]>,
    _cells: Cells,
    _scalar: Option<Field>,
}

impl GeometrySource {
    fn of(actor: &Actor) -> Result<Self, SceneError> {
        Ok(Self {
            _positions: actor.dataset.positions.clone(),
            _cells: actor.dataset.cells.clone(),
            _scalar: selected_field(actor)?.map(|(_, field)| field.clone()),
        })
    }
}

/// Pins canonical source allocations without retaining derived CPU buffers.
#[cfg(any(feature = "gpu", feature = "gpui", test))]
pub(crate) struct SceneRevision {
    actors: Vec<Option<(GeometryIdentity, GeometrySource)>>,
}

#[cfg(any(feature = "gpu", feature = "gpui", test))]
impl SceneRevision {
    #[cfg(any(feature = "gpui", test))]
    pub(crate) fn new(scene: &Scene) -> Result<Self, SceneError> {
        let actors = scene
            .actors
            .iter()
            .map(|actor| {
                if actor.visible {
                    Ok(Some((
                        GeometryIdentity::of(actor)?,
                        GeometrySource::of(actor)?,
                    )))
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<_, SceneError>>()?;
        Ok(Self { actors })
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn from_prepared(prepared: &PreparedScene) -> Self {
        Self {
            actors: prepared
                .actors
                .iter()
                .map(|geometry| geometry.as_ref().map(|g| (g.identity, g.source.clone())))
                .collect(),
        }
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn matches_buffers(&self, scene: &Scene) -> Result<bool, SceneError> {
        if self.actors.len() != scene.actors.len() {
            return Ok(false);
        }
        for (prepared, actor) in self.actors.iter().zip(&scene.actors) {
            match (prepared, actor.visible) {
                (None, false) => {}
                (Some((identity, _)), true) if identity.matches_buffers(actor)? => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    #[cfg(any(feature = "gpui", test))]
    pub(crate) fn matches(&self, scene: &Scene) -> Result<bool, SceneError> {
        if self.actors.len() != scene.actors.len() {
            return Ok(false);
        }
        for (prepared, actor) in self.actors.iter().zip(&scene.actors) {
            match (prepared, actor.visible) {
                (None, false) => {}
                (Some((identity, _source)), true) if *identity == GeometryIdentity::of(actor)? => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}

pub(crate) struct PreparedGeometry {
    identity: GeometryIdentity,
    #[cfg_attr(
        not(feature = "gpu"),
        expect(dead_code, reason = "pins allocations used by cache identities")
    )]
    source: GeometrySource,
    pub(crate) chunks: Vec<GpuChunk>,
    pub(crate) primitives: crate::primitives::PrimitiveCells,
    pub(crate) bounds: Option<Bounds>,
    bytes: u64,
}

impl PreparedGeometry {
    pub(crate) fn scalar_transport(&self) -> crate::scalar::ScalarTransport {
        self.identity.scalar_transport.transport()
    }
}

/// Immutable prepared geometry for one scene revision.
///
/// Palette, opacity, clipping, background and cameras remain live scene state.
/// A transfer domain outside the raw f32 profile also determines the prepared
/// scalar coordinates. Compatible display-range changes reuse these coordinates;
/// zooms needing more precision rebuild from the original f64 source.
/// This object retains topology, positions, the selected scalar/mask and
/// their derived buffers. Unrelated fields are not pinned by the cache.
pub struct PreparedScene {
    pub(crate) actors: Vec<Option<Arc<PreparedGeometry>>>,
    bytes: u64,
}

impl std::fmt::Debug for PreparedScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedScene")
            .field("actors", &self.actors.len())
            .field("bytes", &self.bytes)
            .finish()
    }
}

impl PreparedScene {
    /// Retained derived-buffer bytes, excluding shared canonical dataset data.
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        self.bytes
    }

    /// Whether source allocations and preparation options still match.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing/non-scalar field or invalid transfer settings.
    pub fn matches(&self, scene: &Scene) -> Result<bool, SceneError> {
        if self.actors.len() != scene.actors.len() {
            return Ok(false);
        }
        for (prepared, actor) in self.actors.iter().zip(&scene.actors) {
            match (prepared, actor.visible) {
                (None, false) => {}
                (Some(prepared), true) if prepared.identity.matches_buffers(actor)? => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}

/// Counters for scene-owned geometry retention.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparationStats {
    /// Number of unique geometry preparations since this cache was created.
    pub geometry_builds: u64,
    /// Complete scene preparation hits, including camera/material-only updates.
    pub cache_hits: u64,
    /// Current cached derived-buffer payload bytes.
    pub retained_bytes: u64,
}

#[derive(Debug, Default)]
pub(crate) struct PreparationCache {
    current: Option<Arc<PreparedScene>>,
    stats: PreparationStats,
}

impl PreparationCache {
    pub(crate) fn prepare(&mut self, scene: &Scene) -> Result<Arc<PreparedScene>, SceneError> {
        if let Some(current) = &self.current
            && current.matches(scene)?
        {
            self.stats.cache_hits += 1;
            return Ok(current.clone());
        }
        let mut reusable: HashMap<
            GeometryIdentity,
            smallvec::SmallVec<[Arc<PreparedGeometry>; 1]>,
        > = HashMap::new();
        for geometry in self
            .current
            .as_ref()
            .into_iter()
            .flat_map(|s| s.actors.iter().flatten())
        {
            let entries = reusable
                .entry(geometry.identity.source_identity())
                .or_default();
            if !entries.iter().any(|g| Arc::ptr_eq(g, geometry)) {
                entries.push(geometry.clone());
            }
        }
        let mut actors = Vec::with_capacity(scene.actors.len());
        let mut used = std::collections::HashSet::new();
        let mut bytes = 0u64;
        for actor in &scene.actors {
            if !actor.visible {
                actors.push(None);
                continue;
            }
            let identity = GeometryIdentity::of(actor)?;
            let entries = reusable.entry(identity.source_identity()).or_default();
            let mut compatible = None;
            for geometry in entries.iter() {
                if geometry.identity.matches_buffers(actor)? {
                    compatible = Some(geometry.clone());
                    break;
                }
            }
            let geometry = if let Some(geometry) = compatible {
                geometry
            } else {
                // Preparation validates once per geometry revision before indexing.
                let (bounds, chunks) = prepare_chunks_with_bounds_and_limits(
                    actor,
                    DEFAULT_MAX_TRIANGLES,
                    scene.surface_limits,
                )?;
                let primitives = crate::primitives::PrimitiveCells::prepare(
                    &actor.dataset,
                    selected_field(actor)?.map(|(_, field)| field),
                );
                let bytes =
                    chunks.iter().map(GpuChunk::byte_len).sum::<u64>() + primitives.byte_len();
                let geometry = Arc::new(PreparedGeometry {
                    identity,
                    source: GeometrySource::of(actor)?,
                    chunks,
                    primitives,
                    bounds,
                    bytes,
                });
                entries.push(geometry.clone());
                self.stats.geometry_builds += 1;
                geometry
            };
            if used.insert(geometry.identity) {
                bytes += geometry.bytes;
            }
            actors.push(Some(geometry));
        }
        let prepared = Arc::new(PreparedScene { actors, bytes });
        self.stats.retained_bytes = bytes;
        self.current = Some(prepared.clone());
        Ok(prepared)
    }

    pub(crate) fn clear(&mut self) {
        self.current = None;
        self.stats.retained_bytes = 0;
    }

    pub(crate) fn stats(&self) -> PreparationStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransferFunction;
    use gpui_mesh::{DataSet, Field};

    fn scene() -> Scene {
        let dataset = DataSet::structured_grid("retained", [4; 3], [-1.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 64]));
        Scene::new()
            .with_actor(Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])))
    }

    #[test]
    fn scalar_domains_reuse_compatible_encoded_preparation() {
        let data = gpui_mesh::DataSet::structured_grid("domain", [2; 3], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 8]));
        let mut scene = Scene::new().with_actor(
            Actor::new(data).coloured_by("p", crate::TransferFunction::viridis([0.0, 1.0])),
        );
        let first = scene.prepare().unwrap();
        scene.actors[0].transfer.as_mut().unwrap().range = [0.0, 2.0];
        assert!(Arc::ptr_eq(&first, &scene.prepare().unwrap()));
        scene.actors[0].transfer.as_mut().unwrap().range = [1e100, 2e100];
        let encoded = scene.prepare().unwrap();
        assert!(!Arc::ptr_eq(&first, &encoded));
        scene.actors[0].transfer = Some(crate::TransferFunction::coolwarm([1e100, 2e100]));
        assert!(Arc::ptr_eq(&encoded, &scene.prepare().unwrap()));
        scene.actors[0].transfer.as_mut().unwrap().range = [1e100, 3e100];
        assert!(Arc::ptr_eq(&encoded, &scene.prepare().unwrap()));
        assert_eq!(scene.preparation_stats().geometry_builds, 2);
    }

    #[test]
    fn clones_and_material_updates_reuse_geometry() {
        let scene = scene();
        let first = scene.prepare().unwrap();
        let mut changed = scene.clone();
        changed.background = [1.0; 4];
        changed.actors[0].opacity = 0.5;
        changed.actors[0].transfer = Some(TransferFunction::coolwarm([0.0, 1.0]));
        assert!(Arc::ptr_eq(&first, &changed.prepare().unwrap()));
        assert_eq!(scene.preparation_stats().geometry_builds, 1);
    }

    #[test]
    fn revision_tracking_does_not_pin_derived_geometry() {
        let mut scene = scene();
        let prepared = scene.prepare().unwrap();
        let geometry = Arc::downgrade(prepared.actors[0].as_ref().unwrap());
        let revision = SceneRevision::new(&scene).unwrap();
        scene.clear_preparation();
        drop(prepared);
        assert!(geometry.upgrade().is_none(), "derived buffers released");
        assert!(revision.matches(&scene).unwrap());
        Arc::make_mut(&mut scene.actors[0].dataset.positions)[0][0] += 1.0;
        assert!(!revision.matches(&scene).unwrap());
    }

    #[test]
    fn copy_on_write_positions_fields_and_masks_invalidate() {
        let mut scene = scene();
        let first = scene.prepare().unwrap();
        Arc::make_mut(&mut scene.actors[0].dataset.positions)[0][0] -= 10.0;
        let moved = scene.prepare().unwrap();
        assert!(!Arc::ptr_eq(&first, &moved));
        scene.actors[0].dataset.point_data[0].data = FieldData::Scalar(vec![0.75; 64].into());
        let field = scene.prepare().unwrap();
        assert!(
            field.actors[0].as_ref().unwrap().chunks[0]
                .scalars
                .iter()
                .all(|value| *value == 0.75)
        );
        scene.actors[0].dataset.point_data[0].valid = Some(vec![false; 64].into());
        let masked = scene.prepare().unwrap();
        assert!(
            masked.actors[0]
                .as_ref()
                .unwrap()
                .chunks
                .iter()
                .all(|chunk| chunk.indices.is_empty())
        );
    }

    #[test]
    fn duplicates_and_reordering_share_geometry_and_trim_releases_it() {
        let mut scene = scene();
        scene.actors.push(scene.actors[0].clone());
        let mut other = scene.actors[0].clone();
        Arc::make_mut(&mut other.dataset.positions)[0][0] -= 10.0;
        scene.actors.push(other);
        let prepared = scene.prepare().unwrap();
        assert!(Arc::ptr_eq(
            prepared.actors[0].as_ref().unwrap(),
            prepared.actors[1].as_ref().unwrap()
        ));
        let bytes = prepared.actors[0].as_ref().unwrap().bytes;
        assert_eq!(
            prepared.byte_len(),
            bytes + prepared.actors[2].as_ref().unwrap().bytes
        );
        scene.actors.swap(0, 2);
        scene.prepare().unwrap();
        assert_eq!(scene.preparation_stats().geometry_builds, 2);
        scene.clear_preparation();
        assert_eq!(scene.preparation_stats().retained_bytes, 0);
    }
}
