//! Eject-to-own: copy component source into a consumer tree.
//!
//! [`eject_component`] resolves a ui-kit component name to its source
//! files, copies them (with companion modules) into the target
//! directory, and rewrites `crate::` paths to `gpui_ui_kit::` so the
//! ejected code compiles as consumer-owned modules against the published
//! crate. Sources resolve from the toolkit checkout: [`TOOLKIT_ROOT_ENV`]
//! wins, otherwise the current directory is walked upward for
//! `crates/gpui-ui-kit`. Ejected code keeps the feature expectations of
//! its module; feature-gated components stay feature-gated.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Discriminator for eject receipts in JSON envelopes.
pub const EJECT_TYPE: &str = "eject";

/// Environment variable pointing at a toolkit checkout.
///
/// Shared with the scaffolder: set `GPUI_TOOLKIT_ROOT` when ejecting
/// from outside a checkout.
pub const TOOLKIT_ROOT_ENV: &str = "GPUI_TOOLKIT_ROOT";

/// Crate prefix replacing `crate::` in ejected sources.
const EJECT_CRATE_PREFIX: &str = "gpui_ui_kit::";

/// Receipt describing an eject run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EjectReceipt {
    /// Requested component name.
    pub component: String,
    /// Target directory.
    pub dir: PathBuf,
    /// Written files, relative to `dir`.
    pub files: Vec<String>,
    /// Where to report the gap that forced the eject.
    pub issues_url: String,
}

