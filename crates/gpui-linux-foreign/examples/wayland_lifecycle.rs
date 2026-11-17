//! Native Wayland host/display lifecycle fixture with a real compositor.

// Rust guideline compliant 2026-02-21

#[cfg(target_os = "linux")]
mod native {
    use gpui::{AppContext, Context, IntoElement, Render, Styled, Window, div, rgb};
    use gpui_linux_foreign::{editor::LinuxForeignContext, ffi, wayland};
    use std::{
        cell::Cell,
        fs::OpenOptions,
        io::Write,
        os::fd::AsFd,
        rc::Rc,
        time::{Duration, Instant},
    };
    use wayland_client::{
        Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
        protocol::{wl_buffer, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface},
    };
    use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

    #[derive(Default)]
    struct Host {
        compositor: Option<wl_compositor::WlCompositor>,
        shm: Option<wl_shm::WlShm>,
        shell: Option<xdg_wm_base::XdgWmBase>,
        configured: bool,
    }
    impl Dispatch<wl_registry::WlRegistry, ()> for Host {
        fn event(
            state: &mut Self,
            proxy: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            (): &(),
            _: &Connection,
            qhandle: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
            {
                match interface.as_str() {
                    "wl_compositor" => {
                        state.compositor = Some(proxy.bind(name, version.min(4), qhandle, ()));
                    }
                    "wl_shm" => state.shm = Some(proxy.bind(name, 1, qhandle, ())),
                    "xdg_wm_base" => {
                        state.shell = Some(proxy.bind(name, version.min(2), qhandle, ()));
                    }
                    _ => {}
                }
            }
        }
    }
    impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Host {
        fn event(
            _: &mut Self,
            proxy: &xdg_wm_base::XdgWmBase,
            event: xdg_wm_base::Event,
            (): &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let xdg_wm_base::Event::Ping { serial } = event {
                proxy.pong(serial);
            }
        }
    }
    impl Dispatch<xdg_surface::XdgSurface, ()> for Host {
        fn event(
            state: &mut Self,
            proxy: &xdg_surface::XdgSurface,
            event: xdg_surface::Event,
            (): &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let xdg_surface::Event::Configure { serial } = event {
                proxy.ack_configure(serial);
                state.configured = true;
            }
        }
    }
    delegate_noop!(Host: ignore wl_compositor::WlCompositor);
    delegate_noop!(Host: ignore wl_surface::WlSurface);
    delegate_noop!(Host: ignore wl_shm::WlShm);
    delegate_noop!(Host: ignore wl_shm_pool::WlShmPool);
    delegate_noop!(Host: ignore wl_buffer::WlBuffer);
    delegate_noop!(Host: ignore xdg_toplevel::XdgToplevel);

