//! Scene preparation propagates face-workspace errors for both associations.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field, MeshError, SurfaceExtractionLimits};
use gpui_scene3d::{Actor, SceneError, TransferFunction, prepare_chunks_with_limits};

#[test]
fn point_and_cell_preparation_respect_explicit_face_allowances() {
    let data = DataSet::new(
        "tet",
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
    .with_field(Field::scalar("point", vec![0.0, 0.25, 0.5, 1.0]))
    .with_field(Field::scalar("cell", vec![0.5]).on_cells());
    for actor in [
        Actor::new(data.clone()),
        Actor::new(data.clone()).coloured_by("point", TransferFunction::viridis([0.0, 1.0])),
        Actor::new(data).coloured_by_cell("cell", TransferFunction::viridis([0.0, 1.0])),
    ] {
        assert!(matches!(
            prepare_chunks_with_limits(&actor, 2, SurfaceExtractionLimits { max_face_bytes: 0 }),
            Err(SceneError::Mesh(MeshError::ResourceLimit {
                resource: "surface face workspace bytes",
                limit: 0,
                ..
            }))
        ));
        let chunks =
            prepare_chunks_with_limits(&actor, 2, SurfaceExtractionLimits::default()).unwrap();
        assert_eq!(
            chunks
                .iter()
                .map(gpui_scene3d::GpuChunk::triangle_count)
                .sum::<usize>(),
            4
        );
        assert!(chunks.iter().all(|c| c.triangle_count() <= 2));
    }
}
