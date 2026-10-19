//! Page-owned canvas host shell (wasm only).
//!
//! Owns the DOM side of one foreign editor: the page canvas looked up by
//! id, a hidden sibling input for keyboard/composition events (mirroring
//! `gpui_web`'s styling), the `ResizeObserver`, the `requestAnimationFrame`
//! pump, and the pointer/wheel/keyboard listeners. All listeners normalize
//! through [`crate::input`] and forward to the [`ForeignRenderer`]; the
//! shared [`FrameThrottle`], [`ResizeDebounce`], and [`TransitionCell`]
//! policies gate frames, surface sizes, and focus/hover exactly like the
//! native bindings.
//!
//! Renderer providers implement [`ForeignRenderer`] and attach with
//! [`create_editor_with_renderer`] or [`set_renderer`]. Attaching
//! `gpui_web`'s renderer needs the fork change specified in contract §16;
//! until then editors run headless (observers stay live, frames count as
//! dropped, stats stay queryable).
//!
//! The browser is the frame pump here rather than the page: this module
//! registers `requestAnimationFrame` itself, like `gpui_web` does. The
//! page only provides the canvas, sizes it with CSS, and calls the
//! lifecycle exports. Renderer callbacks may close their editor or replace the
//! provider; active callbacks keep editor resources alive until they return.

// Rust guideline compliant 2026-02-21

use std::cell::{Cell, RefCell};

use gpui::{
    Capslock, KeyDownEvent, KeyUpEvent, Keystroke, ModifiersChangedEvent, MouseButton,
    MouseDownEvent, MouseExitEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, ScrollWheelEvent,
    TouchPhase, point, px,
};
use gpui_foreign_parent::{
    EditorHandle, FrameThrottle, OwnedEditorRegistry, ResizeDebounce, TransitionCell,
};

use crate::dispatch::SerializedProvider;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::console;

use super::input::{
    ClickState, CompositionOp, ModifierFlags, compute_key_char, dom_button_to_mouse,
    dom_key_to_gpui_key, is_modifier_only_key, modifiers_from_flags, scroll_delta,
    validate_canvas_id,
};

