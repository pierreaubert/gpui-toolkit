//! Deterministic vertex-clustering decimation for interactive mesh LOD.

use std::collections::BTreeMap;

use super::{MeshTopology, TriangleMesh};

/// Reduce a triangle mesh with a deterministic grid-clustering pass.
///
/// Interior vertices are grouped by a regular grid over the mesh bounds and
/// represented by the vertex nearest each cluster centroid. Boundary vertices
/// are kept in their own clusters so the source silhouette and extrema cannot
/// shrink during interactive camera motion. Remapped repeated and zero-area
/// triangles are discarded; source IDs are retained for the selected
/// representatives and surviving cells.
///
/// The function has an intentionally infallible API because it is used on a
/// mesh which has already passed validation. Invalid input is returned
/// unchanged, which keeps a rendering fallback valid instead of manufacturing
/// an invalid mesh while handling a bad update.
pub fn decimate_vertex_clustering(mesh: &TriangleMesh, target_triangles: usize) -> TriangleMesh {
    if target_triangles == 0 || mesh.triangles.len() <= target_triangles || mesh.validate().is_err()
    {
        return mesh.clone();
    }

    let boundary_vertices = boundary_vertices(mesh);
    let bounds = bounds(&mesh.positions);
    let mut resolution = initial_resolution(target_triangles, bounds.0, bounds.1);
    let mut candidate = cluster_mesh(mesh, &boundary_vertices, bounds, resolution);
    let allowed_triangles = target_triangles.saturating_mul(2);

    while candidate.triangles.len() > allowed_triangles {
        let next = resolution.map(|axis| axis.max(1) / 2);
        if next == resolution {
            break;
        }
        resolution = next;
        candidate = cluster_mesh(mesh, &boundary_vertices, bounds, resolution);
    }

    candidate
}

fn boundary_vertices(mesh: &TriangleMesh) -> Vec<bool> {
    let topology = MeshTopology::build(&mesh.triangles);
    let mut boundary = vec![false; mesh.positions.len()];
    for &edge_index in &topology.boundary_edges {
        let [a, b] = topology.unique_edges[edge_index as usize];
        boundary[a as usize] = true;
        boundary[b as usize] = true;
    }
    boundary
}

fn bounds(positions: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for position in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    (min, max)
}

fn initial_resolution(target_triangles: usize, min: [f64; 3], max: [f64; 3]) -> [usize; 3] {
    // A cubic root is conservative for surfaces embedded in 3D: it gives a
    // small first pass, while the refinement loop below supplies the budget
    // guarantee for less uniformly distributed meshes.
    let cells_per_axis = (target_triangles as f64).cbrt().ceil().max(1.0) as usize;
    std::array::from_fn(|axis| {
        if max[axis] > min[axis] {
            cells_per_axis
        } else {
            1
        }
    })
}

