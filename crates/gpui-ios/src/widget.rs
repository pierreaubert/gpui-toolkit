//! WidgetKit / Live Activity snapshot bridge.
//!
//! Widget extensions cannot host an interactive GPUI renderer. The containing
//! app renders deterministic image payloads and timeline metadata into an App
//! Group container that Swift WidgetKit/ActivityKit targets can display.

use gpui::{BackgroundExecutor, Task};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetSnapshotKind {
    Widget,
    LiveActivity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetTimelineEntry {
    pub date_unix_seconds: i64,
    pub title: String,
    pub subtitle: Option<String>,
    pub snapshot_file_name: String,
}

impl WidgetTimelineEntry {
    pub fn validate(&self) -> Result<(), String> {
        if self.snapshot_file_name.trim().is_empty() {
            return Err("widget timeline entry snapshot file name must not be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetSnapshotRequest {
    pub id: String,
    pub kind: WidgetSnapshotKind,
    pub width_px: u32,
    pub height_px: u32,
    pub scale: u32,
    pub app_group_dir: PathBuf,
    pub timeline: Vec<WidgetTimelineEntry>,
}

impl WidgetSnapshotRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("widget snapshot id must not be empty".to_string());
        }
        if self.width_px == 0 || self.height_px == 0 {
            return Err("widget snapshot dimensions must be positive".to_string());
        }
        if self.scale == 0 {
            return Err("widget snapshot scale must be positive".to_string());
        }
        if self.app_group_dir.as_os_str().is_empty() {
            return Err("widget snapshot app group directory must not be empty".to_string());
        }
        for entry in &self.timeline {
            entry.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetSnapshotResult {
    pub snapshot_path: PathBuf,
    pub timeline_path: PathBuf,
    pub used_stale_snapshot: bool,
    pub generated_unix_seconds: i64,
}

pub fn render_widget_snapshot(
    request: &WidgetSnapshotRequest,
    png_bytes: &[u8],
) -> Result<WidgetSnapshotResult, String> {
    request.validate()?;

    fs::create_dir_all(&request.app_group_dir)
        .map_err(|err| format!("create widget snapshot dir: {err}"))?;

    let snapshot_path = request
        .app_group_dir
        .join(format!("{}.png", sanitized_file_stem(&request.id)));
    let timeline_path = request.app_group_dir.join(format!(
        "{}.timeline.json",
        sanitized_file_stem(&request.id)
    ));

    let used_stale_snapshot = if png_bytes.is_empty() {
        !snapshot_path.exists()
    } else {
        fs::write(&snapshot_path, png_bytes)
            .map_err(|err| format!("write widget snapshot image: {err}"))?;
        false
    };

    write_timeline_json(&timeline_path, request, used_stale_snapshot)?;

    Ok(WidgetSnapshotResult {
        snapshot_path,
        timeline_path,
        used_stale_snapshot,
        generated_unix_seconds: now_unix_seconds(),
    })
}

/// Asynchronous variant of [`render_widget_snapshot`] that offloads the
/// timeline JSON write (and the PNG write) to the provided background executor.
pub fn render_widget_snapshot_async(
    request: WidgetSnapshotRequest,
    png_bytes: Vec<u8>,
    executor: BackgroundExecutor,
) -> Task<Result<WidgetSnapshotResult, String>> {
    executor.spawn(async move {
        request.validate()?;

        fs::create_dir_all(&request.app_group_dir)
            .map_err(|err| format!("create widget snapshot dir: {err}"))?;

        let snapshot_path = request
            .app_group_dir
            .join(format!("{}.png", sanitized_file_stem(&request.id)));
        let timeline_path = request.app_group_dir.join(format!(
            "{}.timeline.json",
            sanitized_file_stem(&request.id)
        ));

        let used_stale_snapshot = if png_bytes.is_empty() {
            !snapshot_path.exists()
        } else {
            fs::write(&snapshot_path, &png_bytes)
                .map_err(|err| format!("write widget snapshot image: {err}"))?;
            false
        };

        write_timeline_json(&timeline_path, &request, used_stale_snapshot)?;

        Ok(WidgetSnapshotResult {
            snapshot_path,
            timeline_path,
            used_stale_snapshot,
            generated_unix_seconds: now_unix_seconds(),
        })
    })
}

fn write_timeline_json(
    path: &Path,
    request: &WidgetSnapshotRequest,
    used_stale_snapshot: bool,
) -> Result<(), String> {
    let entries = request
        .timeline
        .iter()
        .map(|entry| {
            format!(
                "{{\"date_unix_seconds\":{},\"title\":{:?},\"subtitle\":{},\"snapshot_file_name\":{:?}}}",
                entry.date_unix_seconds,
                entry.title,
                entry
                    .subtitle
                    .as_ref()
                    .map(|value| format!("{value:?}"))
                    .unwrap_or_else(|| "null".to_string()),
                entry.snapshot_file_name,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        "{{\"id\":{:?},\"kind\":{:?},\"width_px\":{},\"height_px\":{},\"scale\":{},\"used_stale_snapshot\":{},\"generated_unix_seconds\":{},\"entries\":[{}]}}",
        request.id,
        request.kind,
        request.width_px,
        request.height_px,
        request.scale,
        used_stale_snapshot,
        now_unix_seconds(),
        entries,
    );
    fs::write(path, json).map_err(|err| format!("write widget timeline: {err}"))
}

fn sanitized_file_stem(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "snapshot".to_string()
    } else {
        out
    }
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_snapshot_request_validates_dimensions() {
        let request = WidgetSnapshotRequest {
            id: "now-playing".to_string(),
            kind: WidgetSnapshotKind::Widget,
            width_px: 0,
            height_px: 160,
            scale: 2,
            app_group_dir: PathBuf::from("/tmp/widgets"),
            timeline: Vec::new(),
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn widget_snapshot_writes_image_and_timeline() {
        let tmp = tempfile::tempdir().unwrap();
        let request = WidgetSnapshotRequest {
            id: "now playing".to_string(),
            kind: WidgetSnapshotKind::Widget,
            width_px: 320,
            height_px: 160,
            scale: 2,
            app_group_dir: tmp.path().to_path_buf(),
            timeline: vec![WidgetTimelineEntry {
                date_unix_seconds: 1,
                title: "Track".to_string(),
                subtitle: Some("Artist".to_string()),
                snapshot_file_name: "now_playing.png".to_string(),
            }],
        };

        let result = render_widget_snapshot(&request, b"png").unwrap();
        assert!(result.snapshot_path.exists());
        assert!(result.timeline_path.exists());
        assert!(!result.used_stale_snapshot);
    }
}
