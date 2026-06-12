//! iOS showcase staticlib — bridges gpui-ui-kit showcase into the iOS app.
//!
//! This crate compiles to a static library (.a) that the Xcode project links.
//! The Swift AppDelegate calls `showcase_ios_start()` to launch the GPUI app.

#[cfg(any(target_os = "ios", target_os = "tvos"))]
mod imp {
    use gpui::*;
    use gpui_ui_kit::i18n::I18nState;
    use gpui_ui_kit::showcase::Showcase;
    use gpui_ui_kit::theme::{ThemeState, ThemeVariant};

    /// Called from Swift to start the GPUI application.
    #[unsafe(no_mangle)]
    pub extern "C" fn showcase_ios_start() {
        // Set up logging to os_log
        oslog::OsLogger::new("org.spinorama.gpui-showcase")
            .level_filter(log::LevelFilter::Info)
            .init()
            .ok();

        log::info!("showcase_ios_start: registering app callback");

        gpui_ios::ios::ffi::set_app_callback(Box::new(|cx: &mut App| {
            log::info!("GPUI app callback: setting up showcase");

            // Initialize theme and i18n
            cx.set_global(ThemeState::with_variant(ThemeVariant::Dark));
            cx.set_global(I18nState::new());

            // Open a fullscreen window with the full showcase
            cx.open_window(
                WindowOptions {
                    window_bounds: None,
                    ..Default::default()
                },
                |_, cx| cx.new(Showcase::new),
            )
            .expect("Failed to open showcase window");

            cx.activate(true);
        }));

        log::info!("showcase_ios_start: calling run_app");
        gpui_ios::ios::ffi::run_app();
    }
}
