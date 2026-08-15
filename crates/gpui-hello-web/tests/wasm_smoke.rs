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
