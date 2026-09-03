//! Minimal GPUI-on-wasm spike: one quad + one text line in a full-page canvas.
//! Build/serve via `just wasm-serve-hello` (Trunk, COOP/COEP headers included).
#![cfg_attr(target_family = "wasm", no_main)]

//! Boot-failure text shared by the wasm entry point and native unit tests.
//!
//! Kept cfg-independent (and DOM-free) so `cargo test -p gpui-hello-web` can
//! pin the fallback wording without a browser.
#[cfg(any(test, target_family = "wasm"))]
fn boot_error_text(detail: &str) -> String {
    format!(
        "gpui-hello-web could not start: {detail}. \
        This demo needs a browser with WebGPU enabled (Chrome 113+, Edge 113+, \
        or Safari 26+); other browsers show this message instead of a blank page."
    )
}

#[cfg(target_family = "wasm")]
mod imp {
    use super::boot_error_text;
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

    /// Log a boot failure to the devtools console and replace the blank page
    /// with a fallback `<div>` so non-WebGPU browsers get an explanation
    /// instead of a white screen. Fully defensive: every DOM step is optional,
    /// so a missing `window`/`document`/`body` degrades to console-only output
    /// rather than panicking inside the panic hook.
    fn report_boot_error(detail: &str) {
        let message = boot_error_text(detail);
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&message));
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(body) = document.body() else {
            return;
        };
        let Ok(banner) = document.create_element("div") else {
            return;
        };
        let _ = banner.set_attribute("id", "gpui-hello-web-boot-error");
        let _ = banner.set_attribute(
            "style",
            "position:fixed;inset:0;display:flex;align-items:center;justify-content:center;\
             background:#1e1e2e;color:#fff;font-family:sans-serif;padding:24px;text-align:center;",
        );
        banner.set_text_content(Some(&message));
        let _ = body.append_child(&banner);
    }

    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn start() {
        gpui_miniapp::web_init();
        let platform = match gpui_miniapp::current_platform() {
            Ok(platform) => platform,
            Err(error) => {
                report_boot_error(&format!("web platform unavailable ({error})"));
                return;
            }
        };
        // `WebPlatform::run` returns immediately after scheduling the launch
        // callback, so the app must be kept alive explicitly: `run_embedded`
        // returns an `ApplicationHandle` owning the app.
        let handle = gpui::Application::with_platform(platform).run_embedded(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(640.), px(560.)), cx);
            if let Err(error) = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| HelloWeb),
            ) {
                report_boot_error(&format!("failed to open window ({error:?})"));
                return;
            }
            cx.activate(true);
            gpui_miniapp::web_mark_ready();
        });
        // Page-lifetime handle ONLY: this starter intentionally leaks the
        // `ApplicationHandle` so the single app instance lives as long as the
        // page. Do NOT copy this into real apps that open/close windows or
        // reboot the app — drop the handle (or shut the app down) there
        // instead of `mem::forget`-ing it once per launch.
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
    use super::boot_error_text;

    #[test]
    fn native_binary_reports_failure() {
        assert_eq!(super::main(), std::process::ExitCode::FAILURE);
    }

    #[test]
    fn boot_error_text_names_cause_and_webgpu_requirement() {
        let message = boot_error_text("web platform unavailable (no WebGPU adapter)");
        assert!(
            message.contains("no WebGPU adapter"),
            "fallback must name the cause, got: {message}"
        );
        assert!(
            message.contains("WebGPU"),
            "fallback must point at the WebGPU requirement, got: {message}"
        );
        assert!(
            message.contains("blank page"),
            "fallback must explain it replaces a blank page, got: {message}"
        );
    }
}