/// One installed DOM listener: its target, event name, and keepalive.
type ListenerEntry = (web_sys::EventTarget, &'static str, Closure<dyn Fn(JsValue)>);

/// Renderer provider seam: whoever paints the canvas implements this.
///
/// Frames, debounced physical sizes, normalized inputs, composition ops,
/// and transition-only focus/hover arrive here. The `gpui_web` provider
/// needs contract §16 first.
pub trait ForeignRenderer {
    /// Paint one frame.
    fn request_frame(&mut self);
    /// Reconfigure the surface to a debounced physical size.
    fn resize(&mut self, physical: (u32, u32));
    /// Consume one normalized input event.
    fn input(&mut self, input: PlatformInput);
    /// Consume one IME composition operation.
    fn composition(&mut self, op: CompositionOp);
    /// Focus transition (canvas/input focus and blur).
    fn set_active(&mut self, active: bool);
    /// Pointer-in-canvas transition.
    fn set_hovered(&mut self, hovered: bool);
}

type ResizeCallback = Closure<dyn Fn(js_sys::Array, web_sys::ResizeObserver)>;

/// One live foreign editor: DOM handles plus shared policy state.
struct WebForeignEditor {
    label: String,
    editor_id: Cell<u32>,
    canvas: web_sys::HtmlCanvasElement,
    input_element: web_sys::HtmlInputElement,
    throttle: FrameThrottle,
    debounce: ResizeDebounce<(u32, u32)>,
    active: TransitionCell,
    hovered: TransitionCell,
    dropped_frames: Cell<usize>,
    clicks: RefCell<ClickState>,
    pressed_button: Cell<Option<MouseButton>>,
    active_pointer: Cell<Option<i32>>,
    renderer: SerializedProvider<Box<dyn ForeignRenderer>, RendererCall>,
    listeners: RefCell<Vec<ListenerEntry>>,
    resize_observer: RefCell<Option<web_sys::ResizeObserver>>,
    resize_closure: RefCell<Option<ResizeCallback>>,
    raf_closure: RefCell<Option<Closure<dyn Fn()>>>,
    raf_id: Cell<Option<i32>>,
}

enum RendererCall {
    Frame,
    Resize((u32, u32)),
    Input(PlatformInput),
    Composition(CompositionOp),
    Active(bool),
    Hovered(bool),
}

impl RendererCall {
    fn dynamic_bytes(&self) -> usize {
        match self {
            Self::Composition(CompositionOp::SetMarked(text) | CompositionOp::Commit(text)) => {
                text.capacity()
            }
            Self::Input(PlatformInput::KeyDown(event)) => {
                event.keystroke.key.capacity().saturating_add(
                    event
                        .keystroke
                        .key_char
                        .as_ref()
                        .map_or(0, String::capacity),
                )
            }
            Self::Input(PlatformInput::KeyUp(event)) => {
                event.keystroke.key.capacity().saturating_add(
                    event
                        .keystroke
                        .key_char
                        .as_ref()
                        .map_or(0, String::capacity),
                )
            }
            _ => 0,
        }
    }
}

impl WebForeignEditor {
    /// Serializes renderer work; bounded nested work drains after the current call.
    fn with_renderer(&self, command: RendererCall) -> bool {
        let bytes = command.dynamic_bytes();
        match self
            .renderer
            .dispatch(command, bytes, |renderer, command| match command {
                RendererCall::Frame => renderer.request_frame(),
                RendererCall::Resize(size) => renderer.resize(size),
                RendererCall::Input(input) => renderer.input(input),
                RendererCall::Composition(op) => renderer.composition(op),
                RendererCall::Active(active) => renderer.set_active(active),
                RendererCall::Hovered(hovered) => renderer.set_hovered(hovered),
            }) {
            Ok(delivered) => delivered,
            Err(error) => {
                destroy_editor(self.editor_id.get());
                console_error(&format!(
                    "gpui-web-foreign: renderer dispatch limit ({error:?}); editor closed"
                ));
                false
            }
        }
    }
}

impl Drop for WebForeignEditor {
    fn drop(&mut self) {
        self.renderer.close();
        for (target, name, closure) in self.listeners.get_mut().iter() {
            let _ =
                target.remove_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
        }
        if let Some(observer) = self.resize_observer.get_mut().as_ref() {
            observer.disconnect();
        }
        if let Some(raf_id) = self.raf_id.take()
            && let Some(window) = web_sys::window()
        {
            let _ = window.cancel_animation_frame(raf_id);
        }
        if let Some(pointer) = self.active_pointer.take() {
            let _ = self.canvas.release_pointer_capture(pointer);
        }
        self.input_element.remove();
        console::log_1(&JsValue::from_str(&format!(
            "gpui-web-foreign: editor {} ('{}') destroyed",
            self.editor_id.get(),
            self.label
        )));
    }
}

thread_local! {
    static EDITORS: OwnedEditorRegistry<WebForeignEditor> = OwnedEditorRegistry::new();
}

/// A lease keeps DOM resources alive through reentrant close.
fn with_editor<R>(editor_id: u32, f: impl FnOnce(&WebForeignEditor) -> R) -> Option<R> {
    let handle = EditorHandle::from_usize(editor_id as usize)?;
    EDITORS.with(|editors| editors.with(handle, f))
}

fn console_error(message: &str) {
    console::error_1(&JsValue::from_str(message));
}

/// Create an editor in the page canvas `canvas_id`, headless.
///
/// Returns the editor id, or 0 when the canvas cannot be resolved. Use
/// [`create_editor_with_renderer`] when a provider is ready, or attach
/// one later with [`set_renderer`].
#[wasm_bindgen]
pub fn create_editor(canvas_id: &str, label: &str) -> u32 {
    create_editor_inner(canvas_id, label, None)
}

/// Create an editor with a renderer provider attached.
pub fn create_editor_with_renderer(
    canvas_id: &str,
    label: &str,
    renderer: impl ForeignRenderer + 'static,
) -> u32 {
    create_editor_inner(
        canvas_id,
        label,
        Some(Box::new(renderer) as Box<dyn ForeignRenderer>),
    )
}

/// Attach (or replace) the renderer provider of a live editor.
pub fn set_renderer(editor_id: u32, renderer: impl ForeignRenderer + 'static) {
    with_editor(editor_id, |editor| {
        editor
            .renderer
            .replace(Box::new(renderer) as Box<dyn ForeignRenderer>);
    });
}

fn create_editor_inner(
    canvas_id: &str,
    label: &str,
    renderer: Option<Box<dyn ForeignRenderer>>,
) -> u32 {
    let canvas_id = match validate_canvas_id(canvas_id) {
        Ok(id) => id,
        Err(error) => {
            console_error(&format!("gpui-web-foreign: invalid canvas id ({error})"));
            return 0;
        }
    };
    let Some(window) = web_sys::window() else {
        console_error("gpui-web-foreign: no window");
        return 0;
    };
    let Some(document) = window.document() else {
        console_error("gpui-web-foreign: no document");
        return 0;
    };
    let Some(element) = document.get_element_by_id(canvas_id) else {
        console_error(&format!(
            "gpui-web-foreign: no element with id '#{canvas_id}'"
        ));
        return 0;
    };
    let Ok(canvas) = element.dyn_into::<web_sys::HtmlCanvasElement>() else {
        console_error(&format!(
            "gpui-web-foreign: '#{canvas_id}' is not a canvas element"
        ));
        return 0;
    };
    if !canvas.is_connected() {
        console::warn_1(&JsValue::from_str(&format!(
            "gpui-web-foreign: canvas '#{canvas_id}' is not connected to the document"
        )));
    }
    canvas.set_tab_index(-1);

    // Hidden sibling input for keyboard focus and composition events,
    // styled exactly like `gpui_web`'s.
    let Some(input_element) = document
        .create_element("input")
        .ok()
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        console_error("gpui-web-foreign: could not create the hidden input element");
        return 0;
    };
    // Styling is best-effort: a rejected property never fails creation.
    let input_style = input_element.style();
    let _ = input_style.set_property("position", "fixed");
    let _ = input_style.set_property("top", "0");
    let _ = input_style.set_property("left", "0");
    let _ = input_style.set_property("width", "1px");
    let _ = input_style.set_property("height", "1px");
    let _ = input_style.set_property("opacity", "0");
    let appended = document
        .body()
        .and_then(|body| body.append_child(&input_element).ok())
        .or_else(|| {
            canvas
                .parent_node()
                .and_then(|parent| parent.append_child(&input_element).ok())
        });
    if appended.is_none() {
        console_error("gpui-web-foreign: could not attach the hidden input element");
        return 0;
    }

    let state = WebForeignEditor {
        label: label.to_string(),
        editor_id: Cell::new(0),
        canvas: canvas.clone(),
        input_element: input_element.clone(),
        throttle: FrameThrottle::default(),
        debounce: ResizeDebounce::default(),
        active: TransitionCell::new(true),
        hovered: TransitionCell::new(false),
        dropped_frames: Cell::new(0),
        clicks: RefCell::new(ClickState::default()),
        pressed_button: Cell::new(None),
        active_pointer: Cell::new(None),
        renderer: SerializedProvider::new(renderer),
        listeners: RefCell::new(Vec::new()),
        resize_observer: RefCell::new(None),
        resize_closure: RefCell::new(None),
        raf_closure: RefCell::new(None),
        raf_id: Cell::new(None),
    };
    let Ok(handle) = EDITORS.with(|editors| editors.insert(state)) else {
        console_error("gpui-web-foreign: editor token space exhausted");
        return 0;
    };
    let Ok(editor_id) = u32::try_from(handle.as_usize()) else {
        EDITORS.with(|editors| editors.close(handle));
        console_error("gpui-web-foreign: editor token exceeds browser ABI");
        return 0;
    };
    with_editor(editor_id, |editor| editor.editor_id.set(editor_id));

    // Forward transition-only focus/hover to the renderer.
    with_editor(editor_id, |editor| {
        editor.active.on_change(move |active| {
            with_editor(editor_id, |editor| {
                editor.with_renderer(RendererCall::Active(active));
            });
        });
        editor.hovered.on_change(move |hovered| {
            with_editor(editor_id, |editor| {
                editor.with_renderer(RendererCall::Hovered(hovered));
            });
        });
    });

    install_canvas_listeners(editor_id, &canvas);
    install_input_listeners(editor_id, &input_element);
    install_resize_observer(editor_id, &canvas, &window);
    console::log_1(&JsValue::from_str(&format!(
        "gpui-web-foreign: editor {editor_id} ('{label}') attached to '#{canvas_id}'"
    )));
    schedule_frame(editor_id);
    editor_id
}

