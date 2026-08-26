//! GPUI Element and View for the sphere gallery

use super::mesh::{SphereMeshConfig, cell_center_3d};
use super::renderer::{SphereGalleryConfig, SphereGalleryRenderer};
use super::wgpu_draw::{GalleryWgpuDraw, GalleryWgpuRegistration};
use crate::gpu3d::{Camera3D, OrbitControls};
use glam::Vec3;
use gpui::*;
#[cfg(feature = "headless-qa")]
use image::{Frame, RgbaImage};
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(feature = "headless-qa")]
use std::sync::Arc;

/// A single item in the sphere gallery
#[derive(Clone)]
pub struct SphereGalleryItem {
    /// RGBA pixel data at cell_size × cell_size
    pub pixels: Vec<u8>,
    /// Optional label to display
    pub label: Option<SharedString>,
}

#[cfg(feature = "headless-qa")]
#[derive(Clone, PartialEq, Eq)]
struct GalleryPaintKey {
    width: u32,
    height: u32,
    camera: [u32; 13],
    selected: Option<u32>,
    hovered: Option<u32>,
    item_count: u32,
}

#[cfg(feature = "headless-qa")]
pub(crate) struct CachedGalleryImage {
    key: GalleryPaintKey,
    image: Arc<RenderImage>,
}

#[cfg(feature = "headless-qa")]
fn gallery_paint_key(state: &SphereGalleryState, width: u32, height: u32) -> GalleryPaintKey {
    let camera = &state.camera;
    GalleryPaintKey {
        width,
        height,
        camera: [
            camera.position.x.to_bits(),
            camera.position.y.to_bits(),
            camera.position.z.to_bits(),
            camera.target.x.to_bits(),
            camera.target.y.to_bits(),
            camera.target.z.to_bits(),
            camera.up.x.to_bits(),
            camera.up.y.to_bits(),
            camera.up.z.to_bits(),
            camera.fov.to_bits(),
            camera.aspect.to_bits(),
            camera.near.to_bits(),
            camera.far.to_bits(),
        ],
        selected: state.selected,
        hovered: state.hovered,
        item_count: state.item_count,
    }
}

/// Interactive state for the sphere gallery
#[derive(Debug, Clone)]
pub struct SphereGalleryState {
    /// Orbit camera controls
    pub controls: OrbitControls,
    /// Camera
    pub camera: Camera3D,
    /// Currently selected cell index
    pub selected: Option<u32>,
    /// Currently hovered cell index
    pub hovered: Option<u32>,
    /// Is mouse currently dragging (for rotation)
    pub dragging: bool,
    /// Last mouse position
    pub last_mouse: Option<Point<Pixels>>,
    /// Total number of items
    pub item_count: u32,
    /// Grid columns
    pub cols: u32,
    /// Grid rows
    pub rows: u32,
    /// Mesh config for hit testing
    pub mesh_config: SphereMeshConfig,
    /// Cached screen positions of cell centers (updated each frame)
    cell_screen_positions: Vec<Option<(f32, f32)>>,
}

impl SphereGalleryState {
    pub fn new(cols: u32, rows: u32, item_count: u32) -> Self {
        // Camera looking at the dome from slightly above and in front
        let controls = OrbitControls::default()
            .with_position(2.2, 0.0, 55.0)
            .with_target(Vec3::new(0.0, 0.5, 0.0));

        let camera = controls.to_camera();

        Self {
            controls,
            camera,
            selected: None,
            hovered: None,
            dragging: false,
            last_mouse: None,
            item_count,
            cols,
            rows,
            mesh_config: SphereMeshConfig::default(),
            cell_screen_positions: Vec::new(),
        }
    }

    /// Update camera from controls
    pub fn update_camera(&mut self) {
        self.controls.update_camera(&mut self.camera);
    }

    /// Update cached screen positions of cell centers
    pub fn update_cell_positions(&mut self, width: f32, height: f32) {
        self.cell_screen_positions.clear();
        for i in 0..self.item_count {
            let world_pos = cell_center_3d(i, self.cols, self.rows, &self.mesh_config);
            if let Some(screen) = self.camera.project_to_screen(world_pos, width, height) {
                if screen.z >= 0.0 && screen.z <= 1.0 {
                    self.cell_screen_positions.push(Some((screen.x, screen.y)));
                } else {
                    self.cell_screen_positions.push(None);
                }
            } else {
                self.cell_screen_positions.push(None);
            }
        }
    }

