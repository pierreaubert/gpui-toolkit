use super::metal_upload_guard::interleaved_upload_fits_retained_vertices;
use super::{
    FieldRevision, GeometryRevision, MeshGpuRenderer, MeshSceneState, replace_retained_field,
};
use crate::gpu3d::Camera3D;
use crate::mesh::MeshUpload;
use crate::mesh::gpu::shaders_metal::MESH_MSL;
use crate::mesh::gpu::shaders3d::MESH_3D_MSL;
use glam::{Mat4, Vec3, Vec3Swizzles};
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_macos::{MetalCustomDraw, MetalCustomDrawAdapter};
use metal::{
    CommandBufferRef, DepthStencilDescriptor, DepthStencilState, DeviceRef, MTLCompareFunction,
    MTLLoadAction, MTLPixelFormat, MTLPrimitiveType, MTLResourceOptions, MTLScissorRect,
    MTLStorageMode, MTLStoreAction, MTLTextureUsage, MTLViewport, RenderPipelineState, Texture,
    TextureDescriptor, TextureRef,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;
use std::time::Instant;

#[repr(C)]
#[derive(Clone, Copy)]
struct MetalVertex {
    /// Keep the Rust layout identical to the MSL `float4`/`float4`/`float`/
    /// three-scalar-padding layout. A Metal `float3` member is 16-byte
    /// aligned, so the shader deliberately uses a scalar array for the final
    /// 12 bytes instead.
    position: [f32; 4],
    normal: [f32; 4],
    value: f32,
    _padding: [f32; 3],
}

const _: () = assert!(std::mem::size_of::<MetalVertex>() == 48);

fn metal_vertex(position: [f32; 3], normal: [f32; 3], value: f32) -> MetalVertex {
    MetalVertex {
        position: [position[0], position[1], position[2], 0.0],
        normal: [normal[0], normal[1], normal[2], 0.0],
        value,
        _padding: [0.0; 3],
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MetalUniform2d {
    view_transform: [[f32; 4]; 4],
    range: [f32; 4],
    style: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MetalUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_dir: [f32; 4],
    params: [f32; 4],
    value_range: [f32; 4],
    isoline: [f32; 4],
    isoline_color: [f32; 4],
}

fn metal_uniform(state: &MeshSceneState, is_3d: bool, camera: Option<&Camera3D>) -> MetalUniform {
    let origin = state
        .upload
        .as_ref()
        .map(|upload| upload.origin)
        .unwrap_or([0.0; 3]);
    let (view_proj, model) = if is_3d {
        // Keep the Metal transform in the same two-matrix form as the WGPU
        // path. The camera projection and rebased upload origin remain
        // separate so the shader performs the same model/view composition on
        // both adapters.
        (
            camera
                .map(Camera3D::view_projection_matrix)
                .unwrap_or_else(|| Mat4::from_cols_array_2d(&state.view_transform))
                .to_cols_array_2d(),
            Mat4::from_translation(Vec3::new(
                origin[0] as f32,
                origin[1] as f32,
                origin[2] as f32,
            ))
            .to_cols_array_2d(),
        )
    } else {
        (
            state.view_transform,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [origin[0] as f32, origin[1] as f32, origin[2] as f32, 1.0],
            ],
        )
    };
    let field_enabled = state
        .upload
        .as_ref()
        .is_some_and(|upload| upload.values_f32.is_some() || upload.cell_values_f32.is_some());
    MetalUniform {
        // The dedicated 3D shader consumes this complete POD layout. The 2D
        // path deliberately keeps its smaller historical ABI in
        // `metal_uniform_2d` below.
        view_proj,
        model,
        light_dir: [0.35, 0.55, 0.75, 0.0],
        params: [
            state.color.colormap as f32,
            (is_3d && state.color.unlit) as u8 as f32,
            0.3,
            0.7,
        ],
        value_range: [
            state.color.range[0],
            state.color.range[1],
            if is_3d {
                field_enabled as u32 as f32
            } else {
                state.color.colormap as f32
            },
            if is_3d { 0.0 } else { state.color.isoline_step },
        ],
        isoline: [
            state.color.isoline_step,
            state.color.isoline_width_px,
            1.0,
            0.0,
        ],
        isoline_color: [0.08, 0.10, 0.14, 1.0],
    }
}

fn metal_uniform_2d(state: &MeshSceneState) -> MetalUniform2d {
    MetalUniform2d {
        view_transform: state.view_transform,
        range: [
            state.color.range[0],
            state.color.range[1],
            state.color.colormap as f32,
            state.color.isoline_step,
        ],
        style: [
            state.color.isoline_width_px,
            state.color.wireframe as u32 as f32,
            0.0,
            0.0,
        ],
    }
}

fn vertex_normals(upload: &MeshUpload) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0; 3]; upload.positions_f32.len()];
    for triangle in upload.indices.chunks_exact(3) {
        let (Some(a), Some(b), Some(c)) = (
            upload.positions_f32.get(triangle[0] as usize),
            upload.positions_f32.get(triangle[1] as usize),
            upload.positions_f32.get(triangle[2] as usize),
        ) else {
            continue;
        };
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let face = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for &index in triangle {
            if let Some(normal) = normals.get_mut(index as usize) {
                for axis in 0..3 {
                    normal[axis] += face[axis];
                }
            }
        }
    }
    for normal in &mut normals {
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length > f32::EPSILON {
            for value in normal {
                *value /= length;
            }
        } else {
            *normal = [0.0, 0.0, 1.0];
        }
    }
    normals
}