/// Closes an editor immediately and defers DOM teardown through active callbacks.
///
/// Queued renderer work is canceled. Unknown or stale IDs warn and do nothing.
#[wasm_bindgen]
pub fn destroy_editor(editor_id: u32) {
    let closed = EditorHandle::from_usize(editor_id as usize).is_some_and(|handle| {
        EDITORS.with(|editors| {
            editors
                .with(handle, |editor| {
                    let closed = editors.close(handle);
                    editor.renderer.close();
                    closed
                })
                .unwrap_or(false)
        })
    });
    if !closed {
        console::warn_1(&JsValue::from_str(&format!(
            "gpui-web-foreign: destroy of unknown editor {editor_id}"
        )));
    }
}

/// Frames dropped while headless (no renderer provider attached).
#[wasm_bindgen]
pub fn dropped_frames(editor_id: u32) -> usize {
    with_editor(editor_id, |editor| editor.dropped_frames.get()).unwrap_or(0)
}

/// `requestAnimationFrame` ticks coalesced by the frame throttle.
#[wasm_bindgen]
pub fn coalesced_frames(editor_id: u32) -> usize {
    with_editor(editor_id, |editor| editor.throttle.coalesced()).unwrap_or(0)
}

/// Attach one canvas listener, keeping the closure alive on the editor.
fn listen(
    editor_id: u32,
    target: &web_sys::EventTarget,
    name: &'static str,
    handler: impl Fn(JsValue) + 'static,
) {
    let closure = Closure::<dyn Fn(JsValue)>::new(handler);
    let _ = target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
    with_editor(editor_id, |editor| {
        editor
            .listeners
            .borrow_mut()
            .push((target.clone(), name, closure));
    });
}

