//! Feature-independent geometry used by headless acoustic report backends.

use d3rs::surface::{OrthographicProjection, Projection, SurfaceData, SurfaceMesh, SurfacePoint3D};

#[test]
fn surface_geometry_is_available_to_a_headless_consumer() {
    let data = SurfaceData::from_grid(vec![
        vec![
            SurfacePoint3D::new(0.0, 0.0, 0.0, 0.0),
            SurfacePoint3D::new(1.0, 0.0, 0.0, 0.0),
        ],
        vec![
            SurfacePoint3D::new(0.0, 1.0, 0.0, 0.0),
            SurfacePoint3D::new(1.0, 1.0, 0.0, 0.0),
        ],
    ]);
    let mesh = SurfaceMesh::from_surface_data(&data);
    assert_eq!(mesh.triangles.len(), 2);
    let projection = OrthographicProjection::default();
    for triangle in &mesh.triangles {
        for vertex in &triangle.vertices {
            let projected = projection.project_point(vertex);
            assert!(projected.x.is_finite() && projected.y.is_finite());
        }
    }
}
