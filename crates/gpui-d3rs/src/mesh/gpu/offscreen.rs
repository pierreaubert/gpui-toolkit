use super::{
    FieldRevision, GeometryRevision, MeshGpuRenderer, MeshSceneState, replace_retained_field,
};
#[cfg(all(feature = "gpu-3d", not(test), not(target_family = "wasm")))]
use crate::gpu3d::Camera3D;
use crate::mesh::MeshUpload;
use image::{Rgba, RgbaImage};
use std::cell::RefCell;
use std::rc::Rc;

/// Readback/fallback renderer sharing the same retained state as the custom
/// draw backends. It is useful on unsupported platforms and in deterministic
/// export/tests; geometry and field writes still follow the common renderer
/// contract rather than bypassing revisions.
#[allow(
    dead_code,
    reason = "deterministic fallback is retained for unsupported backend/export integration"
)]
#[derive(Debug, Clone)]
pub struct OffscreenMeshRenderer {
    state: Rc<RefCell<MeshSceneState>>,
}

#[allow(
    dead_code,
    reason = "deterministic fallback is retained for unsupported backend/export integration"
)]
impl OffscreenMeshRenderer {
    pub fn new(state: Rc<RefCell<MeshSceneState>>) -> Self {
        Self { state }
    }

    pub fn state(&self) -> Rc<RefCell<MeshSceneState>> {
        self.state.clone()
    }

    pub fn render(&self, width: u32, height: u32) -> RgbaImage {
        let state = self.state.borrow();
        render_offscreen(state.upload.as_ref(), &state, width, height)
    }
}

impl MeshGpuRenderer for OffscreenMeshRenderer {
    fn upload_geometry(&mut self, rev: GeometryRevision, upload: &MeshUpload) {
        let mut state = self.state.borrow_mut();
        state.record_geometry_upload(upload);
        state.geometry_rev = rev;
        state.upload = Some(upload.clone());
    }

    fn write_field(&mut self, rev: FieldRevision, values: &[f32]) {
        let mut state = self.state.borrow_mut();
        state.record_field_write(values);
        state.field_rev = rev;
        if let Some(upload) = state.upload.as_mut() {
            if upload.cell_values_f32.is_some() {
                replace_retained_field(&mut upload.cell_values_f32, values);
            } else {
                replace_retained_field(&mut upload.values_f32, values);
            }
        }
    }

    fn geometry_revision(&self) -> Option<GeometryRevision> {
        let state = self.state.borrow();
        state.upload.as_ref().map(|_| state.geometry_rev)
    }
}

