//! Below-GPUI hosting demo: a solid-red `NSView` sits under a
//! transparent GPUI window; GPUI draws only a label and a border, the
//! red native content shows through everywhere else.
//!
//! Run with `cargo run -p gpui-macos-platform-view --example below_gpui`.
//! The example verifies itself once frames flow: it prints the native
//! view's synced frame and its subview index relative to the GPUI view
//! (a lower index means further back). It also prints the window
//! number so the window can be captured with
//! `screencapture -l <windowNumber> out.png`.

#[cfg(target_os = "macos")]
fn main() {
    use gpui::*;
    use gpui_macos_platform_view::*;
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::NSView;
    use objc2_core_graphics::CGColor;
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    struct BelowGpuiExample {
        native: Option<Retained<NSView>>,
        inserted: bool,
    }

    fn schedule_verify(window: &Window, native: ViewHandle, attempt: u32) {
        window.on_next_frame(move |window, _cx| {
            let applied = apply_snapshot(window);
            if applied == 0 && attempt < 240 {
                schedule_verify(window, native, attempt + 1);
                return;
            }
            // SAFETY: immediate main-thread reads of the live view
            // hierarchy; nothing is stored unretained.
            let ns_view = unsafe { &*native.as_ptr().cast::<NSView>() };
            let frame = ns_view.frame();
            let content = {
                // SAFETY: immediate main-thread reads of the live view
                // hierarchy; nothing is stored unretained.
                unsafe { ns_view.superview() }
            }
            .expect("native view inserted");
            let gpui_ptr = gpui_view(window).map(|view| view.as_ptr().cast_const());
            let native_ptr = native.as_ptr().cast::<std::ffi::c_void>().cast_const();
            let mut native_index = None;
            let mut gpui_index = None;
            for (index, subview) in content.subviews().iter().enumerate() {
                let ptr = Retained::as_ptr(&subview).cast::<std::ffi::c_void>();
                if ptr == native_ptr {
                    native_index = Some(index);
                }
                if Some(ptr) == gpui_ptr {
                    gpui_index = Some(index);
                }
            }
            println!(
                "below_gpui: applied={applied} native_frame=({}, {}, {}x{}) \
                 native_index={native_index:?} gpui_index={gpui_index:?} below={}",
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height,
                matches!((native_index, gpui_index), (Some(n), Some(g)) if n < g),
            );
        });
    }

    impl Render for BelowGpuiExample {
        fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            if !self.inserted {
                self.inserted = true;
                let view = NSView::initWithFrame(
                    NSView::alloc(MainThreadMarker::new().expect("example runs on main thread")),
                    NSRect {
                        origin: NSPoint { x: 0.0, y: 0.0 },
                        size: NSSize {
                            width: 10.0,
                            height: 10.0,
                        },
                    },
                );
                view.setWantsLayer(true);
                if let Some(layer) = view.layer() {
                    layer.setBackgroundColor(Some(&CGColor::new_srgb(1.0, 0.0, 0.0, 1.0)));
                }
                // SAFETY: `view` is a live NSView retained by `self`
                // for the app's lifetime; used on the main thread.
                let handle = unsafe {
                    ViewHandle::from_ptr(Retained::as_ptr(&view) as *mut std::ffi::c_void)
                }
                .expect("fresh NSView pointer is non-null");
                register_view("video", handle);
                let inserted = insert_below_gpui(window, handle);
                println!(
                    "below_gpui: inserted={inserted} ns_window={:?} window_number={:?}",
                    ns_window(window),
                    window_number(window),
                );
                self.native = Some(view);
                schedule_verify(window, handle, 0);
            }
            // The production follows-the-view loop applies the latest
            // placeholder frames once per host tick; here the verify
            // callback does it on frame boundaries instead.
            let native = self.native.as_ref().expect("inserted above");
            // SAFETY: same retained-view contract as above.
            let handle =
                unsafe { ViewHandle::from_ptr(Retained::as_ptr(native) as *mut std::ffi::c_void) }
                    .expect("retained NSView pointer is non-null");

            div().size_full().p_4().child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_color(rgb(0xffffff))
                            .child("GPUI chrome above a native NSView"),
                    )
                    .child(
                        div()
                            .h(px(240.0))
                            .border_2()
                            .border_color(rgb(0xffffff))
                            .child(MacPlatformView::new("video", handle)),
                    ),
            )
        }
    }

    let platform: std::rc::Rc<dyn Platform> = std::rc::Rc::new(gpui_macos::MacPlatform::new(false));
    Application::with_platform(platform).run(|cx| {
        let bounds = Bounds::centered(None, size(px(640.0), px(420.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Transparent,
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| BelowGpuiExample {
                    native: None,
                    inserted: false,
                })
            },
        )
        .expect("open window");
        cx.activate(true);
    });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("below_gpui is a macOS-only example");
}
