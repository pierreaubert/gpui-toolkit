use super::consts::IOS_APP_STATE;
use super::consts::IOS_WINDOW_LIST;
use super::consts::install_ios_panic_hook;
use super::misc::retain_application_for_process_lifetime;
use super::take::take_app_callback;
use super::take::take_asset_source;
use super::window_list_wrapper::WindowListWrapper;
use gpui::{App, AppContext, WindowOptions};
use std::rc::Rc;

pub(super) struct IosAppState {
    pub(super) finish_launching: std::cell::UnsafeCell<Option<Box<dyn FnOnce()>>>,
}

unsafe impl Send for IosAppState {}

unsafe impl Sync for IosAppState {}

pub fn run_app() {
    install_ios_panic_hook();
    log::info!("GPUI iOS: Starting application");
    if IOS_APP_STATE.get().is_none() {
        let state = IosAppState {
            finish_launching: std::cell::UnsafeCell::new(None),
        };
        let _ = IOS_APP_STATE.set(state);
        let _ = IOS_WINDOW_LIST.set(WindowListWrapper(std::cell::UnsafeCell::new(Vec::new())));
    }

    let platform = Rc::new(super::super::IosPlatform::new());
    let app = gpui::Application::with_platform(platform);
    let app = if let Some(assets) = take_asset_source() {
        app.with_assets(assets)
    } else {
        app
    };
    retain_application_for_process_lifetime(&app);
    app.run(|cx: &mut App| {
        if let Some(cb) = take_app_callback() {
            cb(cx);
        } else {
            log::warn!("GPUI iOS: No app callback registered — opening default empty window");
            cx.open_window(
                WindowOptions {
                    window_bounds: None,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| gpui::Empty),
            )
            .expect("Failed to open default window");
            cx.activate(true);
        }
    });

    if let Some(state) = IOS_APP_STATE.get() {
        let callback = unsafe { (*state.finish_launching.get()).take() };
        if let Some(callback) = callback {
            callback();
        }
    }
}
