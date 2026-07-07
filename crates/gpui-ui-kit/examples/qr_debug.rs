//! QR Code Debug Example
//!
//! Demonstrates QR code generation and camera-based QR reading:
//! - **Generate** tab: static and animated QR codes at various sizes
//! - **Scan** tab: live camera preview with QR detection overlay
//!
//! Camera permission is requested on first scan. If denied or no camera
//! is available, a clear error message is shown with instructions.

#![allow(unexpected_cfgs)]

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::qr::AnimatedQrCode;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[path = "qr_debug/camera.rs"]
mod camera;
#[path = "qr_debug/misc.rs"]
mod misc;
#[path = "qr_debug/types.rs"]
mod types;

use camera::camera_capture_loop;
use misc::PREVIEW_HEIGHT;
use misc::PREVIEW_WIDTH;
use misc::paint_camera_preview;
#[cfg(target_os = "macos")]
use misc::request_camera_permission_macos;
use misc::tab_button;
use types::CameraPreview;
use types::CameraState;
use types::QrScanResult;
use types::Tab;

pub struct QrDebug {
    entity: Entity<Self>,
    active_tab: Tab,
    // Generate tab
    animated_qr_tiny: Entity<AnimatedQrCode>,
    animated_qr_small: Entity<AnimatedQrCode>,
    // Scan tab
    camera_state: CameraState,
    scan_result_rx: Option<mpsc::Receiver<QrScanResult>>,
    stop_tx: Option<mpsc::Sender<()>>,
    last_preview: Option<CameraPreview>,
    decoded_text: Option<String>,
}