    /// Find the cell nearest to a screen position
    pub fn hit_test(&self, screen_x: f32, screen_y: f32) -> Option<u32> {
        let mut best_index = None;
        let mut best_dist_sq = f32::MAX;
        let max_dist = 80.0; // Maximum pixel distance for a hit

        for (i, pos) in self.cell_screen_positions.iter().enumerate() {
            if let Some((sx, sy)) = pos {
                let dx = screen_x - sx;
                let dy = screen_y - sy;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < best_dist_sq && dist_sq < max_dist * max_dist {
                    best_dist_sq = dist_sq;
                    best_index = Some(i as u32);
                }
            }
        }

        best_index
    }

    /// Move selection by grid offset
    pub fn move_selection(&mut self, dcol: i32, drow: i32) {
        let current = self.selected.unwrap_or(0);
        let col = (current % self.cols) as i32;
        let row = (current / self.cols) as i32;

        let new_col = (col + dcol).clamp(0, self.cols as i32 - 1) as u32;
        let new_row = (row + drow).clamp(0, self.rows as i32 - 1) as u32;
        let new_index = new_row * self.cols + new_col;

        if new_index < self.item_count {
            self.selected = Some(new_index);
        }
    }
}

/// GPUI Element for the sphere gallery
pub struct SphereGalleryElement {
    config: SphereGalleryConfig,
    state: Rc<RefCell<SphereGalleryState>>,
    renderer: Rc<RefCell<Option<SphereGalleryRenderer>>>,
    images_uploaded: Rc<RefCell<bool>>,
    gpu_draw: Rc<GalleryWgpuRegistration>,
    #[cfg(feature = "headless-qa")]
    paint_cache: Rc<RefCell<Option<CachedGalleryImage>>>,
    items: Vec<SphereGalleryItem>,
}

impl SphereGalleryElement {
    /// Create a new sphere gallery element
    pub fn new(
        items: Vec<SphereGalleryItem>,
        config: SphereGalleryConfig,
        state: Rc<RefCell<SphereGalleryState>>,
    ) -> Self {
        let renderer = Rc::new(RefCell::new(None));
        let images_uploaded = Rc::new(RefCell::new(false));
        let gpu_draw = GalleryWgpuDraw::register(
            config.clone(),
            items.clone(),
            Rc::clone(&renderer),
            Rc::clone(&images_uploaded),
        );
        Self {
            config,
            state,
            renderer,
            images_uploaded,
            gpu_draw,
            #[cfg(feature = "headless-qa")]
            paint_cache: Rc::new(RefCell::new(None)),
            items,
        }
    }

    fn rebuild_gpu_draw(&mut self) {
        self.gpu_draw = GalleryWgpuDraw::register(
            self.config.clone(),
            self.items.clone(),
            Rc::clone(&self.renderer),
            Rc::clone(&self.images_uploaded),
        );
    }

    /// Share the renderer across repaints (call from view, pass same Rc each time)
    pub fn with_renderer(mut self, renderer: Rc<RefCell<Option<SphereGalleryRenderer>>>) -> Self {
        self.renderer = renderer;
        self.rebuild_gpu_draw();
        self
    }

    /// Share the upload flag
    pub fn with_upload_flag(mut self, flag: Rc<RefCell<bool>>) -> Self {
        self.images_uploaded = flag;
        self.rebuild_gpu_draw();
        self
    }

    #[cfg(feature = "headless-qa")]
    fn ensure_renderer(&self) -> bool {
        let mut renderer_ref = self.renderer.borrow_mut();
        if renderer_ref.is_none() {
            let Some(mut renderer) = SphereGalleryRenderer::new(self.config.clone()) else {
                return false;
            };
            renderer.build_mesh();
            *renderer_ref = Some(renderer);
        }
        true
    }