/// Deterministic CPU fallback image. It intentionally returns a valid image
/// even when no adapter is present; platform renderers can replace it with a
/// readback frame without changing the element contract.
pub fn render_offscreen(
    upload: Option<&MeshUpload>,
    state: &MeshSceneState,
    width: u32,
    height: u32,
) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let Some(upload) = upload else {
        return image;
    };
    if width == 0 || height == 0 || upload.positions_f32.is_empty() {
        return image;
    }

    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    for point in &upload.positions_f32 {
        min[0] = min[0].min(point[0]);
        min[1] = min[1].min(point[1]);
        max[0] = max[0].max(point[0]);
        max[1] = max[1].max(point[1]);
    }
    if !min.iter().chain(max.iter()).all(|value| value.is_finite()) {
        return image;
    }
    let span_x = (max[0] - min[0]).max(f32::EPSILON);
    let span_y = (max[1] - min[1]).max(f32::EPSILON);
    let scale =
        ((width.saturating_sub(1) as f32) / span_x).min((height.saturating_sub(1) as f32) / span_y);
    let offset_x = (width as f32 - span_x * scale) * 0.5;
    let offset_y = (height as f32 - span_y * scale) * 0.5;
    let use_view_transform = state
        .view_transform
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        && state.view_transform != identity_matrix();
    let point = |index: u32| -> Option<[f32; 2]> {
        // MeshUpload is normally produced from a validated TriangleMesh, but
        // it is a public retained-render input. An invalid index must suppress
        // the primitive rather than silently substituting the origin and
        // rasterizing a different triangle or edge.
        let p = upload.positions_f32.get(index as usize).copied()?;
        if use_view_transform {
            let clip = transform_point(state.view_transform, p)?;
            Some([
                (clip[0] + 1.0) * 0.5 * width as f32,
                (1.0 - clip[1]) * 0.5 * height as f32,
            ])
        } else {
            Some([
                offset_x + (p[0] - min[0]) * scale,
                height as f32 - offset_y - (p[1] - min[1]) * scale,
            ])
        }
    };
    let range = state.color.range;
    let range_span = (range[1] - range[0]).abs().max(f32::EPSILON);

    for (cell_index, triangle) in upload.indices.chunks_exact(3).enumerate() {
        let (Some(a), Some(b), Some(c)) =
            (point(triangle[0]), point(triangle[1]), point(triangle[2]))
        else {
            continue;
        };
        let area = edge(a, b, c);
        if area.abs() <= f32::EPSILON {
            continue;
        }
        let Some((lo_x, hi_x)) = clipped_pixel_range([a[0], b[0], c[0]], width) else {
            continue;
        };
        let Some((lo_y, hi_y)) = clipped_pixel_range([a[1], b[1], c[1]], height) else {
            continue;
        };
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                let weights = [
                    edge(b, c, p) / area,
                    edge(c, a, p) / area,
                    edge(a, b, p) / area,
                ];
                if weights.iter().any(|weight| *weight < -1e-5) {
                    continue;
                }
                let color = if let Some(values) = &upload.cell_values_f32 {
                    let value = values.get(cell_index).copied().unwrap_or(f32::NAN);
                    if !value.is_finite() {
                        continue;
                    }
                    colormap(
                        ((value - range[0]) / range_span).clamp(0.0, 1.0),
                        state.color.colormap,
                    )
                } else if let Some(values) = &upload.values_f32 {
                    let values = [
                        values
                            .get(triangle[0] as usize)
                            .copied()
                            .unwrap_or(f32::NAN),
                        values
                            .get(triangle[1] as usize)
                            .copied()
                            .unwrap_or(f32::NAN),
                        values
                            .get(triangle[2] as usize)
                            .copied()
                            .unwrap_or(f32::NAN),
                    ];
                    if values.iter().any(|value| !value.is_finite()) {
                        continue;
                    }
                    let value =
                        values[0] * weights[0] + values[1] * weights[1] + values[2] * weights[2];
                    colormap(
                        ((value - range[0]) / range_span).clamp(0.0, 1.0),
                        state.color.colormap,
                    )
                } else {
                    [150, 160, 175]
                };
                image.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
            }
        }
    }

    if state.color.isoline_step > 0.0 && state.color.isoline_width_px > 0.0 {
        for (cell_index, triangle) in upload.indices.chunks_exact(3).enumerate() {
            let triangle_values = if let Some(values) = &upload.cell_values_f32 {
                let value = values.get(cell_index).copied().unwrap_or(f32::NAN);
                [value; 3]
            } else if let Some(values) = &upload.values_f32 {
                [
                    values
                        .get(triangle[0] as usize)
                        .copied()
                        .unwrap_or(f32::NAN),
                    values
                        .get(triangle[1] as usize)
                        .copied()
                        .unwrap_or(f32::NAN),
                    values
                        .get(triangle[2] as usize)
                        .copied()
                        .unwrap_or(f32::NAN),
                ]
            } else {
                continue;
            };
            if triangle_values.iter().any(|value| !value.is_finite()) {
                continue;
            }
            let (Some(a_point), Some(b_point), Some(c_point)) =
                (point(triangle[0]), point(triangle[1]), point(triangle[2]))
            else {
                continue;
            };
            let triangle_points = [a_point, b_point, c_point];
            let mut hits = Vec::with_capacity(2);
            for (a, b) in [(0usize, 1usize), (1, 2), (2, 0)] {
                let va = triangle_values[a];
                let vb = triangle_values[b];
                if (va >= state.color.isoline_step) == (vb >= state.color.isoline_step) {
                    continue;
                }
                let t = ((state.color.isoline_step - va) / (vb - va)).clamp(0.0, 1.0);
                hits.push([
                    triangle_points[a][0] + t * (triangle_points[b][0] - triangle_points[a][0]),
                    triangle_points[a][1] + t * (triangle_points[b][1] - triangle_points[a][1]),
                ]);
            }
            if hits.len() == 2 {
                draw_line(&mut image, hits[0], hits[1], [28, 32, 40, 255]);
            }
        }
    }

    if state.color.wireframe {
        for edge_indices in upload.edge_indices.chunks_exact(2) {
            let (Some(a), Some(b)) = (point(edge_indices[0]), point(edge_indices[1])) else {
                continue;
            };
            draw_line(&mut image, a, b, [28, 32, 40, 255]);
        }
    }
    image
}

