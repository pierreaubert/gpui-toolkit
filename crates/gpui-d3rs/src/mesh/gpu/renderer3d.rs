//! Cache-aware 3D mesh renderer facade.
//!
//! The platform renderer owns GPU resources; this type owns the state that
//! must remain stable while a camera is moving.  Keeping that state on top of
//! [`MeshGpuRenderer`] makes the camera contract testable without requiring a
//! graphics adapter and lets the wgpu/Metal implementations share it.

use crate::gpu3d::Camera3D;
#[cfg(not(test))]
use crate::mesh::gpu::GpuTimestampRecorder;
use crate::mesh::gpu::{
    FieldRevision, GeometryRevision, MeshColorConfig, MeshGpuRenderer, RetainedMeshRenderer,
};
#[cfg(not(test))]
use crate::mesh::upload_chunks;
use crate::mesh::{MeshUpload, expand_cell_shading};
#[cfg(not(test))]
use glam::Vec3Swizzles;
#[cfg(not(test))]
use std::borrow::Cow;
#[cfg(not(test))]
use std::time::Instant;

/// Retained 3D mesh state shared by platform renderers.
///
/// `B` is deliberately generic so headless tests can use
/// [`RetainedMeshRenderer`], while a platform can provide its own
/// [`MeshGpuRenderer`] without duplicating revision and camera-cache logic.
#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Clone)]
pub struct Mesh3DRenderer<B = RetainedMeshRenderer> {
    backend: B,
    camera: Camera3D,
    color: MeshColorConfig,
    cell_shading: bool,
    upload: Option<MeshUpload>,
    field_revision: Option<FieldRevision>,
    upload_count: u64,
    upload_bytes: u64,
}

#[cfg_attr(test, allow(dead_code))]
impl<B: MeshGpuRenderer> Mesh3DRenderer<B> {
    /// Wrap an existing platform renderer.
    pub fn with_backend(backend: B) -> Self {
        Self {
            backend,
            camera: Camera3D::default(),
            color: MeshColorConfig::default(),
            cell_shading: false,
            upload: None,
            field_revision: None,
            upload_count: 0,
            upload_bytes: 0,
        }
    }

    /// Access the platform renderer for draw submission or diagnostics.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Mutable access to the platform renderer.
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Return the current camera without causing a geometry upload.
    pub fn camera(&self) -> &Camera3D {
        &self.camera
    }

    /// Update the camera cache.
    ///
    /// This intentionally only changes uniforms/state. Geometry upload is
    /// driven by [`GeometryRevision`], not by camera frames.
    pub fn set_camera(&mut self, camera: &Camera3D) {
        self.camera = camera.clone();
    }

    /// Return the color/lighting configuration consumed by the shader.
    pub fn color_config(&self) -> &MeshColorConfig {
        &self.color
    }

    /// Set the lighting and colormap configuration without touching geometry.
    pub fn set_color_config(&mut self, color: MeshColorConfig) {
        self.color = color;
    }

    /// Enable or disable lighting.
    ///
    /// Scalar views default to unlit so lighting cannot change the scalar to
    /// colormap mapping.
    pub fn set_unlit(&mut self, unlit: bool) {
        self.color.unlit = unlit;
    }

    /// Enable or disable per-cell flat shading.
    ///
    /// Changing this flag changes the vertex representation, so an existing
    /// geometry upload is rebuilt once using the same geometry revision.
    pub fn set_cell_shading(&mut self, enabled: bool) {
        if self.cell_shading == enabled {
            return;
        }
        self.cell_shading = enabled;
        if let (Some(upload), Some(revision)) =
            (self.upload.clone(), self.backend.geometry_revision())
        {
            self.upload_geometry_inner(revision, &upload, true);
        }
    }

    /// Whether per-cell flat shading is active.
    pub fn cell_shading(&self) -> bool {
        self.cell_shading
    }

    /// Number of geometry uploads performed by this cache.
    ///
    /// Field writes and camera changes do not increment this counter.
    pub fn upload_count(&self) -> u64 {
        self.upload_count
    }

    /// Number of retained geometry payload bytes uploaded by this cache.
    pub fn upload_bytes(&self) -> u64 {
        self.upload_bytes
    }

    /// Last field revision written through this facade.
    pub fn field_revision(&self) -> Option<FieldRevision> {
        self.field_revision
    }