/// Expand the scalar association to the same per-triangle-vertex order used
/// by the Metal draw. 3D values live in their own buffer so a field-only
/// update never writes the retained position/normal vertex buffer.
fn metal_field_values(upload: &MeshUpload) -> Vec<f32> {
    upload
        .indices
        .chunks_exact(3)
        .enumerate()
        .flat_map(|(cell, triangle)| {
            let cell_value = upload
                .cell_values_f32
                .as_ref()
                .and_then(|values| values.get(cell).copied());
            triangle.iter().map(move |&index| {
                cell_value
                    .or_else(|| {
                        upload
                            .values_f32
                            .as_ref()
                            .and_then(|values| values.get(index as usize).copied())
                    })
                    .unwrap_or(0.5)
            })
        })
        .collect()
}

/// Small camera-oriented orientation triad rendered in NDC after the mesh.
/// `normal` carries the RGB axis color for the dedicated triad fragment shader.
fn metal_triad_vertices(camera: Option<&Camera3D>) -> [MetalVertex; 6] {
    let origin = [-0.84_f32, -0.84_f32];
    let view = camera.map_or(glam::Mat4::IDENTITY, Camera3D::view_matrix);
    let axes = [
        (Vec3::X, [0.90, 0.16, 0.14]),
        (Vec3::Y, [0.18, 0.82, 0.28]),
        (Vec3::Z, [0.20, 0.42, 0.95]),
    ];
    let mut output = [metal_vertex([0.0; 3], [0.0; 3], 0.0); 6];
    for (axis, (direction, color)) in axes.into_iter().enumerate() {
        let screen = (view * direction.extend(0.0)).truncate();
        let length = screen.xy().length().max(1e-6);
        let screen = screen.xy() / length;
        let end = [
            origin[0] + screen.x * 0.12,
            // The triad vertices are already in NDC. Metal and WGPU both
            // use the same upward-positive NDC Y convention; the drawable
            // viewport mapping must not be applied a second time here.
            origin[1] + screen.y * 0.12,
            0.0,
        ];
        output[axis * 2] = metal_vertex([origin[0], origin[1], 0.0], color, 1.0);
        output[axis * 2 + 1] = metal_vertex(end, color, 1.0);
    }
    output
}

struct MetalResources {
    geometry_rev: GeometryRevision,
    field_rev: FieldRevision,
    vertices: metal::Buffer,
    /// 3D scalar values are separate from positions/normals. The 2D shader
    /// retains its historical interleaved value ABI and leaves this unset.
    values: Option<metal::Buffer>,
    lines: metal::Buffer,
    uniform: metal::Buffer,
    pipeline: RenderPipelineState,
    line_pipeline: RenderPipelineState,
    triad: Option<metal::Buffer>,
    triad_pipeline: Option<RenderPipelineState>,
    depth: Texture,
    depth_state: DepthStencilState,
    triad_depth_state: Option<DepthStencilState>,
    vertex_count: usize,
    value_count: usize,
    line_count: usize,
    triad_count: usize,
    target_width: u64,
    target_height: u64,
    is_3d: bool,
    geometry_bytes: u64,
    field_capacity_bytes: u64,
    resident_bytes: u64,
}