/// Attach one non-passive listener (`{passive: false}`), mirroring
/// `gpui_web`, so `preventDefault()` works for wheel events.
fn listen_non_passive(
    editor_id: u32,
    canvas: &web_sys::HtmlCanvasElement,
    name: &'static str,
    handler: impl Fn(JsValue) + 'static,
) {
    let closure = Closure::<dyn Fn(JsValue)>::new(handler);
    let canvas_js: &JsValue = canvas.as_ref();
    let callback_js: &JsValue = closure.as_ref();
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&options, &"passive".into(), &false.into());
    if let Ok(add_fn_val) = js_sys::Reflect::get(canvas_js, &"addEventListener".into())
        && let Ok(add_fn) = add_fn_val.dyn_into::<js_sys::Function>()
    {
        let _ = add_fn.call3(canvas_js, &name.into(), callback_js, &options);
    }
    with_editor(editor_id, |editor| {
        editor
            .listeners
            .borrow_mut()
            .push((canvas.clone().into(), name, closure));
    });
}

fn pointer_modifiers(event: &web_sys::PointerEvent) -> gpui::Modifiers {
    let mouse: &web_sys::MouseEvent = event.as_ref();
    modifiers_from_flags(ModifierFlags {
        control: mouse.ctrl_key(),
        alt: mouse.alt_key(),
        shift: mouse.shift_key(),
        platform: mouse.meta_key(),
    })
}

