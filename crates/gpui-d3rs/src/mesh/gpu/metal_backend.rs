use super::{
    FieldRevision, GeometryRevision, MeshGpuRenderer, MeshSceneState, replace_retained_field,
};
use crate::mesh::MeshUpload;
use crate::mesh::gpu::shaders_metal::MESH_MSL;
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_macos::{MetalCustomDraw, MetalCustomDrawAdapter};
use metal::{
    CommandBufferRef, DeviceRef, MTLLoadAction, MTLPrimitiveType, MTLResourceOptions,
    MTLScissorRect, MTLStoreAction, RenderPipelineState, TextureRef,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy)]
struct MetalVertex {
    position: [f32; 3],
    value: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MetalTransform {
    columns: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MetalUniform {
    view_transform: [[f32; 4]; 4],
    range: [f32; 4],
    style: [f32; 4],
}

struct MetalResources {
    geometry_rev: GeometryRevision,
    field_rev: FieldRevision,
    vertices: metal::Buffer,
    lines: metal::Buffer,
    uniform: metal::Buffer,
    pipeline: RenderPipelineState,
    line_pipeline: RenderPipelineState,
    vertex_count: usize,
    line_count: usize,
}

impl MetalResources {
    fn new(
        device: &DeviceRef,
        texture: &TextureRef,
        revision: GeometryRevision,
        upload: &MeshUpload,
        state: &MeshSceneState,
    ) -> Option<Self> {
        let mut vertices = Vec::with_capacity(upload.indices.len());
        for (cell, triangle) in upload.indices.chunks_exact(3).enumerate() {
            for &index in triangle {
                let position = *upload.positions_f32.get(index as usize)?;
                let value = upload
                    .cell_values_f32
                    .as_ref()
                    .and_then(|values| values.get(cell).copied())
                    .or_else(|| {
                        upload
                            .values_f32
                            .as_ref()
                            .and_then(|values| values.get(index as usize).copied())
                    })
                    .unwrap_or(0.5);
                vertices.push(MetalVertex { position, value });
            }
        }
        let mut lines = Vec::with_capacity(upload.edge_indices.len());
        for &index in &upload.edge_indices {
            lines.push(MetalVertex {
                position: *upload.positions_f32.get(index as usize)?,
                value: 0.0,
            });
        }
        let options = MTLResourceOptions::StorageModeShared;
        let vertex_buffer = device.new_buffer_with_data(
            vertices.as_ptr() as *const c_void,
            (vertices.len() * std::mem::size_of::<MetalVertex>()) as u64,
            options,
        );
        let line_buffer = device.new_buffer_with_data(
            lines.as_ptr() as *const c_void,
            (lines.len() * std::mem::size_of::<MetalVertex>()) as u64,
            options,
        );
        let uniform = MetalUniform {
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
        };
        let uniform_buffer = device.new_buffer_with_data(
            &uniform as *const MetalUniform as *const c_void,
            std::mem::size_of::<MetalUniform>() as u64,
            options,
        );
        let library = device
            .new_library_with_source(MESH_MSL, &metal::CompileOptions::new())
            .ok()?;
        let vertex = library.get_function("mesh_vertex", None).ok()?;
        let fragment = library.get_function("mesh_fragment", None).ok()?;
        let line_fragment = library.get_function("mesh_line_fragment", None).ok()?;
        let make_pipeline = |label: &str, fragment: &metal::FunctionRef| {
            let descriptor = metal::RenderPipelineDescriptor::new();
            descriptor.set_label(label);
            descriptor.set_vertex_function(Some(vertex.as_ref()));
            descriptor.set_fragment_function(Some(fragment));
            descriptor
                .color_attachments()
                .object_at(0)?
                .set_pixel_format(texture.pixel_format());
            device.new_render_pipeline_state(&descriptor).ok()
        };
        Some(Self {
            geometry_rev: revision,
            field_rev: FieldRevision::default(),
            vertices: vertex_buffer,
            lines: line_buffer,
            uniform: uniform_buffer,
            pipeline: make_pipeline("mesh_metal_fill", fragment.as_ref())?,
            line_pipeline: make_pipeline("mesh_metal_lines", line_fragment.as_ref())?,
            vertex_count: vertices.len(),
            line_count: lines.len(),
        })
    }

    fn update_values(&mut self, upload: &MeshUpload) {
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
    }

    fn update_uniform(&mut self, state: &MeshSceneState) {
        let uniform = MetalUniform {
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
        };
        // SAFETY: `uniform` is a fully initialized POD value, the destination
        // is the shared uniform buffer allocated for this renderer, and the
        // copy length is exactly the `MetalUniform` layout size.
        unsafe {
            ptr::copy_nonoverlapping(
                &uniform as *const MetalUniform as *const u8,
                self.uniform.contents() as *mut u8,
                std::mem::size_of::<MetalUniform>(),
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
        let state = self.state.borrow();
        let Some(upload) = state.upload.as_ref() else {
            return;
        };
        let mut resources = self.resources.borrow_mut();
        if resources
            .as_ref()
            .is_none_or(|resources| resources.geometry_rev != state.geometry_rev)
        {
            *resources =
                MetalResources::new(device, drawable_texture, state.geometry_rev, upload, &state);
        }
        let Some(resources) = resources.as_mut() else {
            return;
        };
        if resources.field_rev != state.field_rev {
            resources.update_values(upload);
            resources.field_rev = state.field_rev;
        }
        resources.update_uniform(&state);
        let descriptor = metal::RenderPassDescriptor::new();
        let Some(attachment) = descriptor.color_attachments().object_at(0) else {
            // A malformed descriptor must not take down the render thread.
            return;
        };
        attachment.set_texture(Some(drawable_texture));
        attachment.set_load_action(MTLLoadAction::Load);
        attachment.set_store_action(MTLStoreAction::Store);
        let encoder = command_buffer.new_render_command_encoder(&descriptor);
        let rect = MTLScissorRect {
            x: (f32::from(bounds.origin.x) * scale_factor).max(0.0) as u64,
            y: (f32::from(bounds.origin.y) * scale_factor).max(0.0) as u64,
            width: (f32::from(bounds.size.width) * scale_factor).max(1.0) as u64,
            height: (f32::from(bounds.size.height) * scale_factor).max(1.0) as u64,
        };
        encoder.set_scissor_rect(rect);
        encoder.set_render_pipeline_state(&resources.pipeline);
        encoder.set_vertex_buffer(0, Some(&resources.vertices), 0);
        encoder.set_vertex_buffer(1, Some(&resources.uniform), 0);
        if resources.vertex_count != 0 {
            encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, resources.vertex_count as u64);
        }
        if state.color.wireframe && resources.line_count != 0 {
            encoder.set_render_pipeline_state(&resources.line_pipeline);
            encoder.set_vertex_buffer(0, Some(&resources.lines), 0);
            encoder.draw_primitives(MTLPrimitiveType::Line, 0, resources.line_count as u64);
        }
        encoder.end_encoding();
    }
}

impl MetalMeshRenderer {
    pub fn new(state: Rc<RefCell<MeshSceneState>>) -> Self {
        let resources = Rc::new(RefCell::new(None));
        let draw: Rc<dyn CustomDraw> = Rc::new(MetalCustomDrawAdapter(Rc::new(MetalMeshDraw {
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
