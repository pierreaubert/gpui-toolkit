//! Minimal GPUI-on-wasm spike: one quad + one text line in a full-page canvas.
//! Build/serve via `just wasm-serve-hello` (Trunk, COOP/COEP headers included).
//!
//! Two copy-paste examples live here alongside the boot path: `?view=`
//! query-param selection (read with `gpui_miniapp::web_query_param`) and a
//! page-lifetime `ResizeObserver` on `document.body` that logs CSS-pixel size
//! changes to the devtools console. Both follow the boot-error fallback style:
//! every DOM step is optional, so a missing handle degrades gracefully.
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

/// View selected via the `?view=` query parameter (`quad`, `text`, or `both`).
///
/// Kept cfg-independent (and DOM-free) so `cargo test -p gpui-hello-web` can
/// pin the selection mapping without a browser; the wasm entry point reads
/// the raw value with `gpui_miniapp::web_query_param`.
#[cfg(any(test, target_family = "wasm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DemoView {
    #[default]
    Both,
    Quad,
    Text,
}

#[cfg(any(test, target_family = "wasm"))]
fn demo_view_from_param(param: Option<&str>) -> DemoView {
    match param
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "quad" => DemoView::Quad,
        "text" => DemoView::Text,
        _ => DemoView::Both,
    }
}

#[cfg(any(test, target_family = "wasm"))]
fn demo_view_label(view: DemoView) -> &'static str {
    match view {
        DemoView::Both => "both",
        DemoView::Quad => "quad",
        DemoView::Text => "text",
    }
}

#[cfg(target_family = "wasm")]
mod imp {
    use super::{DemoView, boot_error_text, demo_view_from_param, demo_view_label};
    use gpui::*;
    use wasm_bindgen::JsCast;

    struct HelloWeb {
        view: DemoView,
    }

    impl Render for HelloWeb {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut root = div()
                .size_full()
                .bg(rgb(0x1e1e2e))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .text_color(rgb(0xffffff));
            if self.view != DemoView::Text {
                root = root.child(div().size(px(64.0)).bg(rgb(0xf38ba8)).rounded(px(8.0)));
            }
            if self.view != DemoView::Quad {
                root = root.child(format!(
                    "Hello from GPUI on wasm (?view={})",
                    demo_view_label(self.view)
                ));
            }
            root
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

    /// Attach a page-lifetime `ResizeObserver` on `document.body` that logs
    /// CSS-pixel size changes to the devtools console. Fully defensive like
    /// [`report_boot_error`]: every DOM step is optional, so a missing
    /// `window`/`document`/`body` (or a browser without `ResizeObserver`)
    /// silently skips observation instead of panicking.
    fn observe_body_resize() {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Some(body) = document.body() else {
            return;
        };
        let callback = wasm_bindgen::closure::Closure::wrap(Box::new(
            move |entries: js_sys::Array, _observer: web_sys::ResizeObserver| {
                entries.for_each(&mut |entry, _, _| {
                    let Ok(entry) = entry.dyn_into::<web_sys::ResizeObserverEntry>() else {
                        return;
                    };
                    let rect = entry.content_rect();
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                        "gpui-hello-web resized: {:.0}x{:.0} css px",
                        rect.width(),
                        rect.height()
                    )));
                });
            },
        )
            as Box<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>);
        let Ok(observer) = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()) else {
            return;
        };
        observer.observe(&body);
        // Page-lifetime observation ONLY: the callback and observer are
        // intentionally leaked so resize logging lives as long as the page,
        // matching the `ApplicationHandle` leak below. Do NOT copy this into
        // real apps that re-observe or tear down — drop (and `unobserve`)
        // there instead of forgetting once per launch.
        callback.forget();
        std::mem::forget(observer);
    }

    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn start() {
        gpui_miniapp::web_init();
        observe_body_resize();
        let view = demo_view_from_param(gpui_miniapp::web_query_param("view").as_deref());
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
        let handle =
            gpui::Application::with_platform(platform).run_embedded(move |cx: &mut App| {
                let bounds = Bounds::centered(None, size(px(640.), px(560.)), cx);
                if let Err(error) = cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        ..Default::default()
                    },
                    |_, cx| cx.new(|_| HelloWeb { view }),
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
    use super::{DemoView, boot_error_text, demo_view_from_param, demo_view_label};

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

    #[test]
    fn view_param_selects_demo_view() {
        assert_eq!(demo_view_from_param(None), DemoView::Both);
        assert_eq!(demo_view_from_param(Some("")), DemoView::Both);
        assert_eq!(demo_view_from_param(Some("quad")), DemoView::Quad);
        assert_eq!(demo_view_from_param(Some("text")), DemoView::Text);
        assert_eq!(demo_view_from_param(Some("both")), DemoView::Both);
        assert_eq!(demo_view_from_param(Some("QUAD")), DemoView::Quad);
        assert_eq!(demo_view_from_param(Some("  text  ")), DemoView::Text);
        assert_eq!(demo_view_from_param(Some("scatter")), DemoView::Both);
    }

    #[test]
    fn view_labels_round_trip_through_param() {
        for view in [DemoView::Both, DemoView::Quad, DemoView::Text] {
            assert_eq!(demo_view_from_param(Some(demo_view_label(view))), view);
        }
    }
}
