use crate :: { BoolExt , DisplayLink , renderer } ;
use cocoa :: { appkit :: { NSScreen , NSView , NSWindow , NSWindowButton , NSWindowOcclusionState , NSWindowStyleMask } , base :: { id , nil } , foundation :: { NSPoint , NSRect , NSSize } } ;
use gpui :: { AnyWindowHandle , BackgroundExecutor , Bounds , FileDropEvent , ForegroundExecutor , KeyDownEvent , Keystroke , MouseMoveEvent , Pixels , PlatformInput , PlatformInputHandler , Point , RequestFrameOptions , Size , WindowBackgroundAppearance , WindowBounds , point , px , size } ;
use core_graphics :: display :: { CGPoint , CGRect } ;
use objc :: { msg_send , runtime :: { BOOL , Object , Sel } , sel , sel_impl } ;
use parking_lot::Mutex;
use std :: { ffi :: { c_void } , ptr :: { NonNull } , sync :: { Arc , Weak , atomic :: { AtomicBool } } , time :: Duration } ;
use util::ResultExt;
use super::consts::WINDOW_STATE_IVAR;
use super::display::display_id_for_screen;
use super::get::get_scale_factor;
use super::get::get_window_state;
use super::get::step;
use super::misc::convert_mouse_position;

pub(super) struct MacWindowState {
    pub(super) handle: AnyWindowHandle,
    pub(super) foreground_executor: ForegroundExecutor,
    pub(super) background_executor: BackgroundExecutor,
    pub(super) native_window: id,
    pub(super) native_view: NonNull<Object>,
    pub(super) blurred_view: Option<id>,
    pub(super) background_appearance: WindowBackgroundAppearance,
    pub(super) display_link: Option<DisplayLink>,
    pub(super) renderer: renderer::Renderer,
    pub(super) request_frame_callback: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(super) event_callback: Option<Box<dyn FnMut(PlatformInput) -> gpui::DispatchEventResult>>,
    pub(super) activate_callback: Option<Box<dyn FnMut(bool)>>,
    pub(super) resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    pub(super) moved_callback: Option<Box<dyn FnMut()>>,
    pub(super) should_close_callback: Option<Box<dyn FnMut() -> bool>>,
    pub(super) close_callback: Option<Box<dyn FnOnce()>>,
    pub(super) appearance_changed_callback: Option<Box<dyn FnMut()>>,
    pub(super) input_handler: Option<PlatformInputHandler>,
    pub(super) last_key_equivalent: Option<KeyDownEvent>,
    pub(super) synthetic_drag_counter: usize,
    pub(super) traffic_light_position: Option<Point<Pixels>>,
    pub(super) transparent_titlebar: bool,
    pub(super) previous_modifiers_changed_event: Option<PlatformInput>,
    pub(super) keystroke_for_do_command: Option<Keystroke>,
    pub(super) do_command_handled: Option<bool>,
    pub(super) external_files_dragged: bool,
    // Whether the next left-mouse click is also the focusing click.
    pub(super) first_mouse: bool,
    pub(super) fullscreen_restore_bounds: Bounds<Pixels>,
    pub(super) move_tab_to_new_window_callback: Option<Box<dyn FnMut()>>,
    pub(super) merge_all_windows_callback: Option<Box<dyn FnMut()>>,
    pub(super) select_next_tab_callback: Option<Box<dyn FnMut()>>,
    pub(super) select_previous_tab_callback: Option<Box<dyn FnMut()>>,
    pub(super) toggle_tab_bar_callback: Option<Box<dyn FnMut()>>,
    pub(super) activated_least_once: bool,
    pub(super) closed: Arc<AtomicBool>,
    // The parent window if this window is a sheet (Dialog kind)
    pub(super) sheet_parent: Option<id>,
}

