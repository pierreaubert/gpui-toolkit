//! Retained GPUI scene presentation with a DPI-aware software fallback.

// Rust guideline compliant 2026-02-21

use crate::camera::Camera;
use crate::presentation::{RenderKey, physical_size};
use crate::raster::render;
use crate::scene::Scene;
use gpui::{
    App, Bounds, Corners, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, RenderImage, Size, Style, Window, relative,
};
use image::{Frame, RgbaImage};
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;
use std::sync::Arc;

/// Shared camera state for a scene element.
///
/// Hosts mutate it from mouse handlers (drag to orbit, wheel to dolly) and
/// request a repaint; the element reads it on every paint.
pub type SceneCamera = Rc<RefCell<Camera>>;

/// Creates shared camera state fitted to a scene's bounds.
#[must_use]
pub fn fitted_camera(scene: &Scene) -> SceneCamera {
    let camera = scene.bounds().map_or_else(Camera::default, Camera::fitted);
    Rc::new(RefCell::new(camera))
}

/// A GPUI element that fills its layout bounds with the rendered scene.
///
/// Give the element a stable [`Self::id`] to retain presentation resources.
/// With `gpui-gpu`, supported scenes use the host wgpu device and command
/// encoder. Other hosts use a cached image from the software rasterizer.
#[derive(Clone)]
pub struct SceneElement {
    scene: Scene,
    camera: SceneCamera,
    id: Option<ElementId>,
}

#[derive(Default)]
struct ElementState {
    image: Option<(RenderKey, Arc<RenderImage>)>,
    #[cfg(feature = "gpui-gpu")]
    gpu: Option<crate::gpu_element::RegisteredSceneDraw>,
}

impl ElementState {
    fn image(
        &mut self,
        scene: &Scene,
        camera: Camera,
        size: [u32; 2],
    ) -> Result<Arc<RenderImage>, crate::SceneError> {
        if let Some((key, image)) = &self.image
            && key.matches(scene, camera, size)?
        {
            return Ok(image.clone());
        }
        // Drop the previous frame before allocating a replacement.
        self.image = None;
        let key = RenderKey::new(scene, camera, size)?;
        let frame = render(scene, &camera, size[0], size[1])?;
        let rgba_image = RgbaImage::from_raw(frame.width, frame.height, frame.rgba)
            .expect("renderer returned a complete RGBA frame");
        let image = Arc::new(RenderImage::new(vec![Frame::new(rgba_image)]));
        self.image = Some((key, image.clone()));
        Ok(image)
    }
}

impl SceneElement {
    /// Creates an element for a scene with shared camera state.
    pub fn new(scene: Scene, camera: SceneCamera) -> Self {
        Self {
            scene,
            camera,
            id: None,
        }
    }

    /// Gives this view stable window-owned presentation state.
    ///
    /// Use a unique ID within its GPUI scope. Identified views retain one
    /// physical-size image and, with `gpui-gpu`, their host GPU resources.
    #[must_use]
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    fn paint_retained(
        &self,
        state: &mut ElementState,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        retained: bool,
    ) {
        let Some(size) = physical_size(
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            window.scale_factor(),
        ) else {
            return;
        };
        let camera = *self.camera.borrow();
        #[cfg(feature = "gpui-gpu")]
        if retained
            && gpui::wgpu_custom_draw_available()
            && crate::gpu_element::supports(&self.scene)
            && crate::RenderLimits::default()
                .pixel_count(size[0], size[1], 8)
                .is_ok()
        {
            state.image = None;
            let draw = state
                .gpu
                .get_or_insert_with(crate::gpu_element::RegisteredSceneDraw::new);
            if draw.update(&self.scene, camera, size).is_ok() {
                window.paint_custom(draw.id(), bounds);
                return;
            }
        }
        #[cfg(not(feature = "gpui-gpu"))]
        let _ = retained;
        #[cfg(feature = "gpui-gpu")]
        drop(state.gpu.take());
        if let Ok(image) = state.image(&self.scene, camera, size) {
            let _ = window.paint_image(bounds, Corners::default(), image, 0, false);
        }
    }

    /// Orbits the shared camera and marks it changed for the next paint.
    pub fn orbit(&self, d_azimuth: f32, d_elevation: f32) {
        self.camera.borrow_mut().orbit(d_azimuth, d_elevation);
    }

    /// Dollies the shared camera.
    pub fn dolly(&self, factor: f32) {
        self.camera.borrow_mut().dolly(factor);
    }
}

impl IntoElement for SceneElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SceneElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        self.id.clone()
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (
            window.request_layout(
                Style {
                    size: Size {
                        width: relative(1.0).into(),
                        height: relative(1.0).into(),
                    },
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        if let Some(id) = id {
            window.with_element_state(id, |state: Option<ElementState>, window| {
                let mut state = state.unwrap_or_default();
                self.paint_retained(&mut state, bounds, window, true);
                ((), state)
            });
        } else {
            self.paint_retained(&mut ElementState::default(), bounds, window, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Actor;
    use gpui_mesh::DataSet;

    #[test]
    fn retained_images_reuse_static_content_and_replace_changed_frames() {
        let mut scene = Scene::new().with_actor(Actor::new(
            DataSet::structured_grid("image", [2; 3], [0.0; 3], [1.0; 3]).unwrap(),
        ));
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut state = ElementState::default();
        let first = state.image(&scene, camera, [24, 24]).unwrap();
        assert!(Arc::ptr_eq(
            &first,
            &state.image(&scene, camera, [24, 24]).unwrap()
        ));
        scene.actors[0].color = [1.0, 0.0, 0.0, 1.0];
        let colored = state.image(&scene, camera, [24, 24]).unwrap();
        assert!(!Arc::ptr_eq(&first, &colored));
        assert!(!Arc::ptr_eq(
            &colored,
            &state.image(&scene, camera, [48, 48]).unwrap()
        ));
        assert_eq!(scene.preparation_stats().geometry_builds, 1);
    }
}
