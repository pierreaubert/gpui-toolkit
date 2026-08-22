use super::super::camera::Camera3D;
use super::super::config::Surface3DConfig;
use super::super::config::SurfacePlotType;
use super::super::mesh::{GpuVertex, SurfaceMesh};
use super::super::shaders;
use super::Uniforms;
use super::misc::background_surface_clear_color;
use super::transparent::transparent_surface_clear_color;
use super::unpremultiply::unpremultiply_rgba;
#[cfg(feature = "gpu-2d")]
use crate::gpu2d::Gpu2DContext;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU-accelerated 3D surface renderer
pub struct Surface3DRenderer {
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,
    pub(super) surface_pipeline: wgpu::RenderPipeline,
    pub(super) wireframe_pipeline: Option<wgpu::RenderPipeline>,
    pub(super) grid_pipeline: wgpu::RenderPipeline,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) uniform_bind_group: wgpu::BindGroup,
    pub(super) vertex_buffer: Option<wgpu::Buffer>,
    pub(super) index_buffer: Option<wgpu::Buffer>,
    pub(super) wireframe_index_buffer: Option<wgpu::Buffer>,
    pub(super) grid_vertex_buffer: wgpu::Buffer,
    pub(super) grid_index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
    pub(super) wireframe_index_count: u32,
    pub(super) grid_index_count: u32,
    pub(super) depth_texture: Option<wgpu::TextureView>,
    pub(super) render_texture: Option<wgpu::Texture>,
    pub(super) render_texture_view: Option<wgpu::TextureView>,
    pub(super) resolve_texture: Option<wgpu::Texture>,
    pub(super) readback_buffer: Option<wgpu::Buffer>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) config: Surface3DConfig,
}

impl Surface3DRenderer {
    fn shared_or_new_device() -> (Arc<wgpu::Device>, Arc<wgpu::Queue>) {
        #[cfg(feature = "gpu-2d")]
        if let Ok(context) = Gpu2DContext::try_global() {
            return (context.device(), context.queue());
        }

        let (device, queue) = pollster::block_on(Self::create_device());
        (Arc::new(device), Arc::new(queue))
    }

    /// Create a new renderer with the given configuration
    pub fn new(config: Surface3DConfig) -> Self {
        let (device, queue) = Self::shared_or_new_device();

        let (
            surface_pipeline,
            wireframe_pipeline,
            grid_pipeline,
            uniform_buffer,
            uniform_bind_group,
        ) = Self::create_pipelines(&device, &config);

        // Create grid mesh buffers
        let grid_mesh = super::super::mesh::generate_bounding_box_mesh();
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
            resolve_texture: None,
            readback_buffer: None,
            width: 0,
            height: 0,
            config,
        }
    }

    pub(super) async fn create_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
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
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .expect("Failed to create device")
    }

    pub(super) fn create_pipelines(
        device: &wgpu::Device,
        config: &Surface3DConfig,
    ) -> (
        wgpu::RenderPipeline,
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
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
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
                buffers: std::slice::from_ref(&vertex_layout),
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: config.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
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
                        buffers: std::slice::from_ref(&vertex_layout),
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
                        depth_write_enabled: Some(true),
                        depth_compare: Some(wgpu::CompareFunction::LessEqual),
                        stencil: Default::default(),
                        bias: Default::default(), // bias not compatible with LineList topology
                    }),
                    multisample: wgpu::MultisampleState {
                        count: config.msaa_samples,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview_mask: None,
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
                depth_write_enabled: Some(true), // Write depth so surface is occluded by grid if behind?
                // No, grid is "behind" surface.
                // If we render grid first, we write depth.
                // Then surface renders. If surface is in front, it overdraws.
                // If surface is behind (impossible if inside box), it would be occluded.
                // So yes, depth write enabled.
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: config.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        (
            surface_pipeline,
            wireframe_pipeline,
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
                super::super::mesh::generate_wireframe_indices(mesh.x_count, mesh.y_count);

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

        // Reuse the resolve and staging allocations across camera frames. The
        // GPUI bridge still needs an owned CPU image at its boundary, but the
        // expensive GPU-side resolve/readback resources no longer churn on
        // every render.
        self.resolve_texture = (self.config.msaa_samples > 1).then(|| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Resolve Texture"),
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
            })
        });

        let bytes_per_row = (width * 4 + 255) & !255;
        self.readback_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Surface Readback Buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));
    }

    /// Render the surface and return RGBA pixel data
    pub fn render(
        &mut self,
        camera: &Camera3D,
        log_settings: Option<(f32, f32)>,
    ) -> Option<Vec<u8>> {
        let clear_color = background_surface_clear_color(&self.config);
        self.render_with_clear(camera, log_settings, clear_color, false)
    }

    pub(in super::super) fn render_transparent(
        &mut self,
        camera: &Camera3D,
        log_settings: Option<(f32, f32)>,
    ) -> Option<Vec<u8>> {
        self.render_with_clear(
            camera,
            log_settings,
            transparent_surface_clear_color(),
            true,
        )
    }

    pub(super) fn render_with_clear(
        &mut self,
        camera: &Camera3D,
        log_settings: Option<(f32, f32)>,
        clear_color: wgpu::Color,
        unpremultiply_alpha: bool,
    ) -> Option<Vec<u8>> {
        if self.vertex_buffer.is_none() || self.width == 0 || self.height == 0 {
            return None;
        }

        // Update uniforms
        let mut surface_config = self.config.clone();
        surface_config.isolines = false;
        let uniforms = Uniforms::new(camera, &surface_config, log_settings);
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let resolve_view = self
            .resolve_texture
            .as_ref()
            .map(|texture| texture.create_view(&Default::default()));

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render pass
        {
            let color_attachment = if self.config.msaa_samples > 1 {
                wgpu::RenderPassColorAttachment {
                    view: self.render_texture_view.as_ref()?,
                    resolve_target: Some(resolve_view.as_ref()?),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }
            } else {
                wgpu::RenderPassColorAttachment {
                    view: self.render_texture_view.as_ref()?,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
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

            // Surface3DElement paints the Cartesian grid as projected GPUI
            // strokes. Keeping the legacy shader grid disabled avoids
            // fragment-space artifacts on grazing box faces.
            let draw_legacy_shader_grid = false;
            if draw_legacy_shader_grid
                && self.config.show_grid
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

            // Draw wireframe if enabled
            if let (Some(pipeline), Some(index_buffer)) =
                (&self.wireframe_pipeline, &self.wireframe_index_buffer)
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.wireframe_index_count, 0, 0..1);
            }
        }

        // Copy to the retained staging buffer for readback.
        let bytes_per_row = (self.width * 4 + 255) & !255; // Align to 256
        let staging_buffer = self.readback_buffer.as_ref()?;

        let copy_source = if self.config.msaa_samples > 1 {
            self.resolve_texture.as_ref()?
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
                buffer: staging_buffer,
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
                if unpremultiply_alpha {
                    unpremultiply_rgba(&mut pixels);
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
        if self.config.msaa_samples != config.msaa_samples {
            self.width = 0;
            self.height = 0;
            self.depth_texture = None;
            self.render_texture = None;
            self.render_texture_view = None;
            self.resolve_texture = None;
            self.readback_buffer = None;
        }
        self.config = config;
        // Note: Full pipeline recreation would be needed for some settings
        // For now, just update the config
    }
}