impl MetalResources {
    fn new(
        device: &DeviceRef,
        texture: &TextureRef,
        revision: GeometryRevision,
        upload: &MeshUpload,
        state: &MeshSceneState,
        is_3d: bool,
        camera: Option<&Camera3D>,
    ) -> Option<Self> {
        let normals = vertex_normals(upload);
        let mut vertices = Vec::with_capacity(upload.indices.len());
        for (cell, triangle) in upload.indices.chunks_exact(3).enumerate() {
            for &index in triangle {
                let position = *upload.positions_f32.get(index as usize)?;
                vertices.push(metal_vertex(
                    position,
                    normals
                        .get(index as usize)
                        .copied()
                        .unwrap_or([0.0, 0.0, 1.0]),
                    // 2D keeps using this interleaved value. The 3D shader
                    // reads its value from the dedicated buffer below.
                    if is_3d {
                        0.0
                    } else {
                        upload
                            .cell_values_f32
                            .as_ref()
                            .and_then(|values| values.get(cell).copied())
                            .or_else(|| {
                                upload
                                    .values_f32
                                    .as_ref()
                                    .and_then(|values| values.get(index as usize).copied())
                            })
                            .unwrap_or(0.5)
                    },
                ));
            }
        }
        let mut lines = Vec::with_capacity(upload.edge_indices.len());
        for &index in &upload.edge_indices {
            lines.push(metal_vertex(
                *upload.positions_f32.get(index as usize)?,
                [0.0, 0.0, 1.0],
                0.0,
            ));
        }
        let options = MTLResourceOptions::StorageModeShared;
        let vertex_buffer = device.new_buffer_with_data(
            vertices.as_ptr() as *const c_void,
            (vertices.len() * std::mem::size_of::<MetalVertex>()) as u64,
            options,
        );
        let field_values = is_3d.then(|| metal_field_values(upload));
        let value_buffer = field_values.as_ref().map(|values| {
            device.new_buffer_with_data(
                values.as_ptr() as *const c_void,
                (values.len() * std::mem::size_of::<f32>()).max(4) as u64,
                options,
            )
        });
        let line_buffer = device.new_buffer_with_data(
            lines.as_ptr() as *const c_void,
            (lines.len() * std::mem::size_of::<MetalVertex>()) as u64,
            options,
        );
        let uniform_buffer = if is_3d {
            let uniform = metal_uniform(state, true, camera);
            device.new_buffer_with_data(
                &uniform as *const MetalUniform as *const c_void,
                std::mem::size_of::<MetalUniform>() as u64,
                options,
            )
        } else {
            let uniform = metal_uniform_2d(state);
            device.new_buffer_with_data(
                &uniform as *const MetalUniform2d as *const c_void,
                std::mem::size_of::<MetalUniform2d>() as u64,
                options,
            )
        };
        let library = device
            .new_library_with_source(
                if is_3d { MESH_3D_MSL } else { MESH_MSL },
                &metal::CompileOptions::new(),
            )
            .ok()?;
        let vertex = library
            .get_function(if is_3d { "vs_main" } else { "mesh_vertex" }, None)
            .ok()?;
        let fragment = library
            .get_function(if is_3d { "fs_main" } else { "mesh_fragment" }, None)
            .ok()?;
        let line_fragment = library
            .get_function(
                if is_3d {
                    "fs_wireframe"
                } else {
                    "mesh_line_fragment"
                },
                None,
            )
            .ok()?;
        let make_pipeline =
            |label: &str, vertex: &metal::FunctionRef, fragment: &metal::FunctionRef| {
                let descriptor = metal::RenderPipelineDescriptor::new();
                descriptor.set_label(label);
                descriptor.set_vertex_function(Some(vertex));
                descriptor.set_fragment_function(Some(fragment));
                descriptor
                    .color_attachments()
                    .object_at(0)?
                    .set_pixel_format(texture.pixel_format());
                descriptor.set_depth_attachment_pixel_format(MTLPixelFormat::Depth32Float);
                device.new_render_pipeline_state(&descriptor).ok()
            };
        let depth_descriptor = TextureDescriptor::new();
        depth_descriptor.set_pixel_format(MTLPixelFormat::Depth32Float);
        depth_descriptor.set_width(texture.width().max(1));
        depth_descriptor.set_height(texture.height().max(1));
        depth_descriptor.set_storage_mode(MTLStorageMode::Private);
        depth_descriptor.set_usage(MTLTextureUsage::RenderTarget);
        let depth = device.new_texture(&depth_descriptor);
        let depth_state_descriptor = DepthStencilDescriptor::new();
        depth_state_descriptor.set_depth_compare_function(MTLCompareFunction::LessEqual);
        depth_state_descriptor.set_depth_write_enabled(true);
        let depth_state = device.new_depth_stencil_state(&depth_state_descriptor);
        let (triad, triad_pipeline, triad_depth_state, triad_count) = if is_3d {
            let triad_vertices = metal_triad_vertices(None);
            let triad = device.new_buffer_with_data(
                triad_vertices.as_ptr() as *const c_void,
                (triad_vertices.len() * std::mem::size_of::<MetalVertex>()) as u64,
                options,
            );
            let triad_vertex = library.get_function("vs_triad", None).ok()?;
            let triad_fragment = library.get_function("fs_triad", None).ok()?;
            let triad_pipeline = make_pipeline(
                "mesh_metal_triad",
                triad_vertex.as_ref(),
                triad_fragment.as_ref(),
            )?;
            let descriptor = DepthStencilDescriptor::new();
            descriptor.set_depth_compare_function(MTLCompareFunction::Always);
            descriptor.set_depth_write_enabled(false);
            (
                Some(triad),
                Some(triad_pipeline),
                Some(device.new_depth_stencil_state(&descriptor)),
                triad_vertices.len(),
            )
        } else {
            (None, None, None, 0)
        };
        let vertex_bytes = (vertices.len() * std::mem::size_of::<MetalVertex>()) as u64;
        let line_bytes = (lines.len() * std::mem::size_of::<MetalVertex>()) as u64;
        let uniform_bytes = if is_3d {
            std::mem::size_of::<MetalUniform>() as u64
        } else {
            std::mem::size_of::<MetalUniform2d>() as u64
        };
        let triad_bytes = if is_3d {
            (triad_count * std::mem::size_of::<MetalVertex>()) as u64
        } else {
            0
        };
        let value_bytes = field_values.as_ref().map_or(0, |values| {
            (values.len() * std::mem::size_of::<f32>()).max(4) as u64
        });
        let depth_bytes = texture
            .width()
            .saturating_mul(texture.height())
            .saturating_mul(4);
        Some(Self {
            geometry_rev: revision,
            field_rev: FieldRevision::default(),
            vertices: vertex_buffer,
            values: value_buffer,
            lines: line_buffer,
            uniform: uniform_buffer,
            pipeline: make_pipeline("mesh_metal_fill", vertex.as_ref(), fragment.as_ref())?,
            line_pipeline: make_pipeline(
                "mesh_metal_lines",
                vertex.as_ref(),
                line_fragment.as_ref(),
            )?,
            triad,
            triad_pipeline,
            depth,
            depth_state,
            triad_depth_state,
            vertex_count: vertices.len(),
            value_count: field_values.as_ref().map_or(vertices.len(), Vec::len),
            line_count: lines.len(),
            triad_count,
            target_width: texture.width(),
            target_height: texture.height(),
            is_3d,
            geometry_bytes: vertex_bytes.saturating_add(line_bytes),
            field_capacity_bytes: if is_3d {
                value_bytes
            } else {
                (vertices.len() * std::mem::size_of::<f32>()) as u64
            },
            resident_bytes: vertex_bytes
                .saturating_add(line_bytes)
                .saturating_add(value_bytes)
                .saturating_add(uniform_bytes)
                .saturating_add(triad_bytes)
                .saturating_add(depth_bytes),
        })
    }

