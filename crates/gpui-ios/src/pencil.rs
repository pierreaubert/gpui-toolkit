//! Apple Pencil and pointer-hover side-channel events.
//!
//! GPUI keeps emitting compatibility mouse/touch events. These samples expose
//! extra iPad-only stylus data to professional drawing or spatial UIs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IosPointerDevice {
    #[default]
    Touch,
    Pencil,
    IndirectPointer,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IosPencilSample {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub altitude_angle: f32,
    pub azimuth_angle: f32,
    pub timestamp_seconds: f64,
    pub device: IosPointerDevice,
}

impl IosPencilSample {
    pub fn normalized(mut self) -> Self {
        self.pressure = finite_or(self.pressure, 0.0).clamp(0.0, 1.0);
        self.altitude_angle = finite_or(self.altitude_angle, 0.0).clamp(0.0, std::f32::consts::PI);
        self.azimuth_angle =
            finite_or(self.azimuth_angle, 0.0).rem_euclid(std::f32::consts::PI * 2.0);
        self.x = finite_or(self.x, 0.0);
        self.y = finite_or(self.y, 0.0);
        if !self.timestamp_seconds.is_finite() {
            self.timestamp_seconds = 0.0;
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IosHoverSample {
    pub x: f32,
    pub y: f32,
    pub altitude_angle: f32,
    pub azimuth_angle: f32,
    pub distance: f32,
    pub timestamp_seconds: f64,
    pub device: IosPointerDevice,
}

impl IosHoverSample {
    pub fn normalized(mut self) -> Self {
        self.x = finite_or(self.x, 0.0);
        self.y = finite_or(self.y, 0.0);
        self.altitude_angle = finite_or(self.altitude_angle, 0.0).clamp(0.0, std::f32::consts::PI);
        self.azimuth_angle =
            finite_or(self.azimuth_angle, 0.0).rem_euclid(std::f32::consts::PI * 2.0);
        self.distance = finite_or(self.distance, 0.0).max(0.0);
        if !self.timestamp_seconds.is_finite() {
            self.timestamp_seconds = 0.0;
        }
        self
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

type PencilCallback = Box<dyn FnMut(IosPencilSample) + Send>;
type HoverCallback = Box<dyn FnMut(IosHoverSample) + Send>;

static PENCIL_CALLBACK: OnceLock<Mutex<Option<PencilCallback>>> = OnceLock::new();
static PENCIL_CALLBACK_ACTIVE: AtomicBool = AtomicBool::new(false);
static HOVER_CALLBACK: OnceLock<Mutex<Option<HoverCallback>>> = OnceLock::new();

fn pencil_callback_slot() -> &'static Mutex<Option<PencilCallback>> {
    PENCIL_CALLBACK.get_or_init(|| Mutex::new(None))
}

fn hover_callback_slot() -> &'static Mutex<Option<HoverCallback>> {
    HOVER_CALLBACK.get_or_init(|| Mutex::new(None))
}

pub fn set_pencil_event_callback(callback: Option<PencilCallback>) {
    let active = callback.is_some();
    *pencil_callback_slot().lock().unwrap() = callback;
    PENCIL_CALLBACK_ACTIVE.store(active, Ordering::Release);
}

/// Whether collecting a high-detail pointer sample can reach an observer.
pub(crate) fn has_pencil_callback() -> bool {
    PENCIL_CALLBACK_ACTIVE.load(Ordering::Acquire)
}

pub fn set_hover_event_callback(callback: Option<HoverCallback>) {
    *hover_callback_slot().lock().unwrap() = callback;
}

pub fn dispatch_pencil_sample(sample: IosPencilSample) -> bool {
    if let Some(callback) = pencil_callback_slot().lock().unwrap().as_mut() {
        callback(sample.normalized());
        true
    } else {
        false
    }
}

pub fn dispatch_hover_sample(sample: IosHoverSample) -> bool {
    if let Some(callback) = hover_callback_slot().lock().unwrap().as_mut() {
        callback(sample.normalized());
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pencil_samples_are_normalized() {
        let sample = IosPencilSample {
            x: f32::NAN,
            y: 10.0,
            pressure: 3.0,
            altitude_angle: -1.0,
            azimuth_angle: -0.25,
            timestamp_seconds: f64::NAN,
            device: IosPointerDevice::Pencil,
        }
        .normalized();

        assert_eq!(sample.x, 0.0);
        assert_eq!(sample.pressure, 1.0);
        assert_eq!(sample.altitude_angle, 0.0);
        assert!(sample.azimuth_angle > 0.0);
        assert_eq!(sample.timestamp_seconds, 0.0);
    }
}