fn mouse_position(event: &web_sys::MouseEvent) -> gpui::Point<gpui::Pixels> {
    point(px(event.offset_x() as f32), px(event.offset_y() as f32))
}

/// Forward a debounced physical size; false (no renderer) stashes it.
fn forward_resize(editor_id: u32, size: (u32, u32)) -> bool {
    with_editor(editor_id, |editor| {
        editor.with_renderer(RendererCall::Resize(size))
    })
    .unwrap_or(false)
}

fn push_active(editor_id: u32, active: bool) {
    with_editor(editor_id, |editor| {
        editor.active.update(active);
    });
}

fn push_hovered(editor_id: u32, hovered: bool) {
    with_editor(editor_id, |editor| {
        editor.hovered.update(hovered);
    });
}

fn install_canvas_listeners(editor_id: u32, canvas: &web_sys::HtmlCanvasElement) {
    let target: web_sys::EventTarget = canvas.clone().into();

    listen(editor_id, &target, "pointerdown", move |event: JsValue| {
        let event: web_sys::PointerEvent = event.unchecked_into();
        event.prevent_default();
        with_editor(editor_id, |editor| {
            if editor.active_pointer.get().is_some() {
                return;
            }
            editor.active_pointer.set(Some(event.pointer_id()));
            let _ = editor.canvas.set_pointer_capture(event.pointer_id());
            let _ = editor.input_element.focus();
            let button = dom_button_to_mouse(event.button());
            let mouse: &web_sys::MouseEvent = event.as_ref();
            let position = mouse_position(mouse);
            let modifiers = pointer_modifiers(&event);
            let time = js_sys::Date::now();
            editor.pressed_button.set(Some(button));
            let click_count = editor.clicks.borrow_mut().register_click(position, time);
            editor.with_renderer(RendererCall::Input(PlatformInput::MouseDown(
                MouseDownEvent {
                    position,
                    button,
                    modifiers,
                    click_count,
                    first_mouse: false,
                },
            )));
        });
    });

    listen(editor_id, &target, "pointerup", move |event: JsValue| {
        let event: web_sys::PointerEvent = event.unchecked_into();
        event.prevent_default();
        with_editor(editor_id, |editor| {
            if editor.active_pointer.get() != Some(event.pointer_id()) {
                return;
            }
            let _ = editor.canvas.release_pointer_capture(event.pointer_id());
            editor.active_pointer.set(None);
            let button = dom_button_to_mouse(event.button());
            let mouse: &web_sys::MouseEvent = event.as_ref();
            let position = mouse_position(mouse);
            let modifiers = pointer_modifiers(&event);
            let click_count = editor.clicks.borrow().current_count();
            editor.pressed_button.set(None);
            editor.with_renderer(RendererCall::Input(PlatformInput::MouseUp(MouseUpEvent {
                position,
                button,
                modifiers,
                click_count,
            })));
        });
    });

    for name in ["pointercancel", "lostpointercapture"] {
        listen(editor_id, &target, name, move |event: JsValue| {
            let event: web_sys::PointerEvent = event.unchecked_into();
            with_editor(editor_id, |editor| {
                if editor.active_pointer.get() == Some(event.pointer_id()) {
                    editor.active_pointer.set(None);
                    editor.pressed_button.set(None);
                }
            });
        });
    }

    listen(editor_id, &target, "pointermove", move |event: JsValue| {
        let event: web_sys::PointerEvent = event.unchecked_into();
        with_editor(editor_id, |editor| {
            let mouse: &web_sys::MouseEvent = event.as_ref();
            let position = mouse_position(mouse);
            let modifiers = pointer_modifiers(&event);
            editor.with_renderer(RendererCall::Input(PlatformInput::MouseMove(
                MouseMoveEvent {
                    position,
                    pressed_button: editor.pressed_button.get(),
                    modifiers,
                },
            )));
        });
    });

    listen(
        editor_id,
        &target,
        "pointerenter",
        move |_event: JsValue| {
            push_hovered(editor_id, true);
        },
    );

    listen(editor_id, &target, "pointerleave", move |event: JsValue| {
        let event: web_sys::PointerEvent = event.unchecked_into();
        with_editor(editor_id, |editor| {
            let mouse: &web_sys::MouseEvent = event.as_ref();
            let position = mouse_position(mouse);
            let modifiers = pointer_modifiers(&event);
            editor.with_renderer(RendererCall::Input(PlatformInput::MouseExited(
                MouseExitEvent {
                    position,
                    pressed_button: editor.pressed_button.get(),
                    modifiers,
                },
            )));
        });
        push_hovered(editor_id, false);
    });

    listen(editor_id, &target, "contextmenu", move |event: JsValue| {
        let event: web_sys::Event = event.unchecked_into();
        event.prevent_default();
    });

    listen_non_passive(editor_id, canvas, "wheel", move |event: JsValue| {
        let event: web_sys::WheelEvent = event.unchecked_into();
        event.prevent_default();
        with_editor(editor_id, |editor| {
            let mouse: &web_sys::MouseEvent = event.as_ref();
            let position = mouse_position(mouse);
            let modifiers = modifiers_from_flags(ModifierFlags {
                control: mouse.ctrl_key(),
                alt: mouse.alt_key(),
                shift: mouse.shift_key(),
                platform: mouse.meta_key(),
            });
            let delta = scroll_delta(event.delta_x(), event.delta_y(), event.delta_mode());
            editor.with_renderer(RendererCall::Input(PlatformInput::ScrollWheel(
                ScrollWheelEvent {
                    position,
                    delta,
                    modifiers,
                    touch_phase: TouchPhase::Moved,
                },
            )));
        });
    });
}

