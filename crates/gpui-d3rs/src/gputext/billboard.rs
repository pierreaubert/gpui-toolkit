//! World-anchored SDF text billboards for the wgpu 3D renderers.
//!
//! Both 3D consumers ([`crate::gpu3d`] surfaces and [`crate::sphere_gallery`]
//! item labels) share this pass: one SDF atlas baked once per process from
//! the bundled Sans, one WGSL shader, and one instance layout. Keeping the
//! shader and layout here guarantees the two renderers cannot drift apart.
//!
//! A billboard keeps a constant *screen* size: each label carries a
//! `px_to_world` scale computed on the CPU at its anchor depth, so labels
//! shrink with distance like the geometry around them yet always face the
//! camera. The pass depth-tests (`Less`) against the scene depth buffer but
//! never writes depth, so labels hide behind geometry without punching holes
//! for later draws.

use super::sdf::{SdfAtlas, bake_atlas, layout_string};
use crate::text::{HorizontalTextAnchor, VerticalTextAnchor};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::sync::{Arc, OnceLock};

/// A camera-facing text label anchored at a world position.
///
/// Single-line only: the SDF atlas covers tick/label strings, not prose (see
/// [`super::sdf::ATLAS_CHARSET`]).
#[derive(Clone, Debug)]
pub struct WorldLabel {
    /// Label text (single line).
    pub text: String,
    /// World-space anchor the alignment applies to.
    pub anchor: Vec3,
    /// Em size in screen px (matches the `GlyphTextConfig` size the
    /// screen-space path would use).
    pub size_px: f32,
    /// Straight-alpha linear RGBA.
    pub color: [f32; 4],
    /// Horizontal alignment of the string relative to the anchor.
    pub horizontal: HorizontalTextAnchor,
    /// Vertical alignment of the string relative to the anchor.
    pub vertical: VerticalTextAnchor,
    /// Extra screen-space nudge in px (`x` right, `y` DOWN), applied along
    /// the camera right/up axes. Lets tick labels keep the perpendicular
    /// offset the screen-space path computes.
    pub screen_offset_px: [f32; 2],
}

/// Per-glyph GPU instance: 16 floats, 64 bytes, `repr(C)` with 4-byte field
/// alignment so the WGSL vertex attribute offsets below stay exact.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct BillboardInstance {
    /// World-space string anchor.
    pub anchor: [f32; 3],
    /// String-px to world-unit scale at the anchor depth.
    pub px_to_world: f32,
    /// Glyph cell top-left in string px (alignment + screen offset baked in).
    pub offset: [f32; 2],
    /// Glyph cell size in string px.
    pub size: [f32; 2],
    /// Atlas UV rect.
    pub uv_min: [f32; 2],
    /// Atlas UV rect.
    pub uv_max: [f32; 2],
    /// Straight-alpha linear RGBA.
    pub color: [f32; 4],
}

/// Process-wide SDF atlas shared by every [`BillboardPass`]. Baked once
/// (~1 MB of SDF); each pass uploads its own GPU copy for its device.
pub fn shared_atlas() -> &'static SdfAtlas {
    static ATLAS: OnceLock<SdfAtlas> = OnceLock::new();
    ATLAS.get_or_init(bake_atlas)
}

