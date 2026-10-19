//! Local-space half-space and homogeneous frustum clipping.

// Rust guideline compliant 2026-02-21

use crate::scene::PlaneEquation;
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 4],
    pub(crate) scalar: f32,
    pub(crate) normal: [f32; 3],
}

pub(crate) type Polygon = SmallVec<[Vertex; 16]>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundsVisibility {
    Inside,
    Outside,
    Intersecting,
}

/// Classifies convex bounds conservatively, with a rounding margin.
pub(crate) fn classify_bounds(
    bounds: gpui_mesh::Bounds,
    origin: [f64; 3],
    matrix: &[f32; 16],
) -> BoundsVisibility {
    let mut inside = true;
    let mut common = 0x3f;
    for corner in 0..8 {
        let point = std::array::from_fn(|axis| {
            ((if corner & (1 << axis) == 0 {
                bounds.min[axis]
            } else {
                bounds.max[axis]
            }) - origin[axis]) as f32
        });
        let clip = crate::camera::transform(matrix, point);
        if clip.iter().any(|v| !v.is_finite()) {
            return BoundsVisibility::Intersecting;
        }
        let tolerance = f64::from(clip[3].abs().max(1.0)) * 0.00001;
        let mut outside = 0;
        for plane in 0..6 {
            let distance = frustum_distance(clip.map(f64::from), plane);
            if distance <= tolerance {
                inside = false;
            }
            if distance < -tolerance {
                outside |= 1 << plane;
            }
        }
        common &= outside;
    }
    if common != 0 {
        BoundsVisibility::Outside
    } else if inside {
        BoundsVisibility::Inside
    } else {
        BoundsVisibility::Intersecting
    }
}

fn interpolate(a: Vertex, b: Vertex, t: f64) -> Vertex {
    Vertex {
        position: std::array::from_fn(|axis| {
            (f64::from(a.position[axis])
                + (f64::from(b.position[axis]) - f64::from(a.position[axis])) * t)
                as f32
        }),
        scalar: (f64::from(a.scalar) + (f64::from(b.scalar) - f64::from(a.scalar)) * t) as f32,
        normal: std::array::from_fn(|axis| {
            (f64::from(a.normal[axis])
                + (f64::from(b.normal[axis]) - f64::from(a.normal[axis])) * t) as f32
        }),
    }
}

#[inline]
fn polygon_plane(poly: &Polygon, distance: impl Fn([f32; 4]) -> f64) -> Polygon {
    let mut out = Polygon::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (da, db) = (distance(a.position), distance(b.position));
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(interpolate(a, b, da / (da - db)));
        }
    }
    out
}

#[inline]
pub(crate) fn clip_planes(poly: &mut Polygon, planes: &[PlaneEquation]) {
    for plane in planes {
        if poly.is_empty() {
            break;
        }
        *poly = polygon_plane(poly, |point| {
            plane.distance([
                f64::from(point[0]),
                f64::from(point[1]),
                f64::from(point[2]),
            ])
        });
    }
}

fn frustum_distance(point: [f64; 4], plane: usize) -> f64 {
    match plane {
        0 => point[0] + point[3],
        1 => point[3] - point[0],
        2 => point[1] + point[3],
        3 => point[3] - point[1],
        4 => point[2],
        _ => point[3] - point[2],
    }
}

fn outcode(point: [f32; 4]) -> u8 {
    u8::from(point[0] < -point[3])
        | (u8::from(point[0] > point[3]) << 1)
        | (u8::from(point[1] < -point[3]) << 2)
        | (u8::from(point[1] > point[3]) << 3)
        | (u8::from(point[2] < 0.0) << 4)
        | (u8::from(point[2] > point[3]) << 5)
}

#[inline]
pub(crate) fn triangle_visibility(vertices: &[Vertex; 3]) -> BoundsVisibility {
    let mut outside = 0;
    let mut common = 0x3f;
    for vertex in vertices {
        if vertex.position.iter().any(|v| !v.is_finite()) {
            return BoundsVisibility::Outside;
        }
        let code = outcode(vertex.position);
        outside |= code;
        common &= code;
    }
    if common != 0 {
        BoundsVisibility::Outside
    } else if outside == 0 {
        BoundsVisibility::Inside
    } else {
        BoundsVisibility::Intersecting
    }
}

