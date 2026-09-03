#![cfg(target_family = "wasm")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn web_platform_constructs_with_embedded_fonts() {
    let platform = gpui_miniapp::current_platform().expect("web platform");
    let names = platform.text_system().all_font_names();
    assert!(
        names.iter().any(|n| n == "IBM Plex Sans"),
        "IBM Plex Sans missing from {names:?}"
    );
}

#[wasm_bindgen_test]
fn web_mark_ready_sets_automation_hook() {
    // `just wasm-visual hello` waits for `html[data-gpui-ready='true']`
    // before capturing; fail here if the marker contract drifts.
    gpui_miniapp::web_mark_ready();
    let document = web_sys::window()
        .expect("browser window")
        .document()
        .expect("html document");
    let root = document.document_element().expect("html root element");
    assert_eq!(
        root.get_attribute("data-gpui-ready"),
        Some("true".to_string()),
        "web_mark_ready must set html[data-gpui-ready='true'] for the wasm-visual harness"
    );
}