    struct Probe {
        context: Cell<*mut LinuxForeignContext>,
        renders: Cell<usize>,
        rendering: Cell<bool>,
        dropped: Cell<usize>,
        closed: Cell<bool>,
        close_in_render: Cell<bool>,
    }
    struct View(Rc<Probe>);
    impl Drop for View {
        fn drop(&mut self) {
            assert!(!self.0.rendering.get());
            self.0.dropped.set(self.0.dropped.get() + 1);
        }
    }
    fn context(state: &Probe) -> *mut LinuxForeignContext {
        state.context.get()
    }
    fn close(context: *mut LinuxForeignContext) {
        let function: unsafe extern "C" fn(*mut LinuxForeignContext) =
            ffi::gpui_linux_wayland_destroy;
        // SAFETY: baseline controls use a live context once; candidate tokens are checked.
        unsafe { function(context) };
    }
    fn frame(context: *mut LinuxForeignContext) {
        let function: unsafe extern "C" fn(*mut LinuxForeignContext) =
            ffi::gpui_linux_wayland_request_frame;
        // SAFETY: baseline controls use live contexts; candidate stale tokens are checked.
        unsafe { function(context) };
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
                close(context(&self.0));
                assert_eq!(self.0.dropped.get(), 0, "app pin released during callback");
            }
            self.0.rendering.set(false);
            div().size_full().bg(rgb(0x123456))
        }
    }
    fn attach(connection: &Connection, parent: &wl_surface::WlSurface) -> Rc<Probe> {
        attach_inner(connection, parent, false)
    }
    fn attach_inner(
        connection: &Connection,
        parent: &wl_surface::WlSurface,
        nested: bool,
    ) -> Rc<Probe> {
        let state = Rc::new(Probe {
            context: Cell::new(std::ptr::null_mut()),
            renders: Cell::new(0),
            rendering: Cell::new(false),
            dropped: Cell::new(0),
            closed: Cell::new(false),
            close_in_render: Cell::new(false),
        });
        let capture = state.clone();
        let display = connection.backend().display_ptr().cast();
        let parent_surface = parent.id().as_ptr().cast();
        let factory = move |_: &mut Window, cx: &mut gpui::App| {
            if nested {
                // SAFETY: same live host objects and UI thread as the outer attach.
                let rejected = unsafe {
                    wayland::linux_wayland_create_with::<View>(
                        display,
                        parent_surface,
                        96,
                        96,
                        1.0,
                        "nested",
                        |_, _| panic!("nested factory ran"),
                    )
                };
                assert!(rejected.is_null(), "nested attach was accepted");
            }
            cx.new(|_| View(capture))
        };
        // SAFETY: host owns connection and parent on this thread until the editor
        // and every active callback have returned. No state crosses threads.
        let editor = unsafe {
            wayland::linux_wayland_create_with(
                display,
                parent_surface,
                96,
                96,
                1.0,
                "native-lifecycle",
                factory,
            )
        };
        assert!(!editor.is_null(), "Wayland attach failed");
        state.context.set(editor);
        state
    }
    pub fn run() {
        let mode = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "ordinary".to_owned());
        let connection = Connection::connect_to_env().unwrap();
        let mut queue = connection.new_event_queue();
        let qh = queue.handle();
        let mut host = Host::default();
        let registry = connection.display().get_registry(&qh, ());
        queue.roundtrip(&mut host).unwrap();
        let surface = host.compositor.as_ref().unwrap().create_surface(&qh, ());
        let shell_surface = host
            .shell
            .as_ref()
            .unwrap()
            .get_xdg_surface(&surface, &qh, ());
        let toplevel = shell_surface.get_toplevel(&qh, ());
        toplevel.set_title("GPUI foreign host".to_owned());
        surface.commit();
        while !host.configured {
            queue.blocking_dispatch(&mut host).unwrap();
        }
        let path =
            std::env::temp_dir().join(format!("gpui-wayland-host-{}.argb", std::process::id()));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let bytes = vec![255u8; 96 * 96 * 4];
        file.write_all(&bytes).unwrap();
        file.flush().unwrap();
        let pool =
            host.shm
                .as_ref()
                .unwrap()
                .create_pool(file.as_fd(), bytes.len() as i32, &qh, ());
        let buffer = pool.create_buffer(0, 96, 96, 96 * 4, wl_shm::Format::Argb8888, &qh, ());
        surface.attach(Some(&buffer), 0, 0);
        surface.damage(0, 0, 96, 96);
        surface.commit();
        connection.flush().unwrap();
        queue.roundtrip(&mut host).unwrap();
        if mode == "factory-panic" {
            // SAFETY: live host objects remain owned on this UI thread through unwind.
            let failed = unsafe {
                wayland::linux_wayland_create_with::<View>(
                    connection.backend().display_ptr().cast(),
                    surface.id().as_ptr().cast(),
                    96,
                    96,
                    1.0,
                    "panic",
                    |_, _| panic!("root factory panic"),
                )
            };
            assert!(failed.is_null());
            queue.roundtrip(&mut host).unwrap();
        }
        let state = attach_inner(&connection, &surface, mode == "nested-attach");
        let deadline = Instant::now() + Duration::from_secs(5);
        if mode == "render-close" {
            state.close_in_render.set(true);
        }
        while (mode == "render-close" && state.dropped.get() == 0)
            || (mode != "render-close" && state.renders.get() < 2)
        {
            if state.closed.get() {
                break;
            }
            assert!(Instant::now() < deadline, "Wayland callback timed out");
            frame(context(&state));
            queue.dispatch_pending(&mut host).unwrap();
            connection.flush().unwrap();
            std::thread::sleep(Duration::from_millis(8));
        }
        if mode == "adversarial" {
            let address = state.context.get().addr();
            std::thread::spawn(move || {
                let token = std::ptr::without_provenance_mut(address);
                close(token);
                frame(token);
            })
            .join()
            .unwrap();
            assert_eq!(state.dropped.get(), 0);
            let wrong_destroy: unsafe extern "C" fn(*mut LinuxForeignContext) =
                ffi::gpui_linux_x11_destroy;
            // SAFETY: candidate API validates the token's backend before accessing state.
            unsafe { wrong_destroy(context(&state)) };
            assert_eq!(state.dropped.get(), 0);
            close(context(&state));
            close(context(&state));
            frame(context(&state));
            for _ in 0..8 {
                let replacement = attach(&connection, &surface);
                assert_ne!(replacement.context.get().addr(), address);
                close(std::ptr::without_provenance_mut(address));
                assert_eq!(replacement.dropped.get(), 0);
                close(context(&replacement));
                assert_eq!(replacement.dropped.get(), 1);
            }
        } else if matches!(
            mode.as_str(),
            "ordinary" | "factory-panic" | "nested-attach"
        ) {
            close(context(&state));
        } else {
            assert_eq!(mode, "render-close");
        }
        assert_eq!(state.dropped.get(), 1, "app/view pin leaked");
        // A roundtrip after guest teardown proves the host display remains live.
        queue.roundtrip(&mut host).unwrap();
        toplevel.destroy();
        shell_surface.destroy();
        surface.destroy();
        buffer.destroy();
        pool.destroy();
        connection.flush().unwrap();
        queue.roundtrip(&mut host).unwrap();
        drop(registry);
        std::fs::remove_file(path).unwrap();
        println!(
            "{{\"mode\":\"{mode}\",\"renders\":{},\"drops\":{},\"host_roundtrip_after_close\":true}}",
            state.renders.get(),
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