/// Lay out `labels` into GPU instances.
///
/// `px_to_world` maps a world anchor to string-px-per-world-unit... rather
/// its inverse: world units per string px at that anchor's depth. Labels
/// whose anchor does not project (behind the camera) are dropped, as are
/// empty layouts.
pub fn build_instances(
    atlas: &SdfAtlas,
    labels: &[WorldLabel],
    px_to_world: impl Fn(Vec3) -> Option<f32>,
) -> Vec<BillboardInstance> {
    debug_assert!(
        labels.iter().all(|label| !label.text.contains('\n')),
        "billboard labels are single-line"
    );
    let mut instances = Vec::new();
    for label in labels {
        let Some(scale) = px_to_world(label.anchor) else {
            continue;
        };
        if !(scale.is_finite() && scale > 0.0) {
            continue;
        }
        let laid = layout_string(atlas, &label.text, label.size_px);
        if laid.glyphs.is_empty() {
            continue;
        }
        let shift_x = match label.horizontal {
            HorizontalTextAnchor::Start => 0.0,
            HorizontalTextAnchor::Middle => -laid.advance_px / 2.0,
            HorizontalTextAnchor::End => -laid.advance_px,
        };
        // String origin sits on the baseline; measure the ink box for the
        // vertical alignment so Top/Middle/Bottom match the screen path.
        let top = laid
            .glyphs
            .iter()
            .map(|glyph| glyph.offset_px[1])
            .fold(f32::INFINITY, f32::min);
        let bottom = laid
            .glyphs
            .iter()
            .map(|glyph| glyph.offset_px[1] + glyph.size_px[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let shift_y = match label.vertical {
            VerticalTextAnchor::Top => -top,
            VerticalTextAnchor::Middle => -(top + bottom) / 2.0,
            VerticalTextAnchor::Alphabetic => 0.0,
            VerticalTextAnchor::Bottom => -bottom,
        };
        let anchor = label.anchor.to_array();
        for glyph in &laid.glyphs {
            instances.push(BillboardInstance {
                anchor,
                px_to_world: scale,
                offset: [
                    glyph.offset_px[0] + shift_x + label.screen_offset_px[0],
                    glyph.offset_px[1] + shift_y + label.screen_offset_px[1],
                ],
                size: glyph.size_px,
                uv_min: glyph.uv_min,
                uv_max: glyph.uv_max,
                color: label.color,
            });
        }
    }
    instances
}

/// Camera uniform: view-projection plus the billboard plane axes. 96 bytes.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct BillboardUniforms {
    view_proj: [[f32; 4]; 4],
    cam_right: [f32; 4],
    cam_up: [f32; 4],
}

/// WGSL billboard pass shared by the surface and gallery renderers.
///
/// Vertex input is a procedural unit quad (buffer 0, 4 verts) plus one
/// [`BillboardInstance`] per glyph (buffer 1, instanced). The vertex shader
/// expands each glyph in the camera plane; the fragment shader thresholds
/// the SDF with a `fwidth` anti-aliased edge.
pub const BILLBOARD_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    cam_right: vec3<f32>,
    _pad0: f32,
    cam_up: vec3<f32>,
    _pad1: f32,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct VSIn {
    @location(0) corner: vec2<f32>,
    @location(1) anchor: vec3<f32>,
    @location(2) px_to_world: f32,
    @location(3) offset: vec2<f32>,
    @location(4) size: vec2<f32>,
    @location(5) uv_min: vec2<f32>,
    @location(6) uv_max: vec2<f32>,
    @location(7) color: vec4<f32>,
};

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;
    // String px are y-down: right grows along cam_right, down against cam_up.
    let local = in.offset + in.corner * in.size;
    let world = in.anchor
        + (cam.cam_right * local.x - cam.cam_up * local.y) * in.px_to_world;
    out.pos = cam.view_proj * vec4<f32>(world, 1.0);
    out.uv = mix(in.uv_min, in.uv_max, in.corner);
    out.color = in.color;
    return out;
}

@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let d = textureSample(atlas, atlas_sampler, in.uv).r;
    let w = fwidth(d) * 0.75 + 1e-4;
    let a = smoothstep(0.5 - w, 0.5 + w, d) * in.color.a;
    if (a < 0.004) {
        discard;
    }
    return vec4<f32>(in.color.rgb, a);
}
"#;

/// Initial instance capacity (512 glyphs = 32 KiB); grows by doubling.
const INITIAL_INSTANCE_CAPACITY: usize = 512;

/// Owns the billboard pipeline, SDF atlas texture, and instance buffer for
/// one renderer device. Created lazily on the first labeled frame so
/// label-free scenes (and the default opt-out) pay nothing.
pub struct BillboardPass {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    atlas_bind_group: wgpu::BindGroup,
    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    instance_count: usize,
    samples: u32,
}

impl BillboardPass {
    /// Build the pass on `device`, uploading the shared SDF atlas via
    /// `queue`. `samples` and `depth_format` must match the render pass the
    /// billboards draw into; `color_format` is the offscreen target format
    /// (`Rgba8Unorm` in both current renderers).
    pub fn new(
        device: &Arc<wgpu::Device>,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        samples: u32,
    ) -> Option<Self> {
        use wgpu::util::DeviceExt;

        let atlas = shared_atlas();
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Billboard SDF Atlas"),
            size: wgpu::Extent3d {
                width: atlas.size_px,
                height: atlas.size_px,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.size_px),
                rows_per_image: Some(atlas.size_px),
            },
            wgpu::Extent3d {
                width: atlas.size_px,
                height: atlas.size_px,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Billboard Atlas Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Billboard Shader"),
            source: wgpu::ShaderSource::Wgsl(BILLBOARD_WGSL.into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Billboard Uniform Buffer"),
            size: std::mem::size_of::<BillboardUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Billboard Uniform Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Billboard Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let atlas_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Billboard Atlas Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Billboard Atlas Bind Group"),
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Billboard Pipeline Layout"),
            bind_group_layouts: &[
                Some(&uniform_bind_group_layout),
                Some(&atlas_bind_group_layout),
            ],
            immediate_size: 0,
        });

