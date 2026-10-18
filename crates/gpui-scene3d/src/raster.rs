//! Software rasterizer: scenes to RGBA frames without any GPU.

// Rust guideline compliant 2026-02-21

use crate::camera::{Camera, transform};
use crate::clipping::{
    BoundsVisibility, Polygon, Vertex, classify_bounds, clip_frustum, clip_frustum_line, clip_line,
    clip_planes, triangle_visibility,
};
use crate::limits::frame_buffer;
use crate::retained::PreparedGeometry;
use crate::scene::Scene;
use crate::{RenderLimits, SceneError};
use smallvec::SmallVec;

/// One rendered frame: tightly packed RGBA8 pixels, row-major.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA bytes, `width * height * 4` long.
    pub rgba: Vec<u8>,
}

impl Frame {
    /// Encodes the frame as a binary PPM image (no dependencies).
    #[must_use]
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for px in self.rgba.as_chunks::<4>().0 {
            out.extend_from_slice(&px[..3]);
        }
        out
    }

    /// Encodes the frame as PNG bytes.
    ///
    /// # Errors
    ///
    /// Returns the image encoder's message on failure.
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        let mut bytes: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
        image::ImageEncoder::write_image(
            encoder,
            &self.rgba,
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}

/// Renders a scene from a camera into an RGBA frame.
///
/// Opaque actors are z-buffered. Transparent actors (opacity or colour
/// alpha below 1) render afterwards, sorted back-to-front by distance and
/// by triangle depth, with source-over blending. Actor clipping planes
/// cut triangles in world space before projection, matching the GPU
/// shader discard semantics. The output is deterministic for a given
/// scene and camera, which makes it suitable for visual-regression
/// baselines and movie frames.
///
/// # Errors
///
/// Returns an error for invalid dimensions, exceeded default memory limits,
/// allocation failure, or an actor with a missing field.
pub fn render(
    scene: &Scene,
    camera: &Camera,
    width: u32,
    height: u32,
) -> Result<Frame, SceneError> {
    render_with_limits(scene, camera, width, height, RenderLimits::default())
}

/// Renders a scene within explicit dimension and frame-memory limits.
///
/// # Errors
///
/// Returns an error for invalid dimensions, exhausted memory, exceeded limits,
/// or an actor with a missing field.
pub fn render_with_limits(
    scene: &Scene,
    camera: &Camera,
    width: u32,
    height: u32,
    limits: RenderLimits,
) -> Result<Frame, SceneError> {
    camera.validate()?;
    scene.validate_clipping()?;
    let pixels = limits.pixel_count(width, height, 8)?;
    let prepared = scene.prepare()?;
    let bg = scene.background;
    let mut rgba = frame_buffer(pixels * 4, 0u8)?;
    for px in rgba.as_chunks_mut::<4>().0 {
        px[0] = (bg[0].clamp(0.0, 1.0) * 255.0) as u8;
        px[1] = (bg[1].clamp(0.0, 1.0) * 255.0) as u8;
        px[2] = (bg[2].clamp(0.0, 1.0) * 255.0) as u8;
        px[3] = (bg[3].clamp(0.0, 1.0) * 255.0) as u8;
    }
    let mut depth = frame_buffer(pixels, f32::INFINITY)?;
    let light = normalize3([-0.4, 0.7, 0.6]);
    let eye = camera.eye();

    // Opaque first, in scene order.
    for (index, actor) in scene
        .actors
        .iter()
        .enumerate()
        .filter(|(_, actor)| actor.visible && !actor.is_transparent())
    {
        draw_actor(
            actor,
            prepared.actors[index]
                .as_ref()
                .expect("visible actor prepared"),
            camera,
            width,
            height,
            light,
            false,
            &mut depth,
            &mut rgba,
        )?;
    }
    // Transparent actors back-to-front by bounds-centre distance.
    let mut transparent: Vec<(usize, &crate::scene::Actor)> = scene
        .actors
        .iter()
        .enumerate()
        .filter(|(_, actor)| actor.visible && actor.is_transparent())
        .collect();
    transparent.sort_by(|(a, _), (b, _)| {
        let da = actor_distance(
            prepared.actors[*a].as_ref().and_then(|actor| actor.bounds),
            eye,
        );
        let db = actor_distance(
            prepared.actors[*b].as_ref().and_then(|actor| actor.bounds),
            eye,
        );
        db.total_cmp(&da)
    });
    for (index, actor) in transparent {
        draw_actor(
            actor,
            prepared.actors[index]
                .as_ref()
                .expect("visible actor prepared"),
            camera,
            width,
            height,
            light,
            true,
            &mut depth,
            &mut rgba,
        )?;
    }
    Ok(Frame {
        width,
        height,
        rgba,
    })
}

