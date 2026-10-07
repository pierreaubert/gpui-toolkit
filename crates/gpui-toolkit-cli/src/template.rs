//! Page and block template registry.
//!
//! Templates are content-only GPUI sources composed from the miniapp
//! shell and ui-kit: start from one instead of composing from scratch.
//! Each entry carries its full `source` plus a `skeleton` structural
//! outline with spatial annotations, so agents can read the layout at a
//! fraction of the token cost before copying. Sources embed at compile
//! time from `templates/`, so the registry cannot drift from the files.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use crate::init::is_plain_file_name;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Discriminator for template list payloads.
pub const TEMPLATE_LIST_TYPE: &str = "template.list";

/// Discriminator for template source payloads.
pub const TEMPLATE_SHOW_TYPE: &str = "template.show";

/// Discriminator for template skeleton payloads.
pub const TEMPLATE_SKELETON_TYPE: &str = "template.skeleton";

/// Discriminator for template copy receipts.
pub const TEMPLATE_COPY_TYPE: &str = "template.copy";

/// Template granularity: full pages or embeddable blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    /// Full page content, wrapped in app chrome by the caller.
    Page,
    /// Embeddable block for use inside a page layout.
    Block,
}

impl TemplateKind {
    /// Canonical wire name of this kind.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::TemplateKind;
    ///
    /// assert_eq!(TemplateKind::Page.as_str(), "page");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Block => "block",
        }
    }
}

/// One registry entry with embedded source and skeleton.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemplateStatic {
    /// Template id as spelled on the command line.
    pub id: &'static str,
    /// One-line title.
    pub title: &'static str,
    /// What the template is for.
    pub description: &'static str,
    /// Page or block granularity.
    pub kind: TemplateKind,
    /// Full Rust source, embedded from `templates/`.
    pub source: &'static str,
    /// Structural outline with spatial annotations.
    pub skeleton: &'static str,
}

/// Template registry; the only hand-maintained template input.
///
/// Sources embed at compile time, so `cargo build` fails when a file
/// goes missing instead of failing at runtime.
pub const TEMPLATES: &[TemplateStatic] = &[
    TemplateStatic {
        id: "settings-page",
        title: "Settings page",
        description: "Title, two labeled fields, and save/cancel actions",
        kind: TemplateKind::Page,
        source: include_str!("../templates/settings_page.rs"),
        skeleton: "VStack [settings page]\n  Text \"Settings\" [heading]\n  Input#settings-name [label=\"Display name\"]\n  Input#settings-email [label=\"Email\"]\n  HStack [actions]\n    Button#settings-save.primary \"Save\"\n    Button#settings-cancel \"Cancel\"\n",
    },
    TemplateStatic {
        id: "dashboard-page",
        title: "Dashboard page",
        description: "Title, two stat blocks, and a primary action",
        kind: TemplateKind::Page,
        source: include_str!("../templates/dashboard_page.rs"),
        skeleton: "VStack [dashboard page]\n  Text \"Dashboard\" [heading]\n  HStack [stats]\n    VStack [stat]\n      Text \"$42k\" [value]\n      Text \"Revenue\" [label]\n    VStack [stat]\n      Text \"128\" [value]\n      Text \"Orders\" [label]\n  Button#dashboard-new.primary \"New report\"\n",
    },
    TemplateStatic {
        id: "form-section",
        title: "Form section",
        description: "Reusable block of two labeled inputs",
        kind: TemplateKind::Block,
        source: include_str!("../templates/form_section.rs"),
        skeleton: "VStack [form block]\n  Input#form-name [label=\"Name\"]\n  Input#form-email [label=\"Email\"]\n",
    },
];

/// One row of template list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateEntry {
    /// Template id.
    pub id: String,
    /// One-line title.
    pub title: String,
    /// What the template is for.
    pub description: String,
    /// `page` or `block`.
    pub kind: String,
}

/// Template list payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateList {
    /// Number of templates.
    pub count: usize,
    /// Entries in registry order.
    pub templates: Vec<TemplateEntry>,
}

/// Template source payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateShow {
    /// Template id.
    pub id: String,
    /// One-line title.
    pub title: String,
    /// What the template is for.
    pub description: String,
    /// `page` or `block`.
    pub kind: String,
    /// Full Rust source.
    pub source: String,
}

/// Template skeleton payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSkeleton {
    /// Template id.
    pub id: String,
    /// One-line title.
    pub title: String,
    /// What the template is for.
    pub description: String,
    /// Structural outline with spatial annotations.
    pub skeleton: String,
}

/// Template copy receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateCopy {
    /// Template id.
    pub id: String,
    /// Written file path.
    pub path: PathBuf,
    /// Written bytes.
    pub bytes: usize,
}