    /// Upload the representation selected by the current shading mode.
    fn upload_for_mode(&self, upload: &MeshUpload) -> MeshUpload {
        if self.cell_shading {
            expand_cell_shading(upload)
        } else {
            upload.clone()
        }
    }

    fn upload_geometry_inner(
        &mut self,
        revision: GeometryRevision,
        upload: &MeshUpload,
        force: bool,
    ) {
        // Geometry preparation is also asynchronous. Never let a late lower
        // revision replace the scene that is already visible.
        if self
            .backend
            .geometry_revision()
            .is_some_and(|current| revision.0 < current.0)
        {
            return;
        }
        // Revisions are the cache key. A camera frame or a repeated render of
        // the same revision must not recreate GPU buffers unless the vertex
        // representation itself changed (for example, flat shading toggled).
        if !force && self.backend.geometry_revision() == Some(revision) {
            self.upload = Some(upload.clone());
            return;
        }

        let prepared = self.upload_for_mode(upload);
        self.backend.upload_geometry(revision, &prepared);
        self.upload = Some(upload.clone());
        self.upload_count += 1;
        self.upload_bytes = self
            .upload_bytes
            .saturating_add(prepared.geometry_byte_len());
    }
}

impl Mesh3DRenderer<RetainedMeshRenderer> {
    /// Construct a headless renderer useful for tests and fallback paths.
    pub fn new() -> Self {
        Self::with_backend(RetainedMeshRenderer::default())
    }
}

impl Default for Mesh3DRenderer<RetainedMeshRenderer> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: MeshGpuRenderer> MeshGpuRenderer for Mesh3DRenderer<B> {
    fn upload_geometry(&mut self, revision: GeometryRevision, upload: &MeshUpload) {
        self.upload_geometry_inner(revision, upload, false);
    }

    fn write_field(&mut self, revision: FieldRevision, values: &[f32]) {
        // Background preparation can finish out of order. A late lower
        // revision must not overwrite the field already visible in the
        // retained scene or trigger another adapter write.
        if self
            .field_revision
            .is_some_and(|current| revision.0 < current.0)
        {
            return;
        }
        self.backend.write_field(revision, values);
        self.field_revision = Some(revision);
    }

    fn geometry_revision(&self) -> Option<GeometryRevision> {
        self.backend.geometry_revision()
    }
}

/// Adapter-backed retained surface renderer used by `MeshPlotView::Surface3d`.
///
/// The renderer is intentionally separate from the headless facade above:
/// `Mesh3DRenderer` owns revision policy, while this type owns the WGPU
/// pipeline, depth targets, and GPUI custom-draw registration. Camera
/// changes only update the uniform buffer; indexed geometry is rebuilt only
/// when its `GeometryRevision` changes.
#[cfg(not(test))]
pub struct WgpuMesh3DRenderer {
    state: std::rc::Rc<std::cell::RefCell<crate::mesh::gpu::MeshSceneState>>,
    camera: std::rc::Rc<std::cell::RefCell<Camera3D>>,
    resources: std::rc::Rc<std::cell::RefCell<Option<WgpuMesh3DResources>>>,
    custom_id: gpui::CustomDrawId,
}

#[cfg(not(test))]
struct WgpuMesh3DResources {
    geometry_rev: GeometryRevision,
    field_rev: FieldRevision,
    vertices: wgpu::Buffer,
    values: wgpu::Buffer,
    indices: wgpu::Buffer,
    edges: wgpu::Buffer,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    surface_pipeline: wgpu::RenderPipeline,
    wire_pipeline: wgpu::RenderPipeline,
    triad_pipeline: wgpu::RenderPipeline,
    triad: wgpu::Buffer,
    index_count: u32,
    edge_count: u32,
    triad_count: u32,
    value_bytes: u64,
    value_count: usize,
    value_is_cell: bool,
    geometry_bytes: u64,
    field_capacity_bytes: u64,
    resident_bytes: u64,
    depth_bytes: u64,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
    #[cfg(not(test))]
    timestamp: Option<GpuTimestampRecorder>,
}

