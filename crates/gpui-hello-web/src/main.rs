//! Minimal GPUI-on-wasm spike: one quad + one text line in a full-page canvas.
//! Build/serve via `just wasm-serve-hello` (Trunk, COOP/COEP headers included).
#![cfg_attr(target_family = "wasm", no_main)]

#[cfg(target_family = "wasm")]
mod imp {
    use gpui::*;

    struct HelloWeb;

    impl Render for HelloWeb {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(rgb(0x1e1e2e))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .text_color(rgb(0xffffff))
                .child(div().size(px(64.0)).bg(rgb(0xf38ba8)).rounded(px(8.0)))
                .child("Hello from GPUI on wasm")
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn start() {
        gpui_miniapp::web_init();
        let platform = gpui_miniapp::current_platform().expect("web platform");
        // `WebPlatform::run` returns immediately after scheduling the launch
        // callback, so the app must be kept alive explicitly: `run_embedded`
        // returns an `ApplicationHandle` owning the app; forget it so the app
        // lives for the page's lifetime.
        let handle = gpui::Application::with_platform(platform).run_embedded(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(640.), px(560.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| HelloWeb),
            )
            .expect("failed to open window");
            cx.activate(true);
            gpui_miniapp::web_mark_ready();
        });
        std::mem::forget(handle);
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() -> std::process::ExitCode {
    eprintln!("gpui-hello-web only runs on wasm32-unknown-unknown; use `just wasm-serve-hello`");
    std::process::ExitCode::FAILURE
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    #[test]
    fn native_binary_reports_failure() {
        assert_eq!(super::main(), std::process::ExitCode::FAILURE);
    }
}
