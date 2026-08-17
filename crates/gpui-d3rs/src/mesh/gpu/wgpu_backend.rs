use super::shaders::MESH_WGSL;
use super::{
    FieldRevision, GeometryRevision, MeshGpuRenderer, MeshSceneState, replace_retained_field,
};
use crate::mesh::{MeshUpload, expand_cell_shading, upload_chunks};
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MeshUniform {
    view_transform: [[f32; 4]; 4],
    range: [f32; 4],
    style: [f32; 4],
}

struct WgpuResources {
    geometry_rev: GeometryRevision,
    field_rev: FieldRevision,
    positions: wgpu::Buffer,
    triangles: wgpu::Buffer,
    edges: wgpu::Buffer,
    values: wgpu::Buffer,
    value_bytes: u64,
    value_count: usize,
    geometry_bytes: u64,
    resident_bytes: u64,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    position_count: usize,
    triangle_index_count: u32,
    edge_index_count: u32,
    field_is_cell: bool,
}

impl WgpuResources {
    fn new(ctx: &WgpuContext, revision: GeometryRevision, upload: &MeshUpload) -> Self {
        let render_upload = expand_cell_upload(upload);
        let field_values = render_upload.values_f32.as_deref().unwrap_or(&[0.5]);
        let positions_bytes =
            (render_upload.positions_f32.len() * std::mem::size_of::<[f32; 3]>()).max(4) as u64;
        let triangles_bytes =
            (render_upload.indices.len() * std::mem::size_of::<u32>()).max(4) as u64;
        let edges_bytes =
            (render_upload.edge_indices.len() * std::mem::size_of::<u32>()).max(4) as u64;
        let positions = create_chunked_buffer(
            ctx,
            "mesh_positions",
            bytemuck::cast_slice(&render_upload.positions_f32),
            wgpu::BufferUsages::VERTEX,
        );
        let triangles = create_chunked_buffer(
            ctx,
            "mesh_triangles",
            bytemuck::cast_slice(&render_upload.indices),
            wgpu::BufferUsages::INDEX,
        );
        let edges = create_chunked_buffer(
            ctx,
            "mesh_edges",
            bytemuck::cast_slice(&render_upload.edge_indices),
            wgpu::BufferUsages::INDEX,
        );
        let values = create_chunked_buffer(
            ctx,
            "mesh_values",
            bytemuck::cast_slice(field_values),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let value_bytes = std::mem::size_of_val(field_values) as u64;
        let uniform_bytes = std::mem::size_of::<MeshUniform>() as u64;
        let uniform = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_uniform"),
            size: std::mem::size_of::<MeshUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("mesh_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
        let bind_group = make_bind_group(ctx, &bind_group_layout, &uniform, &values);
        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mesh_shader"),
                source: wgpu::ShaderSource::Wgsl(MESH_WGSL.into()),
            });
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("mesh_pipeline_layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };
        let pipeline = |topology, fragment_entry| {
            ctx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("mesh_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vertex"),
                        buffers: std::slice::from_ref(&vertex_layout),
                        compilation_options: Default::default(),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some(fragment_entry),
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
        Self {
            geometry_rev: revision,
            field_rev: FieldRevision::default(),
            positions,
            triangles,
            edges,
            values,
            value_bytes,
            value_count: field_values.len(),
            geometry_bytes: positions_bytes
                .saturating_add(triangles_bytes)
                .saturating_add(edges_bytes),
            resident_bytes: positions_bytes
                .saturating_add(triangles_bytes)
                .saturating_add(edges_bytes)
                .saturating_add(value_bytes.max(4))
                .saturating_add(uniform_bytes),
            uniform,
            bind_group,
            fill_pipeline: pipeline(wgpu::PrimitiveTopology::TriangleList, "fragment"),
            line_pipeline: pipeline(wgpu::PrimitiveTopology::LineList, "line_fragment"),
            position_count: render_upload.positions_f32.len(),
            triangle_index_count: render_upload.indices.len() as u32,
            edge_index_count: render_upload.edge_indices.len() as u32,
            field_is_cell: upload.cell_values_f32.is_some(),
        }
    }

    fn update_field(&mut self, ctx: &WgpuContext, revision: FieldRevision, values: &[f32]) -> u64 {
        if values.is_empty() {
            self.field_rev = revision;
            return 0;
        }
        let bytes = std::mem::size_of_val(values) as u64;
        if bytes <= self.value_bytes && values.len() == self.value_count {
            ctx.queue
                .write_buffer(&self.values, 0, bytemuck::cast_slice(values));
            self.field_rev = revision;
            bytes
        } else {
            0
        }
    }

    fn update_uniform(&self, ctx: &WgpuContext, state: &MeshSceneState) {
        let range = state.color.range;
        let field_enabled = state
            .upload
            .as_ref()
            .is_some_and(|upload| upload.values_f32.is_some() || upload.cell_values_f32.is_some());
        let cell_field = state
            .upload
            .as_ref()
            .is_some_and(|upload| upload.cell_values_f32.is_some());
        let uniform = MeshUniform {
            view_transform: state.view_transform,
            range: [
                range[0],
                range[1],
                state.color.colormap as f32,
                field_enabled as u32 as f32,
            ],
            style: [
                state.color.isoline_step,
                state.color.isoline_width_px,
                cell_field as u32 as f32,
                state.color.unlit as u32 as f32,
            ],
        };
        ctx.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
    }
}

/// Create a retained buffer and fill it in bounded queue writes. This avoids
/// passing a multi-hundred-megabyte slice through one staging operation while
/// preserving one GPU allocation per geometry revision.
fn create_chunked_buffer(
    ctx: &WgpuContext,
    label: &str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let size = (bytes.len() as u64).max(4);
    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    for (offset, chunk) in upload_chunks(bytes) {
        if !chunk.is_empty() {
            ctx.queue.write_buffer(&buffer, offset as u64, chunk);
        }
    }
    buffer
}

fn make_bind_group(
    ctx: &WgpuContext,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    values: &wgpu::Buffer,
) -> wgpu::BindGroup {
    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mesh_bind_group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: values.as_entire_binding(),
            },
        ],
    })
}