/// Convert a GPUI chart rectangle to a physical WGPU viewport, clipped to the
/// full resolve target. A custom draw is embedded in GPUI's frame rather than
/// owning that frame, so its layout bounds may be partially outside the target
/// during scroll, clipping, or resize.
fn clipped_target_viewport(
    origin: [f32; 2],
    size: [f32; 2],
    scale_factor: f32,
    target_size: [u32; 2],
) -> Option<[u32; 4]> {
    let scale = scale_factor.max(0.0);
    let left = (origin[0] * scale).floor().max(0.0);
    let top = (origin[1] * scale).floor().max(0.0);
    let right = ((origin[0] + size[0]) * scale)
        .ceil()
        .min(target_size[0] as f32);
    let bottom = ((origin[1] + size[1]) * scale)
        .ceil()
        .min(target_size[1] as f32);
    if right <= left || bottom <= top {
        return None;
    }
    Some([
        left as u32,
        top as u32,
        (right - left) as u32,
        (bottom - top) as u32,
    ])
}

#[cfg(not(test))]
struct WgpuMesh3DDraw {
    state: std::rc::Rc<std::cell::RefCell<crate::mesh::gpu::MeshSceneState>>,
    camera: std::rc::Rc<std::cell::RefCell<Camera3D>>,
    resources: std::rc::Rc<std::cell::RefCell<Option<WgpuMesh3DResources>>>,
}

#[cfg(not(test))]
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Mesh3DVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mesh3DFieldLayout {
    value_count: usize,
    is_cell: bool,
}

fn mesh_field_layout(upload: &MeshUpload) -> Mesh3DFieldLayout {
    Mesh3DFieldLayout {
        value_count: upload.cell_values_f32.as_ref().map_or_else(
            || upload.values_f32.as_ref().map_or(0, Vec::len),
            |values| values.len().saturating_mul(3).min(upload.indices.len()),
        ),
        is_cell: upload.cell_values_f32.is_some(),
    }
}

#[cfg(not(test))]
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Mesh3DUniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_dir: [f32; 4],
    params: [f32; 4],
    value_range: [f32; 4],
    isoline: [f32; 4],
    isoline_color: [f32; 4],
}

