//! wgpu-based 3D surface renderer

use super::camera::Camera3D;
use super::config::Surface3DConfig;
use super::mesh::{GpuVertex, SurfaceMesh};
use super::shaders;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::config::SurfacePlotType;

/// Uniform buffer data (must match shader layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_dir: [f32; 3],
    colormap: f32,
    ambient: f32,
    diffuse: f32,
    opacity: f32,
    z_min: f32,
    x_min_log: f32,
    x_range_log: f32,
    is_log_x: f32,
    show_surface_isolines: f32,
}

impl Uniforms {
    fn new(camera: &Camera3D, config: &Surface3DConfig, log_settings: Option<(f32, f32)>) -> Self {
        let view_proj = camera.view_projection_matrix();
        let model = Mat4::IDENTITY;
        let light_dir = config.normalized_light_direction();

        let (is_log, min_log, range_log) = if let Some((min, max)) = log_settings {
            // min and max are already in linear space (e.g. 20 and 20000)
            // We need their logs.
            // Avoid log(0).
            let min_v = min.max(1e-10).log10();
            let max_v = max.max(1e-10).log10();
            (1.0, min_v, max_v - min_v)
        } else {
            (0.0, 0.0, 1.0)
        };

        Self {
            view_proj: view_proj.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            light_dir: light_dir.to_array(),
            colormap: config.colormap.shader_index() as f32,
            ambient: config.ambient,
            diffuse: config.diffuse,
            opacity: config.opacity,
            z_min: 0.0,
            x_min_log: min_log,
            x_range_log: range_log,
            is_log_x: is_log,
            show_surface_isolines: if config.isolines { 1.0 } else { 0.0 },
        }
    }
}

/// GPU-accelerated 3D surface renderer
pub struct Surface3DRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: Option<wgpu::RenderPipeline>,
    isoline_pipeline: Option<wgpu::RenderPipeline>,
    grid_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    wireframe_index_buffer: Option<wgpu::Buffer>,
    grid_vertex_buffer: wgpu::Buffer,
    grid_index_buffer: wgpu::Buffer,
    index_count: u32,
    wireframe_index_count: u32,
    grid_index_count: u32,
    depth_texture: Option<wgpu::TextureView>,
    render_texture: Option<wgpu::Texture>,
    render_texture_view: Option<wgpu::TextureView>,
    width: u32,
    height: u32,
    config: Surface3DConfig,
}

