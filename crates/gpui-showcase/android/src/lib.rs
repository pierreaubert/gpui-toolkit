//! Android showcase dynamic library.
//!
//! Android's `NativeActivity` loads this `.so` and the `android-activity`
//! glue calls `android_main` with the process `AndroidApp` handle.

#[cfg(target_os = "android")]
mod imp {
    use gpui::{App, AppContext, Application, WindowOptions};
    use gpui_showcase::Showcase;
    use gpui_ui_kit::i18n::I18nState;
    use gpui_ui_kit::theme::{ThemeState, ThemeVariant};

    #[unsafe(no_mangle)]
    pub fn android_main(app: android_activity::AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("gpui-showcase"),
        );

        std::panic::set_hook(Box::new(|info| {
            log::error!("GPUI Android panic: {info}");
        }));

        log::info!("android_main: entered");
        let _platform = gpui_android::android::jni::init_platform(&app);
        let Some(shared_platform) = gpui_android::android::jni::shared_platform() else {
            log::error!("android_main: shared_platform() returned None");
            return;
        };

        Application::with_platform(shared_platform.into_rc()).run(|cx: &mut App| {
            log::info!("Application::run callback: opening showcase");
            cx.set_global(ThemeState::with_variant(ThemeVariant::Dark));
            cx.set_global(I18nState::new());

            let result = cx.open_window(
                WindowOptions {
                    window_bounds: None,
                    ..Default::default()
                },
                |_, cx| cx.new(Showcase::new),
            );

            if let Err(error) = result {
                log::error!("failed to open Android showcase window: {error}");
                return;
            }

            cx.activate(true);
        });

        log::info!("android_main: Application::run returned");
    }
}