#[cfg(not(test))]
impl WgpuMesh3DResources {
    fn new(
        ctx: &gpui_wgpu::WgpuContext,
        state: &crate::mesh::gpu::MeshSceneState,
        revision: GeometryRevision,
    ) -> Self {
        let empty_upload = MeshUpload {
            positions_f32: Vec::new(),
            origin: [0.0; 3],
            indices: Vec::new(),
            edge_indices: Vec::new(),
            values_f32: None,
            cell_values_f32: None,
        };
        let upload = state.upload.as_ref().unwrap_or(&empty_upload);
        // Cell-associated values cannot be represented by shared indexed
        // vertices: one profile vertex may belong to cells with different
        // values. Expand those triangles once per geometry revision so flat
        // cell shading is correct on the retained WGPU path.
        let render_upload = expand_cell_upload(upload);
        let vertices = build_3d_vertices(&render_upload, state.vertex_colors.as_deref());
        let field_values = mesh_field_values(&render_upload);
        let field_layout = mesh_field_layout(upload);
        let triad_vertices = triad_vertices(&Camera3D::default());
        let device = &ctx.device;
        let vertex_bytes = bytemuck::cast_slice::<Mesh3DVertex, u8>(&vertices);
        let vertex_buffer_bytes = (vertex_bytes.len() as u64).max(4);
        let value_bytes = std::mem::size_of_val(field_values.as_ref()) as u64;
        let value_buffer_bytes = value_bytes.max(4);
        let index_buffer_bytes =
            (render_upload.indices.len() * std::mem::size_of::<u32>()).max(4) as u64;
        let edge_buffer_bytes =
            (render_upload.edge_indices.len() * std::mem::size_of::<u32>()).max(4) as u64;
        let triad_buffer_bytes =
            (triad_vertices.len() * std::mem::size_of::<Mesh3DVertex>()).max(4) as u64;
        let uniform_buffer_bytes = std::mem::size_of::<Mesh3DUniforms>() as u64;
        let depth_bytes = 4;
        let vertex_buffer = create_chunked_buffer(
            ctx,
            "mesh_3d_vertices",
            vertex_bytes,
            wgpu::BufferUsages::VERTEX,
        );
        let value_buffer = create_chunked_buffer(
            ctx,
            "mesh_3d_values",
            bytemuck::cast_slice(field_values.as_ref()),
            wgpu::BufferUsages::STORAGE,
        );
        let index_buffer = create_chunked_buffer(
            ctx,
            "mesh_3d_triangles",
            bytemuck::cast_slice(&render_upload.indices),
            wgpu::BufferUsages::INDEX,
        );
        let edge_buffer = create_chunked_buffer(
            ctx,
            "mesh_3d_edges",
            bytemuck::cast_slice(&render_upload.edge_indices),
            wgpu::BufferUsages::INDEX,
        );
        let triad = create_chunked_buffer(
            ctx,
            "mesh_3d_orientation_triad",
            bytemuck::cast_slice(&triad_vertices),
            wgpu::BufferUsages::VERTEX,
        );
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_3d_uniforms"),
            size: std::mem::size_of::<Mesh3DUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh_3d_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh_3d_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: value_buffer.as_entire_binding(),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh_3d_shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders3d::wgsl().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh_3d_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Mesh3DVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 24,
                    shader_location: 2,
                },
            ],
        };
        let pipeline = |topology, depth_bias: wgpu::DepthBiasState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mesh_3d_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: std::slice::from_ref(&vertex_layout),
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    // Scientific meshes may have inconsistent winding. Keep
                    // both sides visible and let the shader use two-sided light.
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: depth_bias,
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if topology == wgpu::PrimitiveTopology::LineList {
                        "fs_wireframe"
                    } else {
                        "fs_main"
                    }),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.color_texture_format(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                cache: None,
                multiview_mask: None,
            })
        };
        let surface_pipeline = pipeline(wgpu::PrimitiveTopology::TriangleList, Default::default());
        let wire_pipeline = pipeline(
            wgpu::PrimitiveTopology::LineList,
            // WGPU rejects depth bias for line topology. The depth comparison
            // remains LessEqual so mesh edges are still visible on coplanar
            // surfaces without creating an invalid adapter pipeline.
            Default::default(),
        );
        let triad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh_3d_orientation_triad_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_triad"),
                buffers: std::slice::from_ref(&vertex_layout),
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            // This is an overlay, but it shares the surface pass so its
            // pipeline must declare the same depth format. `Always` with
            // writes disabled keeps the triad visible without changing the
            // surface depth buffer.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_triad"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.color_texture_format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            cache: None,
            multiview_mask: None,
        });
        let (depth_texture, depth_view) = make_targets(ctx, 1, 1);
        Self {
            geometry_rev: revision,
            field_rev: FieldRevision(0),
            vertices: vertex_buffer,
            values: value_buffer,
            indices: index_buffer,
            edges: edge_buffer,
            uniform,
            bind_group,
            surface_pipeline,
            wire_pipeline,
            triad_pipeline,
            triad,
            index_count: render_upload.indices.len() as u32,
            edge_count: render_upload.edge_indices.len() as u32,
            triad_count: triad_vertices.len() as u32,
            value_bytes: value_buffer_bytes,
            value_count: field_layout.value_count,
            value_is_cell: field_layout.is_cell,
            geometry_bytes: vertex_buffer_bytes
                .saturating_add(index_buffer_bytes)
                .saturating_add(edge_buffer_bytes),
            field_capacity_bytes: value_buffer_bytes,
            resident_bytes: vertex_buffer_bytes
                .saturating_add(value_buffer_bytes)
                .saturating_add(index_buffer_bytes)
                .saturating_add(edge_buffer_bytes)
                .saturating_add(triad_buffer_bytes)
                .saturating_add(uniform_buffer_bytes)
                .saturating_add(depth_bytes),
            depth_bytes,
            depth_texture,
            depth_view,
            width: 1,
            height: 1,
            timestamp: GpuTimestampRecorder::new(ctx, "mesh_3d_gpu_timestamps"),
        }
    }

    fn resize(&mut self, ctx: &gpui_wgpu::WgpuContext, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let (depth_texture, depth_view) = make_targets(ctx, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.width = width;
        self.height = height;
        let depth_bytes = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4);
        self.resident_bytes = self
            .resident_bytes
            .saturating_sub(self.depth_bytes)
            .saturating_add(depth_bytes);
        self.depth_bytes = depth_bytes;
    }

    fn write_values(&mut self, ctx: &gpui_wgpu::WgpuContext, upload: &MeshUpload) -> u64 {
        let values = mesh_field_values(upload);
        let bytes = bytemuck::cast_slice(values.as_ref());
        if bytes.len() as u64 <= self.value_bytes
            && values.len() == self.value_count
            && upload.cell_values_f32.is_some() == self.value_is_cell
        {
            write_chunked_buffer(ctx, &self.values, bytes);
            bytes.len() as u64
        } else {
            0
        }
    }

    fn write_triad(&self, ctx: &gpui_wgpu::WgpuContext, camera: &Camera3D) {
        let vertices = triad_vertices(camera);
        write_chunked_buffer(ctx, &self.triad, bytemuck::cast_slice(&vertices));
    }

    fn write_uniform(
        &self,
        ctx: &gpui_wgpu::WgpuContext,
        state: &crate::mesh::gpu::MeshSceneState,
        camera: &Camera3D,
    ) {
        let range = state.color.range;
        let origin = state
            .upload
            .as_ref()
            .map(|upload| upload.origin)
            .unwrap_or([0.0; 3]);
        let field_enabled = state
            .upload
            .as_ref()
            .is_some_and(|upload| upload.values_f32.is_some() || upload.cell_values_f32.is_some());
        let uniforms = Mesh3DUniforms {
            view_proj: camera.view_projection_matrix().to_cols_array_2d(),
            // `prepare_upload` deliberately rebases vertices around a local
            // f64 origin. Restore that translation in the model matrix before
            // applying the world-space camera, otherwise large-world meshes
            // render/pick against a camera fitted to the wrong location.
            model: glam::Mat4::from_translation(glam::Vec3::new(
                origin[0] as f32,
                origin[1] as f32,
                origin[2] as f32,
            ))
            .to_cols_array_2d(),
            light_dir: [0.35, 0.55, 0.75, 0.0],
            params: [
                state.color.colormap as f32,
                state.color.unlit as u32 as f32,
                0.3,
                0.7,
            ],
            value_range: [
                range[0],
                range[1],
                field_enabled as u32 as f32,
                state.vertex_colors.is_some() as u32 as f32,
            ],
            isoline: [
                state.color.isoline_step,
                state.color.isoline_width_px,
                1.0,
                0.0,
            ],
            isoline_color: [0.08, 0.10, 0.14, 1.0],
        };
        ctx.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));
    }
}

