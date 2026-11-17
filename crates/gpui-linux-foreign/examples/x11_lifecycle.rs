//! Native X11 lifecycle fixture using the actual GPUI renderer and host parent.

// Rust guideline compliant 2026-02-21

#[cfg(target_os = "linux")]
mod native {
    use gpui::{
        AppContext, Context, InteractiveElement, IntoElement, MouseButton, Render, Styled, Window,
        div, rgb,
    };
    use gpui_linux_foreign::{editor::LinuxForeignContext, ffi, x11};
    use std::{
        cell::Cell,
        rc::Rc,
        time::{Duration, Instant},
    };
    use x11rb::{
        connection::Connection,
        protocol::xproto::{self, ConnectionExt, CreateWindowAux, EventMask, WindowClass},
        rust_connection::RustConnection,
    };

    use x11rb::protocol::xtest::ConnectionExt as _;

    struct Probe {
        context: Cell<*mut LinuxForeignContext>,
        rendering: Cell<bool>,
        dropped: Cell<usize>,
        closed: Cell<bool>,
        renders: Cell<usize>,
        clicks: Cell<usize>,
        close_in_render: Cell<bool>,
        close_in_input: Cell<bool>,
        connection: Rc<RustConnection>,
        parent: u32,
    }
    impl Probe {
        fn context(&self) -> *mut LinuxForeignContext {
            self.context.get()
        }
        fn children(&self) -> Vec<u32> {
            self.connection
                .query_tree(self.parent)
                .unwrap()
                .reply()
                .unwrap()
                .children
        }
    }
    fn close(context: *mut LinuxForeignContext) {
        let destroy: unsafe extern "C" fn(*mut LinuxForeignContext) = ffi::gpui_linux_x11_destroy;
        // SAFETY: baseline modes close a live context once; candidate modes use
        // the production ABI's checked opaque-token behavior for repeated calls.
        unsafe { destroy(context) };
    }
    fn frame(context: *mut LinuxForeignContext) {
        let request: unsafe extern "C" fn(*mut LinuxForeignContext) =
            ffi::gpui_linux_x11_request_frame;
        // SAFETY: baseline calls use live contexts; candidate stale tokens are checked.
        unsafe { request(context) };
    }
    struct View(Rc<Probe>);
    impl Drop for View {
        fn drop(&mut self) {
            assert!(!self.0.rendering.get(), "view dropped during render");
            self.0.dropped.set(self.0.dropped.get() + 1);
        }
    }
    impl Render for View {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            if self.0.context.get().is_null() {
                cx.spawn(async |view, cx| {
                    let _ = view.update(cx, |_, cx| cx.notify());
                })
                .detach();
            }
            self.0.rendering.set(true);
            self.0.renders.set(self.0.renders.get() + 1);
            if !self.0.context.get().is_null() && self.0.close_in_render.replace(false) {
                self.0.closed.set(true);
                close(self.0.context());
                assert_eq!(self.0.dropped.get(), 0, "app pin released during callback");
                assert_eq!(
                    self.0.children().len(),
                    1,
                    "native child destroyed during callback"
                );
            }
            self.0.rendering.set(false);
            let state = self.0.clone();
            div()
                .size_full()
                .bg(rgb(0x123456))
                .on_mouse_down(MouseButton::Left, move |_, _, _| {
                    state.clicks.set(state.clicks.get() + 1);
                    if state.close_in_input.replace(false) {
                        state.closed.set(true);
                        close(state.context());
                        assert_eq!(state.dropped.get(), 0, "app pin released during input");
                        assert_eq!(
                            state.children().len(),
                            1,
                            "native child destroyed during input"
                        );
                    }
                })
        }
    }
    fn attach(connection: Rc<RustConnection>, parent: u32) -> Rc<Probe> {
        attach_inner(connection, parent, false)
    }
    fn attach_inner(connection: Rc<RustConnection>, parent: u32, nested: bool) -> Rc<Probe> {
        let state = Rc::new(Probe {
            context: Cell::new(std::ptr::null_mut()),
            rendering: Cell::new(false),
            dropped: Cell::new(0),
            closed: Cell::new(false),
            renders: Cell::new(0),
            clicks: Cell::new(0),
            close_in_render: Cell::new(false),
            close_in_input: Cell::new(false),
            connection,
            parent,
        });
        let capture = state.clone();
        let context =
            x11::linux_x11_create_with(parent, 96, 96, 1.0, "native-lifecycle", move |_, cx| {
                if nested {
                    let rejected = x11::linux_x11_create_with::<View>(
                        parent,
                        96,
                        96,
                        1.0,
                        "nested",
                        |_, _| panic!("nested factory ran"),
                    );
                    assert!(rejected.is_null(), "nested attach was accepted");
                }
                cx.new(|_| View(capture))
            });
        assert!(!context.is_null(), "X11 attach failed");
        state.context.set(context);
        assert_eq!(
            state.children().len(),
            1,
            "expected exactly one foreign child"
        );
        state
    }
    fn pump_until(state: &Probe, predicate: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !predicate() {
            if state.closed.get() {
                break;
            }
            assert!(Instant::now() < deadline, "native callback timed out");
            frame(state.context());
            std::thread::sleep(Duration::from_millis(8));
        }
    }
    pub fn run() {
        let mode = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "ordinary".to_owned());
        let (connection, screen) = x11rb::connect(None).unwrap();
        let connection = Rc::new(connection);
        let root = connection.setup().roots[screen].root;
        let parent = connection.generate_id().unwrap();
        connection
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                parent,
                root,
                0,
                0,
                96,
                96,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new()
                    .background_pixel(0)
                    .event_mask(EventMask::STRUCTURE_NOTIFY),
            )
            .unwrap()
            .check()
            .unwrap();
        connection.map_window(parent).unwrap().check().unwrap();
        connection.flush().unwrap();
        if mode == "factory-panic" {
            let failed =
                x11::linux_x11_create_with::<View>(parent, 96, 96, 1.0, "panic", |_, _| {
                    panic!("root factory panic")
                });
            assert!(failed.is_null());
            assert!(
                connection
                    .query_tree(parent)
                    .unwrap()
                    .reply()
                    .unwrap()
                    .children
                    .is_empty()
            );
        }
        let state = attach_inner(connection.clone(), parent, mode == "nested-attach");
        match mode.as_str() {
            "render-close" => {
                state.close_in_render.set(true);
                pump_until(&state, || state.dropped.get() == 1);
            }
            "input-close" => {
                pump_until(&state, || state.renders.get() >= 2);
                state.close_in_input.set(true);
                let child = state.children()[0];
                assert_ne!(child, 0);
                connection
                    .xtest_fake_input(
                        xproto::MOTION_NOTIFY_EVENT,
                        0,
                        x11rb::CURRENT_TIME,
                        root,
                        8,
                        8,
                        0,
                    )
                    .unwrap()
                    .check()
                    .unwrap();
                connection
                    .xtest_fake_input(
                        xproto::BUTTON_PRESS_EVENT,
                        1,
                        x11rb::CURRENT_TIME,
                        root,
                        0,
                        0,
                        0,
                    )
                    .unwrap()
                    .check()
                    .unwrap();
                connection
                    .xtest_fake_input(
                        xproto::BUTTON_RELEASE_EVENT,
                        1,
                        x11rb::CURRENT_TIME,
                        root,
                        0,
                        0,
                        0,
                    )
                    .unwrap()
                    .check()
                    .unwrap();
                connection.flush().unwrap();
                pump_until(&state, || state.dropped.get() == 1);
                assert_eq!(state.clicks.get(), 1);
            }
            "adversarial" => {
                let address = state.context.get().addr();
                std::thread::spawn(move || {
                    let context = std::ptr::without_provenance_mut(address);
                    close(context);
                    frame(context);
                    let mut dropped = 777usize;
                    let mut coalesced = 888usize;
                    // SAFETY: outputs are live aligned local counters; context is a token.
                    let () = unsafe {
                        ffi::gpui_linux_x11_frame_stats(
                            context,
                            &raw mut dropped,
                            &raw mut coalesced,
                        );
                    };
                    assert_eq!((dropped, coalesced), (777, 888));
                })
                .join()
                .unwrap();
                assert_eq!(state.children().len(), 1);
                let wrong_destroy: unsafe extern "C" fn(*mut LinuxForeignContext) =
                    ffi::gpui_linux_wayland_destroy;
                // SAFETY: context is a checked candidate token; no host objects are read.
                unsafe { wrong_destroy(state.context()) };
                assert_eq!(state.children().len(), 1);
                close(state.context());
                close(state.context());
                frame(state.context());
                for _ in 0..16 {
                    let replacement = attach(connection.clone(), parent);
                    assert_ne!(replacement.context.get().addr(), address);
                    close(std::ptr::without_provenance_mut(address));
                    frame(std::ptr::without_provenance_mut(address));
                    assert_eq!(replacement.children().len(), 1);
                    close(replacement.context());
                    assert_eq!(replacement.dropped.get(), 1);
                }
                let forged = std::ptr::without_provenance_mut(usize::MAX);
                close(forged);
                frame(forged);
            }
            "ordinary" | "factory-panic" | "nested-attach" => {
                pump_until(&state, || state.renders.get() >= 2);
                close(state.context());
            }
            _ => panic!("unknown fixture mode"),
        }
        assert_eq!(state.dropped.get(), 1, "view/app pin leaked after close");
        assert!(state.children().is_empty(), "foreign child leaked");
        connection.destroy_window(parent).unwrap().check().unwrap();
        connection.flush().unwrap();
        println!(
            "{{\"mode\":\"{mode}\",\"renders\":{},\"clicks\":{},\"drops\":{},\"remaining_children\":0}}",
            state.renders.get(),
            state.clicks.get(),
            state.dropped.get()
        );
    }
}
#[cfg(target_os = "linux")]
fn main() {
    native::run();
}
#[cfg(not(target_os = "linux"))]
fn main() {}
