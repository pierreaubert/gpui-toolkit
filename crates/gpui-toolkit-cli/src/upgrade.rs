//! Machine-readable migration registry.
//!
//! [`upgrade_list`] reports the running toolkit version plus every
//! registered [`MigrationNote`]. Notes land here as breaking changes
//! ship, oldest first, so `upgrade` output plus `doctor` detection of
//! deprecated patterns carry consumers across releases. The registry is
//! empty today: no migration has been needed yet.

// Rust guideline compliant 2026-02-21

use serde::Serialize;

/// Discriminator for upgrade payloads in JSON envelopes.
pub const UPGRADE_LIST_TYPE: &str = "upgrade.list";

/// One version-to-version migration note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationNote {
    /// Stable note id, such as `theme-export-v2`.
    pub id: String,
    /// One-line title.
    pub title: String,
    /// Version range the note migrates from.
    pub from_version: String,
    /// Version range the note migrates to.
    pub to_version: String,
    /// What changed and why.
    pub summary: String,
    /// Ordered remediation steps.
    pub items: Vec<String>,
}

/// Registered notes, oldest first; empty until the first migration.
pub const MIGRATIONS: &[MigrationNote] = &[];

/// Upgrade payload: running version plus registered notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeList {
    /// Running toolkit version.
    pub toolkit_version: String,
    /// Number of registered notes.
    pub count: usize,
    /// Notes in registry order.
    pub migrations: Vec<MigrationNote>,
}

/// Lists registered migration notes.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::upgrade_list;
///
/// let list = upgrade_list();
/// assert!(!list.toolkit_version.is_empty());
/// ```
pub fn upgrade_list() -> UpgradeList {
    UpgradeList {
        toolkit_version: env!("CARGO_PKG_VERSION").to_owned(),
        count: MIGRATIONS.len(),
        migrations: MIGRATIONS.to_vec(),
    }
}

/// Renders an upgrade listing as human-readable text.
pub fn render_upgrade_list_text(list: &UpgradeList, dense: bool) -> String {
    let mut text = String::new();
    if list.migrations.is_empty() {
        if dense {
            text.push_str(&format!("{} 0\n", list.toolkit_version));
        } else {
            text.push_str(&format!(
                "No migrations registered (toolkit {}).\n",
                list.toolkit_version
            ));
        }
        return text;
    }
    if !dense {
        text.push_str(&format!(
            "{} migrations (toolkit {}):\n\n",
            list.count, list.toolkit_version
        ));
    }
    for note in &list.migrations {
        if dense {
            text.push_str(&format!(
                "{} {} {}\n",
                note.id, note.from_version, note.to_version
            ));
        } else {
            text.push_str(&format!(
                "{} — {} ({} → {})\n{}\n",
                note.id, note.title, note.from_version, note.to_version, note.summary
            ));
            for item in &note.items {
                text.push_str(&format!("  - {item}\n"));
            }
        }
    }
    text
}