#[cfg(not(test))]
fn make_targets(
    ctx: &gpui_wgpu::WgpuContext,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let extent = wgpu::Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };
    let depth_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mesh_3d_depth"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    (depth_texture, depth_view)
}

#[cfg(not(test))]
fn create_chunked_buffer(
    ctx: &gpui_wgpu::WgpuContext,
    label: &str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (bytes.len() as u64).max(4),
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    write_chunked_buffer(ctx, &buffer, bytes);
    buffer
}

#[cfg(not(test))]
fn write_chunked_buffer(ctx: &gpui_wgpu::WgpuContext, buffer: &wgpu::Buffer, bytes: &[u8]) {
    for (offset, chunk) in upload_chunks(bytes) {
        if !chunk.is_empty() {
            ctx.queue.write_buffer(buffer, offset as u64, chunk);
        }
    }
}

#[cfg(not(test))]
fn build_3d_vertices(upload: &MeshUpload, colors: Option<&[[f32; 4]]>) -> Vec<Mesh3DVertex> {
    let mut normals = vec![[0.0f32; 3]; upload.positions_f32.len()];
    for triangle in upload.indices.chunks_exact(3) {
        let Some(a) = upload.positions_f32.get(triangle[0] as usize) else {
            continue;
        };
        let Some(b) = upload.positions_f32.get(triangle[1] as usize) else {
            continue;
        };
        let Some(c) = upload.positions_f32.get(triangle[2] as usize) else {
            continue;
        };
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let face = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for index in triangle {
            if let Some(normal) = normals.get_mut(*index as usize) {
                for axis in 0..3 {
                    normal[axis] += face[axis];
                }
            }
        }
    }
    normals.iter_mut().for_each(|normal| {
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length > f32::EPSILON {
            normal.iter_mut().for_each(|value| *value /= length);
        } else {
            *normal = [0.0, 0.0, 1.0];
        }
    });
    upload
        .positions_f32
        .iter()
        .enumerate()
        .map(|(index, &position)| Mesh3DVertex {
            position,
            normal: normals[index],
            color: colors
                .and_then(|colors| colors.get(index))
                .copied()
                .unwrap_or([0.35, 0.55, 0.75, 1.0]),
        })
        .collect()
}