/// Finds the toolkit checkout root.
///
/// Honors [`TOOLKIT_ROOT_ENV`] first, then walks upward from the
/// current directory for `crates/gpui-ui-kit`. Returns `None` outside
/// a checkout without the variable set.
///
/// # Examples
///
/// ```rust,no_run
/// use gpui_toolkit_cli::find_toolkit_root;
///
/// let root = find_toolkit_root().expect("run inside a checkout");
/// assert!(root.join("crates/gpui-ui-kit").is_dir());
/// ```
pub fn find_toolkit_root() -> Option<PathBuf> {
    if let Ok(root) = std::env::var(TOOLKIT_ROOT_ENV) {
        let root = PathBuf::from(root);
        if root.join("crates/gpui-ui-kit").is_dir() {
            return Some(root);
        }
    }
    let mut current = std::env::current_dir().ok()?;
    loop {
        if current.join("crates/gpui-ui-kit").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Converts a component name to its module path.
///
/// `PascalCase` maps to `snake_case` (`NumberInput` becomes
/// `number_input`); verified multi-struct modules map explicitly
/// (`QrCode` lives in `qr`, `ColorPickerView` in `color_picker`).
fn module_for_component(name: &str) -> String {
    match name {
        "QrCode" | "AnimatedQrCode" => return String::from("qr"),
        "ColorPickerView" | "ColorPickerMode" => return String::from("color_picker"),
        _ => {}
    }
    let mut snake = String::with_capacity(name.len() + 4);
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

/// Checks that a name is a plausible component identifier.
fn is_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric())
}

/// Collects the source files backing one module.
///
/// Prefers `<module>.rs` with `<module>/` companions; falls back to a
/// `<module>/` directory module (`mod.rs` plus siblings, copied
/// recursively so licenses travel with the code).
fn collect_module_sources(
    ui_kit: &Path,
    module: &str,
) -> Result<Vec<(PathBuf, PathBuf)>, ToolkitError> {
    let flat = ui_kit.join("src").join(format!("{module}.rs"));
    if flat.is_file() {
        let mut sources = vec![(flat, PathBuf::from(format!("{module}.rs")))];
        let companions = ui_kit.join("src").join(module);
        if companions.is_dir() {
            collect_dir_sources(&companions, &PathBuf::from(module), &mut sources)?;
        }
        return Ok(sources);
    }
    let dir = ui_kit.join("src").join(module);
    if dir.join("mod.rs").is_file() {
        let mut sources = Vec::new();
        collect_dir_sources(&dir, &PathBuf::from(module), &mut sources)?;
        return Ok(sources);
    }
    Err(ToolkitError::new(
        ErrorCode::NoSource,
        format!(
            "no ejectable source for module '{module}' (looked for src/{module}.rs and src/{module}/mod.rs)"
        ),
    ))
}

/// Recursively collects files as (absolute, relative) pairs.
fn collect_dir_sources(
    dir: &Path,
    relative: &Path,
    sources: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), ToolkitError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|error| {
            ToolkitError::new(
                ErrorCode::NoSource,
                format!("cannot read '{}': {error}", dir.display()),
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            ToolkitError::new(
                ErrorCode::NoSource,
                format!("cannot read '{}': {error}", dir.display()),
            )
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let rel = relative.join(entry.file_name());
        if path.is_dir() {
            collect_dir_sources(&path, &rel, sources)?;
        } else if path.is_file() {
            sources.push((path, rel));
        }
    }
    Ok(())
}

/// Ejects one component from an explicit checkout root.
///
/// Testable core of [`eject_component`]: copies the module sources
/// into `dir`, rewriting `crate::` paths to [`EJECT_CRATE_PREFIX`] in
/// Rust files. Existing files are never overwritten.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] for malformed names or a
/// missing target directory, [`ErrorCode::NoSource`] when the module
/// has no ejectable files, [`ErrorCode::FileExists`] on collisions,
/// and [`ErrorCode::WriteFailed`] for I/O failures.
pub fn eject_component_from_root(
    name: &str,
    dir: &Path,
    root: &Path,
) -> Result<EjectReceipt, ToolkitError> {
    if !is_component_name(name) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("component '{name}' must be PascalCase alphanumeric"),
        ));
    }
    if !dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", dir.display()),
        ));
    }
    let module = module_for_component(name);
    let ui_kit = root.join("crates/gpui-ui-kit");
    let sources = collect_module_sources(&ui_kit, &module)?;
    let mut files = Vec::with_capacity(sources.len());
    for (_, relative) in &sources {
        let target = dir.join(relative);
        if target.exists() {
            return Err(ToolkitError::new(
                ErrorCode::FileExists,
                format!("refusing to overwrite '{}'", target.display()),
            ));
        }
    }
    for (absolute, relative) in &sources {
        let target = dir.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                ToolkitError::new(
                    ErrorCode::WriteFailed,
                    format!("cannot create '{}': {error}", parent.display()),
                )
            })?;
        }
        if absolute.extension().is_some_and(|ext| ext == "rs") {
            let content = std::fs::read_to_string(absolute).map_err(|error| {
                ToolkitError::new(
                    ErrorCode::WriteFailed,
                    format!("cannot read '{}': {error}", absolute.display()),
                )
            })?;
            let rewritten = content.replace("crate::", EJECT_CRATE_PREFIX);
            std::fs::write(&target, rewritten).map_err(|error| {
                ToolkitError::new(
                    ErrorCode::WriteFailed,
                    format!("cannot write '{}': {error}", target.display()),
                )
            })?;
        } else {
            std::fs::copy(absolute, &target).map_err(|error| {
                ToolkitError::new(
                    ErrorCode::WriteFailed,
                    format!("cannot copy '{}': {error}", absolute.display()),
                )
            })?;
        }
        files.push(relative.to_string_lossy().into_owned());
    }
    Ok(EjectReceipt {
        component: name.to_owned(),
        dir: dir.to_path_buf(),
        files,
        issues_url: format!("{}/issues", env!("CARGO_PKG_REPOSITORY")),
    })
}

/// Ejects one component, resolving the checkout automatically.
///
/// Uses [`find_toolkit_root`]; set [`TOOLKIT_ROOT_ENV`] outside a
/// checkout.
///
/// # Errors
///
/// Returns [`ErrorCode::NoSource`] outside a checkout, plus every
/// [`eject_component_from_root`] failure.
pub fn eject_component(name: &str, dir: &Path) -> Result<EjectReceipt, ToolkitError> {
    let Some(root) = find_toolkit_root() else {
        return Err(ToolkitError::new(
            ErrorCode::NoSource,
            format!("not inside a toolkit checkout; set {TOOLKIT_ROOT_ENV}"),
        ));
    };
    eject_component_from_root(name, dir, &root)
}

/// Renders an eject receipt as human-readable text.
pub fn render_eject_text(receipt: &EjectReceipt, dense: bool) -> String {
    if dense {
        return format!(
            "{} {} {}\n",
            receipt.component,
            receipt.files.len(),
            receipt.dir.display()
        );
    }
    format!(
        "Ejected {} ({} files) to {}\nReport the gap: {}\n",
        receipt.component,
        receipt.files.len(),
        receipt.dir.display(),
        receipt.issues_url
    )
}
