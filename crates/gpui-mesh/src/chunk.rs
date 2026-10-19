//! Boundary-surface extraction and bounded-size chunking for large datasets.

// Rust guideline compliant 2026-02-21

use crate::dataset::{CellType, DataSet, MeshError, ValidatedDataSet};
use crate::structured_boundary::CanonicalHexGrid;
use std::collections::HashMap;
use std::num::NonZeroU8;
#[cfg(test)]
use std::num::NonZeroUsize;

#[derive(Clone, Copy)]
struct Face {
    nodes: [u32; 4],
    // The nonzero length also keeps Option<Face> the same size as Face.
    len: NonZeroU8,
}

impl Face {
    fn new(nodes: &[u32], indices: &[usize; 4]) -> Option<Self> {
        let mut polygon = [0; 4];
        let mut len = 0;
        for &index in indices {
            let point = nodes[index];
            if len == 0 || polygon[len - 1] != point {
                polygon[len] = point;
                len += 1;
            }
        }
        (len >= 3).then(|| Self {
            nodes: polygon,
            len: NonZeroU8::new(len as u8).expect("nonempty face"),
        })
    }

    fn key(self) -> [u32; 5] {
        let len = usize::from(self.len.get());
        let mut key = [0; 5];
        key[0] = len as u32;
        key[1..=len].copy_from_slice(&self.nodes[..len]);
        key[1..=len].sort_unstable();
        key
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct FaceOwner {
    // One-based so Option<FaceOwner> needs no extra discriminant.
    cell: NonZeroUsize,
    face: u8,
}

/// Limits temporary volume-face storage during surface extraction.
///
/// Counts face-record element bytes, excluding allocator overhead, source data,
/// chunk remapping/storage, callback-retained output and sorting stack space.
/// Canonical hexahedral grids need no face-record array. This is not a total
/// extraction or process-memory budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceExtractionLimits {
    /// Maximum temporary volume-face record element bytes.
    pub max_face_bytes: u64,
}

impl Default for SurfaceExtractionLimits {
    fn default() -> Self {
        Self {
            // Allow common moderate unstructured meshes without a giant
            // accidental workspace; constrained hosts should choose less.
            max_face_bytes: 64 * 1024 * 1024,
        }
    }
}

struct FaceRecord {
    key: [u32; 5],
    cell: usize,
    face: u8,
}

enum BoundaryError<E> {
    Mesh(MeshError),
    Callback(E),
}

impl<E> From<MeshError> for BoundaryError<E> {
    fn from(error: MeshError) -> Self {
        Self::Mesh(error)
    }
}

/// One bounded chunk of extracted surface triangles.
///
/// Indices are local to the chunk (`u32`), positions keep the dataset's
/// `f64` coordinates, and `point_ids` maps each local vertex back to the
/// dataset point index so field lookups and picking stay exact. A chunk
/// never exceeds the caller's triangle budget, which is what lets a
/// multi-million-cell dataset upload to a GPU buffer (or a wasm heap)
/// in pieces instead of as one monolithic allocation.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceChunk {
    /// Dataset point index for each local vertex, in first-use order.
    pub point_ids: Vec<u32>,
    /// Local triangle indices, three per triangle.
    pub indices: Vec<u32>,
}

/// Face tables for volume cells, as local node-index quads/triangles.
/// Hexahedron faces follow VTK node ordering.
fn volume_faces(kind: CellType) -> &'static [[usize; 4]] {
    match kind {
        CellType::Tetra => &[[0, 2, 1, 1], [0, 1, 3, 3], [1, 2, 3, 3], [2, 0, 3, 3]],
        CellType::Hexahedron => &[
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ],
        CellType::Wedge => &[
            [0, 2, 1, 1],
            [3, 4, 5, 5],
            [0, 1, 4, 3],
            [1, 2, 5, 4],
            [2, 0, 3, 5],
        ],
        CellType::Pyramid => &[
            [0, 3, 2, 1],
            [0, 1, 4, 4],
            [1, 2, 4, 4],
            [2, 3, 4, 4],
            [3, 0, 4, 4],
        ],
        _ => &[],
    }
}

