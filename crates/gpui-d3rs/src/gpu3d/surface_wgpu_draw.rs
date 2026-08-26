//! Same-device Surface3D custom drawing for GPUI WGPU renderers.

use super::camera::Camera3D;
use super::config::Surface3DConfig;
use super::mesh::SurfaceMesh;
use super::renderer::Surface3DRenderer;
use crate::vello2d::wgpu_draw::{CompositePipeline, CompositeResources, clip_src_rect};
use gpui::{Bounds, CustomDraw, CustomDrawId, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
struct SurfaceFrame {
    camera: Camera3D,
    config: Surface3DConfig,
    log_settings: Option<(f32, f32)>,
    texture_size: [u32; 2],
    mesh_revision: u64,
    config_revision: u64,
}

/// Registered custom draw and its owner. Dropping the last registration
/// unregisters its GPUI draw, matching the lifetime of the element state.
pub(crate) struct SurfaceWgpuRegistration {
    pub(crate) id: CustomDrawId,
    pub(crate) draw: Rc<SurfaceWgpuDraw>,
}

impl Drop for SurfaceWgpuRegistration {
    fn drop(&mut self) {
        gpui::unregister_custom_draw(self.id);
    }
}

/// Records the 3D pass and compositing pass into GPUI's frame encoder.
pub(crate) struct SurfaceWgpuDraw {
    renderer: Rc<RefCell<Option<Surface3DRenderer>>>,
    mesh: Rc<RefCell<Option<SurfaceMesh>>>,
    frame: RefCell<Option<SurfaceFrame>>,
    uploaded_mesh_revision: Cell<u64>,
    configured_revision: Cell<u64>,
    composite: RefCell<Option<CompositePipeline>>,
    failed: Cell<bool>,
}

impl SurfaceWgpuDraw {
    pub(crate) fn register(
        renderer: Rc<RefCell<Option<Surface3DRenderer>>>,
        mesh: Rc<RefCell<Option<SurfaceMesh>>>,
    ) -> Rc<SurfaceWgpuRegistration> {
        let draw = Rc::new(Self {
            renderer,
            mesh,
            frame: RefCell::new(None),
            uploaded_mesh_revision: Cell::new(u64::MAX),
            configured_revision: Cell::new(u64::MAX),
            composite: RefCell::new(None),
            failed: Cell::new(false),
        });
        let adapter: Rc<dyn CustomDraw> = Rc::new(WgpuCustomDrawAdapter(draw.clone()));
        let id = gpui::register_custom_draw(adapter);
        Rc::new(SurfaceWgpuRegistration { id, draw })
    }

    pub(crate) fn update(
        &self,
        camera: Camera3D,
        config: Surface3DConfig,
        log_settings: Option<(f32, f32)>,
        texture_size: [u32; 2],
        mesh_revision: u64,
        config_revision: u64,
    ) {
        *self.frame.borrow_mut() = Some(SurfaceFrame {
            camera,
            config,
            log_settings,
            texture_size,
            mesh_revision,
            config_revision,
        });
    }

    fn initialize_renderer(&self, ctx: &WgpuContext, frame: &SurfaceFrame) -> Option<()> {
        let mut renderer = self.renderer.borrow_mut();
        let needs_recreate = renderer.as_ref().is_none_or(|renderer| {
            !renderer.uses_device(&ctx.device)
                || self.configured_revision.get() != frame.config_revision
        });
        if needs_recreate {
            *renderer = Surface3DRenderer::with_device(
                Arc::clone(&ctx.device),
                Arc::clone(&ctx.queue),
                frame.config.clone(),
            );
            self.configured_revision.set(frame.config_revision);
            self.uploaded_mesh_revision.set(u64::MAX);
            self.composite
                .borrow_mut()
                .as_mut()
                .map(CompositePipeline::invalidate_source);
        }
        renderer.as_ref()?;
        Some(())
    }
}

impl CustomDraw for SurfaceWgpuDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WgpuCustomDraw for SurfaceWgpuDraw {
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
        let Some(frame) = self.frame.borrow().clone() else {
            return;
        };
        if self.initialize_renderer(ctx, &frame).is_none() {
            self.failed.set(true);
            return;
        }

        let texture_size = frame.texture_size;

        let mut renderer = self.renderer.borrow_mut();
        let Some(renderer) = renderer.as_mut() else {
            return;
        };
        let resized = renderer.dimensions() != (texture_size[0], texture_size[1]);
        renderer.resize(texture_size[0], texture_size[1]);
        if resized {
            self.composite
                .borrow_mut()
                .as_mut()
                .map(CompositePipeline::invalidate_source);
        }
        if self.uploaded_mesh_revision.get() != frame.mesh_revision {
            let mesh = self.mesh.borrow();
            let Some(mesh) = mesh.as_ref() else {
                return;
            };
            renderer.set_mesh(mesh);
            self.uploaded_mesh_revision.set(frame.mesh_revision);
        }

        let Some(source) = renderer.encode_render_to_texture(
            encoder,
            &frame.camera,
            frame.log_settings,
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        ) else {
            return;
        };

        let mut composite = self.composite.borrow_mut();
        let replace_composite = composite
            .as_ref()
            .is_none_or(|composite| composite.target_format != target_format);
        if replace_composite {
            *composite = Some(CompositePipeline::new(
                ctx,
                target_format,
                Rc::new(CompositeResources::new(ctx, target_format)),
            ));
        }
        let Some(composite) = composite.as_mut() else {
            return;
        };

        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        let full_width: f32 = full_bounds.size.width.into();
        let source_scale = texture_size[0] as f32 / full_width.max(1.0);
        let (source_origin, source_size) = clip_src_rect(full_bounds, bounds, source_scale);
        let bounds_width: f32 = bounds.size.width.into();
        let bounds_height: f32 = bounds.size.height.into();
        let dst_size = [bounds_width * scale_factor, bounds_height * scale_factor];
        composite.composite(
            ctx,
            encoder,
            target,
            source,
            [origin_x * scale_factor, origin_y * scale_factor],
            dst_size,
            source_origin,
            source_size,
            [texture_size[0] as f32, texture_size[1] as f32],
            [target_size[0] as f32, target_size[1] as f32],
        );
    }
}
