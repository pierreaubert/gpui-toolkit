//! GPU-resident surface rendering through wgpu.
//!
//! A [`GpuScene`] uploads every actor's prepared chunks (positions,
//! normals, scalars, indices) and colour LUT once. Afterwards, drawing a
//! frame only rewrites the 192-byte per-actor uniform block (camera and
//! clip planes): geometry never returns to the CPU, which is the whole
//! point on Metal / CUDA-class adapters. Hosts present the target texture
//! themselves (surface or GPUI custom draw); the native `read_rgba` method
//! exists only for headless QA and movie export.

// Rust guideline compliant 2026-02-21

#[cfg(not(target_family = "wasm"))]
use crate::Frame;
use crate::camera::Camera;
use crate::retained::SceneRevision;
use crate::scene::Scene;
use crate::shaders::{
    PackedClipPlanes, SCENE_UNIFORM_SIZE, SURFACE_WGSL, packed_clip_planes,
    scene_uniform_block_with_equations,
};
use crate::{RenderLimits, SceneError};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Failures reported by the GPU renderer.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GpuRenderError {
    /// No GPU adapter is available on this machine.
    #[error("no wgpu adapter available")]
    NoAdapter,
    /// The device could not be created.
    #[error("wgpu device request failed: {0}")]
    Device(String),
    /// The renderer/readback requires an RGBA8 or BGRA8 color format.
    #[error("unsupported scene color format: {0:?}")]
    UnsupportedFormat(wgpu::TextureFormat),
    /// The requested scene feature has no implementation in this backend.
    #[error("unsupported GPU scene feature: {0}")]
    UnsupportedFeature(&'static str),
    /// Scene preparation failed.
    #[error(transparent)]
    Scene(#[from] SceneError),
}

struct GpuChunkBuffers {
    vertices: [wgpu::Buffer; 3],
    index: wgpu::Buffer,
    index_count: u32,
    origin: [f64; 3],
}

struct GpuActor {
    chunks: Vec<GpuChunkBuffers>,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: Vec<crate::scene::ClipPlane>,
    clip_coefficients: PackedClipPlanes,
    origin: [f64; 3],
    transparent: bool,
    distance: f64,
    lut: wgpu::Texture,
    transfer: Option<crate::TransferFunction>,
    color: [f32; 4],
}

/// A scene whose geometry lives entirely in GPU buffers.
pub struct GpuScene {
    actors: Vec<GpuActor>,
    background: [f32; 4],
    /// Total bytes of uploaded geometry (positions, normals, scalars,
    /// indices) — proof of residency for diagnostics.
    pub geometry_bytes: u64,
    order: Vec<usize>,
    prepared: SceneRevision,
}

/// An offscreen render target owned by the renderer.
pub struct GpuTarget {
    #[cfg(not(target_family = "wasm"))]
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    #[cfg(not(target_family = "wasm"))]
    limits: RenderLimits,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl GpuTarget {
    /// Sampleable premultiplied color view, for GPU-only host compositing.
    #[must_use]
    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }
}

/// Caller-owned attachments for encoding a scene in a host command stream.
#[derive(Debug)]
pub struct GpuAttachments<'a> {
    /// Color view matching the renderer's format.
    pub color: &'a wgpu::TextureView,
    /// `Depth32Float` view with the same physical dimensions.
    pub depth: &'a wgpu::TextureView,
    /// Physical target width and height.
    pub size: [u32; 2],
    /// Whether the host preserves or clears existing color content.
    pub load: wgpu::LoadOp<wgpu::Color>,
}

/// wgpu surface renderer: one device, pipelines per colour format.
pub struct GpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline_opaque: wgpu::RenderPipeline,
    pipeline_transparent: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
}

impl GpuRenderer {
    /// Creates a renderer for `format` (the surface or target format).
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported color formats, unavailable adapters,
    /// or device initialization failure.
    #[cfg(not(target_family = "wasm"))]
    pub fn new(format: wgpu::TextureFormat) -> Result<Self, GpuRenderError> {
        pollster::block_on(Self::new_async(format, wgpu::PowerPreference::LowPower))
    }