/// Extracts boundary triangles as dataset point-index triples.
///
/// Surface cells contribute directly (quads split on the 0-2 diagonal).
/// Volume cells contribute only faces owned by a single cell, so interior
/// faces between two volume cells are culled. Face identity is the sorted
/// node set of the whole face polygon, so the culling does not depend on
/// which diagonal each owner would have chosen; surviving faces are
/// triangulated from the owning cell's node order.
/// Boundary faces are emitted in canonical node-set order for repeatability.
#[must_use]
pub fn boundary_triangles(dataset: &DataSet) -> Vec<[u32; 3]> {
    let mut triangles = Vec::new();
    let result: Result<(), std::convert::Infallible> = visit_boundary(dataset, |_, triangle| {
        triangles.push(triangle);
        Ok(())
    });
    match result {
        Ok(()) => triangles,
        Err(never) => match never {},
    }
}

fn visit_boundary<E>(
    dataset: &DataSet,
    visit: impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), E> {
    // Legacy APIs cannot represent workspace errors. Their allocation behavior
    // remains infallible; new callers should select the explicitly limited API.
    match visit_boundary_with_limits(dataset, u64::MAX, visit) {
        Ok(()) => Ok(()),
        Err(BoundaryError::Callback(error)) => Err(error),
        Err(BoundaryError::Mesh(error)) => panic!("surface extraction failed: {error}"),
    }
}

fn face_workspace(dataset: &DataSet, max_bytes: u64) -> Result<usize, MeshError> {
    let address_limit = (isize::MAX as u64).min(max_bytes);
    let capacity = dataset.cells.types.iter().try_fold(0usize, |count, &kind| {
        count.checked_add(volume_faces(kind).len())
    });
    let bytes = capacity.and_then(|n| n.checked_mul(size_of::<FaceRecord>()));
    let requested = bytes.map_or(u64::MAX, |n| n as u64);
    if requested > address_limit || capacity.is_none() || bytes.is_none() {
        return Err(MeshError::ResourceLimit {
            resource: "surface face workspace bytes",
            requested,
            limit: address_limit,
        });
    }
    Ok(capacity.expect("checked face count"))
}

fn visit_boundary_with_limits<E>(
    dataset: &DataSet,
    max_face_bytes: u64,
    visit: impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), BoundaryError<E>> {
    visit_boundary_with_grid(
        dataset,
        max_face_bytes,
        CanonicalHexGrid::detect(dataset),
        visit,
    )
}

fn visit_boundary_with_grid<E>(
    dataset: &DataSet,
    max_face_bytes: u64,
    grid: Option<CanonicalHexGrid>,
    mut visit: impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), BoundaryError<E>> {
    if let Some(grid) = grid {
        return grid
            .visit(|cell, face| visit_face(dataset, cell, usize::from(face), &mut visit))
            .map_err(BoundaryError::Callback);
    }
    let capacity = face_workspace(dataset, max_face_bytes)?;
    let mut faces = Vec::<FaceRecord>::new();
    faces
        .try_reserve_exact(capacity)
        .map_err(|_| MeshError::Allocation {
            bytes: capacity as u64 * size_of::<FaceRecord>() as u64,
        })?;
    for cell in 0..dataset.cell_count() {
        let Some(nodes) = dataset.cells.cell(cell) else {
            continue;
        };
        let kind = dataset.cells.types[cell];
        match kind {
            CellType::Triangle | CellType::Quad | CellType::Polygon => {
                for i in 1..nodes.len() - 1 {
                    visit(cell, [nodes[0], nodes[i], nodes[i + 1]])
                        .map_err(BoundaryError::Callback)?;
                }
            }
            CellType::Tetra | CellType::Hexahedron | CellType::Wedge | CellType::Pyramid => {
                for (face_index, face) in volume_faces(kind).iter().enumerate() {
                    if let Some(polygon) = Face::new(nodes, face) {
                        faces.push(FaceRecord {
                            key: polygon.key(),
                            cell,
                            face: face_index as u8,
                        });
                    }
                }
            }
            CellType::Vertex | CellType::Line => {}
        }
    }
    // In-place sorting avoids a map plus a second surviving-face allocation.
    // Equal keys are all interior/nonmanifold, irrespective of multiplicity.
    faces.sort_unstable_by_key(|record| record.key);
    let mut start = 0;
    while start < faces.len() {
        let mut end = start + 1;
        while end < faces.len() && faces[end].key == faces[start].key {
            end += 1;
        }
        if end == start + 1 {
            let record = &faces[start];
            visit_face(dataset, record.cell, usize::from(record.face), &mut visit)
                .map_err(BoundaryError::Callback)?;
        }
        start = end;
    }
    Ok(())
}

