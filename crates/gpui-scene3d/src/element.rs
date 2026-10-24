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

enum PaintSource {
    Image(Arc<RenderImage>),
    #[cfg(feature = "gpui-gpu")]
    Custom(gpui::CustomDrawId),
}

impl ElementState {
    fn prepare(
        &mut self,
        scene: &Scene,
        camera: Camera,
        size: [u32; 2],
        gpu_available: bool,
    ) -> Result<PaintSource, crate::SceneError> {
        if let Err(error) = crate::RenderLimits::default().pixel_count(size[0], size[1], 8) {
            self.image = None;
            #[cfg(feature = "gpui-gpu")]
            drop(self.gpu.take());
            return Err(error);
        }
        #[cfg(feature = "gpui-gpu")]
        if gpu_available {
            let draw = self
                .gpu
                .get_or_insert_with(crate::gpu_element::RegisteredSceneDraw::new);
            match draw.update(scene, camera, size) {
                Ok(true) => {
                    self.image = None;
                    return Ok(PaintSource::Custom(draw.id()));
                }
                Ok(false) => {}
                Err(error) => {
                    self.image = None;
                    drop(self.gpu.take());
                    return Err(error);
                }
            }
        } else {
            drop(self.gpu.take());
        }
        #[cfg(not(feature = "gpui-gpu"))]
        let _ = gpu_available;
        self.image(scene, camera, size).map(PaintSource::Image)
    }

    fn image(
        &mut self,
        scene: &Scene,
        camera: Camera,
        size: [u32; 2],
    ) -> Result<Arc<RenderImage>, crate::SceneError> {
        if let Some((key, image)) = &self.image {
            match key.matches(scene, camera, size) {
                Ok(true) => return Ok(image.clone()),
                Ok(false) => {}
                Err(error) => {
                    self.image = None;
                    return Err(error);
                }
            }
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
        let gpu_available = retained && gpui::wgpu_custom_draw_available();
        #[cfg(not(feature = "gpui-gpu"))]
        let gpu_available = {
            let _ = retained;
            false
        };
        match state.prepare(&self.scene, camera, size, gpu_available) {
            Ok(PaintSource::Image(image)) => {
                let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
            }
            #[cfg(feature = "gpui-gpu")]
            Ok(PaintSource::Custom(id)) => window.paint_custom(id, bounds),
            Err(_) => {}
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
    #[cfg(feature = "gpui-gpu")]
    fn camera_precision_fallback_retains_image_and_recovers_without_source_changes() {
        let scene = Scene::new().with_actor(Actor::new(
            DataSet::structured_grid("camera", [2; 3], [-0.5; 3], [1.0; 3]).unwrap(),
        ));
        let good = Camera::fitted(scene.bounds().unwrap());
        let bad = Camera {
            target: [1e100; 3],
            ..good
        };
        bad.validate().unwrap();
        let mut state = ElementState::default();
        assert!(matches!(
            state.prepare(&scene, good, [32, 32], true).unwrap(),
            PaintSource::Custom(_)
        ));
        let PaintSource::Image(first) = state.prepare(&scene, bad, [32, 32], true).unwrap() else {
            panic!("unsupported actor-relative matrices must use software");
        };
        let PaintSource::Image(second) = state.prepare(&scene, bad, [32, 32], true).unwrap() else {
            panic!("software camera frame should stay cached");
        };
        assert!(Arc::ptr_eq(&first, &second));
        assert!(
            matches!(
                state.prepare(&scene, good, [32, 32], true).unwrap(),
                PaintSource::Custom(_)
            ),
            "camera precision must not poison the cached material/topology verdict"
        );
        assert!(state.image.is_none());
        assert_eq!(scene.preparation_stats().geometry_builds, 1);
    }

    #[test]
    #[cfg(feature = "gpui-gpu")]
    fn primitive_precision_fallback_retains_image_and_recovers_gpu_route() {
        let data = DataSet::new(
            "wide primitive",
            vec![[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0]],
            gpui_mesh::Cells::from_cells(&[
                (gpui_mesh::CellType::Line, &[0, 1]),
                (gpui_mesh::CellType::Vertex, &[0]),
            ]),
        )
        .with_field(gpui_mesh::Field::scalar("p", vec![1e300; 2]));
        let mut scene = Scene::new().with_actor(
            Actor::new(data).coloured_by("p", crate::TransferFunction::viridis([0.0, 1.0])),
        );
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut state = ElementState::default();
        let PaintSource::Image(first) = state.prepare(&scene, camera, [48, 48], true).unwrap()
        else {
            panic!("primitive precision needs actual software image");
        };
        let PaintSource::Image(second) = state.prepare(&scene, camera, [48, 48], true).unwrap()
        else {
            panic!("primitive precision image was not retained");
        };
        assert!(Arc::ptr_eq(&first, &second));
        scene.actors[0].dataset.point_data[0] = gpui_mesh::Field::scalar("p", vec![0.0, 1.0]);
        assert!(matches!(
            state.prepare(&scene, camera, [48, 48], true).unwrap(),
            PaintSource::Custom(_)
        ));
        assert!(state.image.is_none());
    }

    #[test]
    #[cfg(feature = "gpui-gpu")]
    fn actual_presentation_decision_uses_and_retains_precision_fallback_image() {
        let mut scene = crate::scene::coefficient_sensitive_scene();
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut state = ElementState::default();
        let PaintSource::Image(first) = state.prepare(&scene, camera, [32, 32], true).unwrap()
        else {
            panic!("precision fallback must paint an image");
        };
        let PaintSource::Image(second) = state.prepare(&scene, camera, [32, 32], true).unwrap()
        else {
            panic!("cached fallback must paint an image");
        };
        assert!(Arc::ptr_eq(&first, &second));
        scene.actors[0].clip_planes[0].normal = [0.0, 1.0, 0.0];
        let PaintSource::Custom(id) = state.prepare(&scene, camera, [32, 32], true).unwrap() else {
            panic!("representable clipping returns to GPU presentation");
        };
        assert!(state.image.is_none());
        assert!(gpui::lookup_custom_draw(id).is_some());
        drop(state);
        assert!(gpui::lookup_custom_draw(id).is_none());
    }

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

    #[test]
    #[cfg(feature = "gpui-gpu")]
    fn wide_transfer_domain_uses_gpu_and_can_return_to_raw_transport() {
        let dataset = DataSet::structured_grid("wide", [2; 3], [-0.5; 3], [1.0; 3])
            .unwrap()
            .with_field(gpui_mesh::Field::scalar("p", vec![0.0; 8]));
        let mut scene = Scene::new().with_actor(
            Actor::new(dataset)
                .coloured_by("p", crate::TransferFunction::viridis([-f64::MAX, f64::MAX])),
        );
        let camera = Camera::fitted(scene.bounds().unwrap());
        let mut state = ElementState::default();
        for _ in 0..3 {
            assert!(matches!(
                state.prepare(&scene, camera, [24, 24], true).unwrap(),
                PaintSource::Custom(_)
            ));
        }
        let mut moved = camera;
        moved.orbit(0.1, 0.0);
        assert!(matches!(
            state.prepare(&scene, moved, [24, 24], true).unwrap(),
            PaintSource::Custom(_)
        ));
        scene.actors[0].transfer.as_mut().unwrap().range = [-1.0, 1.0];
        assert!(matches!(
            state.prepare(&scene, moved, [24, 24], true).unwrap(),
            PaintSource::Custom(_)
        ));
        assert!(state.image.is_none());
        assert_eq!(scene.preparation_stats().geometry_builds, 0);
    }
}
