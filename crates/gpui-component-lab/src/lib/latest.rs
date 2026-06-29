use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Latest modified time among `*.story.json` documents and watched token files.
pub fn latest_story_or_token_modified(dir: &Path, tokens: &[PathBuf]) -> Result<SystemTime> {
    let mut latest = latest_story_modified(dir)?;
    for token in tokens {
        if token.exists() {
            latest = latest.max(token.metadata()?.modified()?);
        }
    }
    Ok(latest)
}

/// Latest modified time for story documents in a directory.
pub fn latest_story_modified(dir: &Path) -> Result<SystemTime> {
    let mut latest = SystemTime::UNIX_EPOCH;
    if !dir.exists() {
        return Ok(latest);
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".story.json"))
        {
            latest = latest.max(entry.metadata()?.modified()?);
        }
    }
    Ok(latest)
}

/// Latest modified time for Rust/TOML sources under a root, ignoring target dirs.
pub fn latest_rust_source_modified(root: &Path) -> Result<SystemTime> {
    let mut latest = SystemTime::UNIX_EPOCH;
    if !root.exists() {
        return Ok(latest);
    }
    for entry in std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "target")
            {
                continue;
            }
            latest = latest.max(latest_rust_source_modified(&path)?);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs" || ext == "toml")
        {
            latest = latest.max(entry.metadata()?.modified()?);
        }
    }
    Ok(latest)
}