// Independent previous implementation is retained only as an exact test oracle.
#[cfg(test)]
fn visit_general_boundary<E>(
    dataset: &DataSet,
    mut visit: impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), E> {
    // Keep the source owner rather than copying its polygon. Reconstruct only
    // surviving boundary faces; the map entry is no larger than the old one.
    let mut faces: HashMap<[u32; 5], Option<FaceOwner>> = HashMap::new();
    for cell in 0..dataset.cell_count() {
        let Some(nodes) = dataset.cells.cell(cell) else {
            continue;
        };
        let kind = dataset.cells.types[cell];
        match kind {
            CellType::Triangle => {
                visit(cell, [nodes[0], nodes[1], nodes[2]])?;
            }
            CellType::Quad => {
                visit(cell, [nodes[0], nodes[1], nodes[2]])?;
                visit(cell, [nodes[0], nodes[2], nodes[3]])?;
            }
            CellType::Polygon => {
                // Fan triangulation around the first node.
                for i in 1..nodes.len() - 1 {
                    visit(cell, [nodes[0], nodes[i], nodes[i + 1]])?;
                }
            }
            CellType::Tetra | CellType::Hexahedron | CellType::Wedge | CellType::Pyramid => {
                for (face_index, face) in volume_faces(kind).iter().enumerate() {
                    // Triangular faces are stored with a repeated last node.
                    let Some(polygon) = Face::new(nodes, face) else {
                        continue;
                    };
                    match faces.entry(polygon.key()) {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(Some(FaceOwner {
                                cell: NonZeroUsize::new(cell + 1).expect("one-based cell index"),
                                face: face_index as u8,
                            }));
                        }
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            entry.insert(None);
                        }
                    }
                }
            }
            CellType::Vertex | CellType::Line => {}
        }
    }
    let mut boundary: Vec<_> = faces
        .into_iter()
        .filter_map(|(key, owner)| owner.map(|owner| (key, owner)))
        .collect();
    boundary.sort_unstable_by_key(|(key, _)| *key);
    for (_, owner) in boundary {
        visit_face(
            dataset,
            owner.cell.get() - 1,
            usize::from(owner.face),
            &mut visit,
        )?;
    }
    Ok(())
}

fn visit_face<E>(
    dataset: &DataSet,
    cell: usize,
    face: usize,
    visit: &mut impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), E> {
    let nodes = dataset.cells.cell(cell).expect("boundary owner cell");
    let face = &volume_faces(dataset.cells.types[cell])[face];
    let polygon = Face::new(nodes, face).expect("surviving nondegenerate face");
    let nodes = polygon.nodes;
    visit(cell, [nodes[0], nodes[1], nodes[2]])?;
    if polygon.len.get() == 4 {
        visit(cell, [nodes[0], nodes[2], nodes[3]])?;
    }
    Ok(())
}

/// Visits surface chunks with each triangle's source cell index.
///
/// `cell_ids[i]` owns the triangle at `chunk.indices[i * 3..i * 3 + 3]`.
/// The cell IDs are borrowed only during the callback. Ordering and face culling
/// match [`boundary_triangles`]. No full triangle array is constructed.
/// Chunk indices remain representable as u32. Source geometry stays immutable.
/// This legacy API has no face-workspace byte limit; prefer
/// [`try_for_each_surface_chunk_with_limits`] for constrained or untrusted input.
///
/// # Errors
///
/// Returns the first callback error and stops visiting immediately.
///
/// # Panics
///
/// Panics when `max_triangles` is zero or surface storage cannot be allocated.
pub fn try_for_each_surface_chunk<E>(
    dataset: ValidatedDataSet<'_>,
    max_triangles: usize,
    visit: impl FnMut(SurfaceChunk, &[usize]) -> Result<(), E>,
) -> Result<(), E> {
    match visit_chunks_with_limits(dataset.as_dataset(), max_triangles, u64::MAX, true, visit) {
        Ok(()) => Ok(()),
        Err(BoundaryError::Callback(error)) => Err(error),
        Err(BoundaryError::Mesh(error)) => panic!("surface extraction failed: {error}"),
    }
}

