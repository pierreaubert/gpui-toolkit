//! Zero-copy vello backend: render the scene to an offscreen texture with
//! the shared wgpu device, then alpha-composite into the GPUI frame.

use crate::vello2d::{ChartScene, to_vello_scene};
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use vello::kurbo::Affine;
use vello::peniko::Color;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions};

/// Scene + the logical element size it was built for. Shared between
/// [`VelloChartElement`](crate::vello2d::VelloChartElement) (writes, at paint
/// time, in logical GPUI pixels) and [`WgpuVelloDraw`] (reads, at draw time,
/// where bounds arrive in physical pixels).
pub struct SharedScene {
    /// The chart scene in logical coordinates.
    pub scene: ChartScene,
    /// Element-local logical size the scene was built for.
    pub logical_size: (f32, f32),
}

/// Shared scene handle + lazily-initialized GPU state.
pub struct WgpuVelloDraw {
    scene: Rc<RefCell<SharedScene>>,
    gpu: RefCell<Option<GpuState>>,
    /// Shared with the element: set when `Renderer::new` fails so the element
    /// can fall back to the CPU rasterizer on its next paint. Never retried
    /// inside the paint loop.
    failed: Rc<Cell<bool>>,
}

struct GpuState {
    renderer: Renderer,
    offscreen_view: Option<wgpu::TextureView>,
    size: [u32; 2],
    composite: Option<CompositePipeline>,
}

/// Bounds size (GPUI px) → physical texture size, clamped to >= 1.
pub fn physical_size(width: f32, height: f32, scale_factor: f32) -> [u32; 2] {
    [
        (width * scale_factor).max(1.0) as u32,
        (height * scale_factor).max(1.0) as u32,
    ]
}

/// Scale mapping logical scene coordinates onto the physical offscreen
/// extent. Guards against a zero logical size (element not painted yet).
pub fn scene_scale(logical_w: f32, logical_h: f32, physical_w: f32, physical_h: f32) -> [f64; 2] {
    [
        if logical_w > 0.0 {
            (physical_w / logical_w) as f64
        } else {
            1.0
        },
        if logical_h > 0.0 {
            (physical_h / logical_h) as f64
        } else {
            1.0
        },
    ]
}

/// Source rectangle (origin, size) in device pixels: the visible sub-region
/// of the full-element offscreen texture.
pub fn clip_src_rect(
    full: Bounds<Pixels>,
    clipped: Bounds<Pixels>,
    scale_factor: f32,
) -> ([f32; 2], [f32; 2]) {
    let full_x: f32 = full.origin.x.into();
    let full_y: f32 = full.origin.y.into();
    let clip_x: f32 = clipped.origin.x.into();
    let clip_y: f32 = clipped.origin.y.into();
    let clip_w: f32 = clipped.size.width.into();
    let clip_h: f32 = clipped.size.height.into();
    (
        [
            ((clip_x - full_x) * scale_factor).max(0.0),
            ((clip_y - full_y) * scale_factor).max(0.0),
        ],
        [clip_w * scale_factor, clip_h * scale_factor],
    )
}

impl WgpuVelloDraw {
    pub fn new(scene: Rc<RefCell<SharedScene>>, failed: Rc<Cell<bool>>) -> Self {
        Self {
            scene,
            gpu: RefCell::new(None),
            failed,
        }
    }

    pub fn into_custom_draw(self) -> Rc<dyn CustomDraw> {
        Rc::new(WgpuCustomDrawAdapter(Rc::new(self)))
    }
}