fn install_input_listeners(editor_id: u32, input: &web_sys::HtmlInputElement) {
    let target: web_sys::EventTarget = input.clone().into();

    for name in ["keydown", "keyup"] {
        listen(editor_id, &target, name, move |event: JsValue| {
            let event: web_sys::KeyboardEvent = event.unchecked_into();
            with_editor(editor_id, |editor| {
                let modifiers = modifiers_from_flags(ModifierFlags {
                    control: event.ctrl_key(),
                    alt: event.alt_key(),
                    shift: event.shift_key(),
                    platform: event.meta_key(),
                });
                let capslock = Capslock {
                    on: event.get_modifier_state("CapsLock"),
                };
                editor.with_renderer(RendererCall::Input(PlatformInput::ModifiersChanged(
                    ModifiersChangedEvent {
                        modifiers,
                        capslock,
                    },
                )));
                let raw_key = event.key();
                let key = dom_key_to_gpui_key(&raw_key);
                if is_modifier_only_key(&key) {
                    return;
                }
                // Browser paste carries the live system clipboard, which the
                // synchronous GPUI clipboard interface cannot await.
                if key == "v" && (modifiers.control || modifiers.platform) {
                    return;
                }
                event.prevent_default();
                let keystroke = Keystroke {
                    modifiers,
                    key: key.clone(),
                    key_char: compute_key_char(&raw_key, &key, &modifiers),
                };
                let input = if name == "keydown" {
                    PlatformInput::KeyDown(KeyDownEvent {
                        keystroke,
                        is_held: event.repeat(),
                        prefer_character_input: false,
                    })
                } else {
                    PlatformInput::KeyUp(KeyUpEvent { keystroke })
                };
                editor.with_renderer(RendererCall::Input(input));
            });
        });
    }

    listen(
        editor_id,
        &target,
        "compositionupdate",
        move |event: JsValue| {
            let event: web_sys::CompositionEvent = event.unchecked_into();
            with_editor(editor_id, |editor| {
                editor.with_renderer(RendererCall::Composition(CompositionOp::SetMarked(
                    event.data().unwrap_or_default(),
                )));
            });
        },
    );

    listen(
        editor_id,
        &target,
        "compositionend",
        move |event: JsValue| {
            let event: web_sys::CompositionEvent = event.unchecked_into();
            with_editor(editor_id, |editor| {
                editor.with_renderer(RendererCall::Composition(CompositionOp::Commit(
                    event.data().unwrap_or_default(),
                )));
            });
        },
    );

    listen(editor_id, &target, "focus", move |_event: JsValue| {
        push_active(editor_id, true);
    });

    listen(editor_id, &target, "blur", move |_event: JsValue| {
        push_active(editor_id, false);
    });
}

