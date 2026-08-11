//! Camera-ray picking for unstructured surface meshes.

use super::MeshPlotPick;
use d3rs::gpu3d::Camera3D;
use d3rs::mesh::{MeshBvh, RevolvedMesh, ScalarAssociation, ScalarField, TriangleMesh};
use std::sync::Arc;

pub fn pick_3d<P: AsRef<str>>(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    camera: &Camera3D,
    screen: [f32; 2],
    viewport: [f32; 2],
    plot_id: P,
) -> Option<MeshPlotPick> {
    // Picking is a library boundary: reject malformed inputs before the BVH
    // builder or field interpolation can index into caller-owned arrays.
    mesh.validate().ok()?;
    if let Some(field) = field {
        field.validate(mesh).ok()?;
    }
    let bvh = MeshBvh::build(mesh);
    pick_3d_with_bvh(
        mesh,
        field,
        &bvh,
        camera,
        screen,
        viewport,
        Arc::from(plot_id.as_ref()),
    )
}

/// Pick using a caller-retained BVH. Live plots use this to avoid rebuilding
/// the geometry accelerator for every click.
pub fn pick_3d_with_bvh<P: Into<Arc<str>>>(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    bvh: &MeshBvh,
    camera: &Camera3D,
    screen: [f32; 2],
    viewport: [f32; 2],
    plot_id: P,
) -> Option<MeshPlotPick> {
    if viewport[0] <= 0.0 || viewport[1] <= 0.0 {
        return None;
    }
    let ndc = glam::Vec3::new(
        2.0 * screen[0] / viewport[0] - 1.0,
        1.0 - 2.0 * screen[1] / viewport[1],
        0.0,
    );
    let ndc_far = glam::Vec3::new(ndc.x, ndc.y, 1.0);
    let inverse = camera.view_projection_matrix().inverse();
    let near = inverse.project_point3(ndc);
    let far = inverse.project_point3(ndc_far);
    let direction = (far - near).normalize_or_zero();
    if direction.length_squared() <= f32::EPSILON {
        return None;
    }
    let hit = bvh.ray_cast(
        near.to_array().map(|v| v as f64),
        direction.to_array().map(|v| v as f64),
    )?;
    let (cell_index, _distance, barycentric) = hit;
    let triangle = *mesh.triangles.get(cell_index as usize)?;
    let vertices = [
        *mesh.positions.get(triangle[0] as usize)?,
        *mesh.positions.get(triangle[1] as usize)?,
        *mesh.positions.get(triangle[2] as usize)?,
    ];
    let position = [
        vertices[0][0] * barycentric[0]
            + vertices[1][0] * barycentric[1]
            + vertices[2][0] * barycentric[2],
        vertices[0][1] * barycentric[0]
            + vertices[1][1] * barycentric[1]
            + vertices[2][1] * barycentric[2],
        vertices[0][2] * barycentric[0]
            + vertices[1][2] * barycentric[1]
            + vertices[2][2] * barycentric[2],
    ];
    let nearest = barycentric
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(slot, _)| triangle[slot]);
    let value = field.and_then(|field| match field.association {
        ScalarAssociation::Vertex => {
            if triangle.iter().any(|&vertex| {
                field
                    .valid
                    .as_ref()
                    .is_some_and(|valid| valid.get(vertex as usize) != Some(&true))
            }) {
                return None;
            }
            let values = [
                *field.values.get(triangle[0] as usize)?,
                *field.values.get(triangle[1] as usize)?,
                *field.values.get(triangle[2] as usize)?,
            ];
            let value = values[0] * barycentric[0]
                + values[1] * barycentric[1]
                + values[2] * barycentric[2];
            value.is_finite().then_some(value)
        }
        ScalarAssociation::Cell => {
            if field
                .valid
                .as_ref()
                .is_some_and(|valid| valid.get(cell_index as usize) != Some(&true))
            {
                return None;
            }
            field
                .values
                .get(cell_index as usize)
                .copied()
                .filter(|value| value.is_finite())
        }
    });
    Some(MeshPlotPick {
        plot_id: plot_id.into(),
        mesh_id: mesh.id.clone(),
        cell_index,
        cell_id: mesh
            .cell_ids
            .as_ref()
            .and_then(|ids| ids.get(cell_index as usize).copied()),
        nearest_vertex_index: nearest,
        vertex_id: nearest.and_then(|i| {
            mesh.vertex_ids
                .as_ref()
                .and_then(|ids| ids.get(i as usize).copied())
        }),
        world_position: position,
        displayed_value: value,
        field_id: field.map(|f| f.id.clone()),
    })
}

