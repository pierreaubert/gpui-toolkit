//! tvOS showcase staticlib -- bridges gpui-showcase into the tvOS app.
//!
//! This crate compiles to a static library (.a) that the Xcode project links.
//! The Swift AppDelegate calls `showcase_tvos_start()` to launch the GPUI app.

#[cfg(target_os = "tvos")]
mod imp {
    use super::misc::ffi_guard;
    use gpui::*;
    use gpui_showcase::Showcase;
    use gpui_ui_kit::i18n::I18nState;
    use gpui_ui_kit::theme::{ThemeState, ThemeVariant};

    /// Called from Swift to start the GPUI application.
    #[unsafe(no_mangle)]
    pub extern "C" fn showcase_tvos_start() {
        ffi_guard(|| {
            oslog::OsLogger::new("org.spinorama.gpui-showcase.tv")
                .level_filter(log::LevelFilter::Info)
                .init()
                .ok();

            log::info!("showcase_tvos_start: registering app callback");

            gpui_ios::ios::ffi::set_app_callback(Box::new(|cx: &mut App| {
                log::info!("GPUI tvOS app callback: setting up showcase");

                cx.set_global(ThemeState::with_variant(ThemeVariant::Dark));
                cx.set_global(I18nState::new());

                let open_result = cx.open_window(
                    WindowOptions {
                        window_bounds: None,
                        ..Default::default()
                    },
                    |_, cx| cx.new(Showcase::new),
                );

                if let Err(error) = open_result {
                    log::error!("[tvOS] Failed to open showcase window: {error}");
                    return;
                }

                cx.activate(true);
            }));

            log::info!("showcase_tvos_start: calling run_app");
            gpui_ios::ios::ffi::run_app();
        })
    }
}

#[cfg(target_os = "tvos")]
mod misc;
