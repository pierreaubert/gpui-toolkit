//! Metal/wgpu renderer lifecycle for [`IosWindow`](super::ios_window::IosWindow).
//!
//! Extracted from `ios_window.rs`: renderer construction (previously inline
//! in `IosWindow::new`), per-frame draws, and cached `sprite_atlas` access.

use super::fallback_atlas::FallbackAtlas;
use super::ios_raw_handles::IosRawHandles;
use super::ios_window::IosWindow;
use gpui::{AnyWindowHandle, GpuSpecs, PlatformAtlas, Scene, Size};
use gpui_wgpu::{WgpuContext, WgpuRenderer, WgpuSurfaceConfig, wgpu};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{cell::RefCell, rc::Rc, sync::Arc};

impl IosWindow {
    pub(super) fn draw_scene(&self, scene: &Scene) {
        let mut guard = self.renderer.lock();
        if let Some(renderer) = guard.as_mut() {
            renderer.draw(scene);
        } else {
            log::trace!("GPUI iOS: draw called but no renderer available");
        }
    }

    pub(super) fn cached_sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        if let Some(atlas) = self.sprite_atlas.lock().as_ref() {
            return atlas.clone();
        }

        let atlas: Arc<dyn PlatformAtlas> = {
            let guard = self.renderer.lock();
            if let Some(renderer) = guard.as_ref() {
                renderer.sprite_atlas().clone()
            } else {
                // Fallback: return a dummy atlas so GPUI doesn't panic before
                // the renderer is initialised.
                Arc::new(FallbackAtlas::new())
            }
        };

        *self.sprite_atlas.lock() = Some(atlas.clone());
        atlas
    }

    pub(super) fn renderer_gpu_specs(&self) -> Option<GpuSpecs> {
        let guard = self.renderer.lock();
        guard.as_ref().map(|r| r.gpu_specs())
    }
}

/// Build the Metal-backed wgpu renderer for a newly created window.
///
/// `gpui_wgpu::WgpuContext::instance()` only enables Vulkan+GL, so this
/// creates a dedicated Metal instance, builds the surface from the
/// UIView's raw window handle, and pre-populates the shared GpuContext so
/// `WgpuRenderer` reuses it instead of falling back to Vulkan+GL.
pub(super) fn init_metal_renderer(
    window: &IosWindow,
    handle: AnyWindowHandle,
    pixel_size: Size<gpui::DevicePixels>,
) -> anyhow::Result<()> {
    let _handle = handle; // consumed but not stored
    // Create the wgpu renderer using the Metal backend.
    //
    // `gpui_wgpu::WgpuContext::instance()` only enables Vulkan+GL,
    // so we create our own wgpu instance with Metal enabled, build
    // a surface from the UIView's raw window handle, construct the
    // WgpuContext with that instance, and pre-populate the
    // shared GpuContext so WgpuRenderer::new() reuses it.
    let config = WgpuSurfaceConfig {
        size: pixel_size,
        transparent: false,
        preferred_present_mode: None,
    };

    // Build raw-window-handle wrapper for the renderer. We can't
    // pass `&IosWindow` directly because WgpuRenderer::new requires
    // `Debug + Clone + Send + Sync + 'static`.
    let window_handle = window
        .window_handle()
        .expect("iOS window handle unavailable");
    let display_handle = window
        .display_handle()
        .expect("iOS display handle unavailable");
    let raw_handles = IosRawHandles {
        window: window_handle.as_raw(),
        display: display_handle.as_raw(),
    };

    let metal_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        flags: wgpu::InstanceFlags::default(),
        backend_options: wgpu::BackendOptions::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: Some(Box::new(raw_handles.clone())),
    });

    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        // The display is attached to the instance. WgpuRenderer::new
        // uses the same convention when it recreates this surface.
        raw_display_handle: None,
        raw_window_handle: raw_handles.window,
    };

    // Build a Metal-backed WgpuContext, pre-populate the shared
    // GpuContext (Rc<RefCell<Option<WgpuContext>>>), then call
    // WgpuRenderer::new which will reuse our context instead of
    // falling back to the Vulkan+GL default.
    // SAFETY: `target` borrows the live UIView's raw handles owned by this
    // window; the surface is consumed by the renderer below before the view
    // can be destroyed (same guarantee as the pre-extraction inline code).
    let surface_result = unsafe { metal_instance.create_surface_unsafe(target) };
    match surface_result {
        Ok(surface) => match WgpuContext::new(metal_instance, &surface, None) {
            Ok(context) => {
                let gpu_context: Rc<RefCell<Option<WgpuContext>>> =
                    Rc::new(RefCell::new(Some(context)));
                match WgpuRenderer::new_from_existing_surface(gpu_context, surface, config, None) {
                    Ok(renderer) => {
                        log::info!("iOS wgpu renderer created (Metal)");
                        *window.renderer.lock() = Some(renderer);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("failed to create iOS wgpu renderer: {e:#}"));
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("failed to create iOS WgpuContext: {e:#}"));
            }
        },
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to create iOS wgpu Metal surface: {e:#}"
            ));
        }
    }
    Ok(())
}