impl Surface3DRenderer {
    /// Create a new renderer with the given configuration
    pub fn new(config: Surface3DConfig) -> Self {
        let (device, queue) = pollster::block_on(Self::create_device());
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let (
            surface_pipeline,
            wireframe_pipeline,
            isoline_pipeline,
            grid_pipeline,
            uniform_buffer,
            uniform_bind_group,
        ) = Self::create_pipelines(&device, &config);

        // Create grid mesh buffers
        let grid_mesh = super::mesh::generate_bounding_box_mesh();
        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: grid_mesh.vertex_bytes(),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let grid_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Index Buffer"),
            contents: grid_mesh.index_bytes(),
            usage: wgpu::BufferUsages::INDEX,
        });
        let grid_index_count = grid_mesh.index_count as u32;

        Self {
            device,
            queue,
            surface_pipeline,
            wireframe_pipeline,
            isoline_pipeline,
            grid_pipeline,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer: None,
            index_buffer: None,
            wireframe_index_buffer: None,
            grid_vertex_buffer,
            grid_index_buffer,
            index_count: 0,
            wireframe_index_count: 0,
            grid_index_count,
            depth_texture: None,
            render_texture: None,
            render_texture_view: None,
            width: 0,
            height: 0,
            config,
        }
    }

    async fn create_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find suitable GPU adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Surface3D Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device")
    }

    fn create_pipelines(
        device: &wgpu::Device,
        config: &Surface3DConfig,
    ) -> (
        wgpu::RenderPipeline,
        Option<wgpu::RenderPipeline>,
        Option<wgpu::RenderPipeline>,
        wgpu::RenderPipeline,
        wgpu::Buffer,
        wgpu::BindGroup,
    ) {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Surface3D Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::combined_shader().into()),
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Surface3D Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Vertex buffer layout
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as u64,
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
                    format: wgpu::VertexFormat::Float32,
                    offset: 24,
                    shader_location: 2,
                },
            ],
        };

        // Create surface pipeline
        let surface_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout.clone()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Show both sides
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: config.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create wireframe pipeline if enabled
        let wireframe_pipeline = if config.wireframe {
            Some(
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Wireframe Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[vertex_layout.clone()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_wireframe"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: Default::default(),
                        bias: wgpu::DepthBiasState {
                            constant: -4,      // Stronger bias to pull lines forward
                            slope_scale: -2.0, // Slope-scaled bias for angled surfaces
                            clamp: 0.0,
                        },
                    }),
                    multisample: wgpu::MultisampleState {
                        count: config.msaa_samples,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                }),
            )
        } else {
            None
        };

        // Create isoline pipeline if enabled
        let isoline_pipeline = if config.isolines {
            Some(
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Isoline Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_projection"),
                        buffers: &[vertex_layout.clone()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_projection"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back), // Cull back faces to avoid seeing wireframe through transparent surface
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: config.msaa_samples,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                }),
            )
        } else {
            None
        };

        // Create grid pipeline
        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_grid"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // Cull front faces to see inside back faces
                // Wait, we want to see INSIDE faces.
                // If we use standard box, normals point OUT.
                // If we view from outside, we see front faces.
                // We want to see the BACK faces (inside).
                // So CullMode::Front will cull front faces and show back faces.
                // BUT, I generated indices for standard box.
                // So I should use CullMode::Front.
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true, // Write depth so surface is occluded by grid if behind?
                // No, grid is "behind" surface.
                // If we render grid first, we write depth.
                // Then surface renders. If surface is in front, it overdraws.
                // If surface is behind (impossible if inside box), it would be occluded.
                // So yes, depth write enabled.
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: config.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        (
            surface_pipeline,
            wireframe_pipeline,
            isoline_pipeline,
            grid_pipeline,
            uniform_buffer,
            bind_group,
        )
    }

    /// Upload mesh data to GPU
    pub fn set_mesh(&mut self, mesh: &SurfaceMesh) {
        if mesh.is_empty() {
            self.vertex_buffer = None;
            self.index_buffer = None;
            self.index_count = 0;
            return;
        }

        self.vertex_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: mesh.vertex_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));

        self.index_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: mesh.index_bytes(),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));

        self.index_count = mesh.index_count as u32;

        // Create wireframe indices if needed
        if self.config.wireframe && mesh.x_count >= 2 && mesh.y_count >= 2 {
            let wireframe_indices =
                super::mesh::generate_wireframe_indices(mesh.x_count, mesh.y_count);

            self.wireframe_index_buffer = Some(self.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Wireframe Index Buffer"),
                    contents: bytemuck::cast_slice(&wireframe_indices),
                    usage: wgpu::BufferUsages::INDEX,
                },
            ));
            self.wireframe_index_count = wireframe_indices.len() as u32;
        }
    }

    /// Resize render target
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        // Create depth texture
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.config.msaa_samples,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_texture = Some(depth_texture.create_view(&Default::default()));

        // Create render texture (for readback)
        let render_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.config.msaa_samples,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.render_texture_view = Some(render_texture.create_view(&Default::default()));
        self.render_texture = Some(render_texture);
    }

    /// Render the surface and return RGBA pixel data
    pub fn render(
        &mut self,
        camera: &Camera3D,
        log_settings: Option<(f32, f32)>,
    ) -> Option<Vec<u8>> {
        if self.vertex_buffer.is_none() || self.width == 0 || self.height == 0 {
            return None;
        }

        // Update uniforms
        let uniforms = Uniforms::new(camera, &self.config, log_settings);
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Create resolve texture for MSAA
        let resolve_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Resolve Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let resolve_view = resolve_texture.create_view(&Default::default());

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render pass
        {
            let bg = &self.config.background_color;
            let color_attachment = if self.config.msaa_samples > 1 {
                wgpu::RenderPassColorAttachment {
                    view: self.render_texture_view.as_ref()?,
                    resolve_target: Some(&resolve_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }
            } else {
                wgpu::RenderPassColorAttachment {
                    view: self.render_texture_view.as_ref()?,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Render Pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.depth_texture.as_ref()?,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw grid box first (background)
            // Use CullMode::Front (cull front, show back).

            if self.config.show_grid
                && self.config.show_axes
                && self.config.plot_type == SurfacePlotType::Cartesian
            {
                render_pass.set_pipeline(&self.grid_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.grid_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.grid_index_count, 0, 0..1);
            }
            // Draw surface
            render_pass.set_pipeline(&self.surface_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref()?.slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.as_ref()?.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);

            // Draw isolines if enabled (before wireframe)
            if let Some(pipeline) = &self.isoline_pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.draw_indexed(0..self.index_count, 0, 0..1);
            }

            // Draw wireframe if enabled
            if let (Some(pipeline), Some(index_buffer)) =
                (&self.wireframe_pipeline, &self.wireframe_index_buffer)
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.wireframe_index_count, 0, 0..1);
            }
        }

        // Copy to staging buffer for readback
        let bytes_per_row = (self.width * 4 + 255) & !255; // Align to 256
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (bytes_per_row * self.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let copy_source = if self.config.msaa_samples > 1 {
            &resolve_texture
        } else {
            self.render_texture.as_ref()?
        };

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: copy_source,
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

        // Read back pixels
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::Wait);

        match rx.recv() {
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

    /// Get current render dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Update configuration
    pub fn set_config(&mut self, config: Surface3DConfig) {
        self.config = config;
        // Note: Full pipeline recreation would be needed for some settings
        // For now, just update the config
    }
}
