//! iOS Platform implementation — implements the GPUI Platform trait.

use super::{IosDispatcher, IosDisplay, IosWindow};
use anyhow::anyhow;
use futures::channel::oneshot;
use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, DummyKeyboardMapper,
    ForegroundExecutor, Keymap, Menu, MenuItem, PathPromptOptions, Platform, PlatformDisplay,
    PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem, PlatformWindow, Result,
    Task, ThermalState, WindowAppearance, WindowParams,
};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

pub struct IosPlatform {
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<dyn PlatformTextSystem>,
    state: Mutex<IosPlatformState>,
}

pub(crate) struct IosPlatformState {
    finish_launching: Option<Box<dyn FnOnce()>>,
    quit_callback: Option<Box<dyn FnMut()>>,
    open_urls_callback: Option<Box<dyn FnMut(Vec<String>)>>,
    thermal_state_callback: Option<Box<dyn FnMut()>>,
}

impl Default for IosPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl IosPlatform {
    pub fn new() -> Self {
        let dispatcher = Arc::new(IosDispatcher);
        let background_executor = BackgroundExecutor::new(dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(dispatcher);
        let text_system: Arc<dyn PlatformTextSystem> = Arc::new(super::IosTextSystem::new());

        Self {
            background_executor,
            foreground_executor,
            text_system,
            state: Mutex::new(IosPlatformState {
                finish_launching: None,
                quit_callback: None,
                open_urls_callback: None,
                thermal_state_callback: None,
            }),
        }
    }
}

struct IosKeyboardLayout;

impl PlatformKeyboardLayout for IosKeyboardLayout {
    fn id(&self) -> &str {
        "ios-default"
    }

    fn name(&self) -> &str {
        "iOS Default"
    }
}

impl Platform for IosPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        let mut state = self.state.lock();
        state.finish_launching = Some(on_finish_launching);
        if let Some(callback) = state.finish_launching.take() {
            drop(state);
            super::ffi::set_finish_launching_callback(callback);
        }
        log::info!("GPUI iOS: Platform::run() completed, waiting for app delegate callback");
    }

    fn quit(&self) {
        log::warn!("iOS apps cannot programmatically quit");
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {
        log::warn!("iOS apps cannot restart themselves");
    }

    fn activate(&self, _ignoring_other_apps: bool) {}

    fn hide(&self) {}

    fn hide_other_apps(&self) {}

    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        IosDisplay::all()
            .into_iter()
            .map(|display| Rc::new(display) as Rc<dyn PlatformDisplay>)
            .collect()
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(IosDisplay::main()))
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        None
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        let window = Box::new(IosWindow::new(handle, options)?);
        window.register_with_ffi();
        Ok(window)
    }

    fn window_appearance(&self) -> WindowAppearance {
        unsafe {
            let style: i64 = {
                let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
                let key_window: *mut Object = msg_send![app, keyWindow];
                if key_window.is_null() {
                    return WindowAppearance::Light;
                }
                let trait_collection: *mut Object = msg_send![key_window, traitCollection];
                msg_send![trait_collection, userInterfaceStyle]
            };
            match style {
                2 => WindowAppearance::Dark,
                _ => WindowAppearance::Light,
            }
        }
    }

    fn open_url(&self, url: &str) {
        unsafe {
            let url_string = super::ns_string_from_str(url);
            let url: *mut Object = msg_send![class!(NSURL), URLWithString: url_string];
            let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
            let _: () = msg_send![app, openURL: url options: std::ptr::null::<Object>() completionHandler: std::ptr::null::<Object>()];
        }
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.state.lock().open_urls_callback = Some(callback);
    }

    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Err(anyhow!("File picker not yet implemented for iOS")));
        rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Err(anyhow!("Save dialog not yet implemented for iOS")));
        rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, _path: &Path) {}

    fn open_with_system(&self, _path: &Path) {}

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.state.lock().quit_callback = Some(callback);
    }

    fn on_reopen(&self, _callback: Box<dyn FnMut()>) {}

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}

    fn on_app_menu_action(&self, _callback: Box<dyn FnMut(&dyn Action)>) {}

    fn on_will_open_app_menu(&self, _callback: Box<dyn FnMut()>) {}

    fn on_validate_app_menu_command(&self, _callback: Box<dyn FnMut(&dyn Action) -> bool>) {}

    fn app_path(&self) -> Result<PathBuf> {
        unsafe {
            let bundle: *mut Object = msg_send![class!(NSBundle), mainBundle];
            let path: *mut Object = msg_send![bundle, bundlePath];
            let utf8: *const i8 = msg_send![path, UTF8String];
            if utf8.is_null() {
                return Err(anyhow!("Failed to get bundle path"));
            }
            let path_str = std::ffi::CStr::from_ptr(utf8).to_str()?;
            Ok(PathBuf::from(path_str))
        }
    }

    fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        let app_path = self.app_path()?;
        Ok(app_path.join(name))
    }

    fn set_cursor_style(&self, _style: CursorStyle) {}

    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        unsafe {
            let pasteboard: *mut Object = msg_send![class!(UIPasteboard), generalPasteboard];
            if let Some(text) = item.text() {
                let ns_string = super::ns_string_from_str(&text);
                let _: () = msg_send![pasteboard, setString: ns_string];
            }
        }
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        unsafe {
            let pasteboard: *mut Object = msg_send![class!(UIPasteboard), generalPasteboard];
            let string: *mut Object = msg_send![pasteboard, string];
            if string.is_null() {
                return None;
            }
            let utf8: *const i8 = msg_send![string, UTF8String];
            if utf8.is_null() {
                return None;
            }
            let text = std::ffi::CStr::from_ptr(utf8).to_str().ok()?;
            Some(ClipboardItem::new_string(text.to_string()))
        }
    }

    fn write_credentials(&self, _url: &str, _username: &str, _password: &[u8]) -> Task<Result<()>> {
        Task::ready(Err(anyhow!("Keychain not yet implemented for iOS")))
    }

    fn read_credentials(&self, _url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        Task::ready(Err(anyhow!("Keychain not yet implemented for iOS")))
    }

    fn delete_credentials(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Err(anyhow!("Keychain not yet implemented for iOS")))
    }

    fn on_keyboard_layout_change(&self, _callback: Box<dyn FnMut()>) {}

    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        self.state.lock().thermal_state_callback = Some(callback);
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(IosKeyboardLayout)
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }
}