/// Projected triangle: three screen points, shaded colour, mean depth.
type ScreenTriangle = (
    (f32, f32, f32),
    (f32, f32, f32),
    (f32, f32, f32),
    [u8; 4],
    f32,
);

fn actor_distance(bounds: Option<gpui_mesh::Bounds>, eye: [f64; 3]) -> f64 {
    let Some(bounds) = bounds else {
        return 0.0;
    };
    let centre = bounds.center();
    let d = [centre[0] - eye[0], centre[1] - eye[1], centre[2] - eye[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[allow(
    clippy::too_many_arguments,
    reason = "raster pass over shared frame buffers"
)]
fn draw_actor(
    actor: &crate::scene::Actor,
    geometry: &PreparedGeometry,
    camera: &Camera,
    width: u32,
    height: u32,
    light: [f32; 3],
    blend: bool,
    depth: &mut [f32],
    rgba: &mut [u8],
) -> Result<(), SceneError> {
    let transfer = actor.active_transfer();
    // Screen triangles with their mean depth, for transparent sorting.
    let mut triangles: Vec<ScreenTriangle> = Vec::new();
    for chunk in &geometry.chunks {
        let view_projection =
            camera.view_projection_relative_to(chunk.origin, width as f32 / height as f32);
        let visibility = geometry
            .bounds
            .map_or(BoundsVisibility::Intersecting, |bounds| {
                classify_bounds(bounds, chunk.origin, &view_projection)
            });
        if visibility == BoundsVisibility::Outside {
            continue;
        }
        let planes = crate::scene::local_planes(&actor.clip_planes, chunk.origin)?;
        for tri in chunk.indices.as_chunks::<3>().0 {
            if tri
                .iter()
                .any(|&index| !chunk.scalars[index as usize].is_finite())
            {
                continue;
            }
            let local_tri = tri.map(|vertex| {
                let i = vertex as usize;
                Vertex {
                    position: [
                        chunk.positions[i * 3],
                        chunk.positions[i * 3 + 1],
                        chunk.positions[i * 3 + 2],
                        1.0,
                    ],
                    scalar: chunk.scalars[i],
                }
            });
            let normal_index = tri[0] as usize * 3;
            let normal = [
                chunk.normals[normal_index],
                chunk.normals[normal_index + 1],
                chunk.normals[normal_index + 2],
            ];
            let project = |clip: [f32; 4]| {
                if clip[3] <= 0.0 {
                    return None;
                }
                Some((
                    (clip[0] / clip[3] * 0.5 + 0.5) * width as f32,
                    (0.5 - clip[1] / clip[3] * 0.5) * height as f32,
                    clip[2] / clip[3],
                ))
            };
            let mut emit = |[a, b, c]: [(f32, f32, f32); 3], mean_scalar: f32| {
                let mut colour = transfer.map_or(actor.color, |tf| tf.map(f64::from(mean_scalar)));
                colour[3] *= actor.opacity;
                let shaded = shade_with_normal(colour, normal, light);
                if blend {
                    triangles.push((a, b, c, shaded, (a.2 + b.2 + c.2) / 3.0));
                } else {
                    rasterize(a, b, c, shaded, width, height, depth, rgba, false);
                }
            };
            let mut poly = if planes.is_empty() {
                let clip_tri = local_tri.map(|vertex| Vertex {
                    position: transform(
                        &view_projection,
                        [vertex.position[0], vertex.position[1], vertex.position[2]],
                    ),
                    ..vertex
                });
                let tri_visibility = if visibility == BoundsVisibility::Inside {
                    BoundsVisibility::Inside
                } else {
                    triangle_visibility(&clip_tri)
                };
                match tri_visibility {
                    BoundsVisibility::Outside => continue,
                    BoundsVisibility::Inside => {
                        let [Some(a), Some(b), Some(c)] = clip_tri.map(|v| project(v.position))
                        else {
                            continue;
                        };
                        emit(
                            [a, b, c],
                            (clip_tri[0].scalar + clip_tri[1].scalar + clip_tri[2].scalar) / 3.0,
                        );
                        continue;
                    }
                    BoundsVisibility::Intersecting => clip_tri.into_iter().collect::<Polygon>(),
                }
            } else {
                let mut poly = local_tri.into_iter().collect::<Polygon>();
                clip_planes(&mut poly, &planes);
                for vertex in &mut poly {
                    vertex.position = transform(
                        &view_projection,
                        [vertex.position[0], vertex.position[1], vertex.position[2]],
                    );
                }
                poly
            };
            if visibility != BoundsVisibility::Inside {
                clip_frustum(&mut poly);
            }
            if poly.len() < 3 {
                continue;
            }
            let projected: SmallVec<[Option<(f32, f32, f32)>; 16]> =
                poly.iter().map(|vertex| project(vertex.position)).collect();
            for i in 1..poly.len() - 1 {
                let (Some(a), Some(b), Some(c)) = (projected[0], projected[i], projected[i + 1])
                else {
                    continue;
                };
                emit(
                    [a, b, c],
                    (poly[0].scalar + poly[i].scalar + poly[i + 1].scalar) / 3.0,
                );
            }
        }
    }
    if blend {
        triangles.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
    }
    for (a, b, c, colour, _) in triangles {
        rasterize(a, b, c, colour, width, height, depth, rgba, blend);
    }
    // Line cells (streamlines, glyphs) draw as 1px projected segments.
    if !geometry.lines.is_empty() {
        draw_lines(actor, &geometry.lines, camera, width, height, depth, rgba)?;
    }
    Ok(())
}

/// Clips and projects line cells before performing bounded screen-space work.
fn draw_lines(
    actor: &crate::scene::Actor,
    lines: &[[u32; 2]],
    camera: &Camera,
    width: u32,
    height: u32,
    depth: &mut [f32],
    rgba: &mut [u8],
) -> Result<(), SceneError> {
    let dataset = &actor.dataset;
    let origin = camera.target;
    let matrix = camera.view_projection_relative_to(origin, width as f32 / height as f32);
    let planes = crate::scene::local_planes(&actor.clip_planes, origin)?;
    let colour = [
        (actor.color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (actor.color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (actor.color[2].clamp(0.0, 1.0) * 255.0) as u8,
        (actor.color[3].clamp(0.0, 1.0) * actor.opacity * 255.0) as u8,
    ];
    for nodes in lines {
        let points = nodes.map(|node| {
            std::array::from_fn(|axis| dataset.positions[node as usize][axis] - origin[axis])
        });
        let Some(points) = clip_line(points, &planes) else {
            continue;
        };
        let points = points.map(|point| transform(&matrix, point.map(|v| v as f32)));
        let Some(points) = clip_frustum_line(points) else {
            continue;
        };
        let [a, b] = points.map(|clip| {
            (
                (clip[0] / clip[3] * 0.5 + 0.5) * width as f32,
                (0.5 - clip[1] / clip[3] * 0.5) * height as f32,
                clip[2] / clip[3],
            )
        });
        // Frustum clipping bounds this work by the viewport, including segments
        // whose original endpoints are far outside it.
        let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil() as usize).max(1);
        let mut previous = None;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = a.0 + (b.0 - a.0) * t;
            let y = a.1 + (b.1 - a.1) * t;
            let z = a.2 + (b.2 - a.2) * t;
            if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                continue;
            }
            let idx = y as usize * width as usize + x as usize;
            if previous == Some(idx) {
                continue;
            }
            previous = Some(idx);
            if z <= depth[idx] {
                source_over(&mut rgba[idx * 4..idx * 4 + 4], colour);
                if colour[3] == 255 {
                    depth[idx] = z;
                }
            }
        }
    }
    Ok(())
}

fn shade_with_normal(colour: [f32; 4], n: [f32; 3], light: [f32; 3]) -> [u8; 4] {
    let diffuse = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2])
        .abs()
        .clamp(0.0, 1.0);
    let intensity = 0.35 + 0.65 * diffuse;
    [
        (colour[0].clamp(0.0, 1.0) * intensity * 255.0) as u8,
        (colour[1].clamp(0.0, 1.0) * intensity * 255.0) as u8,
        (colour[2].clamp(0.0, 1.0) * intensity * 255.0) as u8,
        (colour[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

#[derive(Clone, Copy)]
struct CoverageEdge {
    coefficients: [f64; 3],
    inclusive: bool,
}

impl CoverageEdge {
    fn new(a: [f64; 2], b: [f64; 2]) -> Self {
        Self {
            coefficients: [a[1] - b[1], b[0] - a[0], a[0] * b[1] - a[1] * b[0]],
            inclusive: b[1] < a[1] || (b[1] == a[1] && b[0] > a[0]),
        }
    }

    #[inline]
    fn at(&self, point: [f64; 2]) -> f64 {
        self.coefficients[0] * point[0] + self.coefficients[1] * point[1] + self.coefficients[2]
    }

    #[inline]
    fn contains(&self, value: f64) -> bool {
        value > 0.0 || (value == 0.0 && self.inclusive)
    }
}

fn source_over(destination: &mut [u8], source: [u8; 4]) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    if alpha == 255 {
        destination.copy_from_slice(&source);
        return;
    }
    let inverse = 255 - alpha;
    let destination_alpha = u32::from(destination[3]);
    if destination_alpha == 255 {
        for channel in 0..3 {
            destination[channel] = ((u32::from(source[channel]) * alpha
                + u32::from(destination[channel]) * inverse
                + 127)
                / 255) as u8;
        }
    } else {
        let output_alpha = alpha * 255 + destination_alpha * inverse;
        for channel in 0..3 {
            destination[channel] = ((u32::from(source[channel]) * alpha * 255
                + u32::from(destination[channel]) * destination_alpha * inverse
                + output_alpha / 2)
                / output_alpha) as u8;
        }
        destination[3] = ((output_alpha + 127) / 255) as u8;
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "raster kernel over frame buffers"
)]
fn rasterize(
    a: (f32, f32, f32),
    b: (f32, f32, f32),
    c: (f32, f32, f32),
    colour: [u8; 4],
    width: u32,
    height: u32,
    depth: &mut [f32],
    rgba: &mut [u8],
    blend: bool,
) {
    let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as u32;
    let max_x =
        a.0.max(b.0)
            .max(c.0)
            .ceil()
            .min(width.saturating_sub(1) as f32) as u32;
    let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as u32;
    let max_y =
        a.1.max(b.1)
            .max(c.1)
            .ceil()
            .min(height.saturating_sub(1) as f32) as u32;
    // Match conventional raster coverage on an eight-bit subpixel grid.
    // Consistent edge equations and the top-left rule avoid cracks and double
    // blending when a polygon's triangulation changes after clipping.
    let mut points = [a, b, c].map(|(x, y, z)| {
        (
            [
                (f64::from(x) * 256.0).round(),
                (f64::from(y) * 256.0).round(),
            ],
            z,
        )
    });
    let area = CoverageEdge::new(points[0].0, points[1].0).at(points[2].0);
    if area == 0.0 || colour[3] == 0 {
        return;
    }
    if area < 0.0 {
        points.swap(1, 2);
    }
    let inverse_area = (1.0 / area.abs()) as f32;
    let edges = [
        CoverageEdge::new(points[1].0, points[2].0),
        CoverageEdge::new(points[2].0, points[0].0),
        CoverageEdge::new(points[0].0, points[1].0),
    ];
    let start = [
        (f64::from(min_x) + 0.5) * 256.0,
        (f64::from(min_y) + 0.5) * 256.0,
    ];
    let mut row = edges.map(|edge| edge.at(start));
    let x_step = edges.map(|edge| edge.coefficients[0] * 256.0);
    let y_step = edges.map(|edge| edge.coefficients[1] * 256.0);
    for y in min_y..=max_y {
        let mut values = row;
        for axis in 0..3 {
            row[axis] += y_step[axis];
        }
        for x in min_x..=max_x {
            let [e0, e1, e2] = values;
            for axis in 0..3 {
                values[axis] += x_step[axis];
            }
            if !edges[0].contains(e0) || !edges[1].contains(e1) || !edges[2].contains(e2) {
                continue;
            }
            let z = (e0 as f32 * points[0].1 + e1 as f32 * points[1].1 + e2 as f32 * points[2].1)
                * inverse_area;
            let idx = y as usize * width as usize + x as usize;
            if z >= depth[idx] {
                continue;
            }
            if blend {
                // Transparent passes preserve the destination's alpha contract.
                source_over(&mut rgba[idx * 4..idx * 4 + 4], colour);
            } else {
                depth[idx] = z;
                rgba[idx * 4..idx * 4 + 4].copy_from_slice(&colour);
            }
        }
    }
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Actor, TransferFunction};
    use gpui_mesh::{DataSet, Field};

    #[test]
    fn renders_a_coloured_triangle() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::scalar("p", vec![0.0, 5.0, 10.0]));
        let scene = Scene::new()
            .with_actor(Actor::new(ds).coloured_by("p", TransferFunction::viridis([0.0, 10.0])));
        let camera = Camera::fitted(scene.bounds().unwrap());
        let frame = render(&scene, &camera, 64, 64).unwrap();
        assert_eq!(frame.rgba.len(), 64 * 64 * 4);
        // A substantial area is covered by the triangle, not just background.
        let bg = scene.background;
        let bg_px = [
            (bg[0] * 255.0) as u8,
            (bg[1] * 255.0) as u8,
            (bg[2] * 255.0) as u8,
        ];
        let covered = frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[..3] != bg_px)
            .count();
        assert!(covered > 100, "only {covered} covered pixels");
        assert_eq!(&frame.to_ppm()[..2], b"P6");
    }

    #[test]
    fn empty_scene_is_background_only() {
        let frame = render(&Scene::new(), &Camera::default(), 8, 8).unwrap();
        assert!(frame.rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 255));
    }

    #[test]
    fn unused_transfer_does_not_change_flat_color() {
        let dataset = DataSet::surface(
            "flat",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let mut scene = Scene::new().with_actor(Actor::new(dataset).color([0.0, 1.0, 0.0, 0.5]));
        let camera = Camera::fitted(scene.bounds().unwrap());
        let reference = render(&scene, &camera, 32, 32).unwrap();
        scene.actors[0].transfer = Some(TransferFunction::viridis([0.0, 1.0]));
        assert_eq!(
            reference.rgba,
            render(&scene, &camera, 32, 32).unwrap().rgba
        );
    }

    #[test]
    fn oblique_clipping_multiple_origins_and_fractional_translations_are_invariant() {
        let mut reference = None;
        for offset in [0.0, 1000.125, 1e9 + 0.125, 1e12 + 0.125] {
            let scene = crate::scene::translated_clipped_scene(offset);
            let camera = Camera::fitted(scene.bounds().unwrap());
            let frame = render(&scene, &camera, 96, 64).unwrap();
            let red = frame
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[0] > pixel[1].saturating_add(20))
                .count();
            let green = frame
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[1] > pixel[0].saturating_add(20))
                .count();
            assert!(
                red > 20 && green > 20,
                "both origins visible: {red}, {green}"
            );
            if let Some(reference) = &reference {
                assert_eq!(&frame.rgba, reference, "offset {offset}");
            } else {
                reference = Some(frame.rgba);
            }
        }
    }

    fn forward_camera() -> Camera {
        Camera {
            azimuth: 0.0,
            elevation: 0.0,
            distance: 3.0,
            near: 1.0,
            far: 4.0,
            projection: crate::Projection::Perspective {
                fov_y: std::f32::consts::FRAC_PI_2,
            },
            ..Camera::default()
        }
    }

    #[test]
    fn triangles_outside_near_and_far_planes_do_not_write_pixels() {
        let camera = forward_camera();
        let empty = render(&Scene::new(), &camera, 48, 48).unwrap();
        for z in [-2.0, 2.5, 4.0] {
            let dataset = DataSet::surface(
                "outside",
                vec![[-1.0, -1.0, z], [1.0, -1.0, z], [0.0, 1.0, z]],
                &[[0, 1, 2]],
            );
            let scene = Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.0, 0.0, 1.0]));
            assert_eq!(
                empty.rgba,
                render(&scene, &camera, 48, 48).unwrap().rgba,
                "z={z}"
            );
        }
        let mut invalid = camera;
        invalid.far = invalid.near;
        assert!(matches!(
            render(&Scene::new(), &invalid, 48, 48),
            Err(SceneError::InvalidCamera(_))
        ));
    }

    #[test]
    fn nonfinite_planes_are_rejected_before_geometry_preparation() {
        let mut scene = crate::scene::translated_clipped_scene(0.0);
        scene.actors[0].clip_planes[0].origin[0] = f64::NAN;
        assert!(matches!(
            render(&scene, &Camera::default(), 32, 32),
            Err(SceneError::InvalidClipPlane(_))
        ));
        assert_eq!(scene.preparation_stats().geometry_builds, 0);
    }

    #[test]
    fn triangle_crossing_near_plane_matches_explicitly_clipped_geometry() {
        let camera = forward_camera();
        let original = DataSet::surface(
            "crossing",
            vec![[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [0.0, 1.0, 2.5]],
            &[[0, 1, 2]],
        );
        let clipped = DataSet::surface(
            "reference",
            vec![
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0 / 3.0, 1.0 / 3.0, 2.0],
                [-1.0 / 3.0, 1.0 / 3.0, 2.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        let draw = |dataset| {
            render(
                &Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.0, 0.0, 1.0])),
                &camera,
                64,
                64,
            )
            .unwrap()
        };
        let coverage = |frame: &Frame| {
            frame
                .rgba
                .chunks_exact(4)
                .map(|pixel| pixel[0] > pixel[1].saturating_add(20))
                .collect::<Vec<_>>()
        };
        let actual = coverage(&draw(original));
        let expected = coverage(&draw(clipped));
        assert!(actual.iter().filter(|&&covered| covered).count() > 100);
        let mismatches: Vec<_> = actual
            .iter()
            .zip(&expected)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| (i % 64, i / 64, *a, *b))
            .collect();
        assert!(
            mismatches.len() <= 2,
            "coverage differences: {} {:?}",
            mismatches.len(),
            &mismatches[..mismatches.len().min(24)]
        );
    }

    #[test]
    fn crossing_line_matches_explicitly_clipped_segment_at_large_origins() {
        let camera = Camera {
            near: 0.1,
            far: 10.0,
            ..forward_camera()
        };
        let line = |points: Vec<[f64; 3]>, clip: bool| {
            let mut dataset = DataSet::surface("line", points, &[]);
            dataset.cells = gpui_mesh::Cells::from_cells(&[(gpui_mesh::CellType::Line, &[0, 1])]);
            let actor = Actor::new(dataset).color([1.0; 4]);
            if clip {
                actor.clip(crate::ClipPlane {
                    origin: [0.0; 3],
                    normal: [1.0, 0.0, 0.0],
                })
            } else {
                actor
            }
        };
        let expected = render(
            &Scene::new().with_actor(line(vec![[0.0; 3], [1.0, 0.0, 0.0]], false)),
            &camera,
            64,
            64,
        )
        .unwrap();
        for offset in [0.0, 1e12 + 0.125] {
            for reverse in [false, true] {
                let mut actor = line(vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]], true);
                let mut points = actor.dataset.positions.to_vec();
                if reverse {
                    points.reverse();
                }
                for point in &mut points {
                    for value in point {
                        *value += offset;
                    }
                }
                actor.dataset.positions = points.into();
                actor.clip_planes[0].origin = [offset; 3];
                let shifted = Camera {
                    target: [offset; 3],
                    ..camera
                };
                assert_eq!(
                    expected.rgba,
                    render(&Scene::new().with_actor(actor), &shifted, 64, 64)
                        .unwrap()
                        .rgba
                );
            }
        }
    }

    #[test]
    fn transparent_quad_has_no_triangulation_seams_and_preserves_alpha() {
        let camera = Camera {
            near: 0.1,
            far: 10.0,
            ..forward_camera()
        };
        let draw = |triangles: &[[u32; 3]], alpha: f32| {
            let dataset = DataSet::surface(
                "quad",
                vec![
                    [-1.0, -1.0, 0.0],
                    [1.0, -1.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [-1.0, 1.0, 0.0],
                ],
                triangles,
            );
            let mut scene =
                Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.0, 0.0, alpha]));
            scene.background = [0.0; 4];
            render(&scene, &camera, 64, 64).unwrap()
        };
        let first = draw(&[[0, 1, 2], [0, 2, 3]], 0.5);
        let second = draw(&[[0, 1, 3], [1, 2, 3]], 0.5);
        assert_eq!(
            first.rgba, second.rgba,
            "coverage/composition independent of the internal diagonal"
        );
        let covered: Vec<_> = first.rgba.chunks_exact(4).filter(|p| p[3] > 0).collect();
        assert!(covered.len() > 100);
        assert!(covered.iter().all(|p| p[3] == 127 && p[0] == covered[0][0]));
        assert!(
            draw(&[[0, 1, 2], [0, 2, 3]], 0.0)
                .rgba
                .iter()
                .all(|&v| v == 0)
        );
    }

    #[test]
    fn lines_blend_once_per_pixel_and_opaque_depth_is_order_independent() {
        let camera = Camera {
            near: 0.1,
            far: 10.0,
            ..forward_camera()
        };
        let line = |z: f64, color| {
            let mut dataset = DataSet::surface("line", vec![[-1.0, -0.3, z], [1.0, 0.3, z]], &[]);
            dataset.cells = gpui_mesh::Cells::from_cells(&[(gpui_mesh::CellType::Line, &[0, 1])]);
            Actor::new(dataset).color(color)
        };
        let mut scene = Scene::new().with_actor(line(0.0, [1.0, 0.0, 0.0, 0.5]));
        scene.background = [0.0; 4];
        let blended = render(&scene, &camera, 64, 64).unwrap();
        let covered: Vec<_> = blended.rgba.chunks_exact(4).filter(|p| p[3] > 0).collect();
        assert!(covered.len() > 10);
        assert!(covered.iter().all(|p| *p == [255, 0, 0, 127]));
        let mut opaque = Scene::new()
            .with_actor(line(1.0, [1.0, 0.0, 0.0, 1.0]))
            .with_actor(line(0.0, [0.0, 0.0, 1.0, 1.0]));
        let first = render(&opaque, &camera, 64, 64).unwrap();
        opaque.actors.reverse();
        assert_eq!(first.rgba, render(&opaque, &camera, 64, 64).unwrap().rgba);
    }

    #[test]
    fn frame_limits_reject_before_buffer_allocation() {
        let limits = RenderLimits {
            max_dimension: 100,
            max_frame_bytes: 127,
        };
        assert!(matches!(
            render_with_limits(&Scene::new(), &Camera::default(), 4, 4, limits),
            Err(SceneError::ResourceLimit { .. })
        ));
    }

    #[test]
    fn transfer_alpha_blends_and_invalid_samples_leave_background() {
        let dataset = DataSet::surface(
            "triangle",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::scalar("p", vec![0.5; 3]));
        let camera = Camera::fitted(dataset.bounds().unwrap());
        let scene = |alpha| {
            let actor = Actor::new(dataset.clone()).coloured_by(
                "p",
                TransferFunction::from_stops([0.0, 1.0], &[(0.0, [1.0, 0.0, 0.0, alpha])]),
            );
            let mut scene = Scene::new().with_actor(actor);
            scene.background = [0.0, 0.0, 0.0, 1.0];
            scene
        };
        let total_red = |frame: &Frame| {
            frame
                .rgba
                .chunks_exact(4)
                .map(|p| u64::from(p[0]))
                .sum::<u64>()
        };
        let opaque = render(&scene(1.0), &camera, 64, 64).unwrap();
        let translucent = render(&scene(0.25), &camera, 64, 64).unwrap();
        assert!(total_red(&opaque) > 1000);
        assert!(total_red(&translucent) < total_red(&opaque) / 2);
        let mut masked = scene(1.0);
        masked.actors[0].dataset.point_data[0].valid = Some(vec![false; 3].into());
        assert_eq!(total_red(&render(&masked, &camera, 64, 64).unwrap()), 0);
    }
}