/// Visits surface chunks within an explicit volume-face workspace limit.
///
/// The same ordering, ownership and chunk sizing as
/// [`try_for_each_surface_chunk`] apply. General volume meshes preflight all
/// face-record bytes before allocating the workspace or invoking callbacks.
/// Verified canonical grids use constant-space cursors and no face records.
/// Direct surface cells need no face records. Limits exclude output retained by
/// callbacks and the chunk's vectors/remapping table; triangle caps bound their
/// element counts separately. When a complete canonical boundary fits one chunk,
/// its remapping table uses four bytes per boundary point. Allocation is checked
/// before callbacks. Callbacks can stop traversal, including canonical
/// traversal, with their first error.
///
/// # Errors
///
/// Returns workspace/address limits, failed face/remapping reservation or the first
/// callback error. `E` must convert a [`MeshError`] for resource failures.
///
/// # Panics
///
/// Panics when `max_triangles` is zero.
pub fn try_for_each_surface_chunk_with_limits<E: From<MeshError>>(
    dataset: ValidatedDataSet<'_>,
    max_triangles: usize,
    limits: SurfaceExtractionLimits,
    visit: impl FnMut(SurfaceChunk, &[usize]) -> Result<(), E>,
) -> Result<(), E> {
    visit_chunks_with_limits(
        dataset.as_dataset(),
        max_triangles,
        limits.max_face_bytes,
        true,
        visit,
    )
    .map_err(|error| match error {
        BoundaryError::Mesh(error) => E::from(error),
        BoundaryError::Callback(error) => error,
    })
}

fn visit_chunks_with_limits<E>(
    dataset: &DataSet,
    max_triangles: usize,
    max_face_bytes: u64,
    record_owners: bool,
    visit: impl FnMut(SurfaceChunk, &[usize]) -> Result<(), E>,
) -> Result<(), BoundaryError<E>> {
    let mut chunks = ChunkBuilder::new(max_triangles, record_owners, visit);
    let grid = CanonicalHexGrid::detect(dataset);
    if let Some(local) = PointRemap::for_grid(grid, chunks.max_triangles)? {
        chunks.local = local;
    }
    visit_boundary_with_grid(dataset, max_face_bytes, grid, |cell, triangle| {
        chunks.push(cell, triangle)
    })?;
    chunks.finish().map_err(BoundaryError::Callback)
}

enum PointRemap {
    Hash(HashMap<u32, u32>),
    Boundary {
        grid: CanonicalHexGrid,
        slots: Vec<u32>,
    },
}

impl PointRemap {
    fn for_grid(
        grid: Option<CanonicalHexGrid>,
        max_triangles: usize,
    ) -> Result<Option<Self>, MeshError> {
        let Some((grid, count)) =
            grid.and_then(|g| g.single_chunk_points(max_triangles).map(|n| (g, n)))
        else {
            return Ok(None);
        };
        let requested = (count as u64).saturating_mul(size_of::<u32>() as u64);
        if requested > isize::MAX as u64 {
            return Err(MeshError::ResourceLimit {
                resource: "surface boundary remap bytes",
                requested,
                limit: isize::MAX as u64,
            });
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(count)
            .map_err(|_| MeshError::Allocation { bytes: requested })?;
        // Local IDs never reach this sentinel: the chunk cap bounds all three
        // triangle corners below u32::MAX. Ordinals do not define output order.
        slots.resize(count, u32::MAX);
        Ok(Some(Self::Boundary { grid, slots }))
    }

    fn clear(&mut self) {
        match self {
            Self::Hash(local) => local.clear(),
            Self::Boundary { slots, .. } => slots.fill(u32::MAX),
        }
    }
}

struct ChunkBuilder<F> {
    max_triangles: usize,
    record_owners: bool,
    triangle_count: usize,
    local: PointRemap,
    point_ids: Vec<u32>,
    indices: Vec<u32>,
    cell_ids: Vec<usize>,
    visit: F,
}

impl<F> ChunkBuilder<F> {
    fn new(max_triangles: usize, record_owners: bool, visit: F) -> Self {
        assert!(max_triangles > 0, "max_triangles must be at least 1");
        Self {
            max_triangles: max_triangles.min(u32::MAX as usize / 3),
            record_owners,
            triangle_count: 0,
            local: PointRemap::Hash(HashMap::new()),
            point_ids: Vec::new(),
            indices: Vec::new(),
            cell_ids: Vec::new(),
            visit,
        }
    }

