//! Software rasterizer: scenes to RGBA frames without any GPU.

// Rust guideline compliant 2026-02-21

use crate::camera::{Camera, transform};
use crate::limits::frame_buffer;
use crate::prepare::prepare_chunks;
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
    let pixels = limits.pixel_count(width, height, 8)?;
    let bg = scene.background;
    let mut rgba = frame_buffer(pixels * 4, 0u8)?;
    for px in rgba.as_chunks_mut::<4>().0 {
        px[0] = (bg[0].clamp(0.0, 1.0) * 255.0) as u8;
        px[1] = (bg[1].clamp(0.0, 1.0) * 255.0) as u8;
        px[2] = (bg[2].clamp(0.0, 1.0) * 255.0) as u8;
        px[3] = (bg[3].clamp(0.0, 1.0) * 255.0) as u8;
    }
    let mut depth = frame_buffer(pixels, f32::INFINITY)?;
    let aspect = width as f32 / height as f32;
    let view_projection = camera.view_projection(aspect);
    let light = normalize3([-0.4, 0.7, 0.6]);
    let eye = camera.eye();

    let visible: Vec<&crate::scene::Actor> = scene.actors.iter().filter(|a| a.visible).collect();
    // Opaque first, in scene order.
    for actor in visible.iter().filter(|a| !a.is_transparent()) {
        draw_actor(
            actor,
            &view_projection,
            width,
            height,
            light,
            false,
            &mut depth,
            &mut rgba,
        )?;
    }
    // Transparent actors back-to-front by bounds-centre distance.
    let mut transparent: Vec<&crate::scene::Actor> = visible
        .iter()
        .filter(|a| a.is_transparent())
        .copied()
        .collect();
    transparent.sort_by(|a, b| {
        let da = actor_distance(a, eye);
        let db = actor_distance(b, eye);
        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
    });
    for actor in transparent {
        draw_actor(
            actor,
            &view_projection,
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

fn actor_distance(actor: &crate::scene::Actor, eye: [f32; 3]) -> f32 {
    let Some(bounds) = actor.dataset.bounds() else {
        return 0.0;
    };
    let centre = bounds.center();
    let d = [
        centre[0] as f32 - eye[0],
        centre[1] as f32 - eye[1],
        centre[2] as f32 - eye[2],
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// A world-space vertex with its scalar, for clipping interpolation.
#[derive(Debug, Clone, Copy)]
struct ClipVertex {
    world: [f32; 3],
    scalar: f32,
}

fn clip_triangle(
    tri: [ClipVertex; 3],
    planes: &[crate::scene::ClipPlane],
) -> SmallVec<[ClipVertex; 12]> {
    // A triangle clipped by six planes has at most nine vertices. Keep the
    // ordinary case on the stack, while safely supporting longer plane lists.
    let mut poly = SmallVec::<[ClipVertex; 12]>::new();
    poly.extend_from_slice(&tri);
    for plane in planes {
        if poly.is_empty() {
            break;
        }
        let mut out = SmallVec::<[ClipVertex; 12]>::new();
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let da = plane.distance([
                f64::from(a.world[0]),
                f64::from(a.world[1]),
                f64::from(a.world[2]),
            ]);
            let db = plane.distance([
                f64::from(b.world[0]),
                f64::from(b.world[1]),
                f64::from(b.world[2]),
            ]);
            if da >= 0.0 {
                out.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = (da / (da - db)) as f32;
                out.push(ClipVertex {
                    world: [
                        a.world[0] + (b.world[0] - a.world[0]) * t,
                        a.world[1] + (b.world[1] - a.world[1]) * t,
                        a.world[2] + (b.world[2] - a.world[2]) * t,
                    ],
                    scalar: a.scalar + (b.scalar - a.scalar) * t,
                });
            }
        }
        poly = out;
    }
    poly
}

#[allow(
    clippy::too_many_arguments,
    reason = "raster pass over shared frame buffers"
)]
fn draw_actor(
    actor: &crate::scene::Actor,
    view_projection: &[f32; 16],
    width: u32,
    height: u32,
    light: [f32; 3],
    blend: bool,
    depth: &mut [f32],
    rgba: &mut [u8],
) -> Result<(), SceneError> {
    let chunks = prepare_chunks(actor, crate::prepare::DEFAULT_MAX_TRIANGLES)?;
    let transfer = actor.transfer.as_ref();
    // Screen triangles with their mean depth, for transparent sorting.
    let mut triangles: Vec<ScreenTriangle> = Vec::new();
    for chunk in &chunks {
        for tri in chunk.indices.as_chunks::<3>().0 {
            if tri
                .iter()
                .any(|&index| !chunk.scalars[index as usize].is_finite())
            {
                continue;
            }
            let mut world_tri = [ClipVertex {
                world: [0.0; 3],
                scalar: 0.0,
            }; 3];
            for (slot, vertex) in tri.iter().enumerate() {
                let i = *vertex as usize;
                world_tri[slot] = ClipVertex {
                    world: [
                        chunk.positions[i * 3] + chunk.origin[0] as f32,
                        chunk.positions[i * 3 + 1] + chunk.origin[1] as f32,
                        chunk.positions[i * 3 + 2] + chunk.origin[2] as f32,
                    ],
                    scalar: chunk.scalars[i],
                };
            }
            let poly = clip_triangle(world_tri, &actor.clip_planes);
            if poly.len() < 3 {
                continue;
            }
            // Project the clipped polygon once, then fan-triangulate.
            let mut projected =
                SmallVec::<[Option<(f32, f32, f32)>; 12]>::with_capacity(poly.len());
            for vertex in &poly {
                let clip = transform(view_projection, vertex.world);
                if clip[3] <= 0.0 {
                    projected.push(None);
                    continue;
                }
                let ndc = [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]];
                projected.push(Some((
                    (ndc[0] * 0.5 + 0.5) * width as f32,
                    (1.0 - (ndc[1] * 0.5 + 0.5)) * height as f32,
                    ndc[2],
                )));
            }
            for i in 1..poly.len() - 1 {
                let (Some(a), Some(b), Some(c)) = (projected[0], projected[i], projected[i + 1])
                else {
                    continue;
                };
                let mean_scalar = (poly[0].scalar + poly[i].scalar + poly[i + 1].scalar) / 3.0;
                let colour = match &transfer {
                    Some(tf) => tf.map(f64::from(mean_scalar)),
                    None => actor.color,
                };
                let mut colour = colour;
                colour[3] *= actor.opacity;
                // Flat shading from the first vertex normal of the source
                // triangle (clip preserves the face orientation).
                let normal_index = tri[0] as usize * 3;
                let normal = [
                    chunk.normals[normal_index],
                    chunk.normals[normal_index + 1],
                    chunk.normals[normal_index + 2],
                ];
                let shaded = shade_with_normal(colour, normal, light);
                let mean_depth = (a.2 + b.2 + c.2) / 3.0;
                if blend {
                    triangles.push((a, b, c, shaded, mean_depth));
                } else {
                    // Opaque depth testing does not require retaining a second
                    // screen-space copy of all scene triangles.
                    rasterize(a, b, c, shaded, width, height, depth, rgba, false);
                }
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
    draw_lines(actor, view_projection, width, height, depth, rgba);
    Ok(())
}

/// Projects and rasterizes an actor's line cells with a midpoint depth test.
fn draw_lines(
    actor: &crate::scene::Actor,
    view_projection: &[f32; 16],
    width: u32,
    height: u32,
    depth: &[f32],
    rgba: &mut [u8],
) {
    let dataset = &actor.dataset;
    let colour = [
        (actor.color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (actor.color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (actor.color[2].clamp(0.0, 1.0) * 255.0) as u8,
        (actor.color[3].clamp(0.0, 1.0) * actor.opacity * 255.0) as u8,
    ];
    let project = |point: [f64; 3]| -> Option<(f32, f32, f32)> {
        let clip = transform(
            view_projection,
            [point[0] as f32, point[1] as f32, point[2] as f32],
        );
        if clip[3] <= 0.0 {
            return None;
        }
        let ndc = [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]];
        Some((
            (ndc[0] * 0.5 + 0.5) * width as f32,
            (1.0 - (ndc[1] * 0.5 + 0.5)) * height as f32,
            ndc[2],
        ))
    };
    for cell in 0..dataset.cell_count() {
        if dataset.cells.types[cell] != gpui_mesh::CellType::Line {
            continue;
        }
        let Some(nodes) = dataset.cells.cell(cell) else {
            continue;
        };
        if nodes.len() != 2 {
            continue;
        }
        let (Some(a), Some(b)) = (
            project(dataset.positions[nodes[0] as usize]),
            project(dataset.positions[nodes[1] as usize]),
        ) else {
            continue;
        };
        // Skip segments clipped away by any plane (endpoint test).
        if actor.clip_planes.iter().any(|plane| {
            plane.distance(dataset.positions[nodes[0] as usize]) < 0.0
                || plane.distance(dataset.positions[nodes[1] as usize]) < 0.0
        }) {
            continue;
        }
        let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) as usize).max(1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = a.0 + (b.0 - a.0) * t;
            let y = a.1 + (b.1 - a.1) * t;
            let z = a.2 + (b.2 - a.2) * t;
            if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                continue;
            }
            let idx = y as usize * width as usize + x as usize;
            if z <= depth[idx] + 1e-3 {
                rgba[idx * 4..idx * 4 + 4].copy_from_slice(&colour);
            }
        }
    }
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
    let denom = (b.1 - c.1) * (a.0 - c.0) + (c.0 - b.0) * (a.1 - c.1);
    if denom.abs() < 1e-12 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = ((b.1 - c.1) * (px - c.0) + (c.0 - b.0) * (py - c.1)) / denom;
            let w1 = ((c.1 - a.1) * (px - c.0) + (a.0 - c.0) * (py - c.1)) / denom;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
            let idx = y as usize * width as usize + x as usize;
            if z >= depth[idx] {
                continue;
            }
            if blend {
                // Source-over blending; transparent pass does not write depth.
                let alpha = f32::from(colour[3]) / 255.0;
                for channel in 0..3 {
                    let dst = f32::from(rgba[idx * 4 + channel]);
                    let src = f32::from(colour[channel]);
                    rgba[idx * 4 + channel] = (src * alpha + dst * (1.0 - alpha)).round() as u8;
                }
                rgba[idx * 4 + 3] = 255;
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