/// Lists all templates in registry order.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::template_list;
///
/// let list = template_list();
/// assert!(list.count >= 3);
/// ```
pub fn template_list() -> TemplateList {
    TemplateList {
        count: TEMPLATES.len(),
        templates: TEMPLATES
            .iter()
            .map(|entry| TemplateEntry {
                id: entry.id.to_owned(),
                title: entry.title.to_owned(),
                description: entry.description.to_owned(),
                kind: entry.kind.as_str().to_owned(),
            })
            .collect(),
    }
}

/// Looks up one template by id.
///
/// # Errors
///
/// Returns [`ErrorCode::UnknownTemplate`] when no template matches.
fn lookup_template(id: &str) -> Result<&'static TemplateStatic, ToolkitError> {
    TEMPLATES
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| {
            ToolkitError::new(
                ErrorCode::UnknownTemplate,
                format!("no template named '{id}'"),
            )
        })
}

/// Returns the full source of one template.
///
/// # Errors
///
/// Returns [`ErrorCode::UnknownTemplate`] when no template matches.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::template_show;
///
/// let shown = template_show("settings-page").unwrap();
/// assert!(shown.source.contains("settings_page"));
/// ```
pub fn template_show(id: &str) -> Result<TemplateShow, ToolkitError> {
    let entry = lookup_template(id)?;
    Ok(TemplateShow {
        id: entry.id.to_owned(),
        title: entry.title.to_owned(),
        description: entry.description.to_owned(),
        kind: entry.kind.as_str().to_owned(),
        source: entry.source.to_owned(),
    })
}

/// Returns the structural skeleton of one template.
///
/// # Errors
///
/// Returns [`ErrorCode::UnknownTemplate`] when no template matches.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::template_skeleton;
///
/// let skeleton = template_skeleton("form-section").unwrap();
/// assert!(skeleton.skeleton.contains("form block"));
/// ```
pub fn template_skeleton(id: &str) -> Result<TemplateSkeleton, ToolkitError> {
    let entry = lookup_template(id)?;
    Ok(TemplateSkeleton {
        id: entry.id.to_owned(),
        title: entry.title.to_owned(),
        description: entry.description.to_owned(),
        skeleton: entry.skeleton.to_owned(),
    })
}

/// Copies one template source into a directory.
///
/// The file defaults to `<id>.rs` with dashes underscored
/// (`settings-page` becomes `settings_page.rs`); `file` overrides it.
/// Existing files are never overwritten.
///
/// # Errors
///
/// Returns [`ErrorCode::UnknownTemplate`] for unknown ids,
/// [`ErrorCode::InvalidArgument`] when `dir` is missing or the file
/// name is not plain, and [`ErrorCode::FileExists`] when the target
/// already exists. Write failures report [`ErrorCode::WriteFailed`].
pub fn template_copy(
    id: &str,
    dir: &Path,
    file: Option<&str>,
) -> Result<TemplateCopy, ToolkitError> {
    let entry = lookup_template(id)?;
    if !dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", dir.display()),
        ));
    }
    let file_name = file.map_or_else(|| id.replace('-', "_") + ".rs", str::to_owned);
    if !is_plain_file_name(&file_name) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("file name '{file_name}' must be a plain file name"),
        ));
    }
    let path = dir.join(file_name);
    if path.exists() {
        return Err(ToolkitError::new(
            ErrorCode::FileExists,
            format!("refusing to overwrite '{}'", path.display()),
        ));
    }
    std::fs::write(&path, entry.source).map_err(|error| {
        ToolkitError::new(
            ErrorCode::WriteFailed,
            format!("cannot write '{}': {error}", path.display()),
        )
    })?;
    Ok(TemplateCopy {
        id: id.to_owned(),
        bytes: entry.source.len(),
        path,
    })
}

/// Renders a template list as human-readable text.
pub fn render_template_list_text(list: &TemplateList, dense: bool) -> String {
    let mut text = String::new();
    if !dense {
        text.push_str(&format!("{} templates:\n\n", list.count));
    }
    for entry in &list.templates {
        if dense {
            text.push_str(&format!("{} {}\n", entry.id, entry.kind));
        } else {
            text.push_str(&format!(
                "{} [{}] — {}: {}\n",
                entry.id, entry.kind, entry.title, entry.description
            ));
        }
    }
    text
}

/// Renders a copy receipt as human-readable text.
pub fn render_template_copy_text(copy: &TemplateCopy) -> String {
    format!(
        "Wrote template '{}' to {} ({} bytes)\n",
        copy.id,
        copy.path.display(),
        copy.bytes
    )
}