    fn update_values(&mut self, upload: &MeshUpload) -> u64 {
        if self.is_3d {
            let Some(values_buffer) = self.values.as_ref() else {
                return 0;
            };
            let values = metal_field_values(upload);
            if values.len() > self.value_count {
                return 0;
            }
            // SAFETY: the shared scalar buffer is allocated for at least
            // `value_count` f32 values, and the copied slice is bounded by
            // that capacity.
            unsafe {
                ptr::copy_nonoverlapping(
                    values.as_ptr() as *const u8,
                    values_buffer.contents() as *mut u8,
                    std::mem::size_of_val(values.as_slice()),
                );
            }
            return std::mem::size_of_val(values.as_slice()) as u64;
        }

        // Geometry resources are retained across scalar-field writes, but a
        // caller can replace the retained upload independently. Never let an
        // interleaved update address beyond the vertex buffer that was
        // allocated for this resource generation.
        if !interleaved_upload_fits_retained_vertices(upload.indices.len(), self.vertex_count) {
            return 0;
        }

        let mut offset = 0usize;
        let contents = self.vertices.contents() as *mut MetalVertex;
        for (cell, triangle) in upload.indices.chunks_exact(3).enumerate() {
            let cell_value = upload
                .cell_values_f32
                .as_ref()
                .and_then(|values| values.get(cell).copied());
            for &index in triangle {
                let value = cell_value
                    .or_else(|| {
                        upload
                            .values_f32
                            .as_ref()
                            .and_then(|values| values.get(index as usize).copied())
                    })
                    .unwrap_or(0.5);
                // The buffer was created with shared storage, so scalar-only
                // patches update values without rebuilding geometry or state.
                // SAFETY: `contents` belongs to the shared Metal buffer created
                // for this renderer, `offset` is bounded by the uploaded index
                // count, and `MetalVertex` matches the buffer's vertex layout.
                unsafe {
                    (*contents.add(offset)).value = value;
                }
                offset += 1;
            }
        }
        (offset * std::mem::size_of::<f32>()) as u64
    }

