//! Debug instrumentation hooks for Instruments and Metal capture.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
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
    pub name: Arc<str>,
    pub unix_micros: u128,
}

const SIGNPOST_CAPACITY: usize = 4_096;

static SIGNPOSTS: OnceLock<Mutex<VecDeque<IosSignpostEvent>>> = OnceLock::new();
static METAL_CAPTURE_ACTIVE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn signposts() -> &'static Mutex<VecDeque<IosSignpostEvent>> {
    SIGNPOSTS.get_or_init(|| Mutex::new(VecDeque::with_capacity(SIGNPOST_CAPACITY)))
}

fn capture_label() -> &'static Mutex<Option<String>> {
    METAL_CAPTURE_ACTIVE.get_or_init(|| Mutex::new(None))
}

pub fn emit_signpost(category: IosSignpostCategory, name: impl Into<Arc<str>>) {
    let name = name.into();
    if log::log_enabled!(log::Level::Info) {
        log::info!("GPUI iOS signpost {:?}: {}", category, name);
    }
    let mut signposts = signposts().lock().unwrap();
    if signposts.len() == SIGNPOST_CAPACITY {
        signposts.pop_front();
    }
    signposts.push_back(IosSignpostEvent {
        category,
        name,
        unix_micros: now_unix_micros(),
    });
}

pub fn signpost_snapshot() -> Vec<IosSignpostEvent> {
    signposts().lock().unwrap().iter().cloned().collect()
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
    use std::sync::Mutex;

    static SIGNPOST_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn signposts_and_capture_are_recorded() {
        let _guard = SIGNPOST_TEST_LOCK.lock().unwrap();
        clear_signposts();
        emit_signpost(IosSignpostCategory::Frame, "unit-test-request");
        assert!(signpost_snapshot().iter().any(|event| {
            event.category == IosSignpostCategory::Frame
                && event.name.as_ref() == "unit-test-request"
        }));

        assert!(begin_metal_capture("unit-test"));
        assert!(!begin_metal_capture("second"));
        assert!(is_metal_capture_active());
        end_metal_capture();
        assert!(!is_metal_capture_active());
    }

    #[test]
    fn signpost_history_is_bounded() {
        let _guard = SIGNPOST_TEST_LOCK.lock().unwrap();
        clear_signposts();
        for index in 0..=SIGNPOST_CAPACITY {
            emit_signpost(IosSignpostCategory::Frame, format!("frame-{index}"));
        }
        let snapshot = signpost_snapshot();
        assert_eq!(snapshot.len(), SIGNPOST_CAPACITY);
        assert_eq!(snapshot.first().unwrap().name.as_ref(), "frame-1");
    }

    #[test]
    fn signpost_event_uses_arc_name() {
        let _guard = SIGNPOST_TEST_LOCK.lock().unwrap();
        clear_signposts();
        let name: Arc<str> = Arc::from("unit-test-gpu-frame");
        emit_signpost(IosSignpostCategory::Draw, Arc::clone(&name));
        let snapshot = signpost_snapshot();
        assert!(snapshot.iter().any(|event| Arc::ptr_eq(&event.name, &name)));
    }
}
