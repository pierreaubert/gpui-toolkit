//! Small, host-owned presentation-state store.
//!
//! Python owns application data. This module deliberately persists only native
//! presentation choices that must survive independently of the child process.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

const STATE_FILE: &str = "host-presentation.json";
const STATE_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationState {
    version: u32,
    pub width: f32,
    pub height: f32,
    pub section: Option<String>,
    /// Positive logical distance from the top of the native content pane.
    #[serde(default)]
    pub scroll_y: f32,
}

impl Default for PresentationState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            width: 1240.0,
            height: 820.0,
            section: None,
            scroll_y: 0.0,
        }
    }
}

impl PresentationState {
    fn is_valid(&self) -> bool {
        (self.version == 1 || self.version == STATE_VERSION)
            && self.width.is_finite()
            && self.height.is_finite()
            && (400.0..=7680.0).contains(&self.width)
            && (300.0..=4320.0).contains(&self.height)
            && self.scroll_y.is_finite()
            && self.scroll_y >= 0.0
    }
}

/// Coalesces state writes on a dedicated thread so resize notifications never
/// perform filesystem I/O on the GPUI foreground executor.
#[derive(Clone)]
pub struct PresentationStore {
    state: Arc<Mutex<PresentationState>>,
    updates: mpsc::Sender<PresentationState>,
}

impl PresentationStore {
    pub fn open() -> Self {
        let path = state_path();
        let state = fs::read(&path)
            .ok()
            .and_then(|contents| serde_json::from_slice::<PresentationState>(&contents).ok())
            .filter(PresentationState::is_valid)
            .unwrap_or_default();
        let (updates, receiver) = mpsc::channel::<PresentationState>();
        std::thread::spawn(move || {
            while let Ok(mut state) = receiver.recv() {
                while let Ok(newer) = receiver.try_recv() {
                    state = newer;
                }
                let _ = atomic_write(&path, &state);
            }
        });
        Self {
            state: Arc::new(Mutex::new(state)),
            updates,
        }
    }

    pub fn snapshot(&self) -> PresentationState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    pub fn set_section(&self, section: Option<String>) {
        self.update(|state| state.section = section);
    }

    pub fn set_window_size(&self, width: f32, height: f32) {
        if width.is_finite()
            && height.is_finite()
            && (400.0..=7680.0).contains(&width)
            && (300.0..=4320.0).contains(&height)
        {
            self.update(|state| {
                state.width = width;
                state.height = height;
            });
        }
    }

    pub fn set_scroll_y(&self, scroll_y: f32) {
        if scroll_y.is_finite() && scroll_y >= 0.0 {
            self.update(|state| state.scroll_y = scroll_y);
        }
    }

    fn update(&self, change: impl FnOnce(&mut PresentationState)) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        change(&mut state);
        let _ = self.updates.send(state.clone());
    }
}

fn state_path() -> PathBuf {
    let app_id = env::var("GPUI_TOOLKIT_APP_ID").unwrap_or_else(|_| "gpui-python-runtime".into());
    let safe_app_id: String = app_id
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
        .take(128)
        .collect();
    let app_id = if safe_app_id.is_empty() {
        "gpui-python-runtime"
    } else {
        &safe_app_id
    };
    let root = env::var_os("GPUI_TOOLKIT_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| {
            env::var_os("HOME").map(|home| {
                let home = PathBuf::from(home);
                if cfg!(target_os = "macos") {
                    home.join("Library/Application Support")
                } else {
                    env::var_os("XDG_DATA_HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| home.join(".local/share"))
                }
            })
        })
        .unwrap_or_else(|| PathBuf::from("."));
    root.join(app_id).join(STATE_FILE)
}

fn atomic_write(path: &PathBuf, state: &PresentationState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec(state).expect("presentation state serializes"),
    )?;
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_invalid_dimensions_are_ignored() {
        let state = PresentationState::default();
        assert!(state.is_valid());
        let store = PresentationStore::open();
        let before = store.snapshot();
        store.set_window_size(f32::NAN, 800.0);
        assert_eq!(store.snapshot(), before);
    }

    #[test]
    fn older_presentation_state_migrates_with_zero_scroll() {
        let state: PresentationState =
            serde_json::from_str(r#"{"version":1,"width":1240.0,"height":820.0,"section":"form"}"#)
                .unwrap();
        assert!(state.is_valid());
        assert_eq!(state.scroll_y, 0.0);
    }
}
