//! iOS event handling - converting UIKit events to GPUI's event types.

use core_graphics::geometry::CGPoint;
use gpui::{Pixels, Point, TouchPhase, px};
use objc::{msg_send, runtime::Object, sel, sel_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum UITouchPhase {
    Began = 0,
    Moved = 1,
    Stationary = 2,
    Ended = 3,
    Cancelled = 4,
}

impl From<i64> for UITouchPhase {
    fn from(value: i64) -> Self {
        match value {
            0 => UITouchPhase::Began,
            1 => UITouchPhase::Moved,
            2 => UITouchPhase::Stationary,
            3 => UITouchPhase::Ended,
            4 => UITouchPhase::Cancelled,
            _ => UITouchPhase::Cancelled,
        }
    }
}

impl From<UITouchPhase> for TouchPhase {
    fn from(phase: UITouchPhase) -> Self {
        match phase {
            UITouchPhase::Began => TouchPhase::Started,
            UITouchPhase::Moved => TouchPhase::Moved,
            UITouchPhase::Stationary => TouchPhase::Moved,
            UITouchPhase::Ended => TouchPhase::Ended,
            UITouchPhase::Cancelled => TouchPhase::Ended,
        }
    }
}

pub fn touch_location_in_view(touch: *mut Object, view: *mut Object) -> Point<Pixels> {
    unsafe {
        let location: CGPoint = msg_send![touch, locationInView: view];
        Point::new(px(location.x as f32), px(location.y as f32))
    }
}

pub fn touch_phase(touch: *mut Object) -> UITouchPhase {
    unsafe {
        let phase: i64 = msg_send![touch, phase];
        UITouchPhase::from(phase)
    }
}

pub fn touch_tap_count(touch: *mut Object) -> u32 {
    unsafe {
        let count: i64 = msg_send![touch, tapCount];
        count as u32
    }
}