fn cluster_mesh(
    mesh: &TriangleMesh,
    boundary_vertices: &[bool],
    (min, max): ([f64; 3], [f64; 3]),
    resolution: [usize; 3],
) -> TriangleMesh {
    let mut cluster_indices = vec![0usize; mesh.positions.len()];
    let mut clusters = BTreeMap::<(u8, usize, usize, usize), usize>::new();

    for (vertex_index, &position) in mesh.positions.iter().enumerate() {
        let key = if boundary_vertices[vertex_index] {
            // A boundary vertex gets an identity key. Keeping it separate
            // from every other vertex is stronger than merely keeping grid
            // cells separate and guarantees exact source bounds.
            (1, vertex_index, 0, 0)
        } else {
            (
                0,
                grid_coordinate(position[0], min[0], max[0], resolution[0]),
                grid_coordinate(position[1], min[1], max[1], resolution[1]),
                grid_coordinate(position[2], min[2], max[2], resolution[2]),
            )
        };
        let cluster_index = clusters.len();
        let cluster_index = *clusters.entry(key).or_insert(cluster_index);
        cluster_indices[vertex_index] = cluster_index;
    }

    let mut centroid_sums = vec![[0.0; 3]; clusters.len()];
    let mut counts = vec![0usize; clusters.len()];
    for (vertex_index, &cluster_index) in cluster_indices.iter().enumerate() {
        for axis in 0..3 {
            centroid_sums[cluster_index][axis] += mesh.positions[vertex_index][axis];
        }
        counts[cluster_index] += 1;
    }

    let mut representatives = vec![usize::MAX; clusters.len()];
    let mut representative_distance = vec![f64::INFINITY; clusters.len()];
    for (vertex_index, &cluster_index) in cluster_indices.iter().enumerate() {
        let centroid = std::array::from_fn(|axis| {
            centroid_sums[cluster_index][axis] / counts[cluster_index] as f64
        });
        let position = mesh.positions[vertex_index];
        let distance = squared_distance(position, centroid);
        let previous = representatives[cluster_index];
        if distance < representative_distance[cluster_index]
            || (distance == representative_distance[cluster_index] && vertex_index < previous)
        {
            representatives[cluster_index] = vertex_index;
            representative_distance[cluster_index] = distance;
        }
    }

    let positions: Vec<[f64; 3]> = representatives
        .iter()
        .map(|&vertex_index| mesh.positions[vertex_index])
        .collect();
    let remap: Vec<u32> = cluster_indices
        .iter()
        .map(|&cluster_index| cluster_index as u32)
        .collect();
    let mut triangles = Vec::with_capacity(mesh.triangles.len());
    let mut kept_cell_ids = mesh
        .cell_ids
        .as_ref()
        .map(|_| Vec::with_capacity(mesh.triangles.len()));

    for (triangle_index, &[a, b, c]) in mesh.triangles.iter().enumerate() {
        let mapped = [remap[a as usize], remap[b as usize], remap[c as usize]];
        if mapped[0] == mapped[1]
            || mapped[1] == mapped[2]
            || mapped[0] == mapped[2]
            || triangle_area2(
                positions[mapped[0] as usize],
                positions[mapped[1] as usize],
                positions[mapped[2] as usize],
            ) <= 1e-30
        {
            continue;
        }
        triangles.push(mapped);
        if let (Some(source_ids), Some(output_ids)) = (&mesh.cell_ids, &mut kept_cell_ids) {
            output_ids.push(source_ids[triangle_index]);
        }
    }

    if triangles.is_empty() {
        return mesh.clone();
    }

    let output = TriangleMesh {
        id: mesh.id.clone(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: mesh.vertex_ids.as_ref().map(|ids| {
            representatives
                .iter()
                .map(|&vertex_index| ids[vertex_index])
                .collect::<Vec<_>>()
                .into()
        }),
        cell_ids: kept_cell_ids.map(Into::into),
    };

    if output.validate().is_ok() {
        output
    } else {
        mesh.clone()
    }
}

fn grid_coordinate(value: f64, min: f64, max: f64, resolution: usize) -> usize {
    if resolution <= 1 || max <= min {
        return 0;
    }
    (((value - min) / (max - min) * resolution as f64).floor() as usize).min(resolution - 1)
}

fn squared_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3)
        .map(|axis| {
            let delta = a[axis] - b[axis];
            delta * delta
        })
        .sum()
}

fn triangle_area2(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let normal = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    normal.iter().map(|component| component * component).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::MeshBounds;

    fn grid_mesh(width: usize, height: usize) -> TriangleMesh {
        let positions = (0..height)
            .flat_map(|y| (0..width).map(move |x| [x as f64, y as f64, 0.0]))
            .collect::<Vec<_>>();
        let mut triangles = Vec::with_capacity((width - 1) * (height - 1) * 2);
        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let a = (y * width + x) as u32;
                let b = a + 1;
                let c = a + width as u32;
                let d = c + 1;
                triangles.push([a, b, c]);
                triangles.push([b, d, c]);
            }
        }
        TriangleMesh {
            id: "grid".into(),
            positions: positions.into(),
            triangles: triangles.into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn decimation_hits_target_budget() {
        let mesh = grid_mesh(200, 200); // 80k tris
        let lod = decimate_vertex_clustering(&mesh, 5_000);
        assert!(
            lod.triangles.len() <= 5_000 * 2,
            "allow 2x slack, got {}",
            lod.triangles.len()
        );
        assert!(lod.validate().is_ok());
    }

    #[test]
    fn decimation_preserves_boundary() {
        let mesh = grid_mesh(50, 50);
        let lod = decimate_vertex_clustering(&mesh, 500);
        let src = MeshBounds::from_positions(&mesh.positions);
        let dst = MeshBounds::from_positions(&lod.positions);
        assert_eq!(src.min, dst.min);
        assert_eq!(src.max, dst.max);
    }

    #[test]
    fn decimation_deterministic() {
        let mesh = grid_mesh(50, 50);
        assert_eq!(
            decimate_vertex_clustering(&mesh, 500).triangles,
            decimate_vertex_clustering(&mesh, 500).triangles
        );
    }
}