        // repr(C) float fields: anchor@0, px_to_world@12, offset@16,
        // size@24, uv_min@32, uv_max@40, color@48; stride 64.
        let quad_layout = wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BillboardInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 12,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 16,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 4,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 32,
                    shader_location: 5,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 40,
                    shader_location: 6,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 7,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Billboard Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[quad_layout, instance_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                // Depth-test against the scene, but never write: labels are
                // transparent overlays and must not occlude later draws.
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let quad_vertices: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Billboard Quad Vertices"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Billboard Quad Indices"),
            contents: bytemuck::cast_slice(&quad_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Billboard Instances"),
            size: (INITIAL_INSTANCE_CAPACITY * std::mem::size_of::<BillboardInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Some(Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            atlas_bind_group,
            quad_vertex_buffer,
            quad_index_buffer,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            instance_count: 0,
            samples,
        })
    }

    /// Upload `instances` for the next [`Self::draw`]. Grows the instance
    /// buffer (doubling) when the batch no longer fits.
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[BillboardInstance],
    ) {
        if instances.len() > self.instance_capacity {
            let mut capacity = self.instance_capacity.max(1);
            while capacity < instances.len() {
                capacity *= 2;
            }
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Billboard Instances"),
                size: (capacity * std::mem::size_of::<BillboardInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = capacity;
        }
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }
        self.instance_count = instances.len();
    }

    /// MSAA sample count the pipeline was built for. A pass created for a
    /// different count cannot draw into this pass; see [`render_billboards`].
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Draw the uploaded instances into the caller's open render pass. No-op
    /// when nothing was uploaded. The queue write lands before the pass
    /// executes (queue operations order ahead of subsequently submitted
    /// encoder work on the same queue).
    pub fn draw(
        &self,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        view_proj: [[f32; 4]; 4],
        cam_right: [f32; 3],
        cam_up: [f32; 3],
    ) {
        if self.instance_count == 0 {
            return;
        }
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&BillboardUniforms {
                view_proj,
                cam_right: [cam_right[0], cam_right[1], cam_right[2], 0.0],
                cam_up: [cam_up[0], cam_up[1], cam_up[2], 0.0],
            }),
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_bind_group(1, &self.atlas_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.draw_indexed(0..6, 0, 0..self.instance_count as u32);
    }
}

/// Draw `labels` into the caller's open render pass, creating (or recreating
/// after an MSAA change) the pass in `slot` on first use.
///
/// This is the single funnel both 3D renderers use, so pipeline setup,
/// sample-count handling, and upload/draw ordering stay identical. Empty
/// label lists are a no-op that keeps any existing pass alive for reuse.
/// `px_to_world` maps each label anchor to world units per string px at its
/// depth (`Camera3D::world_per_screen_px` computes exactly this).
#[allow(clippy::too_many_arguments)]
pub fn render_billboards(
    slot: &mut Option<BillboardPass>,
    device: &Arc<wgpu::Device>,
    queue: &wgpu::Queue,
    pass: &mut wgpu::RenderPass<'_>,
    view_proj: [[f32; 4]; 4],
    cam_right: [f32; 3],
    cam_up: [f32; 3],
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
    samples: u32,
    labels: &[WorldLabel],
    px_to_world: impl Fn(Vec3) -> Option<f32>,
) {
    if labels.is_empty() {
        return;
    }
    if slot.as_ref().is_none_or(|pass| pass.samples() != samples) {
        *slot = BillboardPass::new(device, queue, color_format, depth_format, samples);
    }
    let Some(billboards) = slot.as_mut() else {
        return;
    };
    let instances = build_instances(shared_atlas(), labels, px_to_world);
    if instances.is_empty() {
        return;
    }
    billboards.upload(device, queue, &instances);
    billboards.draw(queue, pass, view_proj, cam_right, cam_up);
}