    fn update_uniform(&mut self, state: &MeshSceneState, camera: Option<&Camera3D>) {
        let uniform = metal_uniform(state, self.is_3d, camera);
        // SAFETY: `uniform` is a fully initialized POD value, the destination
        // is the shared uniform buffer allocated for this renderer, and the
        // copy length is exactly the `MetalUniform` layout size.
        unsafe {
            if self.is_3d {
                ptr::copy_nonoverlapping(
                    &uniform as *const MetalUniform as *const u8,
                    self.uniform.contents() as *mut u8,
                    std::mem::size_of::<MetalUniform>(),
                );
            } else {
                let uniform = metal_uniform_2d(state);
                ptr::copy_nonoverlapping(
                    &uniform as *const MetalUniform2d as *const u8,
                    self.uniform.contents() as *mut u8,
                    std::mem::size_of::<MetalUniform2d>(),
                );
            }
        }
    }

    fn update_triad(&mut self, camera: Option<&Camera3D>) {
        let Some(triad) = &self.triad else {
            return;
        };
        let vertices = metal_triad_vertices(camera);
        // SAFETY: the shared buffer was allocated exactly for six
        // `MetalVertex` entries and is updated with an identically sized POD
        // array on the render thread.
        unsafe {
            ptr::copy_nonoverlapping(
                vertices.as_ptr() as *const u8,
                triad.contents() as *mut u8,
                std::mem::size_of_val(&vertices),
            );
        }
    }
}

pub struct MetalMeshRenderer {
    state: Rc<RefCell<MeshSceneState>>,
    resources: Rc<RefCell<Option<MetalResources>>>,
    custom_id: gpui::CustomDrawId,
}

struct MetalMeshDraw {
    state: Rc<RefCell<MeshSceneState>>,
    resources: Rc<RefCell<Option<MetalResources>>>,
    is_3d: bool,
    camera: Option<Rc<RefCell<Camera3D>>>,
}