/// Physical canvas size from one observer entry: the device-pixel box
/// when available, else the CSS rect scaled by the DPR fallback.
fn physical_size_from_entry(
    entry: &web_sys::ResizeObserverEntry,
    device_pixel_ratio: f64,
) -> Option<(u32, u32)> {
    let boxes = entry.device_pixel_content_box_size();
    if boxes.length() > 0
        && let Ok(box_size) = boxes.get(0).dyn_into::<web_sys::ResizeObserverSize>()
    {
        let width = box_size.inline_size().round().max(1.0) as u32;
        let height = box_size.block_size().round().max(1.0) as u32;
        return Some((width, height));
    }
    if device_pixel_ratio.is_finite() && device_pixel_ratio > 0.0 {
        let rect = entry.content_rect();
        let width = (rect.width() * device_pixel_ratio).round().max(1.0) as u32;
        let height = (rect.height() * device_pixel_ratio).round().max(1.0) as u32;
        return Some((width, height));
    }
    None
}

fn install_resize_observer(
    editor_id: u32,
    canvas: &web_sys::HtmlCanvasElement,
    window: &web_sys::Window,
) {
    let window = window.clone();
    let closure = Closure::<dyn Fn(js_sys::Array, web_sys::ResizeObserver)>::new(
        move |entries: js_sys::Array, _observer: web_sys::ResizeObserver| {
            entries.for_each(&mut |entry, _, _| {
                let Ok(entry) = entry.dyn_into::<web_sys::ResizeObserverEntry>() else {
                    return;
                };
                if let Some(size) = physical_size_from_entry(&entry, window.device_pixel_ratio()) {
                    with_editor(editor_id, |editor| {
                        editor
                            .debounce
                            .submit(size, |size| forward_resize(editor_id, size));
                    });
                }
            });
        },
    );
    let Ok(observer) = web_sys::ResizeObserver::new(closure.as_ref().unchecked_ref()) else {
        console_error("gpui-web-foreign: ResizeObserver unavailable");
        return;
    };
    observer.observe(canvas);
    with_editor(editor_id, |editor| {
        editor.resize_closure.replace(Some(closure));
        editor.resize_observer.replace(Some(observer));
    });
}

fn schedule_frame(editor_id: u32) {
    with_editor(editor_id, |editor| {
        if editor.raf_id.get().is_some() {
            return;
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        if editor.raf_closure.borrow().is_none() {
            let closure = Closure::<dyn Fn()>::new(move || {
                tick_frame(editor_id);
                schedule_frame(editor_id);
            });
            editor.raf_closure.replace(Some(closure));
        }
        let callback = editor.raf_closure.borrow();
        let Some(closure) = callback.as_ref() else {
            return;
        };
        let Ok(raf_id) = window.request_animation_frame(closure.as_ref().unchecked_ref()) else {
            console_error("gpui-web-foreign: requestAnimationFrame failed");
            return;
        };
        editor.raf_id.set(Some(raf_id));
    });
}

fn tick_frame(editor_id: u32) {
    with_editor(editor_id, |editor| {
        editor.raf_id.take();
        editor.debounce.poll(|size| forward_resize(editor_id, size));
        if !editor.throttle.should_forward() {
            return;
        }
        if !editor.with_renderer(RendererCall::Frame) {
            editor.dropped_frames.set(editor.dropped_frames.get() + 1);
        }
    });
}
