use super::super::device::Gpu2DContext;
use super::super::primitives::{
    CircleBatch, CircleVertex, Color4, LineBatch, LineVertex, Rect, RectBatch, RectVertex,
    TriangleBatch, TriangleVertex,
};
use super::super::shaders;
use super::super::text::{TextAtlas, TextBatch, TextVertex};
use super::Uniforms;
use super::misc::DEFAULT_FONT;
use std::sync::Arc;

/// GPU allocation retained across chart frames. Capacity grows geometrically
/// so animated data writes do not recreate a buffer every repaint.
struct ReusableBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
}

#[derive(Default)]
struct FrameBuffers {
    triangle_vertices: Option<ReusableBuffer>,
    triangle_indices: Option<ReusableBuffer>,
    rect_vertices: Option<ReusableBuffer>,
    rect_indices: Option<ReusableBuffer>,
    line_vertices: Option<ReusableBuffer>,
    line_indices: Option<ReusableBuffer>,
    circle_vertices: Option<ReusableBuffer>,
    circle_indices: Option<ReusableBuffer>,
    text_vertices: Option<ReusableBuffer>,
    text_indices: Option<ReusableBuffer>,
    staging: Option<ReusableBuffer>,
}

fn upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    slot: &mut Option<ReusableBuffer>,
    contents: &[u8],
    usage: wgpu::BufferUsages,
    label: &'static str,
) -> wgpu::Buffer {
    let required = (contents.len() as u64).max(4);
    if slot
        .as_ref()
        .is_none_or(|buffer| buffer.capacity < required)
    {
        let capacity = required.next_power_of_two();
        *slot = Some(ReusableBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: capacity,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            capacity,
        });
    }
    let buffer = slot.as_ref().expect("buffer inserted");
    queue.write_buffer(&buffer.buffer, 0, contents);
    buffer.buffer.clone()
}

fn reusable_buffer(
    device: &wgpu::Device,
    slot: &mut Option<ReusableBuffer>,
    required: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
) -> wgpu::Buffer {
    let required = required.max(4);
    if slot
        .as_ref()
        .is_none_or(|buffer| buffer.capacity < required)
    {
        let capacity = required.next_power_of_two();
        *slot = Some(ReusableBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: capacity,
                usage,
                mapped_at_creation: false,
            }),
            capacity,
        });
    }
    slot.as_ref().expect("buffer inserted").buffer.clone()
}

/// GPU-accelerated 2D chart renderer
pub struct Chart2DRenderer {
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,

    // Pipelines
    pub(super) line_pipeline: wgpu::RenderPipeline,
    pub(super) rect_pipeline: wgpu::RenderPipeline,
    pub(super) circle_pipeline: wgpu::RenderPipeline,
    pub(super) triangle_pipeline: wgpu::RenderPipeline,
    pub(super) text_pipeline: Option<wgpu::RenderPipeline>,

    // Uniform buffer and bind group
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) uniform_bind_group: wgpu::BindGroup,
    pub(super) uniform_bind_group_layout: wgpu::BindGroupLayout,

    // Primitive batches
    pub(super) line_batch: LineBatch,
    pub(super) rect_batch: RectBatch,
    pub(super) circle_batch: CircleBatch,
    pub(super) triangle_batch: TriangleBatch,
    pub(super) text_batch: TextBatch,

    // Text atlas
    pub(super) text_atlas: Option<TextAtlas>,

    // Render target
    pub(super) render_texture: Option<wgpu::Texture>,
    pub(super) render_texture_view: Option<wgpu::TextureView>,
    pub(super) width: u32,
    pub(super) height: u32,

    // Background color
    pub(super) background_color: [f32; 4],
    frame_buffers: FrameBuffers,

    // Deferred readback state (wasm only): the browser event loop delivers
    // the map_async callback, so pixels arrive one frame later.
    #[cfg(target_family = "wasm")]
    readback: std::rc::Rc<std::cell::RefCell<Readback>>,
}

/// Deferred pixel readback state for the wasm target.
///
/// `in_flight` guards against stacking `map_async` calls, and `pixels`
/// holds the newest completed frame (row padding already stripped) until
/// `Chart2DRenderer::take_readback` consumes it.
#[cfg(target_family = "wasm")]
#[derive(Default)]
pub struct Readback {
    pub in_flight: bool,
    pub pixels: Option<(u32, u32, Vec<u8>)>,
}