impl CustomDraw for MetalMeshDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MetalCustomDraw for MetalMeshDraw {
    fn draw_metal(
        &self,
        device: &DeviceRef,
        command_buffer: &CommandBufferRef,
        drawable_texture: &TextureRef,
        bounds: Bounds<Pixels>,
        scale_factor: f32,
    ) {
        let frame_started = Instant::now();
        let state = self.state.borrow();
        let Some(upload) = state.upload.as_ref() else {
            return;
        };
        let camera = self.camera.as_ref().map(|camera| camera.borrow().clone());
        let mut resources = self.resources.borrow_mut();
        let mut created_geometry_resource = false;
        let mut geometry_upload_time = None;
        if resources.as_ref().is_none_or(|resources| {
            resources.geometry_rev != state.geometry_rev
                || resources.target_width != drawable_texture.width()
                || resources.target_height != drawable_texture.height()
        }) {
            let geometry_started = Instant::now();
            *resources = MetalResources::new(
                device,
                drawable_texture,
                state.geometry_rev,
                upload,
                &state,
                self.is_3d,
                camera.as_ref(),
            );
            geometry_upload_time = Some(geometry_started.elapsed());
            created_geometry_resource = true;
        }
        let Some(resources) = resources.as_mut() else {
            return;
        };
        let mut field_write_bytes = 0;
        let field_write_started = Instant::now();
        if resources.field_rev != state.field_rev {
            field_write_bytes = resources.update_values(upload);
            resources.field_rev = state.field_rev;
        }
        let field_write_time = field_write_started.elapsed();
        let resident_bytes = resources.resident_bytes;
        let field_capacity_bytes = resources.field_capacity_bytes;
        let driver_allocated_bytes = device.current_allocated_size();
        drop(state);
        {
            let mut state = self.state.borrow_mut();
            if created_geometry_resource {
                state.record_gpu_geometry_upload(resources.geometry_bytes);
                if let Some(elapsed) = geometry_upload_time {
                    state.record_gpu_geometry_upload_time(elapsed);
                }
            }
            if field_write_bytes != 0 {
                state.record_gpu_field_write(field_write_bytes);
                state.record_gpu_field_write_time(field_write_time);
            }
            state.set_gpu_memory(resident_bytes, field_capacity_bytes);
            state.set_gpu_driver_memory(driver_allocated_bytes);
        }
        let state = self.state.borrow();
        resources.update_uniform(&state, camera.as_ref());
        resources.update_triad(camera.as_ref());
        let descriptor = metal::RenderPassDescriptor::new();
        let Some(attachment) = descriptor.color_attachments().object_at(0) else {
            // A malformed descriptor must not take down the render thread.
            return;
        };
        attachment.set_texture(Some(drawable_texture));
        attachment.set_load_action(MTLLoadAction::Load);
        attachment.set_store_action(MTLStoreAction::Store);
        let Some(depth_attachment) = descriptor.depth_attachment() else {
            return;
        };
        depth_attachment.set_texture(Some(&resources.depth));
        depth_attachment.set_load_action(MTLLoadAction::Clear);
        depth_attachment.set_clear_depth(1.0);
        depth_attachment.set_store_action(MTLStoreAction::DontCare);
        let encoder = command_buffer.new_render_command_encoder(descriptor);
        let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let target_width = drawable_texture.width() as f32;
        let target_height = drawable_texture.height() as f32;
        let left = (f32::from(bounds.origin.x) * scale)
            .floor()
            .clamp(0.0, target_width);
        let top = (f32::from(bounds.origin.y) * scale)
            .floor()
            .clamp(0.0, target_height);
        let right = ((f32::from(bounds.origin.x) + f32::from(bounds.size.width)) * scale)
            .ceil()
            .clamp(left, target_width);
        let bottom = ((f32::from(bounds.origin.y) + f32::from(bounds.size.height)) * scale)
            .ceil()
            .clamp(top, target_height);
        let viewport_width = (right - left).max(1.0);
        let viewport_height = (bottom - top).max(1.0);
        encoder.set_viewport(MTLViewport {
            originX: left as f64,
            originY: top as f64,
            width: viewport_width as f64,
            height: viewport_height as f64,
            znear: 0.0,
            zfar: 1.0,
        });
        let rect = MTLScissorRect {
            x: left as u64,
            y: top as u64,
            width: viewport_width as u64,
            height: viewport_height as u64,
        };
        encoder.set_scissor_rect(rect);
        encoder.set_depth_stencil_state(&resources.depth_state);
        encoder.set_render_pipeline_state(&resources.pipeline);
        encoder.set_vertex_buffer(0, Some(&resources.vertices), 0);
        encoder.set_vertex_buffer(1, Some(&resources.uniform), 0);
        if self.is_3d {
            encoder.set_vertex_buffer(2, resources.values.as_deref(), 0);
        }
        // Vertex and fragment stages have independent bindings. The old 2D
        // path used the same buffer layout; bind it explicitly so 3D scalar
        // coloring/lighting never reads an unbound fragment uniform.
        encoder.set_fragment_buffer(if self.is_3d { 0 } else { 1 }, Some(&resources.uniform), 0);
        if resources.vertex_count != 0 {
            encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, resources.vertex_count as u64);
        }
        if state.color.wireframe && resources.line_count != 0 {
            encoder.set_render_pipeline_state(&resources.line_pipeline);
            encoder.set_vertex_buffer(0, Some(&resources.lines), 0);
            encoder.draw_primitives(MTLPrimitiveType::Line, 0, resources.line_count as u64);
        }
        if let (Some(pipeline), Some(triad), Some(depth_state)) = (
            resources.triad_pipeline.as_ref(),
            resources.triad.as_ref(),
            resources.triad_depth_state.as_ref(),
        ) && resources.triad_count != 0
        {
            encoder.set_depth_stencil_state(depth_state);
            encoder.set_render_pipeline_state(pipeline);
            encoder.set_vertex_buffer(0, Some(triad), 0);
            encoder.draw_primitives(MTLPrimitiveType::Line, 0, resources.triad_count as u64);
        }
        encoder.end_encoding();
        drop(state);
        self.state
            .borrow_mut()
            .record_gpu_frame_time(frame_started.elapsed());
    }
}

