//! 2D hit testing for unstructured mesh plots.
//!
//! Picking deliberately uses the same projected coordinates and barycentric
//! tolerance as the mesh algorithms in `gpui-d3rs`. The grid is only a
//! candidate accelerator; every candidate is checked with the exact triangle
//! test before it can become a pick.

use super::MeshPlotPick;
use d3rs::mesh::{
    CoordinateAxis, ScalarAssociation, ScalarField, TriGridIndex, TriangleMesh, barycentric_2d,
    project_2d,
};
use std::sync::Arc;

/// Pick a point in a planar or axisymmetric mesh view.
///
/// Candidate triangles come from `index` and are checked in sorted order for
/// deterministic shared-edge behaviour. A masked vertex makes every cell
/// using that vertex unpickable; a masked cell makes only that cell
/// unpickable. The returned world position is interpolated in the original
/// three-dimensional mesh coordinates.
pub fn pick_2d(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    index: &TriGridIndex,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
    point_2d: [f64; 2],
    plot_id: &str,
) -> Option<MeshPlotPick> {
    pick_2d_with_shared_plot_id(
        mesh,
        field,
        index,
        horizontal,
        vertical,
        point_2d,
        &Arc::from(plot_id),
    )
}

/// Pick using an already-interned plot id. Live pointer handlers use this
/// variant so a successful hover/click does not allocate a new `Arc<str>`.
pub(crate) fn pick_2d_with_shared_plot_id(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    index: &TriGridIndex,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
    point_2d: [f64; 2],
    plot_id: &Arc<str>,
) -> Option<MeshPlotPick> {
    if !point_2d.iter().all(|component| component.is_finite()) {
        return None;
    }

    for triangle_index in index.query(point_2d) {
        let cell_index = triangle_index as usize;
        let Some(&triangle) = mesh.triangles.get(cell_index) else {
            continue;
        };
        let Some(&a) = mesh.positions.get(triangle[0] as usize) else {
            continue;
        };
        let Some(&b) = mesh.positions.get(triangle[1] as usize) else {
            continue;
        };
        let Some(&c) = mesh.positions.get(triangle[2] as usize) else {
            continue;
        };

        let projected = [
            project_2d(horizontal, vertical, a),
            project_2d(horizontal, vertical, b),
            project_2d(horizontal, vertical, c),
        ];
        let Some(weights) = barycentric_2d(point_2d, projected[0], projected[1], projected[2])
        else {
            continue;
        };

        let displayed_value = match field {
            None => None,
            Some(field) => {
                let value = match field.association {
                    ScalarAssociation::Vertex => {
                        if !triangle
                            .iter()
                            .all(|&vertex| field_value_is_pickable(field, vertex as usize))
                        {
                            continue;
                        }
                        weights[0] * field.values[triangle[0] as usize]
                            + weights[1] * field.values[triangle[1] as usize]
                            + weights[2] * field.values[triangle[2] as usize]
                    }
                    ScalarAssociation::Cell => {
                        if !field_value_is_pickable(field, cell_index) {
                            continue;
                        }
                        field.values[cell_index]
                    }
                };
                if !value.is_finite() {
                    continue;
                }
                Some(value)
            }
        };

        let world_position = interpolate_position([a, b, c], weights);
        let nearest_vertex_index = nearest_vertex(point_2d, projected, triangle);
        let vertex_id = nearest_vertex_index.and_then(|vertex| {
            mesh.vertex_ids
                .as_ref()
                .and_then(|ids| ids.get(vertex as usize).copied())
        });
        let cell_id = mesh
            .cell_ids
            .as_ref()
            .and_then(|ids| ids.get(cell_index).copied());

        return Some(MeshPlotPick {
            plot_id: Arc::clone(plot_id),
            mesh_id: mesh.id.clone(),
            cell_index: triangle_index,
            cell_id,
            nearest_vertex_index,
            vertex_id,
            world_position,
            displayed_value,
            field_id: field.map(|field| field.id.clone()),
        });
    }

    None
}

fn field_value_is_pickable(field: &ScalarField, index: usize) -> bool {
    let Some(&value) = field.values.get(index) else {
        return false;
    };
    if field
        .valid
        .as_ref()
        .is_some_and(|valid| valid.get(index) != Some(&true))
    {
        return false;
    }
    value.is_finite()
}