/// Render one retained 3D scene through the real WGPU custom-draw backend and
/// read the target texture back to an RGBA image.
///
/// This is intentionally separate from [`render_offscreen`]. The CPU path is
/// deterministic and remains the fallback for export hosts without an
/// adapter; this helper provides adapter-backed framebuffer evidence and a
/// native WGPU readback path for callers that explicitly opt into it.
#[cfg(all(feature = "gpu-3d", not(test), not(target_family = "wasm")))]
pub fn render_offscreen_wgpu(
    state: &MeshSceneState,
    width: u32,
    height: u32,
) -> Result<RgbaImage, String> {
    render_offscreen_wgpu_with_camera(state, width, height, &Camera3D::default())
}

/// Render a retained 3D scene through the real WGPU path with an explicit
/// camera. The default [`render_offscreen_wgpu`] entry point remains stable
/// for existing callers; this variant makes camera-state parity and export
/// QA able to exercise the same retained renderer without mutating state.
#[cfg(all(feature = "gpu-3d", not(test), not(target_family = "wasm")))]
pub fn render_offscreen_wgpu_with_camera(
    state: &MeshSceneState,
    width: u32,
    height: u32,
    camera: &Camera3D,
) -> Result<RgbaImage, String> {
    use gpui::{Bounds, Point, Size, px};
    use gpui_wgpu::WgpuContext;
    use std::sync::mpsc;
    use std::time::Duration;

    if width == 0 || height == 0 {
        return Err("WGPU readback dimensions must be non-zero".into());
    }

    let context = WgpuContext::headless().map_err(|error| error.to_string())?;
    let format = context.color_texture_format();
    let texture = context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mesh_offscreen_wgpu_target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let state = std::rc::Rc::new(std::cell::RefCell::new(state.clone()));
    let renderer = super::WgpuMesh3DRenderer::new_with_camera(
        state,
        std::rc::Rc::new(std::cell::RefCell::new(camera.clone())),
    );
    let draw = gpui::lookup_custom_draw(renderer.custom_id())
        .ok_or_else(|| "WGPU mesh custom draw was not registered".to_string())?;
    let draw = draw
        .as_any()
        .downcast_ref::<gpui_wgpu::WgpuCustomDrawAdapter>()
        .ok_or_else(|| "WGPU mesh custom draw has an unexpected adapter".to_string())?;

    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_offscreen_wgpu_encoder"),
        });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mesh_offscreen_wgpu_clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                // Metal headless captures are returned as an opaque
                // compositor framebuffer. Use the same opaque black
                // background for the paired WGPU capture so alpha does
                // not dominate an otherwise meaningful RGB comparison.
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    draw.0.draw_wgpu(
        &context,
        &mut encoder,
        &view,
        [width, height],
        Bounds::new(
            Point::new(px(0.0), px(0.0)),
            Size::new(px(width as f32), px(height as f32)),
        ),
        1.0,
    );

    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| "WGPU readback row size overflow".to_string())?;
    let padded_row_bytes = row_bytes
        .div_ceil(256)
        .checked_mul(256)
        .ok_or_else(|| "WGPU readback alignment overflow".to_string())?;
    let staging = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mesh_offscreen_wgpu_readback"),
        size: u64::from(padded_row_bytes) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    context.queue.submit(std::iter::once(encoder.finish()));

    let slice = staging.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = context.device.poll(wgpu::PollType::Wait {
        submission_index: Default::default(),
        timeout: Some(Duration::from_secs(5)),
    });
    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|error| format!("WGPU readback callback timed out: {error}"))?
        .map_err(|error| format!("WGPU readback mapping failed: {error}"))?;

    let data = slice.get_mapped_range();
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|width| usize::try_from(height).ok().map(|height| width * height))
        .ok_or_else(|| "WGPU readback image size overflow".to_string())?;
    let mut rgba = vec![0u8; pixel_count * 4];
    for y in 0..height as usize {
        let source_row = y * padded_row_bytes as usize;
        let target_row = y * width as usize * 4;
        for x in 0..width as usize {
            let source = source_row + x * 4;
            let target = target_row + x * 4;
            let pixel = &data[source..source + 4];
            if format == wgpu::TextureFormat::Bgra8Unorm {
                rgba[target..target + 4].copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            } else if format == wgpu::TextureFormat::Rgba8Unorm {
                rgba[target..target + 4].copy_from_slice(pixel);
            } else {
                drop(data);
                staging.unmap();
                return Err(format!("unsupported WGPU readback format {format:?}"));
            }
        }
    }
    drop(data);
    staging.unmap();
    RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "WGPU readback image dimensions do not match payload".to_string())
}

fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

fn identity_matrix() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Apply a column-major 4x4 transform to a rebased position and divide by w.
fn transform_point(matrix: [[f32; 4]; 4], point: [f32; 3]) -> Option<[f32; 3]> {
    let input = [point[0], point[1], point[2], 1.0];
    let mut output = [0.0; 4];
    for column in 0..4 {
        for row in 0..4 {
            output[row] += matrix[column][row] * input[column];
        }
    }
    if !output.iter().all(|value| value.is_finite()) {
        return None;
    }
    let w = if output[3].abs() < f32::EPSILON {
        if output[3].is_sign_negative() {
            -f32::EPSILON
        } else {
            f32::EPSILON
        }
    } else {
        output[3]
    };
    let transformed = [output[0] / w, output[1] / w, output[2] / w];
    transformed
        .iter()
        .all(|value| value.is_finite())
        .then_some(transformed)
}

fn clipped_pixel_range(values: [f32; 3], extent: u32) -> Option<(u32, u32)> {
    if extent == 0 || !values.iter().all(|value| value.is_finite()) {
        return None;
    }
    let min_value = values.into_iter().fold(f32::INFINITY, f32::min);
    let max_value = values.into_iter().fold(f32::NEG_INFINITY, f32::max);
    let max_pixel = extent as f32 - 1.0;
    if max_value < 0.0 || min_value > max_pixel {
        return None;
    }
    let lo = min_value.floor().clamp(0.0, max_pixel) as u32;
    let hi = max_value.ceil().clamp(0.0, max_pixel) as u32;
    (lo <= hi).then_some((lo, hi))
}

