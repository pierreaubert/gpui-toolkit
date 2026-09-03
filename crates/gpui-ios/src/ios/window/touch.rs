//! Touch, gesture, and momentum input for [`IosWindow`](super::ios_window::IosWindow).
//!
//! Extracted from `ios_window.rs`: the tap-vs-scroll state machine
//! (`handle_touch_inner`), two-finger pinch synthesis, indirect (trackpad)
//! scroll, pencil side-channel sampling, tvOS Siri Remote presses, and
//! per-frame momentum pumping.

use super::super::events::*;
use super::consts::SCROLL_SLOP;
use super::ios_window::IosWindow;
#[cfg(target_os = "ios")]
use super::register::input_diag_log;
use super::types::{TouchState, TouchStateMap, pinch_geometry};
use gpui::{DispatchEventResult, Modifiers, Pixels, PlatformInput, Point};
use objc::{
    Message, msg_send,
    runtime::{Object, Sel},
    sel, sel_impl,
};
use std::cell::Cell;

pub(super) struct ReentrancyGuard<'a>(pub(super) &'a Cell<bool>);

impl Drop for ReentrancyGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl IosWindow {
    fn emit_pinch_for_active_touches(
        &self,
        states: &mut TouchStateMap,
        emit: &mut impl FnMut(PlatformInput) -> DispatchEventResult,
        modifiers: Modifiers,
    ) -> bool {
        let Some((first, second)) = states.two_active_points() else {
            return false;
        };
        let Some((x, y, distance)) = pinch_geometry(first, second) else {
            return false;
        };

        self.momentum_scroller.borrow_mut().cancel();
        self.velocity_tracker.borrow_mut().reset();
        states.clear_states();

        let mut pinch = self.pinch_state.borrow_mut();
        let (delta, phase) = if pinch.is_active() {
            (
                pinch.update(distance).unwrap_or(0.0),
                gpui::TouchPhase::Moved,
            )
        } else {
            pinch.start(distance);
            (0.0, gpui::TouchPhase::Started)
        };

        emit(PlatformInput::Pinch(gpui::PinchEvent {
            position: gpui::point(gpui::px(x), gpui::px(y)),
            delta,
            modifiers,
            phase,
        }));
        self.request_forced_frame();
        true
    }

    fn end_active_pinch(
        &self,
        position: Point<Pixels>,
        modifiers: Modifiers,
        emit: &mut impl FnMut(PlatformInput) -> DispatchEventResult,
    ) -> bool {
        let mut pinch = self.pinch_state.borrow_mut();
        if !pinch.is_active() {
            return false;
        }
        pinch.reset();
        self.velocity_tracker.borrow_mut().reset();
        self.momentum_scroller.borrow_mut().cancel();
        emit(PlatformInput::Pinch(gpui::PinchEvent {
            position,
            delta: 0.0,
            modifiers,
            phase: gpui::TouchPhase::Ended,
        }));
        self.request_forced_frame();
        true
    }

    /// Handle a touch event from UIKit.
    ///
    /// Uses a state machine to distinguish **taps** from **drag gestures**:
    ///
    ///   DOWN  → record start position, enter "pending" (NO MouseDown yet)
    ///   MOVE  → if finger moved > threshold → switch to "scrolling",
    ///           emit `ScrollWheel` deltas (for scrollable containers) AND
    ///           `MouseMove` (for interactive canvas screens like Animations)
    ///   UP    → if still "pending" → emit `MouseDown` + `MouseUp` (tap)
    ///           if "scrolling"   → emit final `ScrollWheel` (Ended) +
    ///           `MouseUp` (so drag-to-throw works)
    ///
    /// MouseDown is **deferred** until finger-up so that starting a scroll
    /// near a button or tab doesn't accidentally trigger navigation.
    /// Interactive screens use `MouseMove` to track the finger during drags
    /// and `MouseUp` to detect the end of a throw/drag gesture.
    pub fn handle_touch(&self, touch: *mut Object, event: *mut Object) {
        if self.touch_dispatching.replace(true) {
            unsafe {
                let _: *mut Object = msg_send![touch, retain];
            }
            self.pending_touches.borrow_mut().push_back(touch);
            return;
        }
        let _dispatch_guard = ReentrancyGuard(&self.touch_dispatching);

        self.handle_touch_inner(touch, event);
        while let Some(touch) = self.pending_touches.borrow_mut().pop_front() {
            self.handle_touch_inner(touch, std::ptr::null_mut());
            unsafe {
                let _: () = msg_send![touch, release];
            }
        }
    }

