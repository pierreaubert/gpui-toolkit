//! Window-owned scene presentation on the host's wgpu context.

// Rust guideline compliant 2026-02-21

use crate::presentation::RenderKey;
use crate::{Camera, GpuRenderError, GpuRenderer, GpuScene, GpuTarget, Scene, SceneError};
use gpui::{Bounds, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wgpu::util::DeviceExt;

fn supports_topology(scene: &Scene) -> Result<bool, SceneError> {
    for actor in scene.actors.iter().filter(|actor| actor.visible) {
        if !crate::primitives::gpu_supported(actor)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) struct RegisteredSceneDraw {
    id: gpui::CustomDrawId,
    draw: Rc<SceneDraw>,
}

impl RegisteredSceneDraw {
    pub(crate) fn new() -> Self {
        let draw = Rc::new(SceneDraw {
            state: RefCell::new(DrawState::default()),
        });
        let id = gpui::register_custom_draw(Rc::new(WgpuCustomDrawAdapter(draw.clone())));
        Self { id, draw }
    }

    pub(crate) fn id(&self) -> gpui::CustomDrawId {
        self.id
    }

    pub(crate) fn update(
        &self,
        scene: &Scene,
        camera: Camera,
        size: [u32; 2],
    ) -> Result<bool, SceneError> {
        let mut state = self.draw.state.borrow_mut();
        if let Some(input) = &state.input
            && input.key.matches(scene, camera, size)?
        {
            return Ok(input.gpu_supported);
        }
        let key = RenderKey::new(scene, camera, size)?;
        let same_geometry = if let Some(input) = &state.input {
            input.key.geometry_matches(scene)?
        } else {
            false
        };
        let topology_supported = if same_geometry {
            state
                .input
                .as_ref()
                .expect("existing input")
                .topology_supported
        } else {
            state.compatibility_scans += 1;
            supports_topology(scene)?
        };
        let same_precision = same_geometry
            && state.input.as_ref().is_some_and(|input| {
                input
                    .scene
                    .actors
                    .iter()
                    .zip(&scene.actors)
                    .all(|(old, actor)| {
                        !actor.visible
                            || (old.clip_planes == actor.clip_planes
                                && old.active_transfer().map(|tf| tf.range)
                                    == actor.active_transfer().map(|tf| tf.range))
                    })
            });
        let clip_limit = scene
            .actors
            .iter()
            .filter(|actor| actor.visible)
            .all(|actor| actor.clip_planes.len() <= crate::shaders::MAX_CLIP_PLANES);
        let mut ranges_supported = true;
        if !same_precision {
            for actor in scene.actors.iter().filter(|actor| actor.visible) {
                if let Some(transfer) = actor.active_transfer() {
                    match transfer.gpu_range() {
                        Ok(_) => {}
                        Err(SceneError::UnsupportedPrecision(_)) => {
                            ranges_supported = false;
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        let needs_origins = ranges_supported
            && topology_supported
            && clip_limit
            && scene
                .actors
                .iter()
                .any(|actor| actor.visible && !actor.clip_planes.is_empty());
        let origins = if needs_origins {
            state
                .input
                .as_ref()
                .filter(|_| same_geometry)
                .and_then(|input| input.origins.clone())
                .or_else(|| {
                    Some(
                        scene
                            .actors
                            .iter()
                            .filter(|actor| actor.visible)
                            .map(|actor| {
                                actor
                                    .dataset
                                    .bounds()
                                    .map_or([0.0; 3], |bounds| bounds.center())
                            })
                            .collect::<Vec<_>>()
                            .into(),
                    )
                })
        } else {
            None
        };
        let gpu_supported = if !ranges_supported || !topology_supported || !clip_limit {
            false
        } else if same_precision {
            state.input.as_ref().expect("existing input").gpu_supported
        } else if let Some(origins) = &origins {
            let mut supported = true;
            for (actor, origin) in scene
                .actors
                .iter()
                .filter(|actor| actor.visible)
                .zip(origins.iter())
            {
                match crate::shaders::packed_clip_planes(&actor.clip_planes, *origin) {
                    Ok(_) => {}
                    Err(SceneError::UnsupportedPrecision(_)) => {
                        supported = false;
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            supported
        } else {
            true
        };
        let revision = state
            .input
            .as_ref()
            .map_or(1, |input| input.revision.wrapping_add(1));
        state.input = Some(Input {
            key,
            scene: scene.clone(),
            camera,
            size,
            revision,
            origins,
            topology_supported,
            gpu_supported,
        });
        if !gpu_supported {
            state.resources = None;
            state.failure = None;
        }
        Ok(gpu_supported)
    }
}

impl Drop for RegisteredSceneDraw {
    fn drop(&mut self) {
        gpui::unregister_custom_draw(self.id);
    }
}

struct Input {
    key: RenderKey,
    scene: Scene,
    camera: Camera,
    size: [u32; 2],
    revision: u64,
    origins: Option<Arc<[[f64; 3]]>>,
    topology_supported: bool,
    gpu_supported: bool,
}

#[derive(Default)]
struct DrawState {
    input: Option<Input>,
    resources: Option<Resources>,
    compatibility_scans: u64,
    initialization_attempts: u64,
    failure: Option<ResourceFailure>,
}

struct ResourceFailure {
    device: Arc<wgpu::Device>,
    format: wgpu::TextureFormat,
    key: RenderKey,
}

impl ResourceFailure {
    fn new(ctx: &WgpuContext, format: wgpu::TextureFormat, input: &Input) -> Self {
        Self {
            device: ctx.device.clone(),
            format,
            key: RenderKey::new(&input.scene, input.camera, input.size)
                .expect("presentation input already validated"),
        }
    }

    fn matches(&self, ctx: &WgpuContext, format: wgpu::TextureFormat, input: &Input) -> bool {
        Arc::ptr_eq(&self.device, &ctx.device)
            && self.format == format
            && self
                .key
                .resource_inputs_match(&input.scene, input.size)
                .unwrap_or(false)
    }
}

struct SceneDraw {
    state: RefCell<DrawState>,
}

impl gpui::CustomDraw for SceneDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct Resources {
    device: Arc<wgpu::Device>,
    format: wgpu::TextureFormat,
    renderer: GpuRenderer,
    scene: GpuScene,
    target: GpuTarget,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture: wgpu::BindGroup,
    quad: Option<([f32; 24], wgpu::Buffer)>,
    rendered_revision: Option<u64>,
}

const COMPOSITE: &str = r"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> Vertex {
    return Vertex(vec4<f32>(position, 0.0, 1.0), uv);
}
@fragment fn fs(input: Vertex) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, input.uv);
}
";

impl Resources {
    fn new(
        ctx: &WgpuContext,
        format: wgpu::TextureFormat,
        input: &Input,
    ) -> Result<Self, GpuRenderError> {
        let renderer = GpuRenderer::from_shared_device(
            ctx.device.clone(),
            ctx.queue.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
        )?;
        let scene = renderer.upload(&input.scene)?;
        input.scene.clear_preparation();
        let target = renderer.create_target(input.size[0], input.size[1])?;
        let layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("scene3d composite"),
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
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scene3d composite"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let texture = texture_binding(&ctx.device, &layout, &sampler, &target);
        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("scene3d composite"),
                source: wgpu::ShaderSource::Wgsl(COMPOSITE.into()),
            });
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("scene3d composite"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("scene3d composite"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 16,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        Ok(Self {
            device: ctx.device.clone(),
            format,
            renderer,
            scene,
            target,
            pipeline,
            layout,
            sampler,
            texture,
            quad: None,
            rendered_revision: None,
        })
    }
}

fn texture_binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    target: &GpuTarget,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene3d composite"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(target.color_view()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

impl WgpuCustomDraw for SceneDraw {
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
        let mut state = self.state.borrow_mut();
        let DrawState {
            input,
            resources,
            failure,
            initialization_attempts,
            ..
        } = &mut *state;
        let Some(input) = input.as_ref() else {
            return;
        };
        if !input.gpu_supported {
            return;
        }
        // Stable failures must not recreate pipelines or repack geometry on
        // every paint/camera tick. Only changed resource inputs retry.
        if failure
            .as_ref()
            .is_some_and(|failed| failed.matches(ctx, target_format, input))
        {
            return;
        }
        *failure = None;
        if resources
            .as_ref()
            .is_none_or(|r| !Arc::ptr_eq(&r.device, &ctx.device) || r.format != target_format)
        {
            // Drop the old device's resources before allocating replacements.
            *resources = None;
            *initialization_attempts += 1;
            let Ok(created) = Resources::new(ctx, target_format, input) else {
                input.scene.clear_preparation();
                *failure = Some(ResourceFailure::new(ctx, target_format, input));
                return;
            };
            *resources = Some(created);
        }
        let resource = resources.as_mut().expect("resources initialized");
        if [resource.target.width, resource.target.height] != input.size {
            let Ok(target) = resource
                .renderer
                .create_target(input.size[0], input.size[1])
            else {
                *resources = None;
                *failure = Some(ResourceFailure::new(ctx, target_format, input));
                return;
            };
            resource.target = target;
            resource.texture = texture_binding(
                &ctx.device,
                &resource.layout,
                &resource.sampler,
                &resource.target,
            );
            resource.rendered_revision = None;
        }
        if resource.rendered_revision != Some(input.revision) {
            if resource
                .renderer
                .sync(&mut resource.scene, &input.scene)
                .is_err()
            {
                input.scene.clear_preparation();
                *resources = None;
                *failure = Some(ResourceFailure::new(ctx, target_format, input));
                return;
            }
            input.scene.clear_preparation();
            if resource
                .renderer
                .encode(
                    encoder,
                    &mut resource.scene,
                    &input.camera,
                    &resource.target,
                )
                .is_err()
            {
                *resources = None;
                *failure = Some(ResourceFailure::new(ctx, target_format, input));
                return;
            }
            resource.rendered_revision = Some(input.revision);
        }
        let Some((quad, scissor)) = composite_quad(bounds, full_bounds, scale_factor, target_size)
        else {
            return;
        };
        if resource
            .quad
            .as_ref()
            .is_none_or(|(current, _)| *current != quad)
        {
            resource.quad = Some((
                quad,
                ctx.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("scene3d composite quad"),
                        contents: bytemuck::cast_slice(&quad),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
            ));
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene3d composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
        pass.set_pipeline(&resource.pipeline);
        pass.set_bind_group(0, &resource.texture, &[]);
        pass.set_vertex_buffer(
            0,
            resource
                .quad
                .as_ref()
                .expect("quad initialized")
                .1
                .slice(..),
        );
        pass.draw(0..6, 0..1);
    }
}

fn composite_quad(
    bounds: Bounds<Pixels>,
    full: Bounds<Pixels>,
    scale: f32,
    target: [u32; 2],
) -> Option<([f32; 24], [u32; 4])> {
    if target.contains(&0) || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let full = [
        f32::from(full.origin.x) * scale,
        f32::from(full.origin.y) * scale,
        f32::from(full.size.width) * scale,
        f32::from(full.size.height) * scale,
    ];
    let x = f32::from(bounds.origin.x) * scale;
    let y = f32::from(bounds.origin.y) * scale;
    let width = f32::from(bounds.size.width) * scale;
    let height = f32::from(bounds.size.height) * scale;
    if [x, y, width, height].iter().any(|value| !value.is_finite()) || width <= 0.0 || height <= 0.0
    {
        return None;
    }
    let rect = [
        x.max(0.0),
        y.max(0.0),
        (x + f32::from(bounds.size.width) * scale).min(target[0] as f32),
        (y + f32::from(bounds.size.height) * scale).min(target[1] as f32),
    ];
    if full.iter().chain(rect.iter()).any(|v| !v.is_finite())
        || full[2] <= 0.0
        || full[3] <= 0.0
        || rect[2] <= rect[0]
        || rect[3] <= rect[1]
    {
        return None;
    }
    let vertex = |x: f32, y: f32| {
        [
            2.0 * x / target[0] as f32 - 1.0,
            1.0 - 2.0 * y / target[1] as f32,
            (x - full[0]) / full[2],
            (y - full[1]) / full[3],
        ]
    };
    let [left, top, right, bottom] = rect;
    let vertices = [
        vertex(left, top),
        vertex(right, top),
        vertex(right, bottom),
        vertex(left, top),
        vertex(right, bottom),
        vertex(left, bottom),
    ];
    let mut quad = [0.0; 24];
    for (index, vertex) in vertices.iter().enumerate() {
        quad[index * 4..index * 4 + 4].copy_from_slice(vertex);
    }
    let left = left.floor() as u32;
    let top = top.floor() as u32;
    let right = (right.ceil() as u32).min(target[0]);
    let bottom = (bottom.ceil() as u32).min(target[1]);
    Some((quad, [left, top, right - left, bottom - top]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};

    #[test]
    fn primitive_precision_verdict_is_retained_and_invalidated_by_field_revisions() {
        use gpui_mesh::{CellType, Cells, DataSet, Field};
        let data = DataSet::new(
            "mixed",
            vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            Cells::from_cells(&[(CellType::Line, &[0, 1]), (CellType::Vertex, &[0])]),
        )
        .with_field(Field::scalar("p", vec![0.0, 1.0]));
        let mut scene = Scene::new().with_actor(
            crate::Actor::new(data).coloured_by("p", crate::TransferFunction::viridis([0.0, 1.0])),
        );
        let mut camera = Camera::fitted(scene.bounds().unwrap());
        let draw = RegisteredSceneDraw::new();
        assert!(draw.update(&scene, camera, [96, 64]).unwrap());
        for _ in 0..24 {
            camera.orbit(0.01, 0.0);
            scene.actors[0].opacity = 0.5;
            assert!(draw.update(&scene, camera, [96, 64]).unwrap());
        }
        assert_eq!(draw.draw.state.borrow().compatibility_scans, 1);
        scene.actors[0].dataset.point_data[0] = Field::scalar("p", vec![1e300, 1.0]);
        assert!(!draw.update(&scene, camera, [96, 64]).unwrap());
        for _ in 0..24 {
            camera.orbit(0.01, 0.0);
            assert!(!draw.update(&scene, camera, [96, 64]).unwrap());
        }
        assert_eq!(draw.draw.state.borrow().compatibility_scans, 2);
        scene.actors[0].dataset.point_data[0] = Field::scalar("p", vec![0.0, 1.0]);
        assert!(draw.update(&scene, camera, [96, 64]).unwrap());
        assert_eq!(draw.draw.state.borrow().compatibility_scans, 3);
    }

    #[test]
    fn compatibility_and_origin_metadata_are_retained_across_camera_updates() {
        for wireframe in [false, true] {
            let mut scene = crate::scene::translated_clipped_scene(1e12 + 0.125);
            for actor in &mut scene.actors {
                actor.wireframe = wireframe;
            }
            let draw = RegisteredSceneDraw::new();
            let mut camera = Camera::fitted(scene.bounds().unwrap());
            assert!(draw.update(&scene, camera, [96, 64]).unwrap());
            let origins = draw
                .draw
                .state
                .borrow()
                .input
                .as_ref()
                .unwrap()
                .origins
                .clone()
                .unwrap();
            for _ in 0..24 {
                camera.orbit(0.01, 0.0);
                assert!(draw.update(&scene, camera, [96, 64]).unwrap());
            }
            let state = draw.draw.state.borrow();
            assert_eq!(state.compatibility_scans, 1);
            assert!(Arc::ptr_eq(
                &origins,
                state.input.as_ref().unwrap().origins.as_ref().unwrap()
            ));
        }
    }

    #[test]
    fn coefficient_precision_failure_selects_software_and_caches_the_verdict() {
        let mut scene = crate::scene::coefficient_sensitive_scene();
        let mut camera = Camera::fitted(scene.bounds().unwrap());
        let draw = RegisteredSceneDraw::new();
        assert!(!draw.update(&scene, camera, [96, 64]).unwrap());
        let reference = crate::render(&scene, &camera, 96, 64).unwrap();
        let mut explicit = scene.clone();
        explicit.actors[0].clip_planes[0].normal = [0.0, 1.0, 0.0];
        assert_eq!(
            reference.rgba,
            crate::render(&explicit, &camera, 96, 64).unwrap().rgba
        );
        for _ in 0..24 {
            camera.orbit(0.01, 0.0);
            assert!(!draw.update(&scene, camera, [96, 64]).unwrap());
        }
        assert_eq!(draw.draw.state.borrow().compatibility_scans, 1);
        assert!(draw.draw.state.borrow().resources.is_none());
        scene.actors[0].clip_planes[0].normal = [0.0, 1.0, 0.0];
        assert!(draw.update(&scene, camera, [96, 64]).unwrap());
        assert_eq!(draw.draw.state.borrow().compatibility_scans, 1);
    }

    #[test]
    fn registration_lifetime_is_owned_by_the_view() {
        let draw = RegisteredSceneDraw::new();
        let id = draw.id();
        assert!(gpui::lookup_custom_draw(id).is_some());
        drop(draw);
        assert!(gpui::lookup_custom_draw(id).is_none());
    }

    #[test]
    fn clipped_compositing_keeps_original_texture_coordinates() {
        let full = Bounds {
            origin: point(px(-20.0), px(10.0)),
            size: size(px(100.0), px(50.0)),
        };
        let visible = Bounds {
            origin: point(px(0.0), px(20.0)),
            size: size(px(80.0), px(30.0)),
        };
        let (quad, scissor) = composite_quad(visible, full, 2.0, [200, 200]).unwrap();
        assert_eq!(scissor, [0, 40, 160, 60]);
        assert!((quad[2] - 0.2).abs() < 1e-6 && (quad[3] - 0.2).abs() < 1e-6);
        assert!((quad[6] - 1.0).abs() < 1e-6 && (quad[11] - 0.8).abs() < 1e-6);
        let invalid = Bounds {
            origin: point(px(f32::NAN), px(0.0)),
            ..visible
        };
        assert!(composite_quad(invalid, full, 1.0, [200, 200]).is_none());
    }

    #[test]
    #[cfg(not(target_family = "wasm"))]
    fn resource_failures_are_cached_across_camera_ticks_and_recover_on_policy_change() {
        match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
            Ok(_) => {}
            Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
                return;
            }
            Err(error) => panic!("required presentation GPU: {error}"),
        }
        let ctx = WgpuContext::headless().unwrap();
        let renderer = GpuRenderer::from_shared_device(
            ctx.device.clone(),
            ctx.queue.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
        )
        .unwrap();
        let target = renderer.create_target(48, 48).unwrap();
        let data =
            gpui_mesh::DataSet::structured_grid("policy", [2; 3], [-0.5; 3], [1.0; 3]).unwrap();
        let mut scene = Scene::new().with_actor(crate::Actor::new(data));
        scene.gpu_upload_limits.max_resource_bytes = 0;
        let mut camera = Camera::fitted(scene.bounds().unwrap());
        let draw = RegisteredSceneDraw::new();
        let bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(48.0), px(48.0)),
        };
        let paint = |scene: &Scene, camera| {
            draw.update(scene, camera, [48, 48]).unwrap();
            let mut encoder = ctx.device.create_command_encoder(&Default::default());
            draw.draw.draw_wgpu(
                &ctx,
                &mut encoder,
                target.color_view(),
                wgpu::TextureFormat::Rgba8Unorm,
                [48, 48],
                bounds,
                bounds,
                1.0,
            );
            ctx.queue.submit([encoder.finish()]);
        };
        for i in 0..32 {
            camera.orbit(0.01, 0.0);
            scene.background[0] = i as f32 / 32.0;
            paint(&scene, camera);
        }
        {
            let state = draw.draw.state.borrow();
            assert_eq!(state.initialization_attempts, 1);
            assert!(state.failure.is_some());
            assert!(state.resources.is_none());
        }
        assert_eq!(scene.preparation_stats().retained_bytes, 0);
        scene.retry_gpu_resources();
        paint(&scene, camera);
        paint(&scene, camera);
        assert_eq!(
            draw.draw.state.borrow().initialization_attempts,
            2,
            "explicit retry runs once even while the same policy still fails"
        );
        scene.gpu_upload_limits = crate::GpuUploadLimits::default();
        paint(&scene, camera);
        assert!(draw.draw.state.borrow().resources.is_some());
        assert_eq!(draw.draw.state.borrow().initialization_attempts, 3);
        scene.gpu_upload_limits.max_resource_bytes = 0;
        paint(&scene, camera);
        assert!(
            draw.draw.state.borrow().resources.is_none(),
            "failed sync releases the old owned scene"
        );
        for _ in 0..32 {
            camera.orbit(0.01, 0.0);
            paint(&scene, camera);
        }
        assert_eq!(draw.draw.state.borrow().initialization_attempts, 3);
        scene.gpu_upload_limits = crate::GpuUploadLimits::default();
        paint(&scene, camera);
        assert_eq!(draw.draw.state.borrow().initialization_attempts, 4);
        assert!(draw.draw.state.borrow().failure.is_none());
        assert!(draw.draw.state.borrow().resources.is_some());
    }

    #[test]
    #[cfg(not(target_family = "wasm"))]
    fn host_compositing_preserves_clip_and_alpha_and_releases_cpu_buffers() {
        match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
            Ok(_) => {}
            Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
                eprintln!("SKIP: no scene presentation adapter");
                return;
            }
            Err(error) => panic!("GPU presentation initialization failed: {error}"),
        }
        let ctx = WgpuContext::headless().expect("headless host context");
        let host = GpuRenderer::from_shared_device(
            ctx.device.clone(),
            ctx.queue.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
        )
        .unwrap();
        let target = host.create_target(64, 64).unwrap();
        let mut background = Scene::new();
        background.background = [0.0, 0.0, 1.0, 1.0];
        let mut host_scene = host.upload(&background).unwrap();
        let dataset =
            gpui_mesh::DataSet::structured_grid("host", [2; 3], [-0.5; 3], [1.0; 3]).unwrap();
        let mut scene = Scene::new().with_actor(
            crate::Actor::new(dataset)
                .color([1.0, 0.0, 0.0, 1.0])
                .opacity(0.5),
        );
        scene.background = [0.0; 4];
        let camera = Camera::fitted(scene.bounds().unwrap());
        let draw = RegisteredSceneDraw::new();
        draw.update(&scene, camera, [48, 48]).unwrap();
        let full = Bounds {
            origin: point(px(8.0), px(8.0)),
            size: size(px(48.0), px(48.0)),
        };
        let visible = Bounds {
            origin: point(px(20.0), px(8.0)),
            size: size(px(36.0), px(48.0)),
        };
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        host.encode(&mut encoder, &mut host_scene, &Camera::default(), &target)
            .unwrap();
        draw.draw.draw_wgpu(
            &ctx,
            &mut encoder,
            target.color_view(),
            wgpu::TextureFormat::Rgba8Unorm,
            [64, 64],
            visible,
            full,
            1.0,
        );
        ctx.queue.submit([encoder.finish()]);
        let frame = host.read_rgba(&target).unwrap();
        let pixel = |x: usize, y: usize| &frame.rgba[(y * 64 + x) * 4..][..4];
        assert_eq!(
            pixel(15, 32),
            &[0, 0, 255, 255],
            "clipped region preserves host pixels"
        );
        let center = pixel(32, 32);
        assert!(
            center[0] > 40 && center[1] == 0 && center[2] > 20 && center[3] == 255,
            "premultiplied composite: {center:?}"
        );
        assert_eq!(scene.preparation_stats().geometry_builds, 1);
        assert_eq!(scene.preparation_stats().retained_bytes, 0);
        draw.update(&scene, camera, [48, 48]).unwrap();
        assert_eq!(
            draw.draw.state.borrow().input.as_ref().unwrap().revision,
            1,
            "static presentation revision reused"
        );
    }
}