fn colormap(t: f32, map_id: u32) -> [u8; 3] {
    let rgb = match map_id {
        0 => polynomial_map(
            t,
            [0.2777, 0.0054, 0.3340],
            [0.1050, 0.6387, 0.2383],
            [-0.3308, 0.3143, 0.5287],
            [-4.6342, -5.7991, -19.3324],
            [6.2282, 14.1799, 56.6905],
            [4.7763, -13.7451, -65.3530],
            [-5.4354, 4.6456, 26.3124],
        ),
        1 => polynomial_map(
            t,
            [0.0504, 0.0298, 0.5280],
            [2.0280, -0.3996, -0.1361],
            [-2.1285, 1.3971, -1.8103],
            [-10.2107, 6.8536, 18.8406],
            [33.6908, -21.2851, -41.8887],
            [-38.8641, 25.8915, 35.6632],
            [12.8861, -7.9772, -11.5408],
        ),
        2 => polynomial_map(
            t,
            [0.0002, 0.0016, 0.0139],
            [0.1260, 0.4023, 1.3241],
            [1.1661, 0.0868, -2.1073],
            [-1.0127, 2.0841, 2.4048],
            [-8.8174, 0.1567, -2.5439],
            [17.5174, -4.5424, 0.8282],
            [-9.5028, 3.3025, 0.0987],
        ),
        3 => [
            (0.13572
                + t * (4.6153 + t * (-42.6592 + t * (138.5676 + t * (-152.3494 + t * 59.2859)))))
                .clamp(0.0, 1.0),
            (0.09140 + t * (2.2537 + t * (0.6487 + t * (-23.3910 + t * (38.3522 - t * 18.0858)))))
                .clamp(0.0, 1.0),
            (0.10667
                + t * (12.5925 + t * (-60.5820 + t * (109.7316 + t * (-88.2949 + t * 26.7236)))))
                .clamp(0.0, 1.0),
        ],
        _ => {
            if t < 0.5 {
                lerp_rgb([0.23, 0.30, 0.75], [0.87, 0.87, 0.87], t * 2.0)
            } else {
                lerp_rgb([0.87, 0.87, 0.87], [0.71, 0.02, 0.15], (t - 0.5) * 2.0)
            }
        }
    };
    rgb.map(|value| (value * 255.0).round().clamp(0.0, 255.0) as u8)
}

fn polynomial_map(
    t: f32,
    c0: [f32; 3],
    c1: [f32; 3],
    c2: [f32; 3],
    c3: [f32; 3],
    c4: [f32; 3],
    c5: [f32; 3],
    c6: [f32; 3],
) -> [f32; 3] {
    let powers = [
        1.0,
        t,
        t * t,
        t * t * t,
        t * t * t * t,
        t * t * t * t * t,
        t * t * t * t * t * t,
    ];
    std::array::from_fn(|axis| {
        (c0[axis] * powers[0]
            + c1[axis] * powers[1]
            + c2[axis] * powers[2]
            + c3[axis] * powers[3]
            + c4[axis] * powers[4]
            + c5[axis] * powers[5]
            + c6[axis] * powers[6])
            .clamp(0.0, 1.0)
    })
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * t.clamp(0.0, 1.0))
}