    fn push<E>(&mut self, cell: usize, triangle: [u32; 3]) -> Result<(), E>
    where
        F: FnMut(SurfaceChunk, &[usize]) -> Result<(), E>,
    {
        match &mut self.local {
            PointRemap::Hash(local) => {
                for point in triangle {
                    let id = *local.entry(point).or_insert_with(|| {
                        let id = self.point_ids.len() as u32;
                        self.point_ids.push(point);
                        id
                    });
                    self.indices.push(id);
                }
            }
            PointRemap::Boundary { grid, slots } => {
                for point in triangle {
                    let slot =
                        &mut slots[grid.boundary_slot(point).expect("verified boundary point")];
                    if *slot == u32::MAX {
                        *slot = self.point_ids.len() as u32;
                        self.point_ids.push(point);
                    }
                    self.indices.push(*slot);
                }
            }
        }
        if self.record_owners {
            self.cell_ids.push(cell);
        }
        self.triangle_count += 1;
        if self.triangle_count == self.max_triangles {
            (self.visit)(
                SurfaceChunk {
                    point_ids: std::mem::take(&mut self.point_ids),
                    indices: std::mem::take(&mut self.indices),
                },
                &self.cell_ids,
            )?;
            self.cell_ids.clear();
            self.triangle_count = 0;
            self.local.clear();
        }
        Ok(())
    }

