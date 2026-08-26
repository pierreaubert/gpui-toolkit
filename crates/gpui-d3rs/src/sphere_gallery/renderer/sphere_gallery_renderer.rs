use super::super::mesh::{GalleryVertex, generate_sphere_mesh};
use super::super::shaders;
use super::Uniforms;
use super::sphere_gallery_config::SphereGalleryConfig;
#[cfg(feature = "gpu-2d")]
use crate::gpu2d::Gpu2DContext;
use crate::gpu3d::Camera3D;
use glam::Mat4;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU-accelerated sphere gallery renderer
pub struct SphereGalleryRenderer {
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) vertex_buffer: Option<wgpu::Buffer>,
    pub(super) index_buffer: Option<wgpu::Buffer>,
    pub(super) index_count: u32,
    pub(super) depth_texture: Option<wgpu::TextureView>,
    pub(super) render_texture: Option<wgpu::Texture>,
    pub(super) render_texture_view: Option<wgpu::TextureView>,
    pub(super) atlas_texture: wgpu::Texture,
    pub(super) atlas_view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) config: SphereGalleryConfig,
}

impl SphereGalleryRenderer {
    fn shared_or_new_device() -> Option<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
        #[cfg(feature = "gpu-2d")]
        if let Ok(context) = Gpu2DContext::try_global() {
            return Some((context.device(), context.queue()));
        }

        let (device, queue) = pollster::block_on(Self::create_device())?;
        Some((Arc::new(device), Arc::new(queue)))
    }

    /// Create a new renderer
    pub fn new(config: SphereGalleryConfig) -> Option<Self> {
        let (device, queue) = Self::shared_or_new_device()?;
        Self::with_device(device, queue, config)
    }

    /// Construct resources on a caller-owned WGPU device and queue.
    pub(crate) fn with_device(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: SphereGalleryConfig,
    ) -> Option<Self> {
        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Gallery Atlas Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // Create initial atlas texture (will be replaced when images are set)
        let atlas_size = config.cell_size;
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gallery Atlas Texture"),
            size: wgpu::Extent3d {
                width: atlas_size * config.cols,
                height: atlas_size * config.rows,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&Default::default());

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gallery Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gallery Bind Group Layout"),
            entries: &[
                // Uniforms
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
                // Atlas texture
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
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gallery Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Create shader and pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gallery Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::combined_shader().into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gallery Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GalleryVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // normal
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
                // uv
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 2,
                },
                // cell_index
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 32,
                    shader_location: 3,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gallery Pipeline"),
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
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        Some(Self {
            device,
            queue,
            pipeline,
            uniform_buffer,
            bind_group,
            bind_group_layout,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            depth_texture: None,
            render_texture: None,
            render_texture_view: None,
            atlas_texture,
            atlas_view,
            sampler,
            width: 0,
            height: 0,
            config,
        })
    }

    pub(super) async fn create_device() -> Option<(wgpu::Device, wgpu::Queue)> {
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
            .ok()?;

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("SphereGallery Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .ok()
    }

    /// Upload the sphere mesh to the GPU
    pub fn build_mesh(&mut self) {
        let mesh =
            generate_sphere_mesh(self.config.cols, self.config.rows, &self.config.mesh_config);

        self.vertex_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Gallery Vertex Buffer"),
                contents: mesh.vertex_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));

        self.index_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Gallery Index Buffer"),
                contents: mesh.index_bytes(),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));

        self.index_count = mesh.indices.len() as u32;
    }

    /// Upload images to the texture atlas.
    ///
    /// Each image should be RGBA pixels at `cell_size × cell_size` resolution.
    /// Images are placed in row-major order in the atlas grid.
    pub fn upload_images(&mut self, images: &[&[u8]]) {
        let cell = self.config.cell_size;
        let cols = self.config.cols;

        for (i, image_data) in images.iter().enumerate() {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;

            let expected_size = (cell * cell * 4) as usize;
            if image_data.len() != expected_size {
                continue; // Skip incorrectly sized images
            }

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: col * cell,
                        y: row * cell,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                image_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(cell * 4),
                    rows_per_image: Some(cell),
                },
                wgpu::Extent3d {
                    width: cell,
                    height: cell,
                    depth_or_array_layers: 1,
                },
            );
        }

        // Recreate bind group with the atlas texture
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gallery Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }

    /// Resize the render target
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (self.width == width && self.height == height) {
            return;
        }

        self.width = width;
        self.height = height;

        // Create depth texture
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gallery Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_texture = Some(depth_texture.create_view(&Default::default()));

        // Create render texture
        let render_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gallery Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.render_texture_view = Some(render_texture.create_view(&Default::default()));
        self.render_texture = Some(render_texture);
    }

    /// Encode the gallery into its retained same-device texture and return the
    /// source view for in-frame compositing. This does not submit or read back.
    pub(in crate::sphere_gallery) fn encode_render_to_texture(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        camera: &Camera3D,
        cell_count: u32,
        selected_index: Option<u32>,
        hovered_index: Option<u32>,
    ) -> Option<&wgpu::TextureView> {
        if self.vertex_buffer.is_none() || self.width == 0 || self.height == 0 {
            return None;
        }

        let uniforms = Uniforms {
            view_proj: camera.view_projection_matrix().to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
            atlas_cols: self.config.cols as f32,
            atlas_rows: self.config.rows as f32,
            cell_count: cell_count as f32,
            selected_index: selected_index.map(|index| index as f32).unwrap_or(-1.0),
            hovered_index: hovered_index.map(|index| index as f32).unwrap_or(-1.0),
            ambient: self.config.ambient,
            diffuse: self.config.diffuse,
            _pad: 0.0,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        {
            let background = &self.config.background_color;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gallery custom-draw render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.render_texture_view.as_ref()?,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0] as f64,
                            g: background[1] as f64,
                            b: background[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
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
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref()?.slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.as_ref()?.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.render_texture_view.as_ref()
    }

    pub(in crate::sphere_gallery) fn uses_device(&self, device: &Arc<wgpu::Device>) -> bool {
        Arc::ptr_eq(&self.device, device)
    }

    pub(in crate::sphere_gallery) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Render the gallery and return RGBA pixel data
    #[cfg(feature = "headless-qa")]
    pub fn render(
        &mut self,
        camera: &Camera3D,
        cell_count: u32,
        selected_index: Option<u32>,
        hovered_index: Option<u32>,
    ) -> Option<Vec<u8>> {
        if self.vertex_buffer.is_none() || self.width == 0 || self.height == 0 {
            return None;
        }

        // Update uniforms
        let view_proj = camera.view_projection_matrix();
        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
            atlas_cols: self.config.cols as f32,
            atlas_rows: self.config.rows as f32,
            cell_count: cell_count as f32,
            selected_index: selected_index.map(|i| i as f32).unwrap_or(-1.0),
            hovered_index: hovered_index.map(|i| i as f32).unwrap_or(-1.0),
            ambient: self.config.ambient,
            diffuse: self.config.diffuse,
            _pad: 0.0,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Gallery Render Encoder"),
            });

        // Render pass
        {
            let bg = &self.config.background_color;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gallery Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
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
                    depth_slice: None,
                })],
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref()?.slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.as_ref()?.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        // Copy to staging buffer for readback
        let bytes_per_row = (self.width * 4 + 255) & !255;
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gallery Staging Buffer"),
            size: (bytes_per_row * self.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

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

    /// Get config reference
    pub fn config(&self) -> &SphereGalleryConfig {
        &self.config
    }
}