    fn handle_touch_inner(&self, touch: *mut Object, _event: *mut Object) {
        let position = touch_location_in_view(touch, self.view);
        let phase = touch_phase(touch);
        let tap_count = touch_tap_count(touch);
        let modifiers = self.modifiers.get();

        let logical_x: f32 = position.x.into();
        let logical_y: f32 = position.y.into();

        self.mouse_position.set(position);
        self.dispatch_pointer_sample(touch, logical_x, logical_y);

        let touch_id: usize = unsafe { msg_send![touch, hash] };
        let mut states = self.touch_states.borrow_mut();
        let mut ts = states.get(touch_id).unwrap_or(TouchState::Idle);

        let mut emit = |input: PlatformInput| self.dispatch_input(input);

        match phase {
            UITouchPhase::Began => {
                self.touch_pressed.set(true);
                // Cancel any active momentum fling — the user touched the
                // screen again, so inertia scrolling must stop immediately.
                self.momentum_scroller.borrow_mut().cancel();
                self.velocity_tracker.borrow_mut().reset();

                ts = TouchState::Pending {
                    start_x: logical_x,
                    start_y: logical_y,
                };
                states.insert(touch_id, ts, logical_x, logical_y);
                if self.emit_pinch_for_active_touches(&mut states, &mut emit, modifiers) {
                    return;
                }
                emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                    position,
                    modifiers,
                    pressed_button: None,
                }));
                self.request_forced_frame();
                // Do NOT emit MouseDown here — wait until we know whether
                // this is a tap or a scroll.  Emitting MouseDown immediately
                // causes accidental navigation when the user starts scrolling
                // near a button/tab.
                //
                // - Tap (finger lifts within slop) → emit MouseDown + MouseUp
                //   together in Ended phase.
                // - Scroll (finger exceeds slop) → emit only MouseMove +
                //   ScrollWheel, no MouseDown.
            }

            UITouchPhase::Moved => {
                states.insert(touch_id, ts, logical_x, logical_y);
                if self.emit_pinch_for_active_touches(&mut states, &mut emit, modifiers) {
                    return;
                }
                // Record every move for velocity estimation.
                self.velocity_tracker
                    .borrow_mut()
                    .record(logical_x, logical_y);

                match ts {
                    TouchState::Pending { start_x, start_y } => {
                        let dx = logical_x - start_x;
                        let dy = logical_y - start_y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance > SCROLL_SLOP {
                            let vertical_scroll = dy.abs() >= dx.abs();
                            emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                                position,
                                modifiers,
                                pressed_button: Some(gpui::MouseButton::Left),
                            }));
                            if vertical_scroll {
                                // GPUI stores scroll offsets as negative values
                                // once content moves upward, and its scroll
                                // handler adds deltas directly. A finger moving
                                // up therefore needs a negative y delta. Do not
                                // probe with MouseDown first; menu rows and
                                // buttons would treat the beginning of a scroll
                                // as an activation.
                                #[cfg(target_os = "ios")]
                                input_diag_log(|| {
                                    format!(
                                        "direct_touch scroll started dx={dx:.2} dy={dy:.2} pos=({logical_x:.2},{logical_y:.2})"
                                    )
                                });
                                emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                                    position,
                                    delta: gpui::ScrollDelta::Pixels(gpui::point(
                                        gpui::px(dx),
                                        gpui::px(dy),
                                    )),
                                    modifiers,
                                    touch_phase: gpui::TouchPhase::Started,
                                }));
                                self.request_forced_frame();
                                ts = TouchState::Scrolling {
                                    prev_x: logical_x,
                                    prev_y: logical_y,
                                };
                            } else {
                                // Horizontal gestures are more likely to be
                                // sliders, canvas tools, etc. Probe with
                                // MouseDown so those elements can claim the
                                // touch as a drag.
                                let start_pos = gpui::point(gpui::px(start_x), gpui::px(start_y));
                                let result = emit(PlatformInput::MouseDown(gpui::MouseDownEvent {
                                    button: gpui::MouseButton::Left,
                                    position: start_pos,
                                    modifiers,
                                    click_count: 1,
                                    first_mouse: false,
                                }));

                                if !result.propagate {
                                    ts = TouchState::Dragging;
                                } else {
                                    emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                                        button: gpui::MouseButton::Left,
                                        position: start_pos,
                                        modifiers,
                                        click_count: 1,
                                    }));
                                    ts = TouchState::Scrolling {
                                        prev_x: logical_x,
                                        prev_y: logical_y,
                                    };
                                }
                            }
                        }
                        if matches!(ts, TouchState::Pending { .. }) {
                            // Keep GPUI's mouse hit-test under the finger while
                            // the gesture is still inside the scroll slop.
                            emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                                position,
                                modifiers,
                                pressed_button: Some(gpui::MouseButton::Left),
                            }));
                        }
                    }
                    TouchState::Dragging => {
                        // Element is driving its own drag — only emit
                        // MouseMove (no ScrollWheel).
                        emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                            position,
                            modifiers,
                            pressed_button: Some(gpui::MouseButton::Left),
                        }));
                    }
                    TouchState::Scrolling { prev_x, prev_y } => {
                        let dx = logical_x - prev_x;
                        let dy = logical_y - prev_y;
                        ts = TouchState::Scrolling {
                            prev_x: logical_x,
                            prev_y: logical_y,
                        };
                        // Update GPUI's scroll target before dispatching the
                        // wheel event; scroll hit-testing follows the current
                        // mouse position.
                        emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                            position,
                            modifiers,
                            pressed_button: Some(gpui::MouseButton::Left),
                        }));
                        // Scroll event for scrollable containers.
                        #[cfg(target_os = "ios")]
                        input_diag_log(|| {
                            format!(
                                "direct_touch scroll moved dx={dx:.2} dy={dy:.2} pos=({logical_x:.2},{logical_y:.2})"
                            )
                        });
                        emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                            position,
                            delta: gpui::ScrollDelta::Pixels(gpui::point(
                                gpui::px(dx),
                                gpui::px(dy),
                            )),
                            modifiers,
                            touch_phase: gpui::TouchPhase::Moved,
                        }));
                        self.request_forced_frame();
                    }
                    TouchState::Idle => {
                        // Spurious move without a preceding down — ignore.
                    }
                }
            }

            UITouchPhase::Ended | UITouchPhase::Cancelled => {
                self.touch_pressed.set(false);
                if self.end_active_pinch(position, modifiers, &mut emit) {
                    states.remove(touch_id);
                    return;
                }
                match ts {
                    TouchState::Pending { start_x, start_y } => {
                        // Finger lifted without exceeding slop → tap.
                        // Emit MouseDown + MouseUp together at the original
                        // down position so hit-testing matches the initial
                        // touch point.
                        self.velocity_tracker.borrow_mut().reset();
                        let tap_pos = gpui::point(gpui::px(start_x), gpui::px(start_y));
                        emit(PlatformInput::MouseDown(gpui::MouseDownEvent {
                            button: gpui::MouseButton::Left,
                            position: tap_pos,
                            modifiers,
                            click_count: tap_count as usize,
                            first_mouse: false,
                        }));
                        emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                            button: gpui::MouseButton::Left,
                            position: tap_pos,
                            modifiers,
                            click_count: tap_count as usize,
                        }));
                    }
                    TouchState::Dragging => {
                        // Element was driving a drag — just emit MouseUp
                        // to let it finalize (no scroll, no momentum).
                        self.velocity_tracker.borrow_mut().reset();
                        emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                            button: gpui::MouseButton::Left,
                            position,
                            modifiers,
                            click_count: 1,
                        }));
                    }
                    TouchState::Scrolling { prev_x, prev_y } => {
                        // End the active touch-scroll gesture.
                        let dx = logical_x - prev_x;
                        let dy = logical_y - prev_y;
                        #[cfg(target_os = "ios")]
                        input_diag_log(|| {
                            format!(
                                "direct_touch scroll ended dx={dx:.2} dy={dy:.2} pos=({logical_x:.2},{logical_y:.2})"
                            )
                        });
                        emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                            position,
                            delta: gpui::ScrollDelta::Pixels(gpui::point(
                                gpui::px(dx),
                                gpui::px(dy),
                            )),
                            modifiers,
                            touch_phase: gpui::TouchPhase::Ended,
                        }));
                        self.request_forced_frame();
                        // Also emit MouseUp so interactive screens can
                        // detect the end of a drag (e.g. fling a ball).
                        emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                            button: gpui::MouseButton::Left,
                            position,
                            modifiers,
                            click_count: 1,
                        }));

                        // ── Start momentum / inertia scrolling ───────────
                        // Compute release velocity from recent touch samples
                        // and kick off the momentum scroller.  Subsequent
                        // frames will pump synthetic ScrollWheel events via
                        // `pump_momentum()` until velocity decays below the
                        // threshold.
                        let (vx, vy) = self.velocity_tracker.borrow().velocity();
                        self.velocity_tracker.borrow_mut().reset();
                        self.momentum_scroller
                            .borrow_mut()
                            .fling(vx, vy, logical_x, logical_y);
                    }
                    TouchState::Idle => {}
                }
                states.remove(touch_id);
                return;
            }

            UITouchPhase::Stationary => {
                // No change — ignore.
                return;
            }
        }

        states.insert(touch_id, ts, logical_x, logical_y);
    }

    #[cfg(target_os = "ios")]
    pub fn handle_indirect_scroll(&self, recognizer: *mut Object) {
        if recognizer.is_null() {
            return;
        }

        const GESTURE_BEGAN: i64 = 1;
        const GESTURE_CHANGED: i64 = 2;
        const GESTURE_ENDED: i64 = 3;
        const GESTURE_CANCELLED: i64 = 4;

        unsafe {
            let state: i64 = msg_send![recognizer, state];
            let touch_phase = match state {
                GESTURE_BEGAN => gpui::TouchPhase::Started,
                GESTURE_CHANGED => gpui::TouchPhase::Moved,
                GESTURE_ENDED | GESTURE_CANCELLED => gpui::TouchPhase::Ended,
                _ => return,
            };

            let translation: core_graphics::geometry::CGPoint =
                msg_send![recognizer, translationInView: self.view];
            let location: core_graphics::geometry::CGPoint =
                msg_send![recognizer, locationInView: self.view];
            let position = gpui::point(gpui::px(location.x as f32), gpui::px(location.y as f32));
            let delta = gpui::point(
                gpui::px(translation.x as f32),
                gpui::px(translation.y as f32),
            );
            input_diag_log(|| {
                format!(
                    "indirect_scroll translation=({:.2},{:.2}) location=({:.2},{:.2}) state={state}",
                    translation.x, translation.y, location.x, location.y
                )
            });

            if translation.x != 0.0 || translation.y != 0.0 || state != GESTURE_CHANGED {
                self.dispatch_input(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(delta),
                    modifiers: self.modifiers.get(),
                    touch_phase,
                }));
                self.request_forced_frame();
            }

            let zero = core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 };
            let _: () = msg_send![recognizer, setTranslation: zero inView: self.view];

            if matches!(state, GESTURE_ENDED | GESTURE_CANCELLED) {
                let velocity: core_graphics::geometry::CGPoint =
                    msg_send![recognizer, velocityInView: self.view];
                self.momentum_scroller.borrow_mut().fling(
                    velocity.x as f32,
                    velocity.y as f32,
                    location.x as f32,
                    location.y as f32,
                );
            }
        }
    }

    pub(super) fn dispatch_pointer_sample(
        &self,
        touch: *mut Object,
        logical_x: f32,
        logical_y: f32,
    ) {
        if touch.is_null() || !crate::pencil::has_pencil_callback() {
            return;
        }
        unsafe {
            // SAFETY: UIKit supplies a live UITouch pointer while processing the
            // touch callback on the main thread. Selectors used here are stable
            // UITouch APIs on the supported iOS deployment target.
            static TOUCH_TYPE_SELECTOR: std::sync::OnceLock<Sel> = std::sync::OnceLock::new();
            let touch_type: i64 = (&*touch)
                .send_message(
                    *TOUCH_TYPE_SELECTOR.get_or_init(|| Sel::register("type")),
                    (),
                )
                .unwrap();
            let force: core_graphics::base::CGFloat = msg_send![touch, force];
            let max_force: core_graphics::base::CGFloat = msg_send![touch, maximumPossibleForce];
            let altitude_angle: core_graphics::base::CGFloat = msg_send![touch, altitudeAngle];
            let azimuth_angle: core_graphics::base::CGFloat =
                msg_send![touch, azimuthAngleInView: self.view];
            let timestamp: f64 = msg_send![touch, timestamp];
            let device = match touch_type {
                0 => crate::pencil::IosPointerDevice::Touch,
                1 => crate::pencil::IosPointerDevice::IndirectPointer,
                2 => crate::pencil::IosPointerDevice::Pencil,
                3 => crate::pencil::IosPointerDevice::IndirectPointer,
                _ => crate::pencil::IosPointerDevice::Unknown,
            };
            let pressure = if max_force > 0.0 {
                (force / max_force) as f32
            } else {
                0.0
            };
            crate::pencil::dispatch_pencil_sample(crate::pencil::IosPencilSample {
                x: logical_x,
                y: logical_y,
                pressure,
                altitude_angle: altitude_angle as f32,
                azimuth_angle: azimuth_angle as f32,
                timestamp_seconds: timestamp,
                device,
            });
        }
    }

    // ── tvOS: Siri Remote button handling ─────────────────────────────────
    //
    // Maps hardware button presses to GPUI events:
    //   Select (4)    → MouseDown/MouseUp at last known position (click)
    //   Menu (5)      → Escape keystroke
    //   Play/Pause (6)→ Space keystroke
    //   Arrows (0-3)  → arrow keystrokes consumed by GPUI focus groups
    #[cfg(target_os = "tvos")]
    pub fn handle_press(&self, press_type: i64, is_down: bool) {
        let modifiers = self.modifiers.get();
        let position = self.mouse_position.get();

        let emit = |input: PlatformInput| {
            self.dispatch_input(input);
        };

        // UIPressType constants
        const SELECT: i64 = 4;

        match press_type {
            SELECT => {
                if is_down {
                    emit(PlatformInput::MouseDown(gpui::MouseDownEvent {
                        button: gpui::MouseButton::Left,
                        position,
                        modifiers,
                        click_count: 1,
                        first_mouse: false,
                    }));
                } else {
                    emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                        button: gpui::MouseButton::Left,
                        position,
                        modifiers,
                        click_count: 1,
                    }));
                }
            }
            _ if super::super::text_input::tvos_press_key(press_type).is_some() => {
                let key = super::super::text_input::tvos_press_key(press_type).unwrap();
                let keystroke = gpui::Keystroke::parse(key).unwrap();
                if is_down {
                    emit(PlatformInput::KeyDown(gpui::KeyDownEvent {
                        keystroke,
                        is_held: false,
                        prefer_character_input: false,
                    }));
                } else {
                    emit(PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }));
                }
                self.request_forced_frame();
            }
            _ => {}
        }
    }

    /// Advance the momentum scroller by one frame and emit a synthetic
    /// `ScrollWheel` event if the fling is still active.
    ///
    /// Called from `gpui_ios_request_frame` on every CADisplayLink tick,
    /// **before** the GPUI render callback runs, so that the scroll delta
    /// is picked up during the current frame's layout/paint cycle.
    pub(crate) fn pump_momentum(&self) {
        if self.momentum_pumping.replace(true) {
            return;
        }
        let _pump_guard = ReentrancyGuard(&self.momentum_pumping);

        let mut scroller = self.momentum_scroller.borrow_mut();
        if !scroller.is_active() {
            return;
        }

        if let Some(delta) = scroller.step() {
            let modifiers = self.modifiers.get();
            let position = gpui::point(gpui::px(delta.position_x), gpui::px(delta.position_y));
            let fling_ended = !scroller.is_active();
            let moved = PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position,
                delta: gpui::ScrollDelta::Pixels(gpui::point(
                    gpui::px(delta.dx),
                    gpui::px(delta.dy),
                )),
                modifiers,
                touch_phase: gpui::TouchPhase::Moved,
            });
            let ended = fling_ended.then(|| {
                PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(0.0))),
                    modifiers,
                    touch_phase: gpui::TouchPhase::Ended,
                })
            });
            drop(scroller);

            self.dispatch_input(moved);
            self.request_forced_frame();
            if let Some(ended) = ended {
                self.dispatch_input(ended);
                self.request_forced_frame();
            }
        } else if scroller.is_finished() {
            // Fling truly finished — emit one final Ended event so GPUI knows
            // the scroll gesture is complete.  We only do this when
            // `is_finished()` is true, which distinguishes a natural stop
            // from a sub-microsecond `dt` where `step()` returns `None`
            // but the scroller is still active.
            let position = gpui::point(
                gpui::px(scroller.position_x()),
                gpui::px(scroller.position_y()),
            );
            let modifiers = self.modifiers.get();
            let ended = PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position,
                delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(0.0))),
                modifiers,
                touch_phase: gpui::TouchPhase::Ended,
            });
            drop(scroller);
            self.dispatch_input(ended);
            self.request_forced_frame();
        }
    }
}
