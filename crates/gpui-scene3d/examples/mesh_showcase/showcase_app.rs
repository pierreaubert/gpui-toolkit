//! Mesh showcase root view: sidebar sections over an orbit viewport.
//!
//! Scenes are built lazily per section and cached, so a capture opening
//! `?section=slice` only pays for the slice pipeline.

use super::mesh_section::MeshSection;
use super::scenes::build_scene;
use gpui::{
    Context, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, Render, ScrollDelta,
    ScrollWheelEvent, Window, div, prelude::*, px, rgb,
};
use gpui_scene3d::{
    Scene,
    element::{SceneCamera, SceneElement, fitted_camera},
};
use std::collections::HashMap;

struct DragState {
    last: gpui::Point<gpui::Pixels>,
}

pub(super) struct ShowcaseApp {
    section: MeshSection,
    scenes: HashMap<MeshSection, (Scene, gpui_scene3d::Camera)>,
    scene: Scene,
    home: gpui_scene3d::Camera,
    camera: SceneCamera,
    drag: Option<DragState>,
    scene_error: Option<String>,
    panel_scroll: gpui::ScrollHandle,
}

impl ShowcaseApp {
    fn section_key(value: &str) -> String {
        value
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|character| character.to_ascii_lowercase())
            .collect()
    }

    /// Optional initial-section override for browser capture, matched
    /// case-insensitively against the section label or its slug.
    fn initial_section() -> MeshSection {
        #[cfg(target_family = "wasm")]
        let requested = gpui_miniapp::web_query_param("section");
        #[cfg(not(target_family = "wasm"))]
        let requested = std::env::var("MESH_SHOWCASE_SECTION").ok();

        requested
            .and_then(|name| {
                MeshSection::all()
                    .into_iter()
                    .find(|s| Self::section_key(s.label()) == Self::section_key(name.trim()))
            })
            .unwrap_or_default()
    }

    pub(super) fn new(_cx: &mut Context<Self>) -> Self {
        let section = Self::initial_section();
        let mut app = Self {
            section,
            scenes: HashMap::new(),
            scene: Scene::new(),
            home: gpui_scene3d::Camera::default(),
            camera: fitted_camera(&Scene::new()),
            drag: None,
            scene_error: None,
            panel_scroll: gpui::ScrollHandle::new(),
        };
        app.select_section(section);
        app
    }

    /// Switches to a section, building and caching its scene on first use.
    /// A failed build keeps the previous scene and reports the error.
    fn select_section(&mut self, section: MeshSection) {
        self.section = section;
        if !self.scenes.contains_key(&section) {
            match build_scene(section) {
                Ok(built) => {
                    self.scenes.insert(section, built);
                    self.scene_error = None;
                }
                Err(error) => {
                    self.scene_error = Some(error);
                    return;
                }
            }
        }
        if let Some((scene, home)) = self.scenes.get(&section) {
            self.scene = scene.clone();
            self.home = *home;
            self.camera = fitted_camera(&self.scene);
            // Keep the section framing identical to the cached home camera.
            *self.camera.borrow_mut() = self.home;
        }
    }

    fn section_button(&self, section: MeshSection, cx: &mut Context<Self>) -> impl IntoElement {
        let active = section == self.section;
        div()
            .id(match section {
                MeshSection::Vortices => "section-vortices",
                MeshSection::Slice => "section-slice",
                MeshSection::Contours => "section-contours",
                MeshSection::Shells => "section-shells",
            })
            .px_3()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .bg(if active { rgb(0x2f6fed) } else { rgb(0x2a2e37) })
            .text_color(rgb(0xe8eaf0))
            .child(section.label())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.select_section(section);
                    cx.notify();
                }),
            )
    }

    fn viewport(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let element = SceneElement::new(self.scene.clone(), self.camera.clone()).id("mesh-scene");
        div()
            .relative()
            .flex_1()
            .h_full()
            .min_h(px(200.0))
            .bg(rgb(0x101014))
            .child(element)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    if event.click_count == 2 {
                        *this.camera.borrow_mut() = this.home;
                    } else {
                        this.drag = Some(DragState {
                            last: event.position,
                        });
                    }
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let Some(drag) = &mut this.drag else { return };
                let dx: f32 = (event.position.x - drag.last.x).into();
                let dy: f32 = (event.position.y - drag.last.y).into();
                drag.last = event.position;
                this.camera.borrow_mut().orbit(dx * 0.01, -dy * 0.01);
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.drag = None;
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y,
                    ScrollDelta::Pixels(pixels) => f32::from(pixels.y) * 0.02,
                };
                this.camera
                    .borrow_mut()
                    .dolly((1.0 - delta * 0.08).clamp(0.5, 1.8));
                cx.notify();
            }))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut panel = div()
            .id("mesh-panel")
            .w(px(260.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .overflow_y_scroll()
            .track_scroll(&self.panel_scroll)
            .bg(rgb(0x1d2026))
            .text_color(rgb(0xe8eaf0))
            .child(div().text_lg().child("Mesh Showcase"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa1ad))
                    .child("3D fields on unstructured meshes: vortices, slice, contours, shells."),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa1ad))
                    .child("Drag: rotate · Wheel: zoom · Double-click: reset"),
            );
        for section in MeshSection::all() {
            panel = panel.child(self.section_button(section, cx));
        }
        panel = panel
            .child(div().mt_2().text_base().child(self.section.label()))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa1ad))
                    .child(self.section.caption()),
            );
        if let Some(error) = &self.scene_error {
            panel = panel.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xff7a7a))
                    .child(format!("Scene failed: {error}")),
            );
        }
        panel
    }
}

impl Render for ShowcaseApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Narrow viewports (gallery phones) stack the section bar above the
        // viewport instead of squeezing a side panel next to it.
        let narrow = window.bounds().size.width < px(700.0);
        if narrow {
            let mut bar = div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_2()
                .p_3()
                .bg(rgb(0x1d2026))
                .child(div().text_base().child("Mesh"));
            for section in MeshSection::all() {
                bar = bar.child(self.section_button(section, cx));
            }
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(rgb(0x14161a))
                .text_color(rgb(0xe8eaf0))
                .child(bar)
                .child(self.viewport(cx))
        } else {
            div()
                .flex()
                .size_full()
                .bg(rgb(0x14161a))
                .child(self.viewport(cx))
                .child(self.sidebar(cx))
        }
    }
}