impl MetalMeshRenderer {
    pub fn new(state: Rc<RefCell<MeshSceneState>>) -> Self {
        Self::with_dimension(state, false, None)
    }

    /// Construct the dedicated normal-bearing, lit Metal renderer used for
    /// `Surface3d` and axisymmetric revolve plots. The regular constructor
    /// remains the 2D ABI-compatible scalar renderer.
    pub fn new_3d(state: Rc<RefCell<MeshSceneState>>) -> Self {
        Self::with_dimension(state, true, None)
    }

    /// Construct the 3D renderer with the retained orbit camera used to keep
    /// its orientation triad synchronized with navigation.
    pub fn new_3d_with_camera(
        state: Rc<RefCell<MeshSceneState>>,
        camera: Rc<RefCell<Camera3D>>,
    ) -> Self {
        Self::with_dimension(state, true, Some(camera))
    }

    fn with_dimension(
        state: Rc<RefCell<MeshSceneState>>,
        is_3d: bool,
        camera: Option<Rc<RefCell<Camera3D>>>,
    ) -> Self {
        let resources = Rc::new(RefCell::new(None));
        let draw: Rc<dyn CustomDraw> = Rc::new(MetalCustomDrawAdapter(Rc::new(MetalMeshDraw {
            state: state.clone(),
            resources: resources.clone(),
            is_3d,
            camera,
        })));
        let custom_id = gpui::register_custom_draw(draw);
        Self {
            state,
            resources,
            custom_id,
        }
    }

    pub fn custom_id(&self) -> gpui::CustomDrawId {
        self.custom_id
    }

    pub fn state(&self) -> Rc<RefCell<MeshSceneState>> {
        self.state.clone()
    }
}

impl Drop for MetalMeshRenderer {
    fn drop(&mut self) {
        gpui::unregister_custom_draw(self.custom_id);
        self.resources.borrow_mut().take();
        self.state.borrow_mut().clear_gpu_memory();
    }
}

impl MeshGpuRenderer for MetalMeshRenderer {
    fn upload_geometry(&mut self, rev: GeometryRevision, upload: &MeshUpload) {
        let mut state = self.state.borrow_mut();
        state.record_geometry_upload(upload);
        state.geometry_rev = rev;
        state.upload = Some(upload.clone());
        self.resources.borrow_mut().take();
    }

    fn write_field(&mut self, rev: FieldRevision, values: &[f32]) {
        let mut state = self.state.borrow_mut();
        state.record_field_write(values);
        state.field_rev = rev;
        if let Some(upload) = &mut state.upload {
            if upload.cell_values_f32.is_some() {
                replace_retained_field(&mut upload.cell_values_f32, values);
            } else {
                replace_retained_field(&mut upload.values_f32, values);
            }
        }
    }

    fn geometry_revision(&self) -> Option<GeometryRevision> {
        self.state
            .borrow()
            .upload
            .as_ref()
            .map(|_| self.state.borrow().geometry_rev)
    }
}
