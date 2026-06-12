//! Debug instrumentation hooks for Instruments and Metal capture.

use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IosSignpostCategory {
    Frame,
    Layout,
    Input,
    PlatformView,
    Accessibility,
    Wgpu,
    Draw,
    HotReload,
    Widget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IosSignpostEvent {
    pub category: IosSignpostCategory,
    pub name: String,
    pub unix_micros: u128,
}

static SIGNPOSTS: OnceLock<Mutex<Vec<IosSignpostEvent>>> = OnceLock::new();
static METAL_CAPTURE_ACTIVE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn signposts() -> &'static Mutex<Vec<IosSignpostEvent>> {
    SIGNPOSTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn capture_label() -> &'static Mutex<Option<String>> {
    METAL_CAPTURE_ACTIVE.get_or_init(|| Mutex::new(None))
}

pub fn emit_signpost(category: IosSignpostCategory, name: impl Into<String>) {
    let event = IosSignpostEvent {
        category,
        name: name.into(),
        unix_micros: now_unix_micros(),
    };
    log::info!("GPUI iOS signpost {:?}: {}", event.category, event.name);
    signposts().lock().unwrap().push(event);
}

pub fn signpost_snapshot() -> Vec<IosSignpostEvent> {
    signposts().lock().unwrap().clone()
}

pub fn clear_signposts() {
    signposts().lock().unwrap().clear();
}

pub fn begin_metal_capture(label: impl Into<String>) -> bool {
    let label = label.into();
    if label.trim().is_empty() {
        return false;
    }
    let mut slot = capture_label().lock().unwrap();
    if slot.is_some() {
        return false;
    }
    log::info!("GPUI iOS Metal capture begin: {label}");
    *slot = Some(label);
    true
}

pub fn end_metal_capture() {
    if let Some(label) = capture_label().lock().unwrap().take() {
        log::info!("GPUI iOS Metal capture end: {label}");
    }
}

pub fn is_metal_capture_active() -> bool {
    capture_label().lock().unwrap().is_some()
}

fn now_unix_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signposts_and_capture_are_recorded() {
        clear_signposts();
        emit_signpost(IosSignpostCategory::Frame, "request");
        assert_eq!(signpost_snapshot().len(), 1);

        assert!(begin_metal_capture("unit-test"));
        assert!(!begin_metal_capture("second"));
        assert!(is_metal_capture_active());
        end_metal_capture();
        assert!(!is_metal_capture_active());
    }
}