fn interpolate_position(vertices: [[f64; 3]; 3], weights: [f64; 3]) -> [f64; 3] {
    let mut position = [0.0; 3];
    for (weight, vertex) in weights.into_iter().zip(vertices) {
        for (component, coordinate) in position.iter_mut().zip(vertex) {
            *component += weight * coordinate;
        }
    }
    position
}

fn nearest_vertex(point: [f64; 2], projected: [[f64; 2]; 3], triangle: [u32; 3]) -> Option<u32> {
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    for (slot, vertex) in projected.into_iter().enumerate() {
        let dx = vertex[0] - point[0];
        let dy = vertex[1] - point[1];
        let distance = dx * dx + dy * dy;
        if distance.is_finite() && distance < nearest_distance {
            nearest_distance = distance;
            nearest = Some(triangle[slot]);
        }
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_mesh() -> TriangleMesh {
        TriangleMesh {
            id: "square".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    fn vertex_field(id: &str, values: &[f64]) -> ScalarField {
        ScalarField {
            id: id.into(),
            label: "Pressure".into(),
            unit: Some("Pa".into()),
            values: values.to_vec().into(),
            association: ScalarAssociation::Vertex,
            valid: None,
        }
    }

    fn cell_field(values: &[f64]) -> ScalarField {
        ScalarField {
            id: "cell".into(),
            label: "Cell value".into(),
            unit: None,
            values: values.to_vec().into(),
            association: ScalarAssociation::Cell,
            valid: None,
        }
    }

    fn index(mesh: &TriangleMesh) -> TriGridIndex {
        let positions = mesh
            .positions
            .iter()
            .map(|&position| project_2d(CoordinateAxis::X, CoordinateAxis::Y, position))
            .collect::<Vec<_>>();
        TriGridIndex::build(&positions, &mesh.triangles)
    }

    #[test]
    fn pick_finds_cell_and_interpolated_value() {
        let mesh = square_mesh();
        let field = vertex_field("p", &[0.0, 1.0, 1.0, 0.0]);
        let pick = pick_2d(
            &mesh,
            Some(&field),
            &index(&mesh),
            CoordinateAxis::X,
            CoordinateAxis::Y,
            [0.4, 0.3],
            "plot",
        )
        .unwrap();
        assert_eq!(pick.cell_index, 0);
        assert!((pick.displayed_value.unwrap() - 0.4).abs() < 1e-12);
        assert_eq!(pick.field_id.as_deref(), Some("p"));
        assert!((pick.world_position[0] - 0.4).abs() < 1e-12);
        assert!((pick.world_position[1] - 0.3).abs() < 1e-12);
        assert_eq!(pick.world_position[2], 0.0);
    }

    #[test]
    fn pick_reports_external_ids() {
        let mut mesh = square_mesh();
        mesh.cell_ids = Some(vec![42, 43].into());
        mesh.vertex_ids = Some(vec![7, 8, 9, 10].into());
        let pick = pick_2d(
            &mesh,
            None,
            &index(&mesh),
            CoordinateAxis::X,
            CoordinateAxis::Y,
            [0.2, 0.2],
            "plot",
        )
        .unwrap();
        assert_eq!(pick.cell_id, Some(42));
        assert_eq!(pick.vertex_id, Some(7));
    }

    #[test]
    fn pick_outside_mesh_returns_none() {
        let mesh = square_mesh();
        assert!(
            pick_2d(
                &mesh,
                None,
                &index(&mesh),
                CoordinateAxis::X,
                CoordinateAxis::Y,
                [5.0, 5.0],
                "plot",
            )
            .is_none()
        );
    }

    #[test]
    fn masked_triangle_not_pickable() {
        let mesh = square_mesh();
        let mut field = vertex_field("p", &[0.0, 1.0, 1.0, 0.0]);
        field.valid = Some(vec![false, true, true, true].into());
        assert!(
            pick_2d(
                &mesh,
                Some(&field),
                &index(&mesh),
                CoordinateAxis::X,
                CoordinateAxis::Y,
                [0.2, 0.2],
                "plot",
            )
            .is_none()
        );
    }

    #[test]
    fn cell_field_pick_reports_cell_value() {
        let mesh = square_mesh();
        let field = cell_field(&[3.5, 9.5]);
        let pick = pick_2d(
            &mesh,
            Some(&field),
            &index(&mesh),
            CoordinateAxis::X,
            CoordinateAxis::Y,
            [0.2, 0.2],
            "plot",
        )
        .unwrap();
        assert_eq!(pick.displayed_value, Some(3.5));
    }
}
