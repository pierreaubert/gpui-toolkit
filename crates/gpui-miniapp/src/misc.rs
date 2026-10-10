use gpui::*;
use std::borrow::Cow;
use std::rc::Rc;

/// Construct the GPUI [`Platform`] backend for the current OS.
///
/// Centralizing this here keeps the per-platform `gpui_macos` / `gpui_linux` /
/// `gpui_windows` deps confined to this single (`publish = false`) crate so
/// publishable consumers don't have to declare them. Examples and demos that
/// need a custom `Application::with_platform(...)` should call this helper
/// instead of inlining the cfg-cascade themselves.
pub fn current_platform() -> Result<Rc<dyn gpui::Platform>, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(Rc::new(gpui_macos::MacPlatform::new(false)))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(gpui_linux::current_platform(false))
    }
    #[cfg(target_os = "windows")]
    {
        gpui_windows::WindowsPlatform::new(false)
            .map(|p| Rc::new(p) as Rc<dyn gpui::Platform>)
            .map_err(|e| format!("failed to create Windows platform: {e:?}"))
    }
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        Ok(gpui_ios::current_platform(false))
    }
    #[cfg(target_os = "android")]
    {
        Ok(gpui_android::current_platform(false))
    }
    #[cfg(target_family = "wasm")]
    {
        Ok(Rc::new(gpui_web::WebPlatform::new(true)))
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "ios",
        target_os = "tvos",
        target_os = "android",
        target_family = "wasm"
    )))]
    {
        compile_error!("unsupported platform for gpui-miniapp")
    }
}

/// Initialize browser-side logging and panic hooks. Call once from the
/// `#[wasm_bindgen(start)]` entry point before constructing the platform.
#[cfg(target_family = "wasm")]
pub fn web_init() {
    console_error_panic_hook::set_once();
    gpui_web::init_logging();
}

/// Read a simple query-string parameter from the hosting page.
///
/// Demo pages use this to select a deterministic initial story without
/// synthesizing pointer events against GPUI's single canvas. Values used by
/// the showcase catalog are ASCII slugs, so a small parser is preferable to
/// pulling a URL parser into every wasm demo.
#[cfg(target_family = "wasm")]
fn web_query_param_raw(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == name).then(|| value.replace('+', " ")))
}

/// Read and percent-decode a query-string parameter from the hosting page.
#[cfg(target_family = "wasm")]
pub fn web_query_param(name: &str) -> Option<String> {
    web_query_param_raw(name).and_then(|value| decode_query_component(&value))
}

#[cfg(any(test, target_family = "wasm"))]
pub(super) fn decode_query_component(value: &str) -> Option<String> {
    fn hex(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                let high = *bytes.get(index + 1)?;
                let low = *bytes.get(index + 2)?;
                decoded.push(hex(high)? << 4 | hex(low)?);
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

/// Map a raw `style=` query value to a design language.
///
/// Returns `None` for missing or unknown values so callers keep the
/// platform default. Pure over strings so unit tests cover it on every
/// platform; the wasm entry points feed it `web_query_param("style")`.
pub fn design_language_from_param(value: Option<&str>) -> Option<gpui_design::DesignLanguage> {
    let value = value.unwrap_or_default().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    gpui_design::DesignLanguage::from_id(value.as_str())
}

/// Resolve the optional `style=` query parameter used by generated demo URLs.
#[cfg(target_family = "wasm")]
pub fn web_initial_design_style() -> Option<gpui_design::DesignLanguage> {
    design_language_from_param(web_query_param("style").as_deref())
}

/// Resolve the optional `theme=` query parameter used by generated demo URLs.
#[cfg(target_family = "wasm")]
pub fn web_initial_theme() -> gpui_ui_kit::theme::ThemeVariant {
    let value = web_query_param("theme")
        .unwrap_or_default()
        .to_ascii_lowercase();
    match value.as_str() {
        "light" => gpui_ui_kit::theme::ThemeVariant::Light,
        "midnight" => gpui_ui_kit::theme::ThemeVariant::Midnight,
        "forest" => gpui_ui_kit::theme::ThemeVariant::Forest,
        "black-and-white" | "black_and_white" => gpui_ui_kit::theme::ThemeVariant::BlackAndWhite,
        "onyx" => gpui_ui_kit::theme::ThemeVariant::Onyx,
        "carbon-white" | "carbon_white" => gpui_ui_kit::theme::ThemeVariant::CarbonWhite,
        "carbon-gray-10" | "carbon_gray_10" => gpui_ui_kit::theme::ThemeVariant::CarbonGray10,
        "carbon-gray-90" | "carbon_gray_90" => gpui_ui_kit::theme::ThemeVariant::CarbonGray90,
        "carbon-gray-100" | "carbon_gray_100" => gpui_ui_kit::theme::ThemeVariant::CarbonGray100,
        _ => gpui_ui_kit::theme::ThemeVariant::Dark,
    }
}

/// Mark the GPUI page ready for browser automation after its window has been
/// opened. The capture harness still waits for a canvas and an additional
/// settle period, but this removes the fragile fixed boot-only sleep.
#[cfg(target_family = "wasm")]
pub fn web_mark_ready() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("data-gpui-ready", "true");
    }
}

/// Bundled UI font data shared by every `MiniApp` shell.
///
/// Returns the eight embedded IBM Plex Sans and Lilex files in the same
/// set the wasm platform registers, so native harnesses can resolve the
/// identical families instead of falling back to system fonts (which
/// changes glyph widths, line wrapping, and therefore layout).
///
/// # Examples
///
/// Feed the bytes to any [`PlatformTextSystem`](gpui::PlatformTextSystem):
///
/// ```ignore
/// let text_system = gpui_wgpu::CosmicTextSystem::new_without_system_fonts("IBM Plex Sans");
/// text_system.add_fonts(gpui_miniapp::bundled_ui_fonts()).unwrap();
/// ```
pub fn bundled_ui_fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(
            include_bytes!("../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf").as_slice(),
        ),
        Cow::Borrowed(
            include_bytes!("../../assets/fonts/ibm-plex-sans/IBMPlexSans-Italic.ttf").as_slice(),
        ),
        Cow::Borrowed(
            include_bytes!("../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf").as_slice(),
        ),
        Cow::Borrowed(
            include_bytes!("../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBoldItalic.ttf")
                .as_slice(),
        ),
        Cow::Borrowed(include_bytes!("../../assets/fonts/lilex/Lilex-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/lilex/Lilex-Bold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/lilex/Lilex-Italic.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../assets/fonts/lilex/Lilex-BoldItalic.ttf").as_slice()),
    ]
}