    /// Initializes a standalone device without blocking a platform event loop.
    ///
    /// Interactive hosts should use `from_shared_device` to share presentation
    /// resources. The power preference is explicit for standalone/offline use.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported formats, missing adapters or device failure.
    pub async fn new_async(
        format: wgpu::TextureFormat,
        power_preference: wgpu::PowerPreference,
    ) -> Result<Self, GpuRenderError> {
        validate_format(format)?;
        #[cfg(not(target_family = "wasm"))]
        let backends = wgpu::Backends::PRIMARY;
        #[cfg(target_family = "wasm")]
        let backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .map_err(|_| GpuRenderError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("gpui-scene3d renderer"),
                ..Default::default()
            })
            .await
            .map_err(|error| GpuRenderError::Device(error.to_string()))?;
        Self::from_shared_device(Arc::new(device), Arc::new(queue), format)
    }

    /// Builds pipelines on the host's existing presentation device and queue.
    ///
    /// No adapter is requested and no second device is created.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported color formats.
    pub fn from_shared_device(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        format: wgpu::TextureFormat,
    ) -> Result<Self, GpuRenderError> {
        validate_format(format)?;
        Ok(Self::build(device, queue, format))
    }

    fn build(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpui-scene3d surface"),
            source: wgpu::ShaderSource::Wgsl(SURFACE_WGSL.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpui-scene3d uniforms"),
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
                        view_dimension: wgpu::TextureViewDimension::D1,
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpui-scene3d pipeline"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let vertex_layouts = [
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 4,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 0,
                    shader_location: 2,
                }],
            },
        ];
        let make_pipeline = |depth_write: bool, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gpui-scene3d surface pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gpui-scene3d lut sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        Self {
            pipeline_opaque: make_pipeline(true, None),
            pipeline_transparent: make_pipeline(false, Some(wgpu::BlendState::ALPHA_BLENDING)),
            device,
            queue,
            bind_layout,
            sampler,
            format,
        }
    }

    /// The underlying device, for hosts sharing the wgpu context.
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Uploads a scene's geometry and LUTs; called once per scene change.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid render settings, missing fields,
    /// unsupported scalar/clip precision, or unsupported geometry features.
    pub fn upload(&self, scene: &Scene) -> Result<GpuScene, GpuRenderError> {
        scene.validate_render_state()?;
        for actor in scene.actors.iter().filter(|actor| actor.visible) {
            if let Some(transfer) = actor.active_transfer() {
                transfer.gpu_range()?;
            }
            if actor.wireframe {
                return Err(GpuRenderError::UnsupportedFeature("wireframe"));
            }
            if actor.clip_planes.len() > crate::shaders::MAX_CLIP_PLANES {
                return Err(GpuRenderError::UnsupportedFeature(
                    "more than six clipping planes",
                ));
            }
            if actor
                .dataset
                .cells
                .types
                .contains(&gpui_mesh::CellType::Line)
            {
                return Err(GpuRenderError::UnsupportedFeature("line cells"));
            }
        }
        let prepared = scene.prepare()?;
        let mut actors = Vec::new();
        let mut geometry_bytes = 0u64;
        for (index, actor) in scene
            .actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| actor.visible)
        {
            let geometry = prepared.actors[index]
                .as_ref()
                .expect("visible actor prepared");
            let chunks = &geometry.chunks;
            let origin = geometry.bounds.map_or([0.0; 3], |bounds| bounds.center());
            let clip_coefficients = packed_clip_planes(&actor.clip_planes, origin)?;
            let mut gpu_chunks = Vec::with_capacity(chunks.len());
            for chunk in chunks {
                if chunk.indices.is_empty() {
                    continue;
                }
                geometry_bytes += chunk.byte_len();
                let make = |data: &[f32], label: &str| {
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(label),
                            contents: bytemuck::cast_slice(data),
                            usage: wgpu::BufferUsages::VERTEX,
                        })
                };
                gpu_chunks.push(GpuChunkBuffers {
                    vertices: [
                        make(&chunk.positions, "positions"),
                        make(&chunk.normals, "normals"),
                        make(&chunk.scalars, "scalars"),
                    ],
                    index: self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("indices"),
                            contents: bytemuck::cast_slice(&chunk.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        }),
                    index_count: chunk.indices.len() as u32,
                    origin: chunk.origin,
                });
            }
            // Colour LUT: the actor's transfer function, or a flat ramp.
            let (lut_bytes, scalar_range) = material_lut(actor)?;
            let lut = self.device.create_texture_with_data(
                &self.queue,
                &wgpu::TextureDescriptor {
                    label: Some("gpui-scene3d lut"),
                    size: wgpu::Extent3d {
                        width: 256,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D1,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::wgt::TextureDataOrder::LayerMajor,
                &lut_bytes,
            );
            let uniform = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gpui-scene3d uniforms"),
                size: SCENE_UNIFORM_SIZE as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = self.actor_bind_group(&uniform, &lut);
            actors.push(GpuActor {
                chunks: gpu_chunks,
                uniform,
                bind_group,
                scalar_range,
                opacity: actor.opacity,
                clip_planes: actor.clip_planes.clone(),
                clip_coefficients,
                origin,
                transparent: actor.is_transparent(),
                distance: 0.0,
                lut,
                transfer: actor.active_transfer().cloned(),
                color: actor.effective_color(),
            });
        }
        Ok(GpuScene {
            order: (0..actors.len()).collect(),
            actors,
            background: scene.background,
            geometry_bytes,
            prepared: SceneRevision::new(scene)?,
        })
    }

    /// Synchronizes live materials and source revisions before drawing.
    ///
    /// Camera, background, opacity, clipping and LUT changes preserve geometry
    /// buffers. Geometry changes currently replace the uploaded scene.
    /// Returns `true` when geometry was uploaded.
    ///
    /// # Errors
    ///
    /// Returns the same preparation errors as [`Self::upload`].
    pub fn sync(&self, uploaded: &mut GpuScene, scene: &Scene) -> Result<bool, GpuRenderError> {
        scene.validate_render_state()?;
        for actor in scene.actors.iter().filter(|actor| actor.visible) {
            if let Some(transfer) = actor.active_transfer() {
                transfer.gpu_range()?;
            }
        }
        if !uploaded.prepared.matches(scene)? {
            *uploaded = self.upload(scene)?;
            return Ok(true);
        }
        if scene
            .actors
            .iter()
            .filter(|actor| actor.visible)
            .any(|actor| actor.clip_planes.len() > crate::shaders::MAX_CLIP_PLANES)
        {
            return Err(GpuRenderError::UnsupportedFeature(
                "more than six clipping planes",
            ));
        }
        // Validate every changed plane before mutating any uploaded material.
        let mut clip_updates = smallvec::SmallVec::<[(usize, PackedClipPlanes); 4]>::new();
        for (index, (gpu, actor)) in uploaded
            .actors
            .iter()
            .zip(scene.actors.iter().filter(|actor| actor.visible))
            .enumerate()
        {
            if gpu.clip_planes != actor.clip_planes {
                clip_updates.push((index, packed_clip_planes(&actor.clip_planes, gpu.origin)?));
            }
        }
        let mut clip_updates = clip_updates.into_iter().peekable();
        uploaded.background = scene.background;
        for (index, (gpu, actor)) in uploaded
            .actors
            .iter_mut()
            .zip(scene.actors.iter().filter(|a| a.visible))
            .enumerate()
        {
            if gpu.transfer.as_ref() != actor.active_transfer()
                || gpu.color != actor.effective_color()
            {
                let (bytes, range) = material_lut(actor)?;
                // New material resources keep already-encoded passes immutable.
                // A queue write into the old LUT would affect earlier passes
                // when the host batches both revisions in one submission.
                gpu.lut = self.device.create_texture_with_data(
                    &self.queue,
                    &wgpu::TextureDescriptor {
                        label: Some("gpui-scene3d lut"),
                        size: wgpu::Extent3d {
                            width: 256,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D1,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::wgt::TextureDataOrder::LayerMajor,
                    &bytes,
                );
                gpu.bind_group = self.actor_bind_group(&gpu.uniform, &gpu.lut);
                gpu.scalar_range = range;
                gpu.transfer = actor.active_transfer().cloned();
                gpu.color = actor.effective_color();
            }
            gpu.opacity = actor.opacity;
            if clip_updates
                .peek()
                .is_some_and(|(updated, _)| *updated == index)
            {
                gpu.clip_coefficients = clip_updates.next().expect("matching clip update").1;
                gpu.clip_planes.clone_from(&actor.clip_planes);
            }
            gpu.transparent = actor.is_transparent();
        }
        Ok(false)
    }

    fn actor_bind_group(&self, uniform: &wgpu::Buffer, lut: &wgpu::Texture) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpui-scene3d actor"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &lut.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Creates an offscreen colour+depth target.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions or exceeded host/device limits.
    pub fn create_target(&self, width: u32, height: u32) -> Result<GpuTarget, GpuRenderError> {
        self.create_target_with_limits(width, height, RenderLimits::default())
    }

    /// Creates a target within explicit frame-memory and device limits.
    ///
    /// # Errors
    ///
    /// Returns an error when dimensions or color/depth bytes exceed limits.
    pub fn create_target_with_limits(
        &self,
        width: u32,
        height: u32,
        mut limits: RenderLimits,
    ) -> Result<GpuTarget, GpuRenderError> {
        limits.max_dimension = limits
            .max_dimension
            .min(self.device.limits().max_texture_dimension_2d);
        limits.pixel_count(width, height, 8)?;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let color = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpui-scene3d target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpui-scene3d depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Ok(GpuTarget {
            color_view: color.create_view(&Default::default()),
            depth_view: depth.create_view(&Default::default()),
            #[cfg(not(target_family = "wasm"))]
            color,
            #[cfg(not(target_family = "wasm"))]
            limits,
            width,
            height,
        })
    }

    /// Draws an uploaded scene into a target. Only uniform buffers are
    /// rewritten; geometry buffers are untouched (GPU-resident).
    ///
    /// # Errors
    ///
    /// Returns an error for invalid camera parameters or staging limits.
    pub fn render(
        &self,
        scene: &mut GpuScene,
        camera: &Camera,
        target: &GpuTarget,
    ) -> Result<(), GpuRenderError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpui-scene3d frame"),
            });
        self.encode(&mut encoder, scene, camera, target)?;
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Encodes a scene without submitting the host's command encoder.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid camera parameters or staging limits.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &mut GpuScene,
        camera: &Camera,
        target: &GpuTarget,
    ) -> Result<(), GpuRenderError> {
        let background = scene.background;
        self.encode_to_views(
            encoder,
            scene,
            camera,
            GpuAttachments {
                color: &target.color_view,
                depth: &target.depth_view,
                size: [target.width, target.height],
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(background[0] * background[3]),
                    g: f64::from(background[1] * background[3]),
                    b: f64::from(background[2] * background[3]),
                    a: f64::from(background[3]),
                }),
            },
        )
    }

    /// Encodes into caller-owned views without a submit or pixel readback.
    ///
    /// The host owns attachment lifetime, format, size and submission ordering.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions, camera parameters or staging limits.
    pub fn encode_to_views(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &mut GpuScene,
        camera: &Camera,
        target: GpuAttachments<'_>,
    ) -> Result<(), GpuRenderError> {
        camera.validate()?;
        if target.size.contains(&0) {
            return Err(SceneError::InvalidSize(target.size[0], target.size[1]).into());
        }
        let uniform_bytes = (scene.actors.len() as u64).saturating_mul(SCENE_UNIFORM_SIZE as u64);
        if uniform_bytes > self.device.limits().max_buffer_size {
            return Err(SceneError::ResourceLimit {
                resource: "scene uniform staging bytes",
                requested: uniform_bytes,
                limit: self.device.limits().max_buffer_size,
            }
            .into());
        }
        let aspect = target.size[0] as f32 / target.size[1].max(1) as f32;
        let eye = camera.eye();
        for actor in &mut scene.actors {
            // Distance for transparent sorting: chunk origins are all the
            // dataset centre, so use the first chunk's origin.
            if let Some(first) = actor.chunks.first() {
                let o = first.origin;
                actor.distance =
                    ((o[0] - eye[0]).powi(2) + (o[1] - eye[1]).powi(2) + (o[2] - eye[2]).powi(2))
                        .sqrt();
            }
        }
        // Copies belong to this encoder, so encoding two cameras before one
        // host submission preserves both views. Queue::write_buffer would
        // otherwise make both passes observe the last camera's uniforms.
        let staging = (!scene.actors.is_empty()).then(|| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gpui-scene3d frame uniforms"),
                size: uniform_bytes,
                usage: wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            })
        });
        if let Some(staging) = &staging {
            let mut mapped = staging.slice(..).get_mapped_range_mut();
            for (index, actor) in scene.actors.iter().enumerate() {
                if let Some(first) = actor.chunks.first() {
                    let mvp = camera.view_projection_relative_to(first.origin, aspect);
                    let bytes = scene_uniform_block_with_equations(
                        &mvp,
                        actor.scalar_range,
                        actor.opacity,
                        &actor.clip_coefficients,
                    );
                    mapped
                        .slice(index * SCENE_UNIFORM_SIZE..(index + 1) * SCENE_UNIFORM_SIZE)
                        .copy_from_slice(&bytes);
                }
            }
            drop(mapped);
            staging.unmap();
            for (index, actor) in scene.actors.iter().enumerate() {
                encoder.copy_buffer_to_buffer(
                    staging,
                    index as u64 * SCENE_UNIFORM_SIZE as u64,
                    &actor.uniform,
                    0,
                    SCENE_UNIFORM_SIZE as u64,
                );
            }
        }
        scene.order.sort_unstable_by(|&left, &right| {
            let (a, b) = (&scene.actors[left], &scene.actors[right]);
            a.transparent
                .cmp(&b.transparent)
                .then_with(|| {
                    if a.transparent {
                        b.distance.total_cmp(&a.distance)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then_with(|| left.cmp(&right))
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gpui-scene3d pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: target.load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for &index in &scene.order {
                let actor = &scene.actors[index];
                pass.set_pipeline(if actor.transparent {
                    &self.pipeline_transparent
                } else {
                    &self.pipeline_opaque
                });
                pass.set_bind_group(0, &actor.bind_group, &[]);
                for chunk in &actor.chunks {
                    for (slot, buffer) in chunk.vertices.iter().enumerate() {
                        pass.set_vertex_buffer(slot as u32, buffer.slice(..));
                    }
                    pass.set_index_buffer(chunk.index.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..chunk.index_count, 0, 0..1);
                }
            }
        }
        Ok(())
    }

    /// Reads a target back to a CPU frame (QA and movie export only).
    ///
    /// # Errors
    ///
    /// Returns an error when staging/CPU storage exceeds the target's budget,
    /// allocation fails, or GPU mapping/readback fails.
    #[cfg(not(target_family = "wasm"))]
    pub fn read_rgba(&self, target: &GpuTarget) -> Result<Frame, GpuRenderError> {
        let layout = readback_layout(target.width, target.height, target.limits)?;
        if layout.buffer_bytes > self.device.limits().max_buffer_size {
            return Err(SceneError::ResourceLimit {
                resource: "GPU readback buffer bytes",
                requested: layout.buffer_bytes,
                limit: self.device.limits().max_buffer_size,
            }
            .into());
        }
        let bytes_per_row = layout.bytes_per_row;
        let padded = layout.padded;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpui-scene3d readback"),
            size: layout.buffer_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpui-scene3d readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(target.height),
                },
            },
            wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .map_err(|e| GpuRenderError::Device(e.to_string()))?;
        receiver
            .recv()
            .map_err(|e| GpuRenderError::Device(e.to_string()))?
            .map_err(|e| GpuRenderError::Device(e.to_string()))?;
        let data = slice.get_mapped_range();
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(layout.rgba_len)
            .map_err(|_| SceneError::Allocation {
                bytes: layout.rgba_len as u64,
            })?;
        for row in 0..target.height as usize {
            let start = row * padded as usize;
            rgba.extend_from_slice(&data[start..start + bytes_per_row as usize]);
        }
        drop(data);
        buffer.unmap();
        // Swizzle BGRA targets to RGBA.
        if self.format == wgpu::TextureFormat::Bgra8Unorm
            || self.format == wgpu::TextureFormat::Bgra8UnormSrgb
        {
            for px in rgba.as_chunks_mut::<4>().0 {
                px.swap(0, 2);
            }
        }
        // The sampleable target is premultiplied for compositing; exported
        // frames use straight RGBA, as required by PNG and image consumers.
        for pixel in rgba.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            if alpha == 255 {
                continue;
            }
            for channel in &mut pixel[..3] {
                *channel = if alpha == 0 {
                    0
                } else {
                    ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8
                };
            }
        }
        Ok(Frame {
            width: target.width,
            height: target.height,
            rgba,
        })
    }
}