#[cfg(not(test))]
fn triad_vertices(camera: &Camera3D) -> [Mesh3DVertex; 6] {
    let origin = [-0.84_f32, -0.84_f32];
    let view = camera.view_matrix();
    let axes = [
        (glam::Vec3::X, [0.90, 0.16, 0.14]),
        (glam::Vec3::Y, [0.18, 0.82, 0.28]),
        (glam::Vec3::Z, [0.20, 0.42, 0.95]),
    ];
    let mut output = [Mesh3DVertex {
        position: [0.0; 3],
        normal: [0.0; 3],
        color: [1.0; 4],
    }; 6];
    for (axis, (direction, color)) in axes.into_iter().enumerate() {
        let screen_direction = (view * direction.extend(0.0)).truncate();
        let length = screen_direction.xy().length().max(1e-6);
        let direction = screen_direction.xy() / length;
        let end = [
            origin[0] + direction.x * 0.12,
            origin[1] + direction.y * 0.12,
        ];
        let base = axis * 2;
        output[base] = Mesh3DVertex {
            position: [origin[0], origin[1], 0.0],
            normal: color,
            color: [1.0; 4],
        };
        output[base + 1] = Mesh3DVertex {
            position: [end[0], end[1], 0.0],
            normal: color,
            color: [1.0; 4],
        };
    }
    output
}

#[cfg(not(test))]
fn expand_cell_upload(upload: &MeshUpload) -> MeshUpload {
    if upload.cell_values_f32.is_none() {
        return upload.clone();
    }

    let mut expanded = crate::mesh::expand_cell_shading(upload);
    // The generic upload helper preserves the original unique-edge list.
    // Once vertices are duplicated, construct a matching per-triangle line
    // list so wireframe indices never address the old vertex space.
    expanded.edge_indices = expanded
        .indices
        .chunks_exact(3)
        .flat_map(|triangle| {
            [
                triangle[0],
                triangle[1],
                triangle[1],
                triangle[2],
                triangle[2],
                triangle[0],
            ]
        })
        .collect();
    expanded
}

#[cfg(not(test))]
fn mesh_field_values(upload: &MeshUpload) -> Cow<'_, [f32]> {
    if let Some(cell_values) = &upload.cell_values_f32 {
        return Cow::Owned(
            cell_values
                .iter()
                .flat_map(|&value| [value, value, value])
                .take(upload.indices.len())
                .collect(),
        );
    }
    Cow::Borrowed(upload.values_f32.as_deref().unwrap_or(&[]))
}

