//! QR code rendering example.
//!
//! Demonstrates static and animated QR codes at several sizes and colors. QR
//! scanning is intentionally left to host applications so the UI kit does not
//! impose a camera backend or operating-system permission model.

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::qr::AnimatedQrCode;
use gpui_ui_kit::{Heading, QrCode, Text, TextWeight};

struct QrDebug {
    animated_tiny: Entity<AnimatedQrCode>,
    animated_small: Entity<AnimatedQrCode>,
}

impl QrDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            animated_tiny: cx
                .new(|cx| AnimatedQrCode::new("https://example.com/qr-debug-demo", px(50.0), cx)),
            animated_small: cx
                .new(|cx| AnimatedQrCode::new("https://example.com/qr-debug-demo", px(80.0), cx)),
        }
    }
}

impl Render for QrDebug {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("qr-debug-root")
            .size_full()
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("QR Code Rendering"))
            .child(
                Text::new(
                    "Generate deterministic QR visuals in-process; camera scanning belongs to the host app.",
                )
                .muted(true),
            )
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
                                    .child(QrCode::new("https://github.com/pierreaubert/gpui-toolkit")),
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
                                        QrCode::new("GPUI Toolkit")
                                            .size(px(150.0))
                                            .fg(rgba(0x2da44eff))
                                            .bg(rgba(0x1a1a2eff)),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Text::new("Animated QR Codes (auto-pan at constrained sizes)")
                            .weight(TextWeight::Bold),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .items_end()
                            .child(self.animated_tiny.clone())
                            .child(self.animated_small.clone()),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("QR Code Rendering")
            .size(750.0, 650.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(QrDebug::new),
    );
}