impl CustomDraw for WgpuVelloDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WgpuCustomDraw for WgpuVelloDraw {
    fn draw_wgpu(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: [u32; 2],
        bounds: Bounds<Pixels>,
        full_bounds: Bounds<Pixels>,
        scale_factor: f32,
    ) {
        if self.failed.get() {
            return;
        }
        let full_w: f32 = full_bounds.size.width.into();
        let full_h: f32 = full_bounds.size.height.into();
        let size = physical_size(full_w, full_h, scale_factor);

        let vello_scene = {
            let shared = self.scene.borrow();
            if shared.scene.is_empty() {
                return;
            }
            let [sx, sy] = scene_scale(
                shared.logical_size.0,
                shared.logical_size.1,
                size[0] as f32,
                size[1] as f32,
            );
            to_vello_scene(&shared.scene, Affine::scale_non_uniform(sx, sy))
        }; // RefCell borrow released before GPU work

        let mut gpu_slot = self.gpu.borrow_mut();
        if gpu_slot.is_none() {
            match Renderer::new(
                &ctx.device,
                RendererOptions {
                    antialiasing_support: AaSupport::area_only(),
                    ..Default::default()
                },
            ) {
                Ok(renderer) => {
                    *gpu_slot = Some(GpuState {
                        renderer,
                        offscreen_view: None,
                        size: [0, 0],
                        composite: None,
                    });
                }
                Err(err) => {
                    log::error!(
                        "vello2d: vello::Renderer::new failed ({err}); element falls back to CPU"
                    );
                    self.failed.set(true);
                    return;
                }
            }
        }
        let Some(gpu) = gpu_slot.as_mut() else { return };

        if gpu.size != size {
            let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vello2d_offscreen"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            gpu.offscreen_view = Some(texture.create_view(&Default::default()));
            gpu.size = size;
        }
        if gpu.composite.is_none() {
            gpu.composite = Some(CompositePipeline::new(ctx, target_format));
        }
        if gpu.offscreen_view.is_none() || gpu.composite.is_none() {
            return;
        }

        // Disjoint field borrows: &mut gpu.renderer + &gpu.offscreen_view.
        if let Err(err) = gpu.renderer.render_to_texture(
            &ctx.device,
            &ctx.queue,
            &vello_scene,
            gpu.offscreen_view.as_ref().unwrap(),
            &RenderParams {
                base_color: Color::TRANSPARENT,
                width: size[0],
                height: size[1],
                antialiasing_method: AaConfig::Area,
            },
        ) {
            // Transient: log and leave the previous frame's content.
            log::error!("vello2d: render_to_texture failed: {err}");
            return;
        }

        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        let (src_origin, src_size) = clip_src_rect(full_bounds, bounds, scale_factor);
        gpu.composite.as_ref().unwrap().composite(
            ctx,
            encoder,
            target,
            gpu.offscreen_view.as_ref().unwrap(),
            [origin_x * scale_factor, origin_y * scale_factor],
            src_size,
            src_origin,
            [size[0] as f32, size[1] as f32],
            [target_size[0] as f32, target_size[1] as f32],
        );
    }
}

// ---------------------------------------------------------------------------
// Composite: draw the premultiplied-RGBA offscreen texture over the frame.

struct CompositePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

const COMPOSITE_WGSL: &str = r#"
struct Uniforms {
    dst_origin: vec2<f32>,
    dst_size: vec2<f32>,
    src_origin: vec2<f32>,
    src_size: vec2<f32>,
    tex_size: vec2<f32>,
    target_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var positions = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let p = positions[i];
    let device_px = u.dst_origin + p * u.dst_size;
    let ndc = vec2<f32>(
        device_px.x / u.target_size.x * 2.0 - 1.0,
        1.0 - device_px.y / u.target_size.y * 2.0,
    );
    // Sample only the visible sub-region of the full-element texture.
    let uv = (u.src_origin + p * u.src_size) / u.tex_size;
    return VsOut(vec4<f32>(ndc, 0.0, 1.0), uv);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
"#;

impl CompositePipeline {
    fn new(ctx: &WgpuContext, target_format: wgpu::TextureFormat) -> Self {
        let device = &ctx.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vello2d_composite"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_WGSL.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vello2d_composite_layout"),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vello2d_composite_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vello2d_composite_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    // vello output is premultiplied.
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vello2d_composite_uniform"),
            size: 48,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vello2d_composite_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            pipeline,
            bind_group_layout,
            uniform,
            sampler,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn composite(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        src: &wgpu::TextureView,
        dst_origin: [f32; 2],
        dst_size: [f32; 2],
        src_origin: [f32; 2],
        tex_size: [f32; 2],
        target_size: [f32; 2],
    ) {
        let uniforms: [f32; 12] = [
            dst_origin[0],
            dst_origin[1],
            dst_size[0],
            dst_size[1],
            src_origin[0],
            src_origin[1],
            dst_size[0],
            dst_size[1],
            tex_size[0],
            tex_size[1],
            target_size[0],
            target_size[1],
        ];
        ctx.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&uniforms));
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vello2d_composite_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vello2d_composite"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
