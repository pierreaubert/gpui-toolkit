use std::collections::HashMap;

/// Unique-edge and adjacency data derived once per geometry revision.
/// Edge endpoints are stored sorted (min, max); winding-independent.
#[derive(Debug, Clone)]
pub struct MeshTopology {
    /// [v0, v1] with v0 < v1.
    pub unique_edges: Vec<[u32; 2]>,
    /// unique_edges index for each triangle's 3 edges (slots: opposite vertex 0,1,2
    /// — slot i is the edge between the other two vertices).
    pub triangle_edges: Vec<[u32; 3]>,
    /// Up to two triangles incident on each edge; u32::MAX = none (boundary).
    pub edge_triangles: Vec<[u32; 2]>,
    /// Indices into unique_edges that bound exactly one triangle.
    pub boundary_edges: Vec<u32>,
}

impl MeshTopology {
    /// O(n) build via hash map on sorted endpoint pairs.
    pub fn build(triangles: &[[u32; 3]]) -> Self {
        let mut edge_index: HashMap<(u32, u32), u32> = HashMap::with_capacity(triangles.len() * 2);
        let mut unique_edges: Vec<[u32; 2]> = Vec::with_capacity(triangles.len() * 2);
        let mut edge_triangles: Vec<[u32; 2]> = Vec::with_capacity(triangles.len() * 2);
        let mut triangle_edges: Vec<[u32; 3]> = Vec::with_capacity(triangles.len());

        for (t, tri) in triangles.iter().enumerate() {
            let mut slots = [0u32; 3];
            // slot i = edge opposite vertex i: (v[(i+1)%3], v[(i+2)%3])
            for slot in 0..3 {
                let a = tri[(slot + 1) % 3];
                let b = tri[(slot + 2) % 3];
                let key = if a < b { (a, b) } else { (b, a) };
                let ei = *edge_index.entry(key).or_insert_with(|| {
                    let e = unique_edges.len() as u32;
                    unique_edges.push([key.0, key.1]);
                    edge_triangles.push([u32::MAX, u32::MAX]);
                    e
                });
                let pair = &mut edge_triangles[ei as usize];
                if pair[0] == u32::MAX {
                    pair[0] = t as u32;
                } else if pair[1] == u32::MAX && pair[0] != t as u32 {
                    pair[1] = t as u32;
                }
                slots[slot] = ei;
            }
            triangle_edges.push(slots);
        }

        // Sort unique edges lexicographically so the result is fully
        // deterministic: independent of triangle order and winding.
        // Remap triangle_edges and edge_triangles accordingly.
        let mut order: Vec<u32> = (0..unique_edges.len() as u32).collect();
        order.sort_by_key(|&i| unique_edges[i as usize]);
        let mut remap = vec![0u32; unique_edges.len()];
        for (new_i, &old_i) in order.iter().enumerate() {
            remap[old_i as usize] = new_i as u32;
        }
        let unique_edges: Vec<[u32; 2]> =
            order.iter().map(|&i| unique_edges[i as usize]).collect();
        let edge_triangles: Vec<[u32; 2]> =
            order.iter().map(|&i| edge_triangles[i as usize]).collect();
        for slots in &mut triangle_edges {
            for s in slots.iter_mut() {
                *s = remap[*s as usize];
            }
        }

        let boundary_edges = edge_triangles
            .iter()
            .enumerate()
            .filter(|(_, pair)| pair[1] == u32::MAX)
            .map(|(i, _)| i as u32)
            .collect();

        Self { unique_edges, triangle_edges, edge_triangles, boundary_edges }
    }
}

#[cfg(test)]
mod tests {
    use super::MeshTopology;

    fn square_topology() -> MeshTopology {
        // two triangles sharing edge (1,2)
        MeshTopology::build(&[[0, 1, 2], [1, 3, 2]])
    }

    #[test]
    fn square_has_five_unique_edges() {
        let topo = square_topology();
        assert_eq!(topo.unique_edges.len(), 5);
    }

    #[test]
    fn shared_edge_has_two_triangles() {
        let topo = square_topology();
        let shared = topo.unique_edges.iter().position(|&e| e == [1, 2]).unwrap() as u32;
        let tris = topo.edge_triangles[shared as usize];
        assert!(tris[0] != u32::MAX && tris[1] != u32::MAX);
    }

    #[test]
    fn boundary_edges_have_one_triangle() {
        let topo = square_topology();
        assert_eq!(topo.boundary_edges.len(), 4);
        for &e in &topo.boundary_edges {
            assert_eq!(topo.edge_triangles[e as usize][1], u32::MAX);
        }
    }

    #[test]
    fn deterministic_regardless_of_triangle_order() {
        let a = MeshTopology::build(&[[0, 1, 2], [1, 3, 2]]);
        let b = MeshTopology::build(&[[1, 3, 2], [0, 1, 2]]);
        // same undirected edge set
        let mut ea = a.unique_edges.clone();
        let mut eb = b.unique_edges.clone();
        ea.sort();
        eb.sort();
        assert_eq!(ea, eb);
    }

    #[test]
    fn reversed_winding_same_edges() {
        let a = MeshTopology::build(&[[0, 1, 2]]);
        let b = MeshTopology::build(&[[0, 2, 1]]);
        assert_eq!(a.unique_edges, b.unique_edges);
    }
}
