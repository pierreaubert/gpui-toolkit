//! AU Platform implementation — implements the GPUI Platform trait for
//! macOS Audio Unit extensions (embedded in a host DAW's process).
//!
//! Key differences from a standalone macOS app platform:
//! - `run()` does NOT call [NSApp run] — the host DAW owns the event loop
//! - No NSApplication access (EXTENSION_SAFE_API_ONLY)
//! - No menus, dock menu, file dialogs, or other app-level features
//! - Clipboard uses NSPasteboard (macOS) instead of UIPasteboard

use super::AuDisplay;
use super::{AuDispatcher, AuTextSystem};
use anyhow::anyhow;
use futures::channel::oneshot;
use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, DummyKeyboardMapper,
    ForegroundExecutor, Keymap, Menu, MenuItem, PathPromptOptions, Platform, PlatformDisplay,
    PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem, PlatformWindow, Result,
    Task, ThermalState, WindowAppearance, WindowParams,
};
use objc::runtime::{Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use super::AuWindow;

pub struct AuPlatform(Mutex<AuPlatformState>);

struct AuPlatformState {
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<dyn PlatformTextSystem>,
    cursor_style: CursorStyle,
    cursor_visible: bool,
    _finish_launching: Option<Box<dyn FnOnce()>>,
}

impl Default for AuPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl AuPlatform {
    pub fn new() -> Self {
        let dispatcher = Arc::new(AuDispatcher);
        let text_system: Arc<dyn PlatformTextSystem> = Arc::new(AuTextSystem::new());

        Self(Mutex::new(AuPlatformState {
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            text_system,
            cursor_style: CursorStyle::Arrow,
            cursor_visible: true,
            _finish_launching: None,
        }))
    }
}

struct AuKeyboardLayout;

impl PlatformKeyboardLayout for AuKeyboardLayout {
    fn id(&self) -> &str {
        "au-default"
    }

    fn name(&self) -> &str {
        "AU Default"
    }
}

impl Platform for AuPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.0.lock().background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.0.lock().foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.0.lock().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        use crate::helpers::nslog;
        nslog(b"SOTF AuPlatform::run: calling on_finish_launching immediately");
        // AU extensions don't own the application event loop.
        // Call on_finish_launching immediately — the host DAW's run loop is already active.
        on_finish_launching();
        nslog(b"SOTF AuPlatform::run: on_finish_launching completed");
    }

    fn quit(&self) {
        log::warn!("AU extensions cannot quit the host application");
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {
        log::warn!("AU extensions cannot restart");
    }

    fn activate(&self, _ignoring_other_apps: bool) {}
    fn hide(&self) {}
    fn hide_other_apps(&self) {}
    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![Rc::new(AuDisplay::main()) as Rc<dyn PlatformDisplay>]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(AuDisplay::main()))
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        None
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        use crate::helpers::nslog;
        nslog(b"SOTF AuPlatform::open_window: entry");
        let window = Box::new(AuWindow::new(handle, options)?);
        AuWindow::register_global(&window);
        nslog(b"SOTF AuPlatform::open_window: done");
        Ok(window)
    }

    fn window_appearance(&self) -> WindowAppearance {
        // Query the effective appearance from NSApp (if available in extension context)
        unsafe {
            let appearance_name: *mut objc::runtime::Object = {
                let app: *mut objc::runtime::Object =
                    msg_send![class!(NSAppearance), currentDrawingAppearance];
                if app.is_null() {
                    return WindowAppearance::Light;
                }
                msg_send![app, name]
            };
            if appearance_name.is_null() {
                return WindowAppearance::Light;
            }
            if crate::helpers::is_dark_aqua_appearance_name(appearance_name) {
                WindowAppearance::Dark
            } else {
                WindowAppearance::Light
            }
        }
    }

    fn open_url(&self, _url: &str) {
        // Extension-safe: cannot open URLs
    }

    fn on_open_urls(&self, _callback: Box<dyn FnMut(Vec<String>)>) {}

    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Err(anyhow!("File picker not available in AU extension")));
        rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Err(anyhow!("Save dialog not available in AU extension")));
        rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, _path: &Path) {}
    fn open_with_system(&self, _path: &Path) {}
    fn on_quit(&self, _callback: Box<dyn FnMut()>) {}
    fn on_reopen(&self, _callback: Box<dyn FnMut()>) {}
    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}
    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}
    fn on_app_menu_action(&self, _callback: Box<dyn FnMut(&dyn Action)>) {}
    fn on_will_open_app_menu(&self, _callback: Box<dyn FnMut()>) {}
    fn on_validate_app_menu_command(&self, _callback: Box<dyn FnMut(&dyn Action) -> bool>) {}

    fn app_path(&self) -> Result<PathBuf> {
        unsafe {
            let bundle: *mut objc::runtime::Object = msg_send![class!(NSBundle), mainBundle];
            let path: *mut objc::runtime::Object = msg_send![bundle, bundlePath];
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

    fn set_cursor_style(&self, style: CursorStyle) {
        let action = au_cursor_action(style);
        {
            let mut state = self.0.lock();
            state.cursor_style = style;
            state.cursor_visible = action != AuCursorAction::HideUntilMouseMoves;
        }

        apply_au_cursor_action(action);
    }

    fn hide_cursor_until_mouse_moves(&self) {
        self.0.lock().cursor_visible = false;
        apply_au_cursor_action(AuCursorAction::HideUntilMouseMoves);
    }

    fn is_cursor_visible(&self) -> bool {
        self.0.lock().cursor_visible
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        unsafe {
            let pasteboard: *mut objc::runtime::Object =
                msg_send![class!(NSPasteboard), generalPasteboard];
            let _: () = msg_send![pasteboard, clearContents];
            if let Some(text) = item.text() {
                let ns_string = crate::helpers::ns_string_from_str(&text);
                let array: *mut objc::runtime::Object =
                    msg_send![class!(NSArray), arrayWithObject: ns_string];
                let _: bool = msg_send![pasteboard, writeObjects: array];
            }
        }
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        unsafe {
            let pasteboard: *mut objc::runtime::Object =
                msg_send![class!(NSPasteboard), generalPasteboard];
            let string_type: *mut objc::runtime::Object = msg_send![
                class!(NSString),
                stringWithUTF8String: c"public.utf8-plain-text".as_ptr()
            ];
            let string: *mut objc::runtime::Object =
                msg_send![pasteboard, stringForType: string_type];
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

    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        let url = url.to_owned();
        let username = username.to_owned();
        let password = password.to_vec();
        self.background_executor()
            .spawn(async move { crate::keychain::write_credentials(&url, &username, &password) })
    }

    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        let url = url.to_owned();
        self.background_executor()
            .spawn(async move { crate::keychain::read_credentials(&url) })
    }

    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        let url = url.to_owned();
        self.background_executor()
            .spawn(async move { crate::keychain::delete_credentials(&url) })
    }

    fn on_keyboard_layout_change(&self, _callback: Box<dyn FnMut()>) {}

    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }

    fn on_thermal_state_change(&self, _callback: Box<dyn FnMut()>) {}

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(AuKeyboardLayout)
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }

    fn read_from_find_pasteboard(&self) -> Option<ClipboardItem> {
        None
    }

    fn write_to_find_pasteboard(&self, _item: ClipboardItem) {}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuCursorAction {
    HideUntilMouseMoves,
    Set(&'static str),
}

fn au_cursor_action(style: CursorStyle) -> AuCursorAction {
    match style {
        CursorStyle::Arrow => AuCursorAction::Set("arrowCursor"),
        CursorStyle::IBeam => AuCursorAction::Set("IBeamCursor"),
        CursorStyle::Crosshair => AuCursorAction::Set("crosshairCursor"),
        CursorStyle::ClosedHand => AuCursorAction::Set("closedHandCursor"),
        CursorStyle::OpenHand => AuCursorAction::Set("openHandCursor"),
        CursorStyle::PointingHand => AuCursorAction::Set("pointingHandCursor"),
        CursorStyle::ResizeLeftRight => AuCursorAction::Set("resizeLeftRightCursor"),
        CursorStyle::ResizeUpDown => AuCursorAction::Set("resizeUpDownCursor"),
        CursorStyle::ResizeLeft => AuCursorAction::Set("resizeLeftCursor"),
        CursorStyle::ResizeRight => AuCursorAction::Set("resizeRightCursor"),
        CursorStyle::ResizeColumn => AuCursorAction::Set("resizeLeftRightCursor"),
        CursorStyle::ResizeRow => AuCursorAction::Set("resizeUpDownCursor"),
        CursorStyle::ResizeUp => AuCursorAction::Set("resizeUpCursor"),
        CursorStyle::ResizeDown => AuCursorAction::Set("resizeDownCursor"),
        // AU extensions avoid private AppKit selectors for diagonal resize cursors.
        CursorStyle::ResizeUpLeftDownRight => AuCursorAction::Set("resizeLeftRightCursor"),
        CursorStyle::ResizeUpRightDownLeft => AuCursorAction::Set("resizeUpDownCursor"),
        CursorStyle::IBeamCursorForVerticalLayout => {
            AuCursorAction::Set("IBeamCursorForVerticalLayout")
        }
        CursorStyle::OperationNotAllowed => AuCursorAction::Set("operationNotAllowedCursor"),
        CursorStyle::DragLink => AuCursorAction::Set("dragLinkCursor"),
        CursorStyle::DragCopy => AuCursorAction::Set("dragCopyCursor"),
        CursorStyle::ContextualMenu => AuCursorAction::Set("contextualMenuCursor"),
    }
}

fn apply_au_cursor_action(action: AuCursorAction) {
    unsafe {
        match action {
            AuCursorAction::HideUntilMouseMoves => {
                let _: () = msg_send![class!(NSCursor), setHiddenUntilMouseMoves: true];
            }
            AuCursorAction::Set(selector_name) => {
                let selector = Sel::register(selector_name);
                let new_cursor: *mut Object =
                    msg_send![class!(NSCursor), performSelector: selector];
                if new_cursor.is_null() {
                    return;
                }

                let old_cursor: *mut Object = msg_send![class!(NSCursor), currentCursor];
                if new_cursor != old_cursor {
                    let _: () = msg_send![new_cursor, set];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuCursorAction, au_cursor_action};
    use gpui::CursorStyle;

    #[test]
    fn cursor_style_maps_to_extension_safe_appkit_selectors() {
        let cases = [
            (CursorStyle::Arrow, AuCursorAction::Set("arrowCursor")),
            (CursorStyle::IBeam, AuCursorAction::Set("IBeamCursor")),
            (
                CursorStyle::Crosshair,
                AuCursorAction::Set("crosshairCursor"),
            ),
            (
                CursorStyle::ClosedHand,
                AuCursorAction::Set("closedHandCursor"),
            ),
            (CursorStyle::OpenHand, AuCursorAction::Set("openHandCursor")),
            (
                CursorStyle::PointingHand,
                AuCursorAction::Set("pointingHandCursor"),
            ),
            (
                CursorStyle::ResizeLeftRight,
                AuCursorAction::Set("resizeLeftRightCursor"),
            ),
            (
                CursorStyle::ResizeUpDown,
                AuCursorAction::Set("resizeUpDownCursor"),
            ),
            (
                CursorStyle::ResizeLeft,
                AuCursorAction::Set("resizeLeftCursor"),
            ),
            (
                CursorStyle::ResizeRight,
                AuCursorAction::Set("resizeRightCursor"),
            ),
            (
                CursorStyle::ResizeColumn,
                AuCursorAction::Set("resizeLeftRightCursor"),
            ),
            (
                CursorStyle::ResizeRow,
                AuCursorAction::Set("resizeUpDownCursor"),
            ),
            (CursorStyle::ResizeUp, AuCursorAction::Set("resizeUpCursor")),
            (
                CursorStyle::ResizeDown,
                AuCursorAction::Set("resizeDownCursor"),
            ),
            (
                CursorStyle::ResizeUpLeftDownRight,
                AuCursorAction::Set("resizeLeftRightCursor"),
            ),
            (
                CursorStyle::ResizeUpRightDownLeft,
                AuCursorAction::Set("resizeUpDownCursor"),
            ),
            (
                CursorStyle::IBeamCursorForVerticalLayout,
                AuCursorAction::Set("IBeamCursorForVerticalLayout"),
            ),
            (
                CursorStyle::OperationNotAllowed,
                AuCursorAction::Set("operationNotAllowedCursor"),
            ),
            (CursorStyle::DragLink, AuCursorAction::Set("dragLinkCursor")),
            (CursorStyle::DragCopy, AuCursorAction::Set("dragCopyCursor")),
            (
                CursorStyle::ContextualMenu,
                AuCursorAction::Set("contextualMenuCursor"),
            ),
        ];

        for (style, expected) in cases {
            assert_eq!(au_cursor_action(style), expected);
        }
    }
}