#[cfg(not(target_family = "wasm"))]
struct ReadbackLayout {
    bytes_per_row: u32,
    padded: u32,
    buffer_bytes: u64,
    rgba_len: usize,
}

#[cfg(not(target_family = "wasm"))]
fn readback_layout(
    width: u32,
    height: u32,
    limits: RenderLimits,
) -> Result<ReadbackLayout, SceneError> {
    let pixels = limits.pixel_count(width, height, 16)?;
    let bytes_per_row = u64::from(width) * 4;
    let padded = bytes_per_row.div_ceil(256) * 256;
    let buffer_bytes = padded
        .checked_mul(u64::from(height))
        .ok_or(SceneError::ResourceLimit {
            resource: "readback bytes",
            requested: u64::MAX,
            limit: limits.max_frame_bytes,
        })?;
    // Color, depth, CPU RGBA and padded staging coexist until mapping ends.
    let working_bytes = (pixels as u64)
        .checked_mul(12)
        .and_then(|bytes| bytes.checked_add(buffer_bytes))
        .ok_or(SceneError::ResourceLimit {
            resource: "readback bytes",
            requested: u64::MAX,
            limit: limits.max_frame_bytes,
        })?;
    if working_bytes > limits.max_frame_bytes || padded > u64::from(u32::MAX) {
        return Err(SceneError::ResourceLimit {
            resource: "readback bytes",
            requested: working_bytes,
            limit: limits.max_frame_bytes,
        });
    }
    Ok(ReadbackLayout {
        bytes_per_row: bytes_per_row as u32,
        padded: padded as u32,
        buffer_bytes,
        rgba_len: pixels * 4,
    })
}