impl MacWindowState {
    pub(super) fn move_traffic_light(&self) {
        if let Some(traffic_light_position) = self.traffic_light_position {
            if self.is_fullscreen() {
                // Moving traffic lights while fullscreen doesn't work,
                // see https://github.com/zed-industries/zed/issues/4712
                return;
            }

            let titlebar_height = self.titlebar_height();

            unsafe {
                let close_button: id = msg_send![
                    self.native_window,
                    standardWindowButton: NSWindowButton::NSWindowCloseButton
                ];
                let min_button: id = msg_send![
                    self.native_window,
                    standardWindowButton: NSWindowButton::NSWindowMiniaturizeButton
                ];
                let zoom_button: id = msg_send![
                    self.native_window,
                    standardWindowButton: NSWindowButton::NSWindowZoomButton
                ];

                let mut close_button_frame: CGRect = msg_send![close_button, frame];
                let mut min_button_frame: CGRect = msg_send![min_button, frame];
                let mut zoom_button_frame: CGRect = msg_send![zoom_button, frame];
                let mut origin = point(
                    traffic_light_position.x,
                    titlebar_height
                        - traffic_light_position.y
                        - px(close_button_frame.size.height as f32),
                );
                let button_spacing =
                    px((min_button_frame.origin.x - close_button_frame.origin.x) as f32);

                close_button_frame.origin = CGPoint::new(origin.x.into(), origin.y.into());
                let _: () = msg_send![close_button, setFrame: close_button_frame];
                origin.x += button_spacing;

                min_button_frame.origin = CGPoint::new(origin.x.into(), origin.y.into());
                let _: () = msg_send![min_button, setFrame: min_button_frame];
                origin.x += button_spacing;

                zoom_button_frame.origin = CGPoint::new(origin.x.into(), origin.y.into());
                let _: () = msg_send![zoom_button, setFrame: zoom_button_frame];
                origin.x += button_spacing;
            }
        }
    }

    pub(super) fn start_display_link(&mut self) {
        self.stop_display_link();
        unsafe {
            if !self
                .native_window
                .occlusionState()
                .contains(NSWindowOcclusionState::NSWindowOcclusionStateVisible)
            {
                return;
            }
        }
        let display_id = unsafe { display_id_for_screen(self.native_window.screen()) };
        if let Some(mut display_link) =
            DisplayLink::new(display_id, self.native_view.as_ptr() as *mut c_void, step).log_err()
        {
            display_link.start().log_err();
            self.display_link = Some(display_link);
        }
    }

    pub(super) fn stop_display_link(&mut self) {
        self.display_link = None;
    }

    pub(super) fn is_maximized(&self) -> bool {
        fn rect_to_size(rect: NSRect) -> Size<Pixels> {
            let NSSize { width, height } = rect.size;
            size(width.into(), height.into())
        }

        unsafe {
            let bounds = self.bounds();
            let screen_size = rect_to_size(self.native_window.screen().visibleFrame());
            bounds.size == screen_size
        }
    }

    pub(super) fn is_fullscreen(&self) -> bool {
        unsafe {
            let style_mask = self.native_window.styleMask();
            style_mask.contains(NSWindowStyleMask::NSFullScreenWindowMask)
        }
    }

    pub(super) fn bounds(&self) -> Bounds<Pixels> {
        let mut window_frame = unsafe { NSWindow::frame(self.native_window) };
        let screen = unsafe { NSWindow::screen(self.native_window) };
        if screen == nil {
            return Bounds::new(point(px(0.), px(0.)), gpui::DEFAULT_WINDOW_SIZE);
        }
        let screen_frame = unsafe { NSScreen::frame(screen) };

        // Flip the y coordinate to be top-left origin
        window_frame.origin.y =
            screen_frame.size.height - window_frame.origin.y - window_frame.size.height;

        Bounds::new(
            point(
                px((window_frame.origin.x - screen_frame.origin.x) as f32),
                px((window_frame.origin.y + screen_frame.origin.y) as f32),
            ),
            size(
                px(window_frame.size.width as f32),
                px(window_frame.size.height as f32),
            ),
        )
    }

    pub(super) fn content_size(&self) -> Size<Pixels> {
        let NSSize { width, height, .. } =
            unsafe { NSView::frame(self.native_window.contentView()) }.size;
        size(px(width as f32), px(height as f32))
    }

    pub(super) fn scale_factor(&self) -> f32 {
        get_scale_factor(self.native_window)
    }

