//! Public face-workspace limits, surface order and owner contracts.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{
    CellType, Cells, DataSet, MeshError, SurfaceExtractionLimits, surface_chunks_with_limits,
    try_for_each_surface_chunk_with_limits,
};

fn pair() -> DataSet {
    DataSet::new(
        "pair",
        vec![[0.0; 3]; 7],
        Cells::from_cells(&[
            (CellType::Tetra, &[0, 1, 2, 3]),
            (CellType::Tetra, &[0, 2, 1, 4]),
            (CellType::Triangle, &[0, 4, 5]),
            (CellType::Quad, &[3, 4, 5, 6]),
            (CellType::Polygon, &[0, 1, 2, 3, 4]),
        ]),
    )
}

#[test]
fn mixed_boundary_has_exact_direct_then_sorted_volume_owners_and_winding() {
    let dataset = pair();
    let expected = vec![
        (2, [0, 4, 5]),
        (3, [3, 4, 5]),
        (3, [3, 5, 6]),
        (4, [0, 1, 2]),
        (4, [0, 2, 3]),
        (4, [0, 3, 4]),
        (0, [0, 1, 3]),
        (1, [1, 0, 4]),
        (0, [2, 0, 3]),
        (1, [0, 2, 4]),
        (0, [1, 2, 3]),
        (1, [2, 1, 4]),
    ];
    for triangles in [1, 2, 7, usize::MAX] {
        let mut actual = Vec::new();
        try_for_each_surface_chunk_with_limits(
            dataset.validated().unwrap(),
            triangles,
            SurfaceExtractionLimits::default(),
            |chunk, owners| {
                for (triangle, &owner) in chunk.indices.as_chunks::<3>().0.iter().zip(owners) {
                    actual.push((owner, triangle.map(|i| chunk.point_ids[i as usize])));
                }
                Ok::<(), MeshError>(())
            },
        )
        .unwrap();
        assert_eq!(actual, expected);
        let chunks = surface_chunks_with_limits(
            dataset.validated().unwrap(),
            triangles,
            SurfaceExtractionLimits::default(),
        )
        .unwrap();
        let triangles: Vec<_> = chunks
            .iter()
            .flat_map(|chunk| {
                chunk
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|triangle| triangle.map(|i| chunk.point_ids[i as usize]))
            })
            .collect();
        assert_eq!(
            triangles,
            expected.iter().map(|&(_, t)| t).collect::<Vec<_>>()
        );
    }
}

#[test]
fn face_limit_rejects_before_callbacks_and_zero_allows_no_record_paths() {
    let dataset = pair();
    let mut calls = 0;
    let error = try_for_each_surface_chunk_with_limits(
        dataset.validated().unwrap(),
        1,
        SurfaceExtractionLimits { max_face_bytes: 0 },
        |_, _| {
            calls += 1;
            Ok::<(), MeshError>(())
        },
    )
    .unwrap_err();
    assert_eq!(calls, 0, "preflight must precede direct surface callbacks");
    assert!(matches!(
        error,
        MeshError::ResourceLimit {
            resource: "surface face workspace bytes",
            requested: 1..,
            limit: 0
        }
    ));
    surface_chunks_with_limits(
        dataset.validated().unwrap(),
        2,
        SurfaceExtractionLimits { max_face_bytes: 0 },
    )
    .unwrap_err();
    let canonical = DataSet::structured_grid("grid", [3; 3], [0.0; 3], [1.0; 3]).unwrap();
    let surface = DataSet::surface("surface", vec![[0.0; 3]; 3], &[[0, 1, 2]]);
    for dataset in [&canonical, &surface] {
        let mut count = 0;
        try_for_each_surface_chunk_with_limits(
            dataset.validated().unwrap(),
            1,
            SurfaceExtractionLimits { max_face_bytes: 0 },
            |_, owners| {
                count += owners.len();
                Ok::<(), MeshError>(())
            },
        )
        .unwrap();
        assert!(count > 0);
        let stop = MeshError::Allocation { bytes: 123 };
        let mut calls = 0;
        let result = try_for_each_surface_chunk_with_limits(
            dataset.validated().unwrap(),
            1,
            SurfaceExtractionLimits { max_face_bytes: 0 },
            |_, _| {
                calls += 1;
                Err(stop.clone())
            },
        );
        assert_eq!(result, Err(stop));
        assert_eq!(calls, 1);
    }
}

#[test]
fn nonmanifold_face_with_three_owners_is_culled_rather_than_toggled() {
    let dataset = DataSet::new(
        "triple",
        vec![[0.0; 3]; 6],
        Cells::from_cells(&[
            (CellType::Tetra, &[0, 1, 2, 3]),
            (CellType::Tetra, &[0, 2, 1, 4]),
            (CellType::Tetra, &[0, 1, 2, 5]),
        ]),
    );
    let mut count = 0;
    try_for_each_surface_chunk_with_limits(
        dataset.validated().unwrap(),
        2,
        SurfaceExtractionLimits::default(),
        |chunk, owners| {
            count += owners.len();
            for triangle in chunk.indices.as_chunks::<3>().0 {
                let mut nodes = triangle.map(|i| chunk.point_ids[i as usize]);
                nodes.sort_unstable();
                assert_ne!(nodes, [0, 1, 2]);
            }
            Ok::<(), MeshError>(())
        },
    )
    .unwrap();
    assert_eq!(count, 9);
}