fn material_lut(actor: &crate::Actor) -> Result<(Vec<u8>, [f32; 2]), SceneError> {
    if let (Some(transfer), Some(_)) = (&actor.transfer, actor.scalars()) {
        Ok((transfer.lut_bytes(), transfer.gpu_range()?))
    } else {
        let color = actor.color.map(|value| (value * 255.0) as u8);
        Ok((color.repeat(256), [0.0, 1.0]))
    }
}

fn validate_format(format: wgpu::TextureFormat) -> Result<(), GpuRenderError> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb => Ok(()),
        _ => Err(GpuRenderError::UnsupportedFormat(format)),
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::{Actor, TransferFunction};
    use gpui_mesh::{DataSet, Field};

    fn renderer() -> Option<GpuRenderer> {
        match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
            Ok(renderer) => Some(renderer),
            Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
                eprintln!("SKIP: scene3d GPU tests: no adapter");
                None
            }
            Err(error) => panic!("scene3d GPU initialization failed: {error}"),
        }
    }

    #[test]
    fn gpu_render_matches_scene_and_updates_by_uniform_only() {
        let Some(renderer) = renderer() else {
            return;
        };
        let dataset = DataSet::structured_grid("g", [6, 6, 6], [-1.0; 3], [0.4; 3])
            .unwrap()
            .with_field(Field::scalar(
                "p",
                (0..216).map(|i| i as f64 / 216.0).collect::<Vec<_>>(),
            ));
        let scene = Scene::new().with_actor(
            Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])),
        );
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut gpu_scene = renderer.upload(&scene).unwrap();
        assert!(gpu_scene.geometry_bytes > 0);
        let target = renderer.create_target(96, 96).unwrap();
        renderer.render(&mut gpu_scene, &camera, &target).unwrap();
        let frame = renderer.read_rgba(&target).unwrap();
        let centre = frame.rgba[((48 * 96 + 48) * 4) as usize..][..3].to_vec();
        assert_ne!(centre, vec![20, 23, 28], "centre shows the surface");

        // Orbit and re-render: same buffers, different picture.
        let mut orbited = camera;
        orbited.orbit(1.2, 0.3);
        renderer.render(&mut gpu_scene, &orbited, &target).unwrap();
        let frame2 = renderer.read_rgba(&target).unwrap();
        let changed = frame
            .rgba
            .iter()
            .zip(frame2.rgba.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed > 1_000, "orbit changes the frame ({changed})");
    }

    #[test]
    fn shared_device_encoding_preserves_cameras_in_one_host_submission() {
        let Some(owner) = renderer() else { return };
        let renderer = GpuRenderer::from_shared_device(
            Arc::clone(&owner.device),
            Arc::clone(&owner.queue),
            owner.format,
        )
        .unwrap();
        assert!(Arc::ptr_eq(&owner.device, &renderer.device));
        assert!(Arc::ptr_eq(&owner.queue, &renderer.queue));
        let dataset =
            DataSet::structured_grid("asymmetric", [3, 4, 5], [-1.0; 3], [0.5; 3]).unwrap();
        let scene = Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.2, 0.1, 1.0]));
        let first_camera = Camera::fitted(scene.bounds().unwrap());
        let mut second_camera = first_camera;
        second_camera.orbit(1.4, 0.4);
        let mut uploaded = renderer.upload(&scene).unwrap();
        let first = renderer.create_target(48, 48).unwrap();
        let second = renderer.create_target(48, 48).unwrap();
        let mut encoder = owner.device.create_command_encoder(&Default::default());
        renderer
            .encode(&mut encoder, &mut uploaded, &first_camera, &first)
            .unwrap();
        renderer
            .encode(&mut encoder, &mut uploaded, &second_camera, &second)
            .unwrap();
        owner.queue.submit([encoder.finish()]);
        let actual_first = renderer.read_rgba(&first).unwrap();
        let actual_second = renderer.read_rgba(&second).unwrap();
        assert_ne!(actual_first.rgba, actual_second.rgba);
        renderer
            .render(&mut uploaded, &first_camera, &first)
            .unwrap();
        assert_eq!(actual_first.rgba, renderer.read_rgba(&first).unwrap().rgba);
        renderer
            .render(&mut uploaded, &second_camera, &second)
            .unwrap();
        assert_eq!(
            actual_second.rgba,
            renderer.read_rgba(&second).unwrap().rgba
        );

        // A host can preserve its existing color when encoding empty content.
        let mut empty = renderer.upload(&Scene::new()).unwrap();
        let mut encoder = owner.device.create_command_encoder(&Default::default());
        renderer
            .encode_to_views(
                &mut encoder,
                &mut empty,
                &first_camera,
                GpuAttachments {
                    color: first.color_view(),
                    depth: &first.depth_view,
                    size: [48, 48],
                    load: wgpu::LoadOp::Load,
                },
            )
            .unwrap();
        owner.queue.submit([encoder.finish()]);
        assert_eq!(actual_first.rgba, renderer.read_rgba(&first).unwrap().rgba);
    }

    #[test]
    fn material_updates_keep_geometry_and_encoded_material_revisions() {
        let Some(renderer) = renderer() else {
            return;
        };
        let dataset = DataSet::structured_grid("material", [2; 3], [-0.5; 3], [1.0; 3]).unwrap();
        let mut scene = Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.0, 0.0, 1.0]));
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut uploaded = renderer.upload(&scene).unwrap();
        let buffer = &uploaded.actors[0].chunks[0].vertices[0] as *const wgpu::Buffer;
        scene.clear_preparation();
        let red = renderer.create_target(32, 32).unwrap();
        let green = renderer.create_target(32, 32).unwrap();
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        renderer
            .encode(&mut encoder, &mut uploaded, &camera, &red)
            .unwrap();
        scene.actors[0].color = [0.0, 1.0, 0.0, 1.0];
        assert!(!renderer.sync(&mut uploaded, &scene).unwrap());
        assert_eq!(
            buffer,
            &uploaded.actors[0].chunks[0].vertices[0] as *const wgpu::Buffer
        );
        assert_eq!(scene.preparation_stats().retained_bytes, 0);
        assert_eq!(scene.preparation_stats().geometry_builds, 1);
        renderer
            .encode(&mut encoder, &mut uploaded, &camera, &green)
            .unwrap();
        renderer.queue.submit([encoder.finish()]);
        let center = ((16 * 32 + 16) * 4) as usize;
        let r = renderer.read_rgba(&red).unwrap();
        let g = renderer.read_rgba(&green).unwrap();
        assert!(r.rgba[center] > 100 && r.rgba[center + 1] == 0);
        assert!(g.rgba[center] == 0 && g.rgba[center + 1] > 100);
        scene.actors[0].clip_planes = vec![
            crate::ClipPlane {
                origin: [0.0; 3],
                normal: [1.0, 0.0, 0.0]
            };
            7
        ];
        assert!(matches!(
            renderer.sync(&mut uploaded, &scene),
            Err(GpuRenderError::UnsupportedFeature(_))
        ));
    }

    #[test]
    fn readback_exports_straight_rgba_and_rejects_silent_feature_loss() {
        let Some(renderer) = renderer() else {
            return;
        };
        let mut scene = Scene::new();
        scene.background = [1.0, 0.0, 0.0, 0.5];
        let target = renderer.create_target(4, 4).unwrap();
        let mut uploaded = renderer.upload(&scene).unwrap();
        renderer
            .render(&mut uploaded, &Camera::default(), &target)
            .unwrap();
        let frame = renderer.read_rgba(&target).unwrap();
        assert_eq!(&frame.rgba[..4], &[255, 0, 0, 128]);
        let mut flat = Actor::new(DataSet::surface(
            "flat",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        ))
        .color([0.0, 1.0, 0.0, 0.5]);
        flat.transfer = Some(TransferFunction::viridis([0.0, 1.0]));
        let mut flat_scene = Scene::new().with_actor(flat);
        flat_scene.background = [0.0; 4];
        let camera = Camera::fitted(flat_scene.bounds().unwrap());
        let target = renderer.create_target(32, 32).unwrap();
        let mut uploaded = renderer.upload(&flat_scene).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let frame = renderer.read_rgba(&target).unwrap();
        // Flat colors pass through the byte LUT: 0.5 truncates to 127.
        let green = frame
            .rgba
            .chunks_exact(4)
            .max_by_key(|pixel| pixel[1])
            .unwrap();
        assert!(
            green[0] == 0 && green[1] > 100 && green[2] == 0 && green[3] == 127,
            "flat color with inactive transfer: {green:?}"
        );
        let cpu = crate::render(&flat_scene, &camera, 32, 32).unwrap();
        let mut compared = 0;
        for (gpu, cpu) in frame.rgba.chunks_exact(4).zip(cpu.rgba.chunks_exact(4)) {
            if gpu[3] > 0 && cpu[3] > 0 {
                compared += 1;
                assert_eq!(gpu[3], cpu[3]);
                assert!(
                    gpu.iter().zip(cpu).all(|(a, b)| a.abs_diff(*b) <= 2),
                    "straight RGBA CPU/GPU quantization: {cpu:?}, {gpu:?}"
                );
            }
        }
        assert!(compared > 50);
        let actor = Actor::new(DataSet::surface(
            "triangle",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        ))
        .wireframe(true);
        assert!(matches!(
            renderer.upload(&Scene::new().with_actor(actor)),
            Err(GpuRenderError::UnsupportedFeature("wireframe"))
        ));
    }

    #[test]
    fn clipped_surface_is_translation_invariant() {
        let Some(renderer) = renderer() else { return };
        let mut expected = None;
        for center in [0.0, 1000.125, 1e9 + 0.125, 1e12 + 0.125] {
            let dataset = DataSet::structured_grid(
                "translated",
                [3; 3],
                [center - 1.0, -1.0, -1.0],
                [1.0; 3],
            )
            .unwrap();
            let scene =
                Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.0, 0.0, 1.0]).clip(
                    crate::ClipPlane {
                        origin: [center, 0.0, 0.0],
                        normal: [1.0, 0.0, 0.0],
                    },
                ));
            let camera = Camera::fitted(scene.bounds().unwrap());
            let mut uploaded = renderer.upload(&scene).unwrap();
            let target = renderer.create_target(64, 64).unwrap();
            renderer.render(&mut uploaded, &camera, &target).unwrap();
            let frame = renderer.read_rgba(&target).unwrap();
            let red = frame
                .rgba
                .chunks_exact(4)
                .filter(|p| p[0] > p[1].saturating_add(20))
                .count();
            assert!(red > 100, "surface disappeared at {center}");
            if let Some(count) = expected {
                assert_eq!(red, count, "translation {center}");
            } else {
                expected = Some(red);
            }
            {
                let cpu = crate::render(&scene, &camera, 64, 64).unwrap();
                let cpu_red = cpu
                    .rgba
                    .chunks_exact(4)
                    .filter(|p| p[0] > p[1].saturating_add(20))
                    .count();
                assert_eq!(red, cpu_red);
            }
        }
    }

    #[test]
    fn oblique_multi_actor_clipping_matches_cpu_coverage_at_large_origins() {
        let Some(renderer) = renderer() else {
            return;
        };
        let target = renderer.create_target(96, 64).unwrap();
        let coverage = |frame: &Frame, channel: usize| {
            frame
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[channel] > pixel[1 - channel].saturating_add(20))
                .count()
        };
        let mut reference = None;
        for offset in [0.0, 1000.125, 1e9 + 0.125, 1e12 + 0.125] {
            let scene = crate::scene::translated_clipped_scene(offset);
            let camera = Camera::fitted(scene.bounds().unwrap());
            let mut uploaded = renderer.upload(&scene).unwrap();
            renderer.render(&mut uploaded, &camera, &target).unwrap();
            let gpu = renderer.read_rgba(&target).unwrap();
            let cpu = crate::render(&scene, &camera, 96, 64).unwrap();
            for channel in 0..2 {
                let (gpu_count, cpu_count) = (coverage(&gpu, channel), coverage(&cpu, channel));
                assert!(gpu_count > 20);
                assert!(
                    gpu_count.abs_diff(cpu_count) <= 4,
                    "channel {channel}, offset {offset}: GPU {gpu_count}, CPU {cpu_count}"
                );
            }
            if let Some(reference) = &reference {
                assert_eq!(&gpu.rgba, reference, "GPU translation {offset}");
            } else {
                reference = Some(gpu.rgba);
            }
        }
    }

    #[test]
    fn clipping_is_invariant_to_normal_magnitude_and_distant_tangential_points() {
        let Some(renderer) = renderer() else {
            return;
        };
        let dataset = DataSet::structured_grid("equations", [3; 3], [-1.0; 3], [1.0; 3]).unwrap();
        let mut scene =
            Scene::new().with_actor(Actor::new(dataset).color([1.0, 0.0, 0.0, 1.0]).clip(
                crate::ClipPlane {
                    origin: [0.125, 0.0, 0.0],
                    normal: [1.0, 1.0, 0.0],
                },
            ));
        let camera = Camera::fitted(scene.bounds().unwrap());
        let target = renderer.create_target(64, 64).unwrap();
        let mut uploaded = renderer.upload(&scene).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let reference_gpu = renderer.read_rgba(&target).unwrap();
        let reference_cpu = crate::render(&scene, &camera, 64, 64).unwrap();
        for magnitude in [1e-300, 1.0, 1e300] {
            for tangent in [0.0, 1e12] {
                scene.actors[0].clip_planes[0] = crate::ClipPlane {
                    origin: [tangent + 0.125, -tangent, 1e250],
                    normal: [magnitude, magnitude, 0.0],
                };
                assert!(!renderer.sync(&mut uploaded, &scene).unwrap());
                renderer.render(&mut uploaded, &camera, &target).unwrap();
                assert_eq!(
                    reference_gpu.rgba,
                    renderer.read_rgba(&target).unwrap().rgba
                );
                assert_eq!(
                    reference_cpu.rgba,
                    crate::render(&scene, &camera, 64, 64).unwrap().rgba
                );
            }
        }
        scene.actors[0].clip_planes.clear();
        renderer.sync(&mut uploaded, &scene).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let full_gpu = renderer.read_rgba(&target).unwrap();
        let full_cpu = crate::render(&scene, &camera, 64, 64).unwrap();
        for (origin, normal, keep) in [
            ([1e250, 0.0, 0.0], [1.0, 0.0, 0.0], false),
            ([1e250, 0.0, 0.0], [-1.0, 0.0, 0.0], true),
            ([1e250; 3], [0.0; 3], true),
        ] {
            scene.actors[0].clip_planes = vec![crate::ClipPlane { origin, normal }];
            renderer.sync(&mut uploaded, &scene).unwrap();
            renderer.render(&mut uploaded, &camera, &target).unwrap();
            let gpu = renderer.read_rgba(&target).unwrap();
            let cpu = crate::render(&scene, &camera, 64, 64).unwrap();
            if keep {
                assert_eq!(full_gpu.rgba, gpu.rgba);
                assert_eq!(full_cpu.rgba, cpu.rgba);
            } else {
                assert!(gpu.rgba.chunks_exact(4).all(|p| p[0] < 30));
                assert!(cpu.rgba.chunks_exact(4).all(|p| p[0] < 30));
            }
        }
    }

    #[test]
    fn unsupported_clip_precision_does_not_partially_update_other_materials() {
        let Some(renderer) = renderer() else {
            return;
        };
        let mut scene = crate::scene::translated_clipped_scene(0.0);
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut uploaded = renderer.upload(&scene).unwrap();
        let target = renderer.create_target(64, 64).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let before = renderer.read_rgba(&target).unwrap();
        scene.background = [1.0; 4];
        scene.actors[0].color = [0.0, 0.0, 1.0, 1.0];
        scene.actors[1].clip_planes[0].normal = [1.0, 1e-100, 0.0];
        assert!(matches!(
            renderer.sync(&mut uploaded, &scene),
            Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
        ));
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        assert_eq!(before.rgba, renderer.read_rgba(&target).unwrap().rgba);
        let sensitive = crate::scene::coefficient_sensitive_scene();
        assert!(matches!(
            renderer.upload(&sensitive),
            Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
        ));
    }

    #[test]
    fn invalid_camera_errors_before_changing_the_render_target() {
        let Some(renderer) = renderer() else {
            return;
        };
        let mut uploaded = renderer.upload(&Scene::new()).unwrap();
        let target = renderer.create_target(4, 4).unwrap();
        let mut camera = Camera::default();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let before = renderer.read_rgba(&target).unwrap();
        camera.near = camera.far;
        assert!(matches!(
            renderer.render(&mut uploaded, &camera, &target),
            Err(GpuRenderError::Scene(SceneError::InvalidCamera(_)))
        ));
        assert_eq!(before.rgba, renderer.read_rgba(&target).unwrap().rgba);
    }

    #[test]
    fn rejected_transfer_updates_preserve_uploaded_materials_and_background() {
        let Some(renderer) = renderer() else { return };
        let dataset = DataSet::structured_grid("transaction", [2; 3], [-0.5; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 8]));
        let base = Scene::new()
            .with_actor(Actor::new(dataset.clone()).color([1.0, 0.0, 0.0, 1.0]))
            .with_actor(
                Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])),
            );
        let camera = Camera::fitted(base.bounds().unwrap());
        let target = renderer.create_target(32, 32).unwrap();
        let mut uploaded = renderer.upload(&base).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let reference = renderer.read_rgba(&target).unwrap();
        for range in [[f64::NAN, 1.0], [-f64::MAX, f64::MAX], [1e12, 1e12 + 0.125]] {
            let mut changed = base.clone();
            changed.background = [1.0; 4];
            changed.actors[0].color = [0.0, 0.0, 1.0, 1.0];
            changed.actors[1].transfer.as_mut().unwrap().range = range;
            assert!(matches!(
                renderer.sync(&mut uploaded, &changed),
                Err(GpuRenderError::Scene(
                    SceneError::InvalidMaterial(_) | SceneError::UnsupportedPrecision(_)
                ))
            ));
            assert!(renderer.upload(&changed).is_err());
            assert_eq!(uploaded.actors[0].color, base.actors[0].color);
            assert_eq!(uploaded.background, base.background);
            renderer.render(&mut uploaded, &camera, &target).unwrap();
            assert_eq!(reference.rgba, renderer.read_rgba(&target).unwrap().rgba);
        }
    }

    #[test]
    fn masked_scalar_placeholders_do_not_occlude_the_background() {
        let Some(renderer) = renderer() else { return };
        let mut field = Field::scalar("p", vec![0.5; 27]);
        field.valid = Some(vec![false; 27].into());
        let dataset = DataSet::structured_grid("masked", [3; 3], [-1.0; 3], [1.0; 3])
            .unwrap()
            .with_field(field);
        let scene = Scene::new().with_actor(
            Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])),
        );
        let camera = Camera::fitted(scene.bounds().unwrap());
        let target = renderer.create_target(32, 32).unwrap();
        let mut uploaded = renderer.upload(&scene).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        let masked = renderer.read_rgba(&target).unwrap();
        let mut empty = renderer.upload(&Scene::new()).unwrap();
        renderer.render(&mut empty, &camera, &target).unwrap();
        assert_eq!(masked.rgba, renderer.read_rgba(&target).unwrap().rgba);
    }

    #[test]
    fn readback_budget_includes_padding_and_live_targets() {
        let limits = RenderLimits {
            max_dimension: 1000,
            max_frame_bytes: 268,
        };
        let layout = readback_layout(1, 1, limits).unwrap();
        assert_eq!(layout.buffer_bytes, 256);
        assert_eq!(layout.rgba_len, 4);
        assert!(
            readback_layout(
                1,
                1,
                RenderLimits {
                    max_frame_bytes: 267,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(readback_layout(65_536, 65_536, RenderLimits::default()).is_err());
    }
}
