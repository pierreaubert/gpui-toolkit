//! Same-device SphereGallery custom drawing for GPUI WGPU renderers.

use super::element::SphereGalleryItem;
use super::renderer::{SphereGalleryConfig, SphereGalleryRenderer};
use crate::gpu3d::Camera3D;
use crate::vello2d::wgpu_draw::{CompositePipeline, CompositeResources, clip_src_rect};
use gpui::{Bounds, CustomDraw, CustomDrawId, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
struct GalleryFrame {
    camera: Camera3D,
    cell_count: u32,
    selected: Option<u32>,
    hovered: Option<u32>,
}

pub(crate) struct GalleryWgpuRegistration {
    pub(crate) id: CustomDrawId,
    #[allow(dead_code)] // The headless-QA element bypasses the GPU draw.
    pub(crate) draw: Rc<GalleryWgpuDraw>,
}

impl Drop for GalleryWgpuRegistration {
    fn drop(&mut self) {
        gpui::unregister_custom_draw(self.id);
    }
}

/// Records an offscreen gallery pass and a same-frame GPU composite.
pub(crate) struct GalleryWgpuDraw {
    config: SphereGalleryConfig,
    items: Vec<SphereGalleryItem>,
    renderer: Rc<RefCell<Option<SphereGalleryRenderer>>>,
    images_uploaded: Rc<RefCell<bool>>,
    frame: RefCell<Option<GalleryFrame>>,
    composite: RefCell<Option<CompositePipeline>>,
    failed: Cell<bool>,
}

impl GalleryWgpuDraw {
    pub(crate) fn register(
        config: SphereGalleryConfig,
        items: Vec<SphereGalleryItem>,
        renderer: Rc<RefCell<Option<SphereGalleryRenderer>>>,
        images_uploaded: Rc<RefCell<bool>>,
    ) -> Rc<GalleryWgpuRegistration> {
        let draw = Rc::new(Self {
            config,
            items,
            renderer,
            images_uploaded,
            frame: RefCell::new(None),
            composite: RefCell::new(None),
            failed: Cell::new(false),
        });
        let adapter: Rc<dyn CustomDraw> = Rc::new(WgpuCustomDrawAdapter(draw.clone()));
        let id = gpui::register_custom_draw(adapter);
        Rc::new(GalleryWgpuRegistration { id, draw })
    }

    #[allow(dead_code)] // Called only by the non-headless GPUI paint path.
    pub(crate) fn update(
        &self,
        camera: Camera3D,
        cell_count: u32,
        selected: Option<u32>,
        hovered: Option<u32>,
    ) {
        *self.frame.borrow_mut() = Some(GalleryFrame {
            camera,
            cell_count,
            selected,
            hovered,
        });
    }

    fn ensure_renderer(&self, ctx: &WgpuContext) -> Option<()> {
        let mut renderer = self.renderer.borrow_mut();
        let needs_recreate = renderer
            .as_ref()
            .is_none_or(|renderer| !renderer.uses_device(&ctx.device));
        if needs_recreate {
            *renderer = SphereGalleryRenderer::with_device(
                Arc::clone(&ctx.device),
                Arc::clone(&ctx.queue),
                self.config.clone(),
            );
            let renderer = renderer.as_mut()?;
            renderer.build_mesh();
            *self.images_uploaded.borrow_mut() = false;
            self.composite
                .borrow_mut()
                .as_mut()
                .map(CompositePipeline::invalidate_source);
        }
        Some(())
    }
}

impl CustomDraw for GalleryWgpuDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WgpuCustomDraw for GalleryWgpuDraw {
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
        if self.ensure_renderer(ctx).is_none() {
            self.failed.set(true);
            return;
        }

        let full_width: f32 = full_bounds.size.width.into();
        let full_height: f32 = full_bounds.size.height.into();
        let texture_size = [
            (full_width * scale_factor).max(1.0) as u32,
            (full_height * scale_factor).max(1.0) as u32,
        ];

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
        if !*self.images_uploaded.borrow() {
            let images: Vec<&[u8]> = self
                .items
                .iter()
                .map(|item| item.pixels.as_slice())
                .collect();
            renderer.upload_images(&images);
            *self.images_uploaded.borrow_mut() = true;
        }
        let Some(source) = renderer.encode_render_to_texture(
            encoder,
            &frame.camera,
            frame.cell_count,
            frame.selected,
            frame.hovered,
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
        let (source_origin, source_size) = clip_src_rect(full_bounds, bounds, scale_factor);
        composite.composite(
            ctx,
            encoder,
            target,
            source,
            [origin_x * scale_factor, origin_y * scale_factor],
            source_size,
            source_origin,
            source_size,
            [texture_size[0] as f32, texture_size[1] as f32],
            [target_size[0] as f32, target_size[1] as f32],
        );
    }
}