#[cfg(not(test))]
impl gpui::CustomDraw for WgpuMesh3DDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(not(test))]
impl gpui_wgpu::WgpuCustomDraw for WgpuMesh3DDraw {
    fn draw_wgpu(
        &self,
        ctx: &gpui_wgpu::WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _target_format: wgpu::TextureFormat,
        target_size: [u32; 2],
        bounds: gpui::Bounds<gpui::Pixels>,
        _full_bounds: gpui::Bounds<gpui::Pixels>,
        scale_factor: f32,
    ) {
        let frame_started = Instant::now();
        let gpu_elapsed = self
            .resources
            .borrow_mut()
            .as_mut()
            .and_then(|resources| resources.timestamp.as_mut())
            .and_then(|timestamp| timestamp.poll(ctx));
        if let Some(elapsed) = gpu_elapsed {
            self.state.borrow_mut().record_gpu_frame_gpu_time(elapsed);
        }
        let state = self.state.borrow();
        let Some(upload) = state.upload.as_ref() else {
            return;
        };
        let revision = state.geometry_rev;
        let expected_field_layout = mesh_field_layout(upload);
        let mut resources = self.resources.borrow_mut();
        let mut created_geometry_resource = false;
        let mut geometry_upload_time = None;
        if resources.as_ref().is_none_or(|resource| {
            resource.geometry_rev != revision
                || resource.value_count != expected_field_layout.value_count
                || resource.value_is_cell != expected_field_layout.is_cell
        }) {
            let geometry_started = Instant::now();
            let mut created = WgpuMesh3DResources::new(ctx, &state, revision);
            // The newly allocated scalar buffer is initialized from the
            // current retained upload, so do not count that initialization as
            // a later field-only queue write.
            created.field_rev = state.field_rev;
            *resources = Some(created);
            geometry_upload_time = Some(geometry_started.elapsed());
            created_geometry_resource = true;
        }
        let Some(resources) = resources.as_mut() else {
            return;
        };
        let Some([x, y, viewport_width, viewport_height]) = clipped_target_viewport(
            [f32::from(bounds.origin.x), f32::from(bounds.origin.y)],
            [f32::from(bounds.size.width), f32::from(bounds.size.height)],
            scale_factor,
            target_size,
        ) else {
            return;
        };
        // The color target is GPUI's full frame. Depth resources follow that
        // extent; chart bounds only define the viewport and scissor below.
        // Draw directly into the target rather than an intermediate MSAA
        // resolve texture, because resolving a full-size intermediate would
        // overwrite content outside this embedded chart rectangle.
        resources.resize(ctx, target_size[0].max(1), target_size[1].max(1));
        let mut field_write_bytes = 0;
        let field_write_started = Instant::now();
        if resources.field_rev != state.field_rev {
            field_write_bytes = resources.write_values(ctx, upload);
            resources.field_rev = state.field_rev;
        }
        let field_write_time = field_write_started.elapsed();
        let resident_bytes = resources.resident_bytes;
        let field_capacity_bytes = resources.field_capacity_bytes;
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
        }
        let state = self.state.borrow();
        let camera = self.camera.borrow();
        resources.write_uniform(ctx, &state, &camera);
        resources.write_triad(ctx, &camera);
        let timestamp_active = resources
            .timestamp
            .as_mut()
            .is_some_and(|timestamp| timestamp.begin());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh_3d_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &resources.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: resources
                    .timestamp
                    .as_ref()
                    .and_then(|timestamp| timestamp.render_pass_writes(timestamp_active)),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_viewport(
                x as f32,
                y as f32,
                viewport_width as f32,
                viewport_height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(x, y, viewport_width, viewport_height);
            pass.set_bind_group(0, &resources.bind_group, &[]);
            pass.set_vertex_buffer(0, resources.vertices.slice(..));
            pass.set_pipeline(&resources.surface_pipeline);
            pass.set_index_buffer(resources.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..resources.index_count, 0, 0..1);
            if state.color.wireframe && resources.edge_count > 0 {
                pass.set_pipeline(&resources.wire_pipeline);
                pass.set_index_buffer(resources.edges.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..resources.edge_count, 0, 0..1);
            }
            pass.set_pipeline(&resources.triad_pipeline);
            pass.set_vertex_buffer(0, resources.triad.slice(..));
            pass.draw(0..resources.triad_count, 0..1);
        }
        if let Some(timestamp) = resources.timestamp.as_mut() {
            timestamp.finish(encoder, timestamp_active);
        }
        drop(state);
        self.state
            .borrow_mut()
            .record_gpu_frame_time(frame_started.elapsed());
    }
}

#[cfg(not(test))]
impl WgpuMesh3DRenderer {
    pub fn new(state: std::rc::Rc<std::cell::RefCell<crate::mesh::gpu::MeshSceneState>>) -> Self {
        Self::new_with_camera(
            state,
            std::rc::Rc::new(std::cell::RefCell::new(Camera3D::default())),
        )
    }

    /// Construct a renderer that shares its camera with the owning plot
    /// state. This keeps pointer navigation and retained custom drawing on
    /// the same camera without rebuilding mesh buffers.
    pub fn new_with_camera(
        state: std::rc::Rc<std::cell::RefCell<crate::mesh::gpu::MeshSceneState>>,
        camera: std::rc::Rc<std::cell::RefCell<Camera3D>>,
    ) -> Self {
        let resources = std::rc::Rc::new(std::cell::RefCell::new(None));
        let draw: std::rc::Rc<dyn gpui::CustomDraw> = std::rc::Rc::new(
            gpui_wgpu::WgpuCustomDrawAdapter(std::rc::Rc::new(WgpuMesh3DDraw {
                state: state.clone(),
                camera: camera.clone(),
                resources: resources.clone(),
            })),
        );
        let custom_id = gpui::register_custom_draw(draw);
        Self {
            state,
            camera,
            resources,
            custom_id,
        }
    }