#[inline]
pub(crate) fn clip_frustum(poly: &mut Polygon) {
    let mut outside = 0;
    let mut common = 0x3f;
    for vertex in poly.iter() {
        if vertex.position.iter().any(|v| !v.is_finite()) {
            poly.clear();
            return;
        }
        let code = outcode(vertex.position);
        outside |= code;
        common &= code;
    }
    if common != 0 {
        poly.clear();
        return;
    }
    for plane in 0..6 {
        if outside & (1 << plane) != 0 {
            *poly = polygon_plane(poly, |point| frustum_distance(point.map(f64::from), plane));
        }
    }
}

fn segment_interval<const N: usize>(
    points: [[f64; N]; 2],
    count: usize,
    distance: impl Fn([f64; N], usize) -> f64,
) -> Option<[f64; 2]> {
    let [a, b] = points;
    let (mut enter, mut exit) = (0.0f64, 1.0f64);
    for plane in 0..count {
        let (da, db) = (distance(a, plane), distance(b, plane));
        if !da.is_finite() || !db.is_finite() || (da < 0.0 && db < 0.0) {
            return None;
        }
        if (da < 0.0) != (db < 0.0) {
            let t = da / (da - db);
            if da < 0.0 {
                enter = enter.max(t);
            } else {
                exit = exit.min(t);
            }
        }
    }
    if enter > exit {
        return None;
    }
    Some([enter, exit])
}

fn segment<const N: usize>(
    points: [[f64; N]; 2],
    count: usize,
    distance: impl Fn([f64; N], usize) -> f64,
) -> Option<[[f64; N]; 2]> {
    let [a, b] = points;
    Some(
        segment_interval(points, count, distance)?
            .map(|t| std::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * t)),
    )
}

pub(crate) fn clip_line_vertices(
    vertices: [Vertex; 2],
    planes: &[PlaneEquation],
) -> Option<[Vertex; 2]> {
    let points = vertices.map(|v| {
        [
            f64::from(v.position[0]),
            f64::from(v.position[1]),
            f64::from(v.position[2]),
        ]
    });
    Some(
        segment_interval(points, planes.len(), |p, i| planes[i].distance(p))?
            .map(|t| interpolate(vertices[0], vertices[1], t)),
    )
}

pub(crate) fn clip_frustum_line_vertices(vertices: [Vertex; 2]) -> Option<[Vertex; 2]> {
    let points = vertices.map(|v| v.position.map(f64::from));
    let mut clipped = segment_interval(points, 6, frustum_distance)?
        .map(|t| interpolate(vertices[0], vertices[1], t));
    for v in &mut clipped {
        let w = v.position[3];
        if w <= 0.0 || v.position.iter().any(|v| !v.is_finite()) {
            return None;
        }
        v.position[0] = v.position[0].clamp(-w, w);
        v.position[1] = v.position[1].clamp(-w, w);
        v.position[2] = v.position[2].clamp(0.0, w);
    }
    Some(clipped)
}

pub(crate) fn clip_line(points: [[f64; 3]; 2], planes: &[PlaneEquation]) -> Option<[[f64; 3]; 2]> {
    segment(points, planes.len(), |point, plane| {
        planes[plane].distance(point)
    })
}