    #[cfg(feature = "headless-qa")]
    fn ensure_images_uploaded(&self) {
        if *self.images_uploaded.borrow() {
            return;
        }

        let mut renderer_ref = self.renderer.borrow_mut();
        if let Some(renderer) = renderer_ref.as_mut() {
            let image_refs: Vec<&[u8]> = self
                .items
                .iter()
                .map(|item| item.pixels.as_slice())
                .collect();
            renderer.upload_images(&image_refs);
            *self.images_uploaded.borrow_mut() = true;
            self.paint_cache.borrow_mut().take();
        }
    }
}

impl IntoElement for SphereGalleryElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SphereGalleryElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size {
                width: relative(1.0).into(),
                height: relative(1.0).into(),
            },
            ..Default::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        if height > 0.0 {
            let mut state = self.state.borrow_mut();
            state.camera.aspect = width / height;
            state.update_cell_positions(width, height);
        }
    }

    #[cfg(not(feature = "headless-qa"))]
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let state = self.state.borrow();
        self.gpu_draw.draw.update(
            state.camera.clone(),
            state.item_count,
            state.selected,
            state.hovered,
        );
        drop(state);
        window.paint_custom(self.gpu_draw.id, bounds);
    }

    #[cfg(feature = "headless-qa")]
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let width_u32 = width as u32;
        let height_u32 = height as u32;

        if width_u32 > 0 && height_u32 > 0 {
            let key = {
                let state = self.state.borrow();
                gallery_paint_key(&state, width_u32, height_u32)
            };
            if let Some(image) = self
                .paint_cache
                .borrow()
                .as_ref()
                .filter(|cache| cache.key == key)
                .map(|cache| Arc::clone(&cache.image))
            {
                let _ = window.paint_image(bounds, Corners::default(), image, 0, false);
                return;
            }

            if !self.ensure_renderer() {
                return;
            }
            self.ensure_images_uploaded();
            if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
                renderer.resize(width_u32, height_u32);
                let state = self.state.borrow();
                if let Some(pixels) = renderer.render(
                    &state.camera,
                    state.item_count,
                    state.selected,
                    state.hovered,
                ) && let Some(rgba_image) = RgbaImage::from_raw(width_u32, height_u32, pixels)
                {
                    let image = Arc::new(RenderImage::new(vec![Frame::new(rgba_image)]));
                    let _ = window.paint_image(
                        bounds,
                        Corners::default(),
                        Arc::clone(&image),
                        0,
                        false,
                    );
                    *self.paint_cache.borrow_mut() = Some(CachedGalleryImage { key, image });
                }
            }
        }
    }
}

/// Callback types for the sphere gallery view
pub type OnSelectCallback = Box<dyn Fn(u32, &mut Window, &mut App) + 'static>;
pub type OnHoverCallback = Box<dyn Fn(Option<u32>, &mut Window, &mut App) + 'static>;

/// High-level GPUI View wrapping the sphere gallery with full interaction support.
///
/// Usage:
/// ```ignore
/// let view = cx.new(|_| SphereGalleryView::new(items, config));
/// ```
pub struct SphereGalleryView {
    pub state: Rc<RefCell<SphereGalleryState>>,
    pub config: SphereGalleryConfig,
    pub items: Vec<SphereGalleryItem>,
    renderer: Rc<RefCell<Option<SphereGalleryRenderer>>>,
    images_uploaded: Rc<RefCell<bool>>,
    on_select: Option<OnSelectCallback>,
    on_hover: Option<OnHoverCallback>,
}

impl SphereGalleryView {
    pub fn new(items: Vec<SphereGalleryItem>, config: SphereGalleryConfig) -> Self {
        let item_count = items.len() as u32;
        let state = Rc::new(RefCell::new(SphereGalleryState::new(
            config.cols,
            config.rows,
            item_count,
        )));
        state.borrow_mut().mesh_config = config.mesh_config.clone();

        Self {
            state,
            config,
            items,
            renderer: Rc::new(RefCell::new(None)),
            images_uploaded: Rc::new(RefCell::new(false)),
            on_select: None,
            on_hover: None,
        }
    }

    /// Set callback for when a cell is selected (clicked or Enter pressed)
    pub fn on_select(mut self, cb: impl Fn(u32, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(cb));
        self
    }

    /// Set callback for when hover changes
    pub fn on_hover(mut self, cb: impl Fn(Option<u32>, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(cb));
        self
    }