    pub fn custom_id(&self) -> gpui::CustomDrawId {
        self.custom_id
    }

    pub fn camera(&self) -> Camera3D {
        self.camera.borrow().clone()
    }

    /// Share the retained camera with native pointer interaction. This keeps
    /// orbit/zoom events and the custom draw synchronized without rebuilding
    /// mesh resources.
    pub fn camera_handle(&self) -> std::rc::Rc<std::cell::RefCell<Camera3D>> {
        self.camera.clone()
    }

    pub fn set_camera(&self, camera: &Camera3D) {
        *self.camera.borrow_mut() = camera.clone();
    }
}

#[cfg(not(test))]
impl Drop for WgpuMesh3DRenderer {
    fn drop(&mut self) {
        gpui::unregister_custom_draw(self.custom_id);
        self.resources.borrow_mut().take();
        self.state.borrow_mut().clear_gpu_memory();
    }
}

#[cfg(test)]
mod tests {
    use super::{MeshUpload, clipped_target_viewport, mesh_field_layout};

    #[test]
    fn embedded_viewport_preserves_an_offset_chart_rectangle() {
        assert_eq!(
            clipped_target_viewport([25.0, 20.0], [80.0, 50.0], 1.0, [160, 120]),
            Some([25, 20, 80, 50])
        );
    }

    #[test]
    fn embedded_viewport_clips_to_the_target_on_every_edge() {
        assert_eq!(
            clipped_target_viewport([-20.0, 10.0], [50.0, 40.0], 1.0, [160, 120]),
            Some([0, 10, 30, 40])
        );
        assert_eq!(
            clipped_target_viewport([130.0, 100.0], [50.0, 40.0], 1.0, [160, 120]),
            Some([130, 100, 30, 20])
        );
        assert_eq!(
            clipped_target_viewport([170.0, 0.0], [20.0, 20.0], 1.0, [160, 120]),
            None
        );
    }

    #[test]
    fn embedded_viewport_scales_before_clipping_after_a_resize() {
        assert_eq!(
            clipped_target_viewport([10.0, 5.0], [30.0, 20.0], 2.0, [80, 40]),
            Some([20, 10, 60, 30])
        );
    }

    #[test]
    fn field_layout_distinguishes_vertex_and_cell_storage() {
        let vertex_upload = MeshUpload {
            positions_f32: vec![[0.0; 3]; 3],
            origin: [0.0; 3],
            indices: vec![0, 1, 2],
            edge_indices: vec![0, 1, 1, 2, 2, 0],
            values_f32: Some(vec![0.0, 0.5, 1.0]),
            cell_values_f32: None,
        };
        assert_eq!(
            mesh_field_layout(&vertex_upload),
            super::Mesh3DFieldLayout {
                value_count: 3,
                is_cell: false,
            }
        );

        let cell_upload = MeshUpload {
            values_f32: None,
            cell_values_f32: Some(vec![0.5]),
            ..vertex_upload
        };
        assert_eq!(
            mesh_field_layout(&cell_upload),
            super::Mesh3DFieldLayout {
                value_count: 3,
                is_cell: true,
            }
        );
    }
}

#[cfg(not(test))]
impl MeshGpuRenderer for WgpuMesh3DRenderer {
    fn upload_geometry(&mut self, revision: GeometryRevision, upload: &MeshUpload) {
        let mut state = self.state.borrow_mut();
        state.record_geometry_upload(upload);
        state.geometry_rev = revision;
        state.upload = Some(upload.clone());
        self.resources.borrow_mut().take();
    }

    fn write_field(&mut self, revision: FieldRevision, values: &[f32]) {
        let mut state = self.state.borrow_mut();
        state.record_field_write(values);
        state.field_rev = revision;
        if let Some(upload) = state.upload.as_mut() {
            if upload.cell_values_f32.is_some() {
                crate::mesh::gpu::replace_retained_field(&mut upload.cell_values_f32, values);
            } else {
                crate::mesh::gpu::replace_retained_field(&mut upload.values_f32, values);
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