    pub(super) fn titlebar_height(&self) -> Pixels {
        unsafe {
            let frame = NSWindow::frame(self.native_window);
            let content_layout_rect: CGRect = msg_send![self.native_window, contentLayoutRect];
            px((frame.size.height - content_layout_rect.size.height) as f32)
        }
    }

    pub(super) fn window_bounds(&self) -> WindowBounds {
        if self.is_fullscreen() {
            WindowBounds::Fullscreen(self.fullscreen_restore_bounds)
        } else {
            WindowBounds::Windowed(self.bounds())
        }
    }
}

unsafe impl Send for MacWindowState {}

pub(super) unsafe fn drop_window_state(object: &Object) {
    unsafe {
        let raw: *mut c_void = *object.get_ivar(WINDOW_STATE_IVAR);
        Arc::from_raw(raw as *mut Mutex<MacWindowState>);
    }
}

pub(super) fn update_window_scale_factor(window_state: &Arc<Mutex<MacWindowState>>) {
    let mut lock = window_state.as_ref().lock();
    let scale_factor = lock.scale_factor();
    let size = lock.content_size();
    let drawable_size = size.to_device_pixels(scale_factor);
    if let Some(layer) = lock.renderer.layer() {
        unsafe {
            let _: () = msg_send![
                layer,
                setContentsScale: scale_factor as f64
            ];
        }
    }

    lock.renderer.update_drawable_size(drawable_size);

    if let Some(mut callback) = lock.resize_callback.take() {
        let content_size = lock.content_size();
        let scale_factor = lock.scale_factor();
        drop(lock);
        callback(content_size, scale_factor);
        window_state.as_ref().lock().resize_callback = Some(callback);
    };
}

pub(super) extern "C" fn perform_drag_operation(this: &Object, _: Sel, dragging_info: id) -> BOOL {
    let window_state = unsafe { get_window_state(this) };
    let position = drag_event_position(&window_state, dragging_info);
    send_file_drop_event(window_state, FileDropEvent::Submit { position }).to_objc()
}

pub(super) extern "C" fn conclude_drag_operation(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    send_file_drop_event(window_state, FileDropEvent::Exited);
}

pub(super) async fn synthetic_drag(
    window_state: Weak<Mutex<MacWindowState>>,
    drag_id: usize,
    event: MouseMoveEvent,
    executor: BackgroundExecutor,
) {
    loop {
        executor.timer(Duration::from_millis(16)).await;
        if let Some(window_state) = window_state.upgrade() {
            let mut lock = window_state.lock();
            if lock.synthetic_drag_counter == drag_id {
                if let Some(mut callback) = lock.event_callback.take() {
                    drop(lock);
                    callback(PlatformInput::MouseMove(event.clone()));
                    window_state.lock().event_callback = Some(callback);
                }
            } else {
                break;
            }
        }
    }
}

/// Sends the specified FileDropEvent using `PlatformInput::FileDrop` to the window
/// state and updates the window state according to the event passed.
pub(super) fn send_file_drop_event(
    window_state: Arc<Mutex<MacWindowState>>,
    file_drop_event: FileDropEvent,
) -> bool {
    let external_files_dragged = match file_drop_event {
        FileDropEvent::Entered { .. } => Some(true),
        FileDropEvent::Exited => Some(false),
        _ => None,
    };

    let mut lock = window_state.lock();
    if let Some(mut callback) = lock.event_callback.take() {
        drop(lock);
        callback(PlatformInput::FileDrop(file_drop_event));
        let mut lock = window_state.lock();
        lock.event_callback = Some(callback);
        if let Some(external_files_dragged) = external_files_dragged {
            lock.external_files_dragged = external_files_dragged;
        }
        true
    } else {
        false
    }
}

pub(super) fn drag_event_position(window_state: &Mutex<MacWindowState>, dragging_info: id) -> Point<Pixels> {
    let drag_location: NSPoint = unsafe { msg_send![dragging_info, draggingLocation] };
    convert_mouse_position(drag_location, window_state.lock().content_size().height)
}