#[cfg(test)]
mod clip_transparency_tests {
    use super::*;
    use crate::Actor;
    use crate::scene::ClipPlane;
    use gpui_mesh::DataSet;

    fn quad(z: f64) -> DataSet {
        DataSet::surface(
            "quad",
            vec![
                [-1.0, -1.0, z],
                [1.0, -1.0, z],
                [1.0, 1.0, z],
                [-1.0, 1.0, z],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        )
    }

    fn covered(frame: &Frame) -> usize {
        frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[..3] != [20, 22, 28])
            .count()
    }

    #[test]
    fn clipping_removes_half_the_surface() {
        let full = Scene::new().with_actor(Actor::new(quad(0.0)).color([1.0, 0.0, 0.0, 1.0]));
        let camera = Camera::fitted(full.bounds().unwrap());
        let full_frame = render(&full, &camera, 64, 64).unwrap();
        let clipped =
            Scene::new().with_actor(Actor::new(quad(0.0)).color([1.0, 0.0, 0.0, 1.0]).clip(
                ClipPlane {
                    origin: [0.0; 3],
                    normal: [1.0, 0.0, 0.0],
                },
            ));
        let clipped_frame = render(&clipped, &camera, 64, 64).unwrap();
        let (a, b) = (covered(&full_frame), covered(&clipped_frame));
        assert!(a > 500, "full coverage {a}");
        assert!(
            b < a * 3 / 4 && b > a / 4,
            "clipped coverage {b} vs full {a}"
        );
    }

    #[test]
    fn transparent_actor_blends_over_opaque() {
        let mut scene = Scene::new();
        scene.background = [20.0 / 255.0, 22.0 / 255.0, 28.0 / 255.0, 1.0];
        let blended = scene.with_actor(
            Actor::new(quad(0.0))
                .color([1.0, 0.0, 0.0, 1.0])
                .opacity(0.5),
        );
        let camera = Camera::fitted(blended.bounds().unwrap());
        let frame = render(&blended, &camera, 64, 64).unwrap();
        // Centre pixel is a blend: neither pure red nor background.
        let centre = frame.rgba[((32 * 64 + 32) * 4) as usize];
        assert!(centre > 40 && centre < 200, "centre red channel {centre}");
    }
}