/// Expand cell-associated values into the vertex-local representation used by
/// the portable 2D shader. Shared indexed vertices cannot carry two different
/// cell values, so the cell path duplicates each triangle's vertices once per
/// triangle and rebuilds its wireframe indices in that expanded vertex space.
fn expand_cell_upload(upload: &MeshUpload) -> MeshUpload {
    if upload.cell_values_f32.is_none() {
        return upload.clone();
    }

    let mut expanded = expand_cell_shading(upload);
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

fn field_values(upload: &MeshUpload) -> Cow<'_, [f32]> {
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

/// WGPU renderer that dispatches through GPUI's zero-copy custom primitive.
pub struct WgpuMeshRenderer {
    state: Rc<RefCell<MeshSceneState>>,
    resources: Rc<RefCell<Option<WgpuResources>>>,
    custom_id: gpui::CustomDrawId,
}

struct WgpuMeshDraw {
    state: Rc<RefCell<MeshSceneState>>,
    resources: Rc<RefCell<Option<WgpuResources>>>,
}

impl CustomDraw for WgpuMeshDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WgpuCustomDraw for WgpuMeshDraw {
    fn draw_wgpu(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _target_size: [u32; 2],
        bounds: Bounds<Pixels>,
        scale_factor: f32,
    ) {
        let frame_started = Instant::now();
        // Hold the retained state mutably for the complete custom draw. The
        // resource registry is a separate RefCell, so this avoids a
        // read-then-write reborrow of the same state during telemetry updates
        // (which otherwise panics on cell-field updates) without cloning the
        // retained upload on every frame.
        let mut state = self.state.borrow_mut();
        let Some(upload) = state.upload.as_ref() else {
            return;
        };
        let mut resources = self.resources.borrow_mut();
        let mut created_geometry_resource = false;
        let mut geometry_upload_time = None;
        let field = field_values(upload);
        let field_is_cell = upload.cell_values_f32.is_some();
        let position_count = if field_is_cell {
            upload.indices.len()
        } else {
            upload.positions_f32.len()
        };
        let edge_index_count = if field_is_cell {
            upload.indices.chunks_exact(3).count().saturating_mul(6)
        } else {
            upload.edge_indices.len()
        };
        let value_count = field.len().max(1);
        if resources.as_ref().is_none_or(|resources| {
            resources.geometry_rev != state.geometry_rev
                || resources.position_count != position_count
                || resources.triangle_index_count != upload.indices.len() as u32
                || resources.edge_index_count != edge_index_count as u32
                || resources.value_count != value_count
                || resources.field_is_cell != field_is_cell
        }) {
            let geometry_started = Instant::now();
            *resources = Some(WgpuResources::new(ctx, state.geometry_rev, upload));
            geometry_upload_time = Some(geometry_started.elapsed());
            created_geometry_resource = true;
        }
        let Some(resources) = resources.as_mut() else {
            return;
        };
        let field_write_started = Instant::now();
        let field_write_bytes = if resources.field_rev != state.field_rev {
            resources.update_field(ctx, state.field_rev, field.as_ref())
        } else {
            0
        };
        let field_write_time = field_write_started.elapsed();
        drop(field);
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
        state.set_gpu_memory(resources.resident_bytes, resources.value_bytes);
        resources.update_uniform(ctx, &state);
        let x = (f32::from(bounds.origin.x) * scale_factor).max(0.0) as u32;
        let y = (f32::from(bounds.origin.y) * scale_factor).max(0.0) as u32;
        let width = (f32::from(bounds.size.width) * scale_factor).max(1.0) as u32;
        let height = (f32::from(bounds.size.height) * scale_factor).max(1.0) as u32;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh_custom_draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_scissor_rect(x, y, width, height);
            pass.set_vertex_buffer(0, resources.positions.slice(..));
            pass.set_bind_group(0, &resources.bind_group, &[]);
            if resources.triangle_index_count != 0 {
                pass.set_pipeline(&resources.fill_pipeline);
                pass.set_index_buffer(resources.triangles.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..resources.triangle_index_count, 0, 0..1);
            }
            if state.color.wireframe && resources.edge_index_count != 0 {
                pass.set_pipeline(&resources.line_pipeline);
                pass.set_index_buffer(resources.edges.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..resources.edge_index_count, 0, 0..1);
            }
        }
        state.record_gpu_frame_time(frame_started.elapsed());
    }
}

impl WgpuMeshRenderer {
    pub fn new(state: Rc<RefCell<MeshSceneState>>) -> Self {
        let resources = Rc::new(RefCell::new(None));
        let draw: Rc<dyn CustomDraw> = Rc::new(WgpuCustomDrawAdapter(Rc::new(WgpuMeshDraw {
            state: state.clone(),
            resources: resources.clone(),
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

impl Drop for WgpuMeshRenderer {
    fn drop(&mut self) {
        gpui::unregister_custom_draw(self.custom_id);
        self.resources.borrow_mut().take();
        self.state.borrow_mut().clear_gpu_memory();
    }
}

impl MeshGpuRenderer for WgpuMeshRenderer {
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