impl Chart2DRenderer {
    /// Create a new renderer, panicking when the GPU context is unavailable
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|err| panic!("{err}"))
    }

    /// Try to create a new renderer.
    ///
    /// Returns `Err` instead of panicking when no GPU adapter exists or, on
    /// wasm, while the device is still initializing asynchronously.
    pub fn try_new() -> Result<Self, &'static str> {
        let ctx = Gpu2DContext::try_global()?;
        let device = ctx.device();
        let queue = ctx.queue();

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chart2D Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Chart2D Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chart2D Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipelines
        let line_pipeline = Self::create_line_pipeline(&device, &uniform_bind_group_layout);
        let rect_pipeline = Self::create_rect_pipeline(&device, &uniform_bind_group_layout);
        let circle_pipeline = Self::create_circle_pipeline(&device, &uniform_bind_group_layout);
        let triangle_pipeline = Self::create_triangle_pipeline(&device, &uniform_bind_group_layout);

        // Initialize text atlas with embedded font
        let text_atlas = TextAtlas::new(device.clone(), queue.clone(), DEFAULT_FONT, 1024);
        let text_pipeline = text_atlas
            .bind_group_layout()
            .map(|layout| Self::create_text_pipeline(&device, &uniform_bind_group_layout, layout));

        Ok(Self {
            device,
            queue,
            line_pipeline,
            rect_pipeline,
            circle_pipeline,
            triangle_pipeline,
            text_pipeline,
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            line_batch: LineBatch::new(),
            rect_batch: RectBatch::new(),
            circle_batch: CircleBatch::new(),
            triangle_batch: TriangleBatch::new(),
            text_batch: TextBatch::new(),
            text_atlas: Some(text_atlas),
            render_texture: None,
            render_texture_view: None,
            width: 0,
            height: 0,
            background_color: [0.0, 0.0, 0.0, 0.0], // Transparent by default
            frame_buffers: FrameBuffers::default(),
            #[cfg(target_family = "wasm")]
            readback: std::rc::Rc::new(std::cell::RefCell::new(Readback::default())),
        })
    }

    pub(super) fn create_line_pipeline(
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::line_shader().into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_line"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 32,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 36,
                            shader_location: 4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_line"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub(super) fn create_rect_pipeline(
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::rect_shader().into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_rect"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 16,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 24,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 28,
                            shader_location: 4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_rect"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub(super) fn create_circle_pipeline(
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Circle Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::circle_shader().into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Circle Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Circle Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_circle"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<CircleVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 16,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 20,
                            shader_location: 3,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_circle"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub(super) fn create_triangle_pipeline(
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::triangle_shader().into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Triangle Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_triangle"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TriangleVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_triangle"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub(super) fn create_text_pipeline(
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
        atlas_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::text_shader().into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout), Some(atlas_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_text"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_text"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self, width: u32, height: u32, background_color: Color4) {
        // Guard against zero viewport to prevent divide-by-zero in pixel_to_ndc shader
        if width == 0 || height == 0 {
            self.background_color = background_color;
            self.line_batch.clear();
            self.rect_batch.clear();
            self.circle_batch.clear();
            self.triangle_batch.clear();
            self.text_batch.clear();
            return;
        }
        self.resize(width, height);
        self.background_color = background_color;
        self.line_batch.clear();
        self.rect_batch.clear();
        self.circle_batch.clear();
        self.triangle_batch.clear();
        self.text_batch.clear();
    }

    /// Resize the render target if needed
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        // Create render texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Chart2D Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        self.render_texture_view = Some(texture.create_view(&Default::default()));
        self.render_texture = Some(texture);
    }

    /// Draw a line segment
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Color4) {
        self.line_batch.add_line(x0, y0, x1, y1, width, color);
    }

    /// Draw a rectangle
    pub fn draw_rect(&mut self, rect: Rect, color: Color4, corner_radius: f32) {
        self.rect_batch.add_rect(rect, color, corner_radius);
    }

    /// Draw a circle
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color4) {
        self.circle_batch.add_circle(cx, cy, radius, color);
    }

    /// Draw a single triangle
    pub fn draw_triangle(&mut self, p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], color: Color4) {
        self.triangle_batch.add_triangle(p0, p1, p2, color);
    }

    /// Draw a filled polygon (automatically triangulated)
    pub fn draw_polygon(&mut self, points: &[[f32; 2]], color: Color4) {
        self.triangle_batch.add_polygon(points, color);
    }

    /// Draw text at the given position
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color4) {
        self.draw_text_rotated(text, x, y, size, color, 0.0);
    }

    /// Draw text at the given position, rotating glyph quads around the origin point.
    pub fn draw_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color4,
        rotation: f32,
    ) {
        let atlas = match &mut self.text_atlas {
            Some(a) => a,
            None => return,
        };

        let mut cursor_x = x;
        let base_idx = self.text_batch.vertices.len() as u32;
        let mut idx_offset = 0u32;
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        let rotate = |px: f32, py: f32| -> [f32; 2] {
            let lx = px - x;
            let ly = py - y;
            [x + lx * cos_r - ly * sin_r, y + lx * sin_r + ly * cos_r]
        };

        for c in text.chars() {
            if let Some(glyph) = atlas.get_glyph(c, size) {
                if glyph.width > 0 && glyph.height > 0 {
                    let gx = cursor_x + glyph.bearing[0];
                    // fontdue's ymin is negative for glyphs that extend above baseline
                    // Add it to y to position the top of the glyph correctly
                    let gy = y + glyph.bearing[1];
                    let gw = glyph.width as f32;
                    let gh = glyph.height as f32;

                    // Four vertices for the glyph quad
                    self.text_batch.vertices.push(TextVertex::new(
                        rotate(gx, gy),
                        [glyph.uv[0], glyph.uv[1]],
                        color,
                    ));
                    self.text_batch.vertices.push(TextVertex::new(
                        rotate(gx + gw, gy),
                        [glyph.uv[2], glyph.uv[1]],
                        color,
                    ));
                    self.text_batch.vertices.push(TextVertex::new(
                        rotate(gx, gy + gh),
                        [glyph.uv[0], glyph.uv[3]],
                        color,
                    ));
                    self.text_batch.vertices.push(TextVertex::new(
                        rotate(gx + gw, gy + gh),
                        [glyph.uv[2], glyph.uv[3]],
                        color,
                    ));

                    // Two triangles
                    let bi = base_idx + idx_offset;
                    self.text_batch.indices.extend_from_slice(&[
                        bi,
                        bi + 2,
                        bi + 1,
                        bi + 1,
                        bi + 2,
                        bi + 3,
                    ]);
                    idx_offset += 4;
                }
                cursor_x += glyph.advance;
            }
        }
    }

    /// Write uniforms, record the render pass, copy the render texture into a
    /// fresh staging buffer and submit. Returns the staging buffer, or None
    /// when there is nothing to read back.
    fn submit_frame(&mut self) -> Option<wgpu::Buffer> {
        if self.width == 0 || self.height == 0 {
            return None;
        }

        // Update uniforms
        let uniforms = Uniforms {
            viewport_size: [self.width as f32, self.height as f32],
            _padding: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Chart2D Render Encoder"),
            });

        // Render pass
        {
            let bg = &self.background_color;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Chart2D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.render_texture_view.as_ref()?,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: bg[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            // Draw triangles (filled polygons, background layer)
            if !self.triangle_batch.is_empty() {
                let vertex_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.triangle_vertices,
                    self.triangle_batch.vertex_bytes(),
                    wgpu::BufferUsages::VERTEX,
                    "Triangle Vertex Buffer",
                );
                let index_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.triangle_indices,
                    self.triangle_batch.index_bytes(),
                    wgpu::BufferUsages::INDEX,
                    "Triangle Index Buffer",
                );

                render_pass.set_pipeline(&self.triangle_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.triangle_batch.indices.len() as u32, 0, 0..1);
            }

            // Draw rectangles
            if !self.rect_batch.is_empty() {
                let vertex_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.rect_vertices,
                    self.rect_batch.vertex_bytes(),
                    wgpu::BufferUsages::VERTEX,
                    "Rect Vertex Buffer",
                );
                let index_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.rect_indices,
                    self.rect_batch.index_bytes(),
                    wgpu::BufferUsages::INDEX,
                    "Rect Index Buffer",
                );

                render_pass.set_pipeline(&self.rect_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.rect_batch.indices.len() as u32, 0, 0..1);
            }

            // Draw lines
            if !self.line_batch.is_empty() {
                let vertex_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.line_vertices,
                    self.line_batch.vertex_bytes(),
                    wgpu::BufferUsages::VERTEX,
                    "Line Vertex Buffer",
                );
                let index_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.line_indices,
                    self.line_batch.index_bytes(),
                    wgpu::BufferUsages::INDEX,
                    "Line Index Buffer",
                );

                render_pass.set_pipeline(&self.line_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.line_batch.indices.len() as u32, 0, 0..1);
            }

            // Draw circles
            if !self.circle_batch.is_empty() {
                let vertex_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.circle_vertices,
                    self.circle_batch.vertex_bytes(),
                    wgpu::BufferUsages::VERTEX,
                    "Circle Vertex Buffer",
                );
                let index_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.circle_indices,
                    self.circle_batch.index_bytes(),
                    wgpu::BufferUsages::INDEX,
                    "Circle Index Buffer",
                );

                render_pass.set_pipeline(&self.circle_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.circle_batch.indices.len() as u32, 0, 0..1);
            }

            // Draw text (foreground layer)
            if !self.text_batch.is_empty()
                && let (Some(pipeline), Some(atlas)) = (&self.text_pipeline, &self.text_atlas)
                && let Some(bind_group) = atlas.bind_group()
            {
                let vertex_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.text_vertices,
                    self.text_batch.vertex_bytes(),
                    wgpu::BufferUsages::VERTEX,
                    "Text Vertex Buffer",
                );
                let index_buffer = upload_buffer(
                    &self.device,
                    &self.queue,
                    &mut self.frame_buffers.text_indices,
                    self.text_batch.index_bytes(),
                    wgpu::BufferUsages::INDEX,
                    "Text Index Buffer",
                );

                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.text_batch.indices.len() as u32, 0, 0..1);
            }
        }

        // Copy to staging buffer for readback
        let bytes_per_row = (self.width * 4 + 255) & !255; // Align to 256
        let staging_buffer = reusable_buffer(
            &self.device,
            &mut self.frame_buffers.staging,
            (bytes_per_row * self.height) as u64,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            "Chart2D Staging Buffer",
        );

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.render_texture.as_ref()?,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        Some(staging_buffer)
    }

    /// End the frame and render to texture, returning RGBA pixels
    #[cfg(not(target_family = "wasm"))]
    pub fn end_frame(&mut self) -> Option<Vec<u8>> {
        let staging_buffer = self.submit_frame()?;
        let bytes_per_row = (self.width * 4 + 255) & !255; // Align to 256

        // Read back pixels
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: Default::default(),
            timeout: Some(std::time::Duration::from_secs(5)),
        });

        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();

                // Remove row padding
                let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
                for row in 0..self.height {
                    let start = (row * bytes_per_row) as usize;
                    let end = start + (self.width * 4) as usize;
                    pixels.extend_from_slice(&data[start..end]);
                }

                drop(data);
                staging_buffer.unmap();

                Some(pixels)
            }
            _ => None,
        }
    }

    /// End the frame: submit the draw commands and start an asynchronous
    /// readback whose callback is delivered by the browser event loop.
    /// Completed pixels are retrieved one frame later via `take_readback`
    /// (wasm only — blocking `device.poll`/`mpsc::recv` is forbidden there).
    #[cfg(target_family = "wasm")]
    pub fn end_frame_async(&mut self) {
        // Never stack maps: a previous readback is still in flight.
        if self.readback.borrow().in_flight {
            return;
        }
        let Some(staging_buffer) = self.submit_frame() else {
            return;
        };
        let (width, height) = (self.width, self.height);
        let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 (as in submit_frame)

        self.readback.borrow_mut().in_flight = true;
        let slot = self.readback.clone();
        // The map closure needs the buffer beyond the slice's borrow.
        let mapped_buffer = staging_buffer.clone();
        staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let mut slot = slot.borrow_mut();
                slot.in_flight = false;
                if result.is_ok() {
                    let data = mapped_buffer.slice(..).get_mapped_range();

                    // Remove row padding
                    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                    for row in 0..height {
                        let start = (row * bytes_per_row) as usize;
                        let end = start + (width * 4) as usize;
                        pixels.extend_from_slice(&data[start..end]);
                    }

                    drop(data);
                    mapped_buffer.unmap();

                    slot.pixels = Some((width, height, pixels));
                }
            });
        // No device.poll, no channel recv on wasm — the browser event loop
        // delivers the callback.
    }

    /// Take the newest completed readback, if any (wasm only).
    #[cfg(target_family = "wasm")]
    pub fn take_readback(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        self.readback.borrow_mut().pixels.take()
    }

    /// True while a readback is in flight or completed pixels are unpainted
    /// (wasm only).
    #[cfg(target_family = "wasm")]
    pub fn readback_pending(&self) -> bool {
        let readback = self.readback.borrow();
        readback.in_flight || readback.pixels.is_some()
    }

    /// Get current dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Load a font for text rendering
    ///
    /// Must be called before draw_text if no default font is embedded.
    pub fn load_font(&mut self, font_data: &[u8]) {
        let atlas = TextAtlas::new(self.device.clone(), self.queue.clone(), font_data, 1024);

        // Create text pipeline if not already created
        if self.text_pipeline.is_none()
            && let Some(layout) = atlas.bind_group_layout()
        {
            self.text_pipeline = Some(Self::create_text_pipeline(
                &self.device,
                &self.uniform_bind_group_layout,
                layout,
            ));
        }

        self.text_atlas = Some(atlas);
    }

    /// Check if text rendering is available
    pub fn has_font(&self) -> bool {
        self.text_atlas.is_some()
    }
}

impl Default for Chart2DRenderer {
    fn default() -> Self {
        Self::new()
    }
}
