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

/// FNV-style signature of a scalar field's validity pattern (mask +
/// finiteness per entry). Triangle survival in preparation depends
/// only on this pattern, so an equal signature under equal topology
/// makes a scalar-only rebuild sound.
fn scalar_pattern(field: &Field) -> u64 {
    let FieldData::Scalar(values) = &field.data else {
        return 0;
    };
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for (index, value) in values.iter().enumerate() {
        let valid = field.valid.as_ref().is_none_or(|mask| mask[index]) && value.is_finite();
        hash ^= u64::from(valid);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

impl GeometryIdentity {
    /// Topology identity: everything except the selected scalar source
    /// and its transport. Equal topology under an equal scalar pattern
    /// is the scalar-only-update precondition.
    pub(crate) fn same_topology(self, other: Self) -> bool {
        self.positions == other.positions
            && self.cell_types == other.cell_types
            && self.offsets == other.offsets
            && self.connectivity == other.connectivity
            && self.wireframe == other.wireframe
    }

    pub(crate) fn scalar_association(self) -> Option<Association> {
        self.scalar.map(|(association, _, _)| association)
    }

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
    actors: Vec<Option<(GeometryIdentity, GeometrySource, u64)>>,
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
                        selected_field(actor)?.map_or(0, |(_, field)| scalar_pattern(field)),
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
                .map(|geometry| {
                    geometry
                        .as_ref()
                        .map(|g| (g.identity, g.source.clone(), g.scalar_pattern))
                })
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
                (Some((identity, _, _)), true) if identity.matches_buffers(actor)? => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Whether `prepared` differs from this revision only by packed
    /// point-scalar values: same topology, transport, and validity
    /// pattern per actor, with no line/point primitives (their vertex
    /// buffers embed scalars and still need a full upload).
    #[cfg(feature = "gpu")]
    pub(crate) fn scalar_only_matches(&self, prepared: &PreparedScene) -> bool {
        if self.actors.len() != prepared.actors.len() {
            return false;
        }
        for (previous, next) in self.actors.iter().zip(&prepared.actors) {
            match (previous, next) {
                (None, None) => {}
                (Some((identity, _, pattern)), Some(geometry)) => {
                    if !identity.same_topology(geometry.identity)
                        || identity.scalar_transport != geometry.identity.scalar_transport
                        || identity.scalar_association() != Some(Association::Point)
                        || *pattern != geometry.scalar_pattern
                        || !geometry.primitives.lines.len() == 0
                        || !geometry.primitives.points.len() == 0
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    #[cfg(any(feature = "gpui", test))]
    pub(crate) fn matches(&self, scene: &Scene) -> Result<bool, SceneError> {
        if self.actors.len() != scene.actors.len() {
            return Ok(false);
        }
        for (prepared, actor) in self.actors.iter().zip(&scene.actors) {
            match (prepared, actor.visible) {
                (None, false) => {}
                (Some((identity, _source, _)), true)
                    if *identity == GeometryIdentity::of(actor)? => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}

pub(crate) struct PreparedGeometry {
    identity: GeometryIdentity,
    /// Global point ids per chunk, retained for scalar-only rebuilds.
    point_ids: Vec<Arc<[u32]>>,
    /// Validity-pattern signature of the selected scalar field.
    scalar_pattern: u64,
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

    pub(crate) fn identity(&self) -> GeometryIdentity {
        self.identity
    }

    pub(crate) fn scalar_pattern(&self) -> u64 {
        self.scalar_pattern
    }

    pub(crate) fn byte_len(&self) -> u64 {
        self.bytes
    }

    /// Rebuild only packed scalars from `actor`'s current field,
    /// reusing this geometry's positions/normals/indices/edges.
    /// Callers must have verified equal topology and scalar pattern.
    fn with_updated_scalars(
        &self,
        actor: &Actor,
        identity: GeometryIdentity,
    ) -> Result<Self, SceneError> {
        let field = selected_field(actor)?.map(|(_, field)| field);
        let values = field.and_then(|field| field.as_scalar());
        let valid = field.and_then(|field| field.valid.as_deref());
        let transport = crate::scalar::ScalarTransport::of(actor)?;
        let mut chunks = Vec::with_capacity(self.chunks.len());
        for (chunk, point_ids) in self.chunks.iter().zip(&self.point_ids) {
            let mut scalars = vec![0.0f32; point_ids.len()];
            if let Some(values) = values {
                for (local, &point) in point_ids.iter().enumerate() {
                    let invalid = valid.is_some_and(|mask| !mask[point as usize]);
                    let value = values[point as usize];
                    scalars[local] = if invalid || !value.is_finite() {
                        f32::NAN
                    } else {
                        transport.pack(value)?
                    };
                }
            }
            chunks.push(GpuChunk {
                origin: chunk.origin,
                positions: chunk.positions.clone(),
                normals: chunk.normals.clone(),
                scalars,
                scalar_domain: transport.domain(),
                indices: chunk.indices.clone(),
                edges: chunk.edges.clone(),
            });
        }
        let primitives = crate::primitives::PrimitiveCells::prepare(
            &actor.dataset,
            selected_field(actor)?.map(|(_, field)| field),
        );
        let bytes = chunks.iter().map(GpuChunk::byte_len).sum::<u64>()
            + primitives.byte_len()
            + self
                .point_ids
                .iter()
                .map(|ids| (ids.len() * 4) as u64)
                .sum::<u64>();
        Ok(Self {
            identity,
            point_ids: self.point_ids.clone(),
            scalar_pattern: self.scalar_pattern,
            source: GeometrySource::of(actor)?,
            chunks,
            primitives,
            bounds: self.bounds,
            bytes,
        })
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
    /// Scalar-only rebuilds that reused topology buffers.
    pub scalar_updates: u64,
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
            let mut compatible = None;
            if let Some(existing) = reusable.get(&identity.source_identity()) {
                for geometry in existing.iter() {
                    if geometry.identity.matches_buffers(actor)? {
                        compatible = Some(geometry.clone());
                        break;
                    }
                }
            }
            // Scalar-only fast path: same topology, same transport,
            // point-associated field, and an unchanged validity pattern
            // mean only packed scalars can differ. Cell-associated
            // fields fall through (their splits depend on values).
            // Search every reusable geometry, not just this identity's
            // bucket: the bucket key still contains the old scalar Arc.
            let scalar_reuse = compatible
                .is_none()
                .then(|| {
                    reusable.values().flatten().find(|geometry| {
                        geometry.identity.same_topology(identity)
                            && geometry.identity.scalar_transport == identity.scalar_transport
                            && identity.scalar_association() == Some(Association::Point)
                            && selected_field(actor).is_ok_and(|selected| {
                                selected.is_some_and(|(_, field)| {
                                    scalar_pattern(field) == geometry.scalar_pattern
                                })
                            })
                    })
                })
                .flatten()
                .cloned();
            let entries = reusable.entry(identity.source_identity()).or_default();
            let geometry = if let Some(geometry) = compatible {
                geometry
            } else if let Some(previous) = scalar_reuse {
                let geometry = Arc::new(previous.with_updated_scalars(actor, identity)?);
                entries.push(geometry.clone());
                self.stats.scalar_updates += 1;
                geometry
            } else {
                // Preparation validates once per geometry revision before indexing.
                let (bounds, chunks, point_ids) = prepare_chunks_with_bounds_and_limits(
                    actor,
                    DEFAULT_MAX_TRIANGLES,
                    scene.surface_limits,
                )?;
                let primitives = crate::primitives::PrimitiveCells::prepare(
                    &actor.dataset,
                    selected_field(actor)?.map(|(_, field)| field),
                );
                let bytes = chunks.iter().map(GpuChunk::byte_len).sum::<u64>()
                    + primitives.byte_len()
                    + point_ids
                        .iter()
                        .map(|ids| (ids.len() * 4) as u64)
                        .sum::<u64>();
                let geometry = Arc::new(PreparedGeometry {
                    identity,
                    point_ids,
                    scalar_pattern: selected_field(actor)?
                        .map_or(0, |(_, field)| scalar_pattern(field)),
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
    fn scalar_only_changes_rebuild_scalars_without_geometry_build() {
        let mut scene = scene();
        let first = scene.prepare().unwrap();
        assert_eq!(scene.preparation_stats().geometry_builds, 1);
        // New scalar allocation, same topology and validity pattern.
        scene.actors[0].dataset.point_data[0].data =
            gpui_mesh::FieldData::Scalar(vec![0.75; 64].into());
        let updated = scene.prepare().unwrap();
        let stats = scene.preparation_stats();
        assert_eq!(stats.geometry_builds, 1, "no surface extraction ran");
        assert_eq!(stats.scalar_updates, 1);
        let first_chunk = &first.actors[0].as_ref().unwrap().chunks[0];
        let updated_chunk = &updated.actors[0].as_ref().unwrap().chunks[0];
        assert_eq!(updated_chunk.positions, first_chunk.positions);
        assert_eq!(updated_chunk.normals, first_chunk.normals);
        assert_eq!(updated_chunk.indices, first_chunk.indices);
        assert!(updated_chunk.scalars.iter().all(|v| *v == 0.75));
        // A validity-pattern change (masking) falls back to full build.
        scene.actors[0].dataset.point_data[0].valid = Some(vec![false; 64].into());
        scene.prepare().unwrap();
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