impl QrDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        let animated_qr_tiny =
            cx.new(|cx| AnimatedQrCode::new("https://example.com/qr-debug-demo", px(50.0), cx));
        let animated_qr_small =
            cx.new(|cx| AnimatedQrCode::new("https://example.com/qr-debug-demo", px(80.0), cx));

        Self {
            entity: cx.entity().clone(),
            active_tab: Tab::Generate,
            animated_qr_tiny,
            animated_qr_small,
            camera_state: CameraState::Idle,
            scan_result_rx: None,
            stop_tx: None,
            last_preview: None,
            decoded_text: None,
        }
    }

    fn start_camera(&mut self, cx: &mut Context<Self>) {
        // Stop any existing camera
        self.stop_camera();

        self.camera_state = CameraState::Active;
        self.decoded_text = None;
        self.last_preview = None;

        let (result_tx, result_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();

        self.scan_result_rx = Some(result_rx);
        self.stop_tx = Some(stop_tx);

        // Spawn background thread: request permission (macOS), then start capture
        thread::spawn(move || {
            #[cfg(target_os = "macos")]
            {
                if !request_camera_permission_macos() {
                    let _ = result_tx.send(QrScanResult::Error(
                        "Camera access denied. Grant permission in System Settings > \
                         Privacy & Security > Camera."
                            .to_string(),
                    ));
                    return;
                }
            }

            if let Err(e) = camera_capture_loop(result_tx.clone(), stop_rx) {
                let _ = result_tx.send(QrScanResult::Error(e));
            }
        });

        // Spawn polling loop to pull frames into the UI
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                smol::Timer::after(Duration::from_millis(50)).await;
                let Ok(()) = this.update(cx, |this, cx| {
                    this.poll_camera(cx);
                }) else {
                    break;
                };
            }
        })
        .detach();
    }

    fn stop_camera(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        self.scan_result_rx = None;
        if self.camera_state == CameraState::Active {
            self.camera_state = CameraState::Idle;
        }
    }

    fn poll_camera(&mut self, cx: &mut Context<Self>) {
        let Some(rx) = self.scan_result_rx.take() else {
            return;
        };

        // Drain all pending results, keep the latest
        let mut changed = false;
        while let Ok(result) = rx.try_recv() {
            match result {
                QrScanResult::Success(text, preview) => {
                    self.decoded_text = Some(text);
                    self.last_preview = Some(preview);
                    changed = true;
                }
                QrScanResult::Scanning(preview) | QrScanResult::Detected(preview) => {
                    self.last_preview = Some(preview);
                    changed = true;
                }
                QrScanResult::Error(e) => {
                    self.camera_state = CameraState::Denied(e);
                    // Don't call stop_camera here — we already took rx
                    if let Some(tx) = self.stop_tx.take() {
                        let _ = tx.send(());
                    }
                    changed = true;
                }
            }
        }

        // Put rx back unless we got an error
        if !matches!(self.camera_state, CameraState::Denied(_)) {
            self.scan_result_rx = Some(rx);
        }

        if changed {
            cx.notify();
        }
    }

    // ----- Render helpers -----

    fn render_generate_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .flex()
            .flex_col()
            .gap_6()
            // Static QR
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Static QR Codes").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .items_end()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("200px").muted(true))
                                    .child(QrCode::new(
                                        "https://github.com/anthropics/claude-code",
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("120px").muted(true))
                                    .child(QrCode::new("https://example.com").size(px(120.0))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("Custom colors").muted(true))
                                    .child(
                                        QrCode::new("SOTF Audio Engine")
                                            .size(px(150.0))
                                            .fg(rgba(0x2da44eff))
                                            .bg(rgba(0x1a1a2eff)),
                                    ),
                            ),
                    ),
            )
            // Animated QR
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Text::new("Animated QR Codes (too small — auto-pans)")
                            .weight(TextWeight::Bold),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .items_end()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("50px").muted(true))
                                    .child(self.animated_qr_tiny.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("80px").muted(true))
                                    .child(self.animated_qr_small.clone()),
                            ),
                    ),
            )
            // Border around section
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
    }

    fn render_scan_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        let mut content = div().flex().flex_col().gap_4();

        match &self.camera_state {
            CameraState::Idle => {
                // Show start button
                content = content.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_4()
                        .py_8()
                        .child(Text::new("Point your camera at a QR code to scan it."))
                        .child(
                            Text::new(
                                "Camera permission will be requested when you start scanning.",
                            )
                            .muted(true),
                        )
                        .child(
                            div()
                                .id("start-scan-btn")
                                .px_6()
                                .py_3()
                                .bg(theme.accent)
                                .text_color(rgba(0xffffffff))
                                .rounded_lg()
                                .cursor_pointer()
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(|s| s.opacity(0.9))
                                .child("Start Camera")
                                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.start_camera(cx);
                                    });
                                }),
                        ),
                );
            }

            CameraState::Denied(err) => {
                // Show error with instructions
                let surface_hover = theme.surface_hover;
                content = content.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_6()
                        .bg(rgba(0xff000015))
                        .border_1()
                        .border_color(theme.error)
                        .rounded_lg()
                        .child(
                            Text::new("Camera Unavailable")
                                .weight(TextWeight::Bold)
                                .color(theme.error),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.text_secondary)
                                .child(err.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .text_sm()
                                .text_color(theme.text_muted)
                                .child("To grant camera access:")
                                .child("1. Open System Settings > Privacy & Security > Camera")
                                .child("2. Enable access for this application")
                                .child("3. Restart the app and try again"),
                        )
                        .child(
                            div()
                                .id("retry-scan-btn")
                                .mt_2()
                                .px_4()
                                .py_2()
                                .bg(theme.surface)
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .hover(move |s| s.bg(surface_hover))
                                .child("Retry")
                                .on_mouse_up(MouseButton::Left, {
                                    let entity = self.entity.clone();
                                    move |_event, _window, cx| {
                                        entity.update(cx, |this, cx| {
                                            this.camera_state = CameraState::Idle;
                                            this.start_camera(cx);
                                        });
                                    }
                                }),
                        ),
                );
            }

            CameraState::Active => {
                // Camera preview
                let stop_entity = self.entity.clone();
                let surface_hover = theme.surface_hover;

                // Status line
                let status = if self.decoded_text.is_some() {
                    "QR Code Detected!"
                } else {
                    "Scanning..."
                };

                let status_color = if self.decoded_text.is_some() {
                    theme.success
                } else {
                    theme.text_muted
                };

                content = content.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            Text::new(status)
                                .weight(TextWeight::Semibold)
                                .color(status_color),
                        )
                        .child(
                            div()
                                .id("stop-scan-btn")
                                .px_4()
                                .py_2()
                                .bg(theme.surface)
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .hover(move |s| s.bg(surface_hover))
                                .child("Stop Camera")
                                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                                    stop_entity.update(cx, |this, cx| {
                                        this.stop_camera();
                                        cx.notify();
                                    });
                                }),
                        ),
                );

                // Camera preview canvas
                if let Some(preview) = &self.last_preview {
                    let rgb = preview.rgb_pixels.clone();
                    let pw = preview.width;
                    let ph = preview.height;
                    let qr_bounds = preview.qr_bounds;
                    let accent = theme.accent;
                    let success = theme.success;
                    let has_decode = self.decoded_text.is_some();

                    content = content.child(
                        div()
                            .border_1()
                            .border_color(if has_decode {
                                theme.success
                            } else {
                                theme.border
                            })
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                canvas(
                                    move |_bounds, _window, _cx| (rgb, pw, ph, qr_bounds),
                                    move |bounds, (rgb, pw, ph, qr_bounds), window, _cx| {
                                        paint_camera_preview(
                                            bounds, &rgb, pw, ph, qr_bounds, accent, success,
                                            has_decode, window,
                                        );
                                    },
                                )
                                .w(px(PREVIEW_WIDTH as f32 * 2.0))
                                .h(px(PREVIEW_HEIGHT as f32 * 2.0)),
                            ),
                    );
                } else {
                    // No frame yet — show placeholder
                    content = content.child(
                        div()
                            .w(px(PREVIEW_WIDTH as f32 * 2.0))
                            .h(px(PREVIEW_HEIGHT as f32 * 2.0))
                            .bg(rgba(0x1a1a1aff))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Initializing camera...").muted(true)),
                    );
                }

                // Decoded result
                if let Some(text) = &self.decoded_text {
                    content = content.child(
                        div()
                            .p_4()
                            .bg(rgba(0x00ff0010))
                            .border_1()
                            .border_color(theme.success)
                            .rounded_lg()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                Text::new("Decoded Content:")
                                    .weight(TextWeight::Bold)
                                    .color(theme.success),
                            )
                            .child(
                                div()
                                    .p_3()
                                    .bg(theme.surface)
                                    .rounded_md()
                                    .text_sm()
                                    .text_color(theme.text_primary)
                                    .child(text.clone()),
                            ),
                    );
                }
            }
        }

        content
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
    }
}