pub(crate) fn clip_frustum_line(points: [[f32; 4]; 2]) -> Option<[[f32; 4]; 2]> {
    segment(points.map(|p| p.map(f64::from)), 6, frustum_distance).and_then(|points| {
        if points
            .iter()
            .any(|point| point[3] <= 0.0 || point.iter().any(|v| !v.is_finite()))
        {
            return None;
        }
        // Interpolation of very long segments can leave a boundary one
        // float step outside. Clamp that residual before perspective divide.
        Some(points.map(|point| {
            [
                point[0].clamp(-point[3], point[3]) as f32,
                point[1].clamp(-point[3], point[3]) as f32,
                point[2].clamp(0.0, point[3]) as f32,
                point[3] as f32,
            ]
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClipPlane;

    #[test]
    fn clipped_line_vertices_interpolate_scalar_attributes() {
        let vertices = [
            Vertex {
                position: [-1.0, 0.0, 0.0, 1.0],
                scalar: 0.0,
                normal: [0.0; 3],
            },
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                scalar: 1.0,
                normal: [0.0; 3],
            },
        ];
        let planes = crate::scene::local_planes(
            &[ClipPlane {
                origin: [0.0; 3],
                normal: [1.0, 0.0, 0.0],
            }],
            [0.0; 3],
        )
        .unwrap();
        let clipped = clip_line_vertices(vertices, &planes).unwrap();
        assert_eq!(clipped[0].position, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(clipped[0].scalar, 0.5);
        let mut vertices = vertices;
        vertices[0].position = [-0.5, 0.0, -0.5, 1.0];
        vertices[1].position = [0.5, 0.0, 0.5, 1.0];
        let clipped = clip_frustum_line_vertices(vertices).unwrap();
        assert_eq!(clipped[0].position, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(clipped[0].scalar, 0.5);
    }

    #[test]
    fn bounds_classification_is_conservative_at_frustum_boundaries() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let bounds = gpui_mesh::Bounds {
            min: [-0.5, -0.5, 0.25],
            max: [0.5, 0.5, 0.75],
        };
        assert_eq!(
            classify_bounds(bounds, [0.0; 3], &identity),
            BoundsVisibility::Inside
        );
        assert_eq!(
            classify_bounds(
                gpui_mesh::Bounds {
                    min: [2.0, -0.5, 0.25],
                    max: [3.0, 0.5, 0.75]
                },
                [0.0; 3],
                &identity
            ),
            BoundsVisibility::Outside
        );
        for max_x in [1.0, 1.0 - 1e-7, 2.0] {
            assert_eq!(
                classify_bounds(
                    gpui_mesh::Bounds {
                        max: [max_x, 0.5, 0.75],
                        ..bounds
                    },
                    [0.0; 3],
                    &identity
                ),
                BoundsVisibility::Intersecting
            );
        }
    }

    #[test]
    fn near_plane_clipping_interpolates_attributes_instead_of_dropping_triangles() {
        let mut poly = [
            [-0.5, -0.5, -0.5, 1.0],
            [0.5, -0.5, 0.5, 1.0],
            [0.0, 0.5, 0.5, 1.0],
        ]
        .into_iter()
        .enumerate()
        .map(|(i, position)| Vertex {
            position,
            scalar: if i == 0 { 0.0 } else { 1.0 },
            normal: if i == 0 {
                [1.0, 0.0, 0.0]
            } else {
                [0.0, 1.0, 0.0]
            },
        })
        .collect();
        clip_frustum(&mut poly);
        let clipped = poly;
        assert_eq!(clipped.len(), 4);
        assert_eq!(
            clipped.iter().filter(|vertex| vertex.scalar == 0.5).count(),
            2
        );
        assert!(clipped.iter().all(|vertex| outcode(vertex.position) == 0));
        assert_eq!(
            clipped
                .iter()
                .filter(|vertex| vertex.normal == [0.5, 0.5, 0.0])
                .count(),
            2
        );
        let mut far: Polygon = clipped
            .iter()
            .map(|vertex| Vertex {
                position: [vertex.position[0], vertex.position[1], 2.0, 1.0],
                ..*vertex
            })
            .collect();
        clip_frustum(&mut far);
        assert!(far.is_empty());
    }

    #[test]
    fn lines_crossing_user_planes_are_clipped_in_both_directions() {
        let planes = [
            ClipPlane {
                origin: [0.0; 3],
                normal: [1.0, 0.0, 0.0],
            },
            ClipPlane {
                origin: [0.5, 0.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
            },
        ];
        let planes = crate::scene::local_planes(&planes, [0.0; 3]).unwrap();
        let points = [[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let expected = [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0]];
        assert_eq!(clip_line(points, &planes).unwrap(), expected);
        assert_eq!(
            clip_line([points[1], points[0]], &planes).unwrap(),
            [expected[1], expected[0]]
        );
        assert!(clip_line([[-2.0; 3], [-1.0; 3]], &planes).is_none());
    }

    #[test]
    fn long_lines_clip_before_screen_iteration_and_reject_nonfinite_input() {
        let clipped = clip_frustum_line([[-1e9, 0.0, 0.5, 1.0], [1e9, 0.0, 0.5, 1.0]]).unwrap();
        assert_eq!(clipped, [[-1.0, 0.0, 0.5, 1.0], [1.0, 0.0, 0.5, 1.0]]);
        assert!(clip_frustum_line([[f32::NAN; 4], [0.0, 0.0, 0.5, 1.0]]).is_none());
        assert!(clip_frustum_line([[0.0, 0.0, 2.0, 1.0], [0.5, 0.5, 3.0, 1.0]]).is_none());
    }
}