    /// Update the items displayed in the gallery
    pub fn set_items(&mut self, items: Vec<SphereGalleryItem>) {
        let item_count = items.len() as u32;
        self.items = items;
        self.state.borrow_mut().item_count = item_count;
        *self.images_uploaded.borrow_mut() = false;
    }
}

impl Render for SphereGalleryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let element =
            SphereGalleryElement::new(self.items.clone(), self.config.clone(), self.state.clone())
                .with_renderer(self.renderer.clone())
                .with_upload_flag(self.images_uploaded.clone());

        div()
            .id("sphere-gallery")
            .size_full()
            .focusable()
            .child(element)
            // Mouse down - start drag or select
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, event: &MouseDownEvent, window, cx| {
                    if event.click_count == 2 {
                        // Double click - reset view
                        let mut state = view.state.borrow_mut();
                        state.controls.reset();
                        state.update_camera();
                        cx.notify();
                    } else {
                        // Single click - select hovered cell, or start drag
                        let mut state = view.state.borrow_mut();
                        if let Some(hovered) = state.hovered {
                            state.selected = Some(hovered);
                            drop(state);
                            if let Some(cb) = &view.on_select {
                                cb(hovered, window, cx);
                            }
                        } else {
                            state.dragging = true;
                            state.last_mouse = Some(event.position);
                        }
                        cx.notify();
                    }
                }),
            )
            // Mouse up - stop drag
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, _event: &MouseUpEvent, _window, _cx| {
                    let mut state = view.state.borrow_mut();
                    state.dragging = false;
                }),
            )
            // Mouse move - hover detection and drag rotation
            .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, window, cx| {
                let mut state = view.state.borrow_mut();

                if state.dragging {
                    if let Some(last) = state.last_mouse {
                        let dx: f32 = (event.position.x - last.x).into();
                        let dy: f32 = (event.position.y - last.y).into();
                        state.controls.rotate(dx, dy);
                        state.update_camera();
                    }
                    state.last_mouse = Some(event.position);
                    cx.notify();
                } else {
                    // Hit test for hover
                    let local_x: f32 = event.position.x.into();
                    let local_y: f32 = event.position.y.into();
                    let new_hovered = state.hit_test(local_x, local_y);

                    if new_hovered != state.hovered {
                        state.hovered = new_hovered;
                        drop(state);
                        if let Some(cb) = &view.on_hover {
                            cb(new_hovered, window, cx);
                        }
                        cx.notify();
                    }
                }
            }))
            // Scroll - zoom
            .on_scroll_wheel(cx.listener(|view, event: &ScrollWheelEvent, _window, cx| {
                let delta = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y * 0.5,
                    ScrollDelta::Pixels(pixels) => {
                        let py: f32 = pixels.y.into();
                        py * 0.01
                    }
                };
                let mut state = view.state.borrow_mut();
                state.controls.zoom(delta);
                state.update_camera();
                cx.notify();
            }))
            // Keyboard navigation
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                let mut handled = true;
                match &event.keystroke.key {
                    key if key == "left" => {
                        view.state.borrow_mut().move_selection(-1, 0);
                    }
                    key if key == "right" => {
                        view.state.borrow_mut().move_selection(1, 0);
                    }
                    key if key == "up" => {
                        view.state.borrow_mut().move_selection(0, -1);
                    }
                    key if key == "down" => {
                        view.state.borrow_mut().move_selection(0, 1);
                    }
                    key if key == "enter" || key == "space" => {
                        let state = view.state.borrow();
                        if let Some(selected) = state.selected {
                            drop(state);
                            if let Some(cb) = &view.on_select {
                                cb(selected, window, cx);
                            }
                        }
                    }
                    key if key == "home" => {
                        view.state.borrow_mut().selected = Some(0);
                    }
                    key if key == "end" => {
                        let state = view.state.borrow();
                        let last = state.item_count.saturating_sub(1);
                        drop(state);
                        view.state.borrow_mut().selected = Some(last);
                    }
                    key if key == "escape" => {
                        view.state.borrow_mut().selected = None;
                    }
                    key if key == "r" => {
                        // Reset camera
                        let mut state = view.state.borrow_mut();
                        state.controls.reset();
                        state.update_camera();
                    }
                    _ => {
                        handled = false;
                    }
                }
                if handled {
                    cx.notify();
                }
            }))
    }
}
