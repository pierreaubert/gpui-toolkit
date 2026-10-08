//! Project bootstrap writing the managed agent catalog.
//!
//! [`run_init`] builds the component catalog from the component lab and
//! writes it into the project's agent docs between [`CATALOG_BEGIN`] and
//! [`CATALOG_END`]. An existing managed block is replaced in place;
//! otherwise the block is appended (creating the file when missing), so
//! hand-written notes outside the markers are never touched.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use gpui_component_lab::builtin_story_registry;
use serde::Serialize;
use std::path::PathBuf;

/// Discriminator for init receipts in JSON envelopes.
pub const INIT_TYPE: &str = "init.run";

/// Marker opening the managed catalog block.
pub const CATALOG_BEGIN: &str = "<!-- BEGIN GENERATED: toolkit-catalog -->";

/// Marker closing the managed catalog block.
pub const CATALOG_END: &str = "<!-- END GENERATED: toolkit-catalog -->";

/// Options selecting where [`run_init`] writes the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitOptions {
    /// Project directory receiving the catalog; must exist.
    pub dir: PathBuf,
    /// Agent doc file name inside `dir`, such as `AGENTS.md`.
    pub agents_file: String,
}

impl Default for InitOptions {
    /// Targets `./AGENTS.md` in the current directory.
    fn default() -> Self {
        Self {
            dir: PathBuf::from("."),
            agents_file: String::from("AGENTS.md"),
        }
    }
}

/// Receipt describing what [`run_init`] wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReceipt {
    /// Catalog file that was written.
    pub path: PathBuf,
    /// Number of cataloged stories.
    pub stories: usize,
    /// Whether the file was created by this run.
    pub created: bool,
    /// Whether an existing managed block was replaced.
    pub updated: bool,
}

/// Writes the component catalog into the project's agent docs.
///
/// Builds the catalog from the component lab, then creates or refreshes
/// the managed block in `dir/agents_file`. Returns a receipt with the
/// written path, the story count, and whether the file was created or
/// an existing block was replaced.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] when `dir` is missing or not a
/// directory, when `agents_file` is not a plain file name, or when an
/// existing managed block has no closing marker. Returns
/// [`ErrorCode::CatalogLoad`] when the story registry cannot build and
/// [`ErrorCode::WriteFailed`] when the file cannot be read or written.
///
/// # Examples
///
/// ```rust,no_run
/// use gpui_toolkit_cli::{InitOptions, run_init};
///
/// let options = InitOptions::default();
/// let receipt = run_init(&options).unwrap();
/// assert!(receipt.stories > 0);
/// ```
pub fn run_init(options: &InitOptions) -> Result<InitReceipt, ToolkitError> {
    if !options.dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", options.dir.display()),
        ));
    }
    if !is_plain_file_name(&options.agents_file) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!(
                "agents file '{}' must be a plain file name",
                options.agents_file
            ),
        ));
    }
    let registry = builtin_story_registry()
        .map_err(|error| ToolkitError::new(ErrorCode::CatalogLoad, error.to_string()))?;
    let mut stories: Vec<_> = registry.stories().collect();
    stories.sort_by(|left, right| left.id.cmp(&right.id));
    let block = render_catalog_block(&stories);
    let path = options.dir.join(&options.agents_file);
    let existing = match std::fs::read_to_string(&path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(ToolkitError::new(
                ErrorCode::WriteFailed,
                format!("cannot read '{}': {error}", path.display()),
            ));
        }
    };
    let created = existing.is_none();
    let (content, updated) = match existing {
        None => (format!("{block}\n"), false),
        Some(previous) => splice_catalog_block(&previous, &block, &path)?,
    };
    std::fs::write(&path, content).map_err(|error| {
        ToolkitError::new(
            ErrorCode::WriteFailed,
            format!("cannot write '{}': {error}", path.display()),
        )
    })?;
    Ok(InitReceipt {
        path,
        stories: stories.len(),
        created,
        updated,
    })
}

/// Rejects anything but a plain file name.
///
/// The catalog must stay inside the project directory, so separators,
/// parent references, and empty names are invalid user input.
pub(crate) fn is_plain_file_name(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    std::path::Path::new(name)
        .file_name()
        .is_some_and(|base| base.to_string_lossy() == name)
}

/// Merges the catalog block into existing agent docs.
///
/// Replaces the previous managed block when the markers are present;
/// otherwise appends the block after a blank line. Reports whether a
/// block was replaced.
fn splice_catalog_block(
    previous: &str,
    block: &str,
    path: &std::path::Path,
) -> Result<(String, bool), ToolkitError> {
    let Some(begin) = previous.find(CATALOG_BEGIN) else {
        let separator = if previous.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        return Ok((format!("{previous}{separator}{block}\n"), false));
    };
    let Some(end) = previous.find(CATALOG_END) else {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!(
                "'{}' has an unterminated managed catalog block",
                path.display()
            ),
        ));
    };
    let end = end + CATALOG_END.len();
    let mut content = String::with_capacity(previous.len() + block.len());
    content.push_str(previous[..begin].trim_end());
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    content.push_str(block);
    let rest = previous[end..].trim_start_matches('\n');
    if rest.trim().is_empty() {
        content.push('\n');
    } else {
        content.push_str("\n\n");
        content.push_str(rest.trim_end());
        content.push('\n');
    }
    Ok((content, true))
}

/// Renders the managed catalog block for sorted stories.
fn render_catalog_block(stories: &[&gpui_component_lab::ComponentStory]) -> String {
    let mut block = String::from(CATALOG_BEGIN);
    block.push_str("\n## Toolkit catalog (managed by `gpui-toolkit init`; do not hand-edit)\n");
    for story in stories {
        block.push_str(&format!(
            "\n- `{}` ({}) — {}: {}",
            story.id, story.crate_name, story.title, story.description
        ));
    }
    block.push('\n');
    block.push_str(CATALOG_END);
    block
}