fn draw_line(image: &mut RgbaImage, a: [f32; 2], b: [f32; 2], color: [u8; 4]) {
    let steps = (a[0] - b[0]).abs().max((a[1] - b[1]).abs()).ceil() as u32;
    for step in 0..=steps.max(1) {
        let t = step as f32 / steps.max(1) as f32;
        let x = (a[0] + (b[0] - a[0]) * t).round() as i32;
        let y = (a[1] + (b[1] - a[1]) * t).round() as i32;
        if x >= 0 && y >= 0 && (x as u32) < image.width() && (y as u32) < image.height() {
            image.put_pixel(x as u32, y as u32, Rgba(color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{MeshTopology, TriangleMesh, prepare_upload};
    use std::sync::Arc;

    fn triangle_upload() -> MeshUpload {
        MeshUpload {
            positions_f32: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            origin: [0.0; 3],
            indices: vec![0, 1, 2],
            edge_indices: vec![0, 1, 1, 2, 2, 0],
            values_f32: Some(vec![0.0, 0.5, 1.0]),
            cell_values_f32: None,
        }
    }

    #[test]
    fn offscreen_renderer_draws_mesh_and_retains_geometry_for_field_updates() {
        let state = Rc::new(RefCell::new(MeshSceneState::default()));
        let mut renderer = OffscreenMeshRenderer::new(state.clone());
        let upload = triangle_upload();

        renderer.upload_geometry(GeometryRevision(4), &upload);
        let first = renderer.render(32, 32);
        assert!(first.pixels().any(|pixel| pixel.0[3] != 0));

        renderer.write_field(FieldRevision(7), &[1.0, 1.0, 1.0]);
        assert_eq!(renderer.geometry_revision(), Some(GeometryRevision(4)));
        let retained = state.borrow();
        assert_eq!(retained.geometry_upload_count, 1);
        assert_eq!(retained.geometry_upload_bytes, upload.geometry_byte_len());
        assert_eq!(retained.field_write_count, 1);
        assert_eq!(
            retained.field_write_bytes,
            3 * std::mem::size_of::<f32>() as u64
        );
        assert_eq!(retained.field_rev, FieldRevision(7));
        drop(retained);

        let second = renderer.render(32, 32);
        assert!(second.pixels().any(|pixel| pixel.0[3] != 0));
    }

    #[test]
    fn offscreen_renderer_returns_empty_frame_without_geometry() {
        let state = Rc::new(RefCell::new(MeshSceneState::default()));
        let renderer = OffscreenMeshRenderer::new(state);
        let image = renderer.render(8, 8);
        assert!(image.pixels().all(|pixel| pixel.0 == [0, 0, 0, 0]));
    }

    #[test]
    fn fallback_rasterizes_mesh_instead_of_returning_placeholder() {
        let mesh = TriangleMesh {
            id: "fallback".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let topology = MeshTopology::build(&mesh.triangles);
        let upload = prepare_upload(&mesh, &topology);
        let image = render_offscreen(Some(&upload), &MeshSceneState::default(), 32, 32);
        assert!(image.pixels().any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn fallback_honors_masked_cell_values() {
        let mesh = TriangleMesh {
            id: "masked-cell".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let topology = MeshTopology::build(&mesh.triangles);
        let mut upload = prepare_upload(&mesh, &topology);
        upload.cell_values_f32 = Some(vec![f32::NAN]);
        let image = render_offscreen(Some(&upload), &MeshSceneState::default(), 32, 32);
        assert!(image.pixels().all(|pixel| pixel[3] == 0));
    }

    #[test]
    fn fallback_clips_transformed_triangles_outside_viewport() {
        let mesh = TriangleMesh {
            id: "clipped".into(),
            positions: Arc::from([[-10.0, -10.0, 0.0], [-9.0, -10.0, 0.0], [-10.0, -9.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let topology = MeshTopology::build(&mesh.triangles);
        let upload = prepare_upload(&mesh, &topology);
        let mut state = MeshSceneState::default();
        state.view_transform[3][0] = -100.0;
        state.view_transform[3][1] = -100.0;
        let image = render_offscreen(Some(&upload), &state, 32, 32);
        assert!(image.pixels().all(|pixel| pixel[3] == 0));
    }

    #[test]
    fn fallback_skips_primitives_with_out_of_range_indices() {
        let upload = MeshUpload {
            positions_f32: vec![[10.0, 10.0, 0.0], [20.0, 10.0, 0.0], [10.0, 20.0, 0.0]],
            origin: [0.0; 3],
            indices: vec![0, 1, 99],
            edge_indices: vec![0, 1, 1, 99, 99, 0],
            values_f32: None,
            cell_values_f32: None,
        };
        let image = render_offscreen(Some(&upload), &MeshSceneState::default(), 32, 32);
        assert!(image.pixels().all(|pixel| pixel.0[3] == 0));
    }

    #[test]
    fn raster_helpers_cover_transforms_clipping_colormaps_and_lines() {
        let identity = identity_matrix();
        assert_eq!(
            transform_point(identity, [0.25, -0.5, 2.0]),
            Some([0.25, -0.5, 2.0])
        );

        let mut translated = identity;
        translated[3][0] = 2.0;
        translated[3][1] = -3.0;
        assert_eq!(
            transform_point(translated, [1.0, 2.0, 3.0]),
            Some([3.0, -1.0, 3.0])
        );

        let mut zero_w = identity;
        zero_w[3][3] = -0.0;
        assert!(transform_point(zero_w, [1.0, 2.0, 3.0]).is_some());
        let mut non_finite = identity;
        non_finite[0][0] = f32::NAN;
        assert!(transform_point(non_finite, [1.0, 2.0, 3.0]).is_none());

        assert_eq!(clipped_pixel_range([0.0, 2.0, 1.0], 4), Some((0, 2)));
        assert_eq!(clipped_pixel_range([-3.0, 1.5, 2.0], 4), Some((0, 2)));
        assert_eq!(clipped_pixel_range([4.0, 5.0, 6.0], 4), None);
        assert_eq!(clipped_pixel_range([-4.0, -3.0, -2.0], 4), None);
        assert_eq!(clipped_pixel_range([f32::NAN, 0.0, 1.0], 4), None);
        assert_eq!(clipped_pixel_range([0.0, 1.0, 2.0], 0), None);

        for map_id in 0..=3 {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let color = colormap(t, map_id);
                assert!(color.iter().any(|channel| *channel > 0));
            }
        }
        assert_ne!(colormap(0.25, 99), colormap(0.75, 99));
        assert_eq!(
            lerp_rgb([0.0, 0.5, 1.0], [1.0, 0.5, 0.0], -1.0),
            [0.0, 0.5, 1.0]
        );
        assert_eq!(
            lerp_rgb([0.0, 0.5, 1.0], [1.0, 0.5, 0.0], 2.0),
            [1.0, 0.5, 0.0]
        );

        let mut image = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        draw_line(&mut image, [-2.0, 0.0], [7.0, 7.0], [1, 2, 3, 255]);
        assert!(image.pixels().any(|pixel| pixel.0 == [1, 2, 3, 255]));

        let mut cell_upload = triangle_upload();
        cell_upload.values_f32 = None;
        cell_upload.cell_values_f32 = Some(vec![0.75]);
        let mut styled = MeshSceneState::default();
        styled.color.isoline_step = 0.5;
        styled.color.isoline_width_px = 1.0;
        styled.color.wireframe = true;
        let styled_image = render_offscreen(Some(&cell_upload), &styled, 32, 32);
        assert!(styled_image.pixels().any(|pixel| pixel.0[3] != 0));

        let mut uncolored = triangle_upload();
        uncolored.values_f32 = None;
        uncolored.cell_values_f32 = None;
        let plain_image = render_offscreen(Some(&uncolored), &MeshSceneState::default(), 16, 16);
        assert!(plain_image.pixels().any(|pixel| pixel.0[3] != 0));
        assert!(
            render_offscreen(Some(&uncolored), &MeshSceneState::default(), 0, 16)
                .pixels()
                .all(|pixel| pixel.0[3] == 0)
        );
        assert!(
            render_offscreen(Some(&uncolored), &MeshSceneState::default(), 16, 0)
                .pixels()
                .all(|pixel| pixel.0[3] == 0)
        );

        let mut non_finite_upload = uncolored;
        non_finite_upload.positions_f32[0][0] = f32::NAN;
        assert!(
            render_offscreen(Some(&non_finite_upload), &MeshSceneState::default(), 16, 16)
                .pixels()
                .all(|pixel| pixel.0[3] == 0)
        );
    }
}
