use super::shaders::MESH_WGSL;
use super::{
    FieldRevision, GeometryRevision, MeshGpuRenderer, MeshSceneState, replace_retained_field,
};
use crate::mesh::{MeshUpload, upload_chunks};
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::RefCell;
use std::rc::Rc;

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
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    triangle_index_count: u32,
    edge_index_count: u32,
}

impl WgpuResources {
    fn new(ctx: &WgpuContext, revision: GeometryRevision, upload: &MeshUpload) -> Self {
        let field_values = upload
            .values_f32
            .as_deref()
            .or(upload.cell_values_f32.as_deref())
            .unwrap_or(&[0.5]);
        let positions = create_chunked_buffer(
            ctx,
            "mesh_positions",
            bytemuck::cast_slice(&upload.positions_f32),
            wgpu::BufferUsages::VERTEX,
        );
        let triangles = create_chunked_buffer(
            ctx,
            "mesh_triangles",
            bytemuck::cast_slice(&upload.indices),
            wgpu::BufferUsages::INDEX,
        );
        let edges = create_chunked_buffer(
            ctx,
            "mesh_edges",
            bytemuck::cast_slice(&upload.edge_indices),
            wgpu::BufferUsages::INDEX,
        );
        let values = create_chunked_buffer(
            ctx,
            "mesh_values",
            bytemuck::cast_slice(field_values),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let value_bytes = std::mem::size_of_val(field_values) as u64;
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
            uniform,
            bind_group,
            fill_pipeline: pipeline(wgpu::PrimitiveTopology::TriangleList, "fragment"),
            line_pipeline: pipeline(wgpu::PrimitiveTopology::LineList, "line_fragment"),
            triangle_index_count: upload.indices.len() as u32,
            edge_index_count: upload.edge_indices.len() as u32,
        }
    }

    fn update_field(&mut self, ctx: &WgpuContext, revision: FieldRevision, values: &[f32]) {
        let values = if values.is_empty() { &[0.5] } else { values };
        let bytes = std::mem::size_of_val(values) as u64;
        // Field patches are deliberately queue-only. A field with a different
        // cardinality is a geometry/schema change and must arrive with a new
        // geometry revision, which recreates this retained resource.
        if bytes <= self.value_bytes && values.len() == self.value_count {
            ctx.queue
                .write_buffer(&self.values, 0, bytemuck::cast_slice(values));
            self.field_rev = revision;
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
        let state = self.state.borrow();
        let Some(upload) = state.upload.as_ref() else {
            return;
        };
        let mut resources = self.resources.borrow_mut();
        if resources
            .as_ref()
            .is_none_or(|resources| resources.geometry_rev != state.geometry_rev)
        {
            *resources = Some(WgpuResources::new(ctx, state.geometry_rev, upload));
        }
        let Some(resources) = resources.as_mut() else {
            return;
        };
        let field = upload
            .values_f32
            .as_deref()
            .or(upload.cell_values_f32.as_deref())
            .unwrap_or(&[0.5]);
        if resources.field_rev != state.field_rev {
            resources.update_field(ctx, state.field_rev, field);
        }
        resources.update_uniform(ctx, &state);

        let x = (f32::from(bounds.origin.x) * scale_factor).max(0.0) as u32;
        let y = (f32::from(bounds.origin.y) * scale_factor).max(0.0) as u32;
        let width = (f32::from(bounds.size.width) * scale_factor).max(1.0) as u32;
        let height = (f32::from(bounds.size.height) * scale_factor).max(1.0) as u32;
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