/// Pick a revolved surface while reporting source-profile cell/vertex IDs.
/// The derived field is created only for the ray query; geometry and source
/// metadata remain owned by the retained [`RevolvedMesh`].
pub fn pick_revolved_3d<P: AsRef<str>>(
    source_mesh: &TriangleMesh,
    revolved: &RevolvedMesh,
    field: Option<&ScalarField>,
    camera: &Camera3D,
    screen: [f32; 2],
    viewport: [f32; 2],
    plot_id: P,
) -> Option<MeshPlotPick> {
    source_mesh.validate().ok()?;
    if let Some(field) = field {
        field.validate(source_mesh).ok()?;
    }
    let derived_field = field.map(|field| revolved_field(field, revolved));
    let bvh = MeshBvh::build(&revolved.mesh);
    pick_revolved_3d_with_bvh(
        source_mesh,
        revolved,
        derived_field.as_ref(),
        &bvh,
        field.map(|field| field.id.clone()),
        camera,
        screen,
        viewport,
        Arc::from(plot_id.as_ref()),
    )
}

/// Replicate a profile field onto its revolved mesh while preserving explicit
/// missing-data entries. Kept crate-visible so live interaction can use a
/// retained geometry/BVH without changing scalar semantics.
pub(crate) fn revolved_field(field: &ScalarField, revolved: &RevolvedMesh) -> ScalarField {
    let valid = field.valid.as_ref().map(|valid| match field.association {
        ScalarAssociation::Vertex => revolved
            .source_vertex
            .iter()
            .map(|&source| valid.get(source as usize).copied().unwrap_or(false))
            .collect::<Vec<_>>()
            .into(),
        ScalarAssociation::Cell => revolved
            .source_triangle
            .iter()
            .map(|&source| valid.get(source as usize).copied().unwrap_or(false))
            .collect::<Vec<_>>()
            .into(),
    });
    ScalarField {
        id: field.id.clone(),
        label: field.label.clone(),
        unit: field.unit.clone(),
        values: d3rs::mesh::revolve_field(field, revolved).into(),
        association: field.association,
        valid,
    }
}

/// Pick a revolved surface through a retained derived field and BVH. This is
/// the live-chart counterpart to [`pick_revolved_3d`]; callers own cache
/// invalidation through their source geometry and field revisions.
pub fn pick_revolved_3d_with_bvh<P: Into<Arc<str>>>(
    source_mesh: &TriangleMesh,
    revolved: &RevolvedMesh,
    derived_field: Option<&ScalarField>,
    bvh: &MeshBvh,
    field_id: Option<Arc<str>>,
    camera: &Camera3D,
    screen: [f32; 2],
    viewport: [f32; 2],
    plot_id: P,
) -> Option<MeshPlotPick> {
    let mut pick = pick_3d_with_bvh(
        &revolved.mesh,
        derived_field,
        bvh,
        camera,
        screen,
        viewport,
        plot_id,
    )?;
    let derived_cell = pick.cell_index as usize;
    pick.mesh_id = source_mesh.id.clone();
    pick.cell_index = revolved.source_triangle.get(derived_cell).copied()?;
    pick.cell_id = source_mesh
        .cell_ids
        .as_ref()
        .and_then(|ids| ids.get(pick.cell_index as usize).copied());
    if let Some(derived_vertex) = pick.nearest_vertex_index {
        let source_vertex = revolved
            .source_vertex
            .get(derived_vertex as usize)
            .copied()?;
        pick.nearest_vertex_index = Some(source_vertex);
        pick.vertex_id = source_mesh
            .vertex_ids
            .as_ref()
            .and_then(|ids| ids.get(source_vertex as usize).copied());
    }
    pick.field_id = field_id;
    Some(pick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use d3rs::mesh::ScalarAssociation;

    fn mesh() -> TriangleMesh {
        TriangleMesh {
            id: "surface".into(),
            positions: Arc::from([
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn pick_returns_interpolated_vertex_value() {
        let mesh = mesh();
        let field = ScalarField {
            id: "height".into(),
            label: "Height".into(),
            unit: None,
            values: Arc::from([0.0, 1.0, 2.0, 1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let camera = Camera3D::default().with_position(glam::Vec3::new(0.0, 0.0, 3.0));
        let pick = pick_3d(
            &mesh,
            Some(&field),
            &camera,
            [50.0, 50.0],
            [100.0, 100.0],
            "plot",
        )
        .unwrap();
        assert_eq!(pick.cell_index, 0);
        assert!((pick.displayed_value.unwrap() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn malformed_field_is_rejected_without_panicking() {
        let mesh = mesh();
        let field = ScalarField {
            id: "bad".into(),
            label: "Bad".into(),
            unit: None,
            values: Arc::from([1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let camera = Camera3D::default();
        assert!(
            pick_3d(
                &mesh,
                Some(&field),
                &camera,
                [50.0, 50.0],
                [100.0, 100.0],
                "plot"
            )
            .is_none()
        );
    }
}