    fn finish<E>(mut self) -> Result<(), E>
    where
        F: FnMut(SurfaceChunk, &[usize]) -> Result<(), E>,
    {
        if self.triangle_count != 0 {
            (self.visit)(
                SurfaceChunk {
                    point_ids: self.point_ids,
                    indices: self.indices,
                },
                &self.cell_ids,
            )?;
        }
        Ok(())
    }
}

/// Splits the boundary surface into chunks of at most `max_triangles` each.
///
/// Chunks are collected without a full intermediate triangle array. This legacy
/// convenience API retains all output and has no workspace byte limit. Prefer
/// [`try_for_each_surface_chunk_with_limits`] for a streaming consumer.
///
/// # Panics
///
/// Panics when `max_triangles` is zero or surface storage cannot be allocated.
#[must_use]
pub fn surface_chunks(dataset: &DataSet, max_triangles: usize) -> Vec<SurfaceChunk> {
    let mut output = Vec::new();
    let result = visit_chunks_with_limits(
        dataset,
        max_triangles,
        u64::MAX,
        false,
        |chunk, _: &[usize]| {
            output.push(chunk);
            Ok::<(), std::convert::Infallible>(())
        },
    );
    match result {
        Ok(()) => output,
        Err(BoundaryError::Mesh(error)) => panic!("surface extraction failed: {error}"),
        Err(BoundaryError::Callback(never)) => match never {},
    }
}

/// Collects surface chunks with a volume-face workspace limit.
///
/// Unlike the streaming visitor, this retains all chunks. The limit covers only
/// temporary face records, as described by [`SurfaceExtractionLimits`]. No
/// per-triangle source-cell array or full intermediate triangle array is built.
///
/// # Errors
///
/// Returns workspace/address limits or failed face/remapping reservation.
///
/// # Panics
///
/// Panics when `max_triangles` is zero.
pub fn surface_chunks_with_limits(
    dataset: ValidatedDataSet<'_>,
    max_triangles: usize,
    limits: SurfaceExtractionLimits,
) -> Result<Vec<SurfaceChunk>, MeshError> {
    let mut output = Vec::new();
    visit_chunks_with_limits(
        dataset.as_dataset(),
        max_triangles,
        limits.max_face_bytes,
        false,
        |chunk, _: &[usize]| {
            output.push(chunk);
            Ok::<(), MeshError>(())
        },
    )
    .map_err(|error| match error {
        BoundaryError::Mesh(error) | BoundaryError::Callback(error) => error,
    })?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::Cells;

    #[test]
    fn canonical_point_remapping_preserves_independent_first_encounter_order() {
        for dims in [
            [2, 2, 2],
            [2, 3, 7],
            [5, 2, 4],
            [4, 5, 2],
            [3, 4, 5],
            [6; 3],
        ] {
            let mut dataset =
                DataSet::structured_grid("mapping", dims, [0.0; 3], [1.0; 3]).unwrap();
            // Topological proof must remain valid for arbitrary curvilinear positions.
            for p in std::sync::Arc::make_mut(&mut dataset.positions) {
                *p = [p[0] + p[1] * p[2], p[1] - p[0], p[2] + p[0] * p[0]];
            }
            let mut reference = Vec::new();
            visit_general_boundary(&dataset, |owner, triangle| {
                reference.push((owner, triangle));
                Ok::<_, MeshError>(())
            })
            .unwrap();
            let count = reference.len();
            for cap in [1, 2, 11, count - 1, count, count + 1, usize::MAX] {
                let expected: Vec<_> = reference
                    .chunks(cap)
                    .map(|part| {
                        let mut point_ids = Vec::new();
                        let mut indices = Vec::new();
                        let mut owners = Vec::new();
                        for &(owner, triangle) in part {
                            owners.push(owner);
                            for point in triangle {
                                let local = point_ids
                                    .iter()
                                    .position(|p| *p == point)
                                    .unwrap_or_else(|| {
                                        point_ids.push(point);
                                        point_ids.len() - 1
                                    });
                                indices.push(local as u32);
                            }
                        }
                        (SurfaceChunk { point_ids, indices }, owners)
                    })
                    .collect();
                let mut actual = Vec::new();
                try_for_each_surface_chunk_with_limits(
                    dataset.validated().unwrap(),
                    cap,
                    SurfaceExtractionLimits { max_face_bytes: 0 },
                    |chunk, owners| {
                        actual.push((chunk, owners.to_vec()));
                        Ok::<_, MeshError>(())
                    },
                )
                .unwrap();
                assert_eq!(actual, expected, "{dims:?}, cap {cap}");
                let expected_chunks: Vec<_> =
                    expected.into_iter().map(|(chunk, _)| chunk).collect();
                assert_eq!(
                    surface_chunks_with_limits(
                        dataset.validated().unwrap(),
                        cap,
                        SurfaceExtractionLimits { max_face_bytes: 0 }
                    )
                    .unwrap(),
                    expected_chunks
                );
                assert_eq!(surface_chunks(&dataset, cap), expected_chunks);
            }
        }
    }

    #[test]
    fn boundary_remap_selection_is_limited_to_one_complete_chunk() {
        let dataset = DataSet::structured_grid("selection", [4; 3], [0.0; 3], [1.0; 3]).unwrap();
        let grid = CanonicalHexGrid::detect(&dataset).unwrap();
        let triangles = 6 * 2 * 3 * 3;
        assert!(
            PointRemap::for_grid(Some(grid), triangles - 1)
                .unwrap()
                .is_none()
        );
        assert!(PointRemap::for_grid(None, triangles).unwrap().is_none());
        let Some(PointRemap::Boundary { slots, .. }) =
            PointRemap::for_grid(Some(grid), triangles).unwrap()
        else {
            panic!("complete boundary should use compact slots")
        };
        assert_eq!(slots.len(), 4usize.pow(3) - 2usize.pow(3));
        assert!(slots.iter().all(|&v| v == u32::MAX));
        let mut calls = 0;
        let result = try_for_each_surface_chunk_with_limits(
            dataset.validated().unwrap(),
            triangles,
            SurfaceExtractionLimits { max_face_bytes: 0 },
            |_, owners| {
                calls += 1;
                assert_eq!(owners.len(), triangles);
                Err(MeshError::Allocation { bytes: 123 })
            },
        );
        assert_eq!(result, Err(MeshError::Allocation { bytes: 123 }));
        assert_eq!(calls, 1);
    }

    #[test]
    fn face_record_budget_boundary_is_checked_before_reservation() {
        let dataset = DataSet::new(
            "tet",
            vec![[0.0; 3]; 4],
            Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
        );
        let bytes = 4 * size_of::<FaceRecord>() as u64;
        assert_eq!(face_workspace(&dataset, bytes).unwrap(), 4);
        assert_eq!(
            face_workspace(&dataset, bytes - 1),
            Err(MeshError::ResourceLimit {
                resource: "surface face workspace bytes",
                requested: bytes,
                limit: bytes - 1,
            })
        );
        assert!(size_of::<FaceRecord>() <= 32);
    }

    #[test]
    fn canonical_grids_match_general_face_order_owners_and_winding() {
        for nx in 2..=6 {
            for ny in 2..=6 {
                for nz in 2..=6 {
                    let dims = [nx, ny, nz];
                    let dataset = DataSet::structured_grid(
                        "grid",
                        dims,
                        [1e12, -2.0, 0.25],
                        [-0.5, 1.25, 0.75],
                    )
                    .unwrap();
                    assert!(
                        crate::structured_boundary::CanonicalHexGrid::detect(&dataset).is_some()
                    );
                    let mut expected = Vec::new();
                    visit_general_boundary(&dataset, |cell, tri| {
                        expected.push((cell, tri));
                        Ok::<(), ()>(())
                    })
                    .unwrap();
                    let mut actual = Vec::new();
                    visit_boundary(&dataset, |cell, tri| {
                        actual.push((cell, tri));
                        Ok::<(), ()>(())
                    })
                    .unwrap();
                    assert_eq!(
                        actual.len(),
                        4 * ((nx - 1) * (ny - 1) + (nx - 1) * (nz - 1) + (ny - 1) * (nz - 1))
                    );
                    assert_eq!(
                        actual, expected,
                        "owner, order or winding differs for {dims:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn edited_or_reordered_grids_use_the_general_boundary_contract() {
        let original = DataSet::structured_grid("grid", [3, 4, 5], [0.0; 3], [1.0; 3]).unwrap();
        for edit in 0..4 {
            let mut dataset = original.clone();
            let nodes = std::sync::Arc::make_mut(&mut dataset.cells.connectivity);
            match edit {
                0 => {
                    let first: [u32; 8] = nodes[..8].try_into().unwrap();
                    let last = nodes.len() - 8;
                    nodes.copy_within(last..last + 8, 0);
                    nodes[last..last + 8].copy_from_slice(&first);
                }
                1 => {
                    let last = nodes.len() - 8;
                    nodes[last..last + 4].rotate_left(1);
                    nodes[last + 4..].rotate_left(1);
                }
                2 => {
                    nodes[nodes.len() - 1] = nodes[nodes.len() - 2];
                }
                _ => {
                    let mut positions = dataset.positions.to_vec();
                    positions.push([100.0; 3]);
                    dataset.positions = positions.into();
                }
            }
            dataset.validate().unwrap();
            assert!(crate::structured_boundary::CanonicalHexGrid::detect(&dataset).is_none());
            let mut expected = Vec::new();
            visit_general_boundary(&dataset, |cell, tri| {
                expected.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            let mut actual = Vec::new();
            visit_boundary(&dataset, |cell, tri| {
                actual.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(actual, expected, "fallback differs for edit {edit}");
        }
    }

    #[test]
    fn thin_grids_preserve_canonical_order_at_long_row_and_plane_boundaries() {
        for dims in [
            [2, 2, 257],
            [2, 257, 2],
            [257, 2, 2],
            [2, 3, 257],
            [31, 2, 7],
            [2, 31, 7],
        ] {
            let dataset = DataSet::structured_grid("thin", dims, [0.0; 3], [1.0; 3]).unwrap();
            let mut expected = Vec::new();
            visit_general_boundary(&dataset, |cell, tri| {
                expected.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            let mut actual = Vec::new();
            visit_boundary(&dataset, |cell, tri| {
                actual.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(actual, expected, "thin grid differs for {dims:?}");
        }
    }

    #[test]
    fn structured_boundary_visit_stops_on_the_first_callback_error() {
        let dataset = DataSet::structured_grid("grid", [4, 5, 6], [0.0; 3], [1.0; 3]).unwrap();
        let mut visits = 0;
        let result = visit_boundary(&dataset, |_, _| {
            visits += 1;
            Err("stop")
        });
        assert_eq!(result, Err("stop"));
        assert_eq!(visits, 1);
    }

    #[test]
    fn owned_chunks_keep_exact_surface_and_volume_cell_ids() {
        let volume = DataSet::structured_grid("pair", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        for budget in [1, 3, usize::MAX] {
            let mut got = Vec::new();
            let mut counts = [0usize; 2];
            try_for_each_surface_chunk(volume.validated().unwrap(), budget, |chunk, owners| {
                assert_eq!(chunk.indices.len(), owners.len() * 3);
                assert!(owners.len() <= budget);
                for (tri, &cell) in chunk.indices.as_chunks::<3>().0.iter().zip(owners) {
                    let global = tri.map(|i| chunk.point_ids[i as usize]);
                    let xs = global.map(|i| volume.positions[i as usize][0]);
                    assert!(!xs.iter().all(|&x| x == 1.0), "interior face leaked");
                    let expected = usize::from(xs.iter().all(|&x| x >= 1.0));
                    assert_eq!(cell, expected);
                    counts[cell] += 1;
                    got.push(global);
                }
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(got, boundary_triangles(&volume));
            assert_eq!(counts, [10, 10]);
        }
        let surface = DataSet::new(
            "mixed",
            vec![[0.0; 3]; 5],
            Cells::from_cells(&[
                (CellType::Line, &[0, 1]),
                (CellType::Vertex, &[0]),
                (CellType::Triangle, &[0, 1, 2]),
                (CellType::Quad, &[0, 1, 2, 3]),
                (CellType::Polygon, &[0, 1, 2, 3, 4]),
            ]),
        );
        let mut cells = Vec::new();
        try_for_each_surface_chunk(surface.validated().unwrap(), 2, |_, owners| {
            cells.extend_from_slice(owners);
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!(cells, [2, 3, 3, 4, 4, 4]);
    }

    #[test]
    fn every_volume_face_retains_its_owner_without_larger_map_entries() {
        let dataset = DataSet::new(
            "types",
            vec![[0.0; 3]; 23],
            Cells::from_cells(&[
                (CellType::Tetra, &[0, 1, 2, 3]),
                (CellType::Wedge, &[4, 5, 6, 7, 8, 9]),
                (CellType::Pyramid, &[10, 11, 12, 13, 14]),
                (CellType::Hexahedron, &[15, 16, 17, 18, 19, 20, 21, 22]),
            ]),
        );
        dataset.validate().unwrap();
        let mut counts = [0usize; 4];
        let mut got = Vec::new();
        try_for_each_surface_chunk(dataset.validated().unwrap(), 7, |chunk, owners| {
            for (tri, &cell) in chunk.indices.as_chunks::<3>().0.iter().zip(owners) {
                let global = tri.map(|i| chunk.point_ids[i as usize]);
                assert!(
                    global
                        .iter()
                        .all(|id| dataset.cells.cell(cell).unwrap().contains(id))
                );
                counts[cell] += 1;
                got.push(global);
            }
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!(counts, [4, 8, 6, 12]);
        assert_eq!(got, boundary_triangles(&dataset));
        assert!(
            size_of::<([u32; 5], Option<FaceOwner>)>() <= size_of::<([u32; 5], Option<Face>)>()
        );
    }

    #[test]
    fn owned_chunk_callback_error_stops_visiting() {
        let dataset = DataSet::structured_grid("pair", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        let mut calls = 0;
        let result = try_for_each_surface_chunk(dataset.validated().unwrap(), 1, |_, _| {
            calls += 1;
            Err("stop")
        });
        assert_eq!(result, Err("stop"));
        assert_eq!(calls, 1);
    }

    #[test]
    fn single_hexahedron_has_twelve_boundary_triangles() {
        let ds = DataSet::structured_grid("g", [2, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        assert_eq!(boundary_triangles(&ds).len(), 12);
    }

    #[test]
    fn two_hexahedra_cull_the_shared_face() {
        let ds = DataSet::structured_grid("g", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        // Boundary quads: 2*6 - 2 for the culled shared face = 10 quads = 20 triangles.
        assert_eq!(boundary_triangles(&ds).len(), 20);
    }

    #[test]
    fn chunks_respect_budget_and_round_trip_points() {
        let ds = DataSet::structured_grid("g", [5, 5, 5], [0.0; 3], [1.0; 3]).unwrap();
        let total = boundary_triangles(&ds).len();
        let chunks = surface_chunks(&ds, 10);
        assert!(chunks.iter().all(|c| c.indices.len() / 3 <= 10));
        assert_eq!(
            chunks.iter().map(|c| c.indices.len() / 3).sum::<usize>(),
            total
        );
        for chunk in &chunks {
            for &local in &chunk.indices {
                assert!((local as usize) < chunk.point_ids.len());
            }
        }
        let _ = Cells::triangles(&[[0, 1, 2]]);
    }

    #[test]
    fn extraction_is_deterministic_and_tetra_shared_faces_are_culled() {
        let dataset = DataSet::structured_grid("g", [8; 3], [0.0; 3], [1.0; 3]).unwrap();
        let expected = boundary_triangles(&dataset);
        for _ in 0..4 {
            assert_eq!(boundary_triangles(&dataset), expected);
        }
        let tetrahedra = DataSet::new(
            "tets",
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ],
            Cells::from_cells(&[
                (CellType::Tetra, &[0, 1, 2, 3]),
                (CellType::Tetra, &[0, 2, 1, 4]),
            ]),
        );
        tetrahedra.validate().unwrap();
        assert_eq!(boundary_triangles(&tetrahedra).len(), 6);
    }

    #[test]
    fn face_arity_is_part_of_identity_and_optional_storage_stays_compact() {
        let triangle = Face::new(&[0, 1, 2], &[0, 1, 2, 2]).unwrap();
        let quad = Face::new(&[0, 1, 2, u32::MAX], &[0, 1, 2, 3]).unwrap();
        assert_ne!(triangle.key(), quad.key());
        assert_eq!(
            std::mem::size_of::<Face>(),
            std::mem::size_of::<Option<Face>>()
        );
    }
}
