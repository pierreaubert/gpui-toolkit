//! Sphere Gallery Demo
//!
//! Demonstrates the GPU-accelerated sphere gallery component with
//! procedurally generated colored tiles as placeholder album art.
//!
//! ## Controls
//! - **Left Click**: Select a cell
//! - **Left Drag**: Rotate the sphere
//! - **Scroll Wheel**: Zoom in/out
//! - **Arrow Keys**: Navigate between cells
//! - **Enter/Space**: Confirm selection
//! - **Double Click**: Reset camera
//! - **R**: Reset camera
//! - **Escape**: Clear selection
//! - **P**: Cycle through projections
//!
//! Run with: `cargo run --features gpu-3d --example sphere_gallery_demo`

use d3rs::sphere_gallery::{Projection, SphereGalleryConfig, SphereGalleryItem, SphereGalleryView};
use gpui::*;
use gpui_ui_kit::{ButtonSet, ButtonSetOption, ButtonSetSize, NumberInput, NumberInputSize};

/// Generate a colored placeholder image (RGBA, cell_size x cell_size)
fn generate_placeholder(index: u32, cell_size: u32) -> Vec<u8> {
    let hue = (index as f32 * 37.0) % 360.0;
    let (r, g, b) = hsl_to_rgb(hue, 0.6, 0.5);

    let size = cell_size as usize;
    let mut pixels = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let offset = (y * size + x) * 4;
            let fx = x as f32 / size as f32;
            let fy = y as f32 / size as f32;

            let cx = fx - 0.5;
            let cy = fy - 0.5;
            let dist = (cx * cx + cy * cy).sqrt() * 2.0;
            let vignette = 1.0 - (dist * 0.3).min(0.3);
            let pattern = ((fx * 4.0).sin() * (fy * 4.0).sin()).abs() * 0.15 + 0.85;
            let factor = vignette * pattern;

            pixels[offset] = (r * factor * 255.0) as u8;
            pixels[offset + 1] = (g * factor * 255.0) as u8;
            pixels[offset + 2] = (b * factor * 255.0) as u8;
            pixels[offset + 3] = 255;
        }
    }

    pixels
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (r + m, g + m, b + m)
}

const COLS: u32 = 5;
const ROWS: u32 = 4;
const CELL_SIZE: u32 = 256;

/// Map projection index to a string key for ButtonSet
fn projection_key(proj: Projection) -> SharedString {
    SharedString::from(proj.name())
}

/// Map a ButtonSet key back to a projection index
fn key_to_projection_index(key: &str) -> Option<usize> {
    Projection::ALL.iter().position(|p| p.name() == key)
}

struct DemoView {
    gallery: Entity<SphereGalleryView>,
    items: Vec<SphereGalleryItem>,
    projection_index: usize,
    apex_height: f64,
}

impl DemoView {
    fn new(cx: &mut Context<Self>) -> Self {
        let item_count = COLS * ROWS;

        let items: Vec<SphereGalleryItem> = (0..item_count)
            .map(|i| SphereGalleryItem {
                pixels: generate_placeholder(i, CELL_SIZE),
                label: Some(format!("Album {}", i).into()),
            })
            .collect();

        let apex_height = 0.5;
        let config = SphereGalleryConfig::new(COLS, ROWS)
            .cell_size(CELL_SIZE)
            .projection(Projection::Stereographic)
            .apex_height(apex_height as f32);

        let gallery = cx.new(|_cx| {
            SphereGalleryView::new(items.clone(), config).on_select(|index, _window, _cx| {
                eprintln!("Selected cell: {}", index);
            })
        });

        Self {
            gallery,
            items,
            projection_index: 1, // Stereographic
            apex_height,
        }
    }

    fn rebuild_gallery(&mut self, cx: &mut Context<Self>) {
        let projection = Projection::ALL[self.projection_index];
        let config = SphereGalleryConfig::new(COLS, ROWS)
            .cell_size(CELL_SIZE)
            .projection(projection)
            .apex_height(self.apex_height as f32);

        let items = self.items.clone();
        self.gallery = cx.new(|_cx| {
            SphereGalleryView::new(items, config).on_select(|index, _window, _cx| {
                eprintln!("Selected cell: {}", index);
            })
        });
        cx.notify();
    }
}

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_proj_key = projection_key(Projection::ALL[self.projection_index]);
        let entity_proj = cx.entity().clone();
        let entity_apex = cx.entity().clone();

        // Build ButtonSet options from all projections
        let proj_options: Vec<ButtonSetOption> = Projection::ALL
            .iter()
            .map(|p| ButtonSetOption::new(p.name(), p.name()))
            .collect();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0d0d11))
            // P key cycles projections (wraps around)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _window, cx| {
                if event.keystroke.key == "p" {
                    view.projection_index = (view.projection_index + 1) % Projection::ALL.len();
                    view.rebuild_gallery(cx);
                }
            }))
            .child(
                // Toolbar
                div()
                    .px_4()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        // Projection selector
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888888))
                                    .child("Projection"),
                            )
                            .child(
                                ButtonSet::new("projection-select")
                                    .options(proj_options)
                                    .selected(current_proj_key)
                                    .size(ButtonSetSize::Xs)
                                    .on_change(move |value, _window, cx| {
                                        if let Some(idx) = key_to_projection_index(value) {
                                            entity_proj.update(cx, |view, cx| {
                                                view.projection_index = idx;
                                                view.rebuild_gallery(cx);
                                            });
                                        }
                                    }),
                            ),
                    )
                    .child(
                        // Apex height input
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888888))
                                    .child("Apex Height"),
                            )
                            .child(
                                NumberInput::new("apex-height")
                                    .value(self.apex_height)
                                    .min(0.0)
                                    .max(2.0)
                                    .step(0.05)
                                    .decimals(2)
                                    .size(NumberInputSize::Xs)
                                    .width(90.0)
                                    .on_change(move |value, _window, cx| {
                                        entity_apex.update(cx, |view, cx| {
                                            view.apex_height = value;
                                            view.rebuild_gallery(cx);
                                        });
                                    }),
                            ),
                    )
                    .child(
                        // Help text
                        div()
                            .flex_1()
                            .text_right()
                            .text_xs()
                            .text_color(rgb(0x555555))
                            .child(
                                "[P] cycle projection  |  Drag=rotate  Scroll=zoom  Arrows=select",
                            ),
                    ),
            )
            .child(
                // Gallery
                div().flex_1().child(self.gallery.clone()),
            )
    }
}

fn main() {
    let platform = gpui_miniapp::current_platform().expect("failed to initialize GPUI platform");
    gpui::Application::with_platform(platform).run(move |cx: &mut gpui::App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(100.0), px(100.0)),
                    size: Size {
                        width: px(1200.0),
                        height: px(800.0),
                    },
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Sphere Gallery Demo")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(DemoView::new),
        )
        .unwrap();
    });
}