impl Drop for QrDebug {
    fn drop(&mut self) {
        self.stop_camera();
    }
}

impl Render for QrDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();
        let active_tab = self.active_tab;

        // Tab bar
        let tab_bar = div()
            .flex()
            .gap_1()
            .mb_4()
            .child({
                let entity = entity.clone();
                tab_button(
                    "tab-generate",
                    "Generate QR",
                    active_tab == Tab::Generate,
                    &theme,
                )
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    entity.update(cx, |this, cx| {
                        this.active_tab = Tab::Generate;
                        cx.notify();
                    });
                })
            })
            .child({
                let entity = entity.clone();
                tab_button("tab-scan", "Scan QR", active_tab == Tab::Scan, &theme).on_mouse_up(
                    MouseButton::Left,
                    move |_event, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.active_tab = Tab::Scan;
                            cx.notify();
                        });
                    },
                )
            });

        let content = match active_tab {
            Tab::Generate => self.render_generate_tab(cx).into_any_element(),
            Tab::Scan => self.render_scan_tab(cx).into_any_element(),
        };

        div()
            .id("qr-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_4()
            .child(Heading::h1("QR Code Debug"))
            .child(tab_bar)
            .child(content)
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("QR Code Debug")
            .size(750.0, 800.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(QrDebug::new),
    );
}
