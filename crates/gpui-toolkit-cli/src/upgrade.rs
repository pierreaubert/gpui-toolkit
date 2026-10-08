//! Machine-readable migration registry plus deprecated-pattern detection.
//!
//! [`upgrade_list`] reports the running toolkit version plus every
//! registered [`MigrationNote`]. Notes land here as breaking changes
//! ship, oldest first. [`upgrade_detect`] scans a tree for deprecated
//! patterns — built-in rules plus `[upgrade.rules]` from `toolkit.toml` —
//! so `upgrade` output carries consumers across releases. The note
//! registry is empty today: no migration has been needed yet.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Discriminator for upgrade payloads in JSON envelopes.
pub const UPGRADE_LIST_TYPE: &str = "upgrade.list";

/// Discriminator for upgrade detection payloads.
pub const UPGRADE_DETECT_TYPE: &str = "upgrade.detect";

/// One deprecated-pattern rule: literal substring, message, and fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeRule {
    /// Stable rule id, such as `no-build-with-theme`.
    pub id: String,
    /// Literal substring matched against code lines.
    pub pattern: String,
    /// Why the pattern is deprecated.
    pub message: String,
    /// Remediation, omitted when the message suffices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// Built-in rule definitions: id, pattern, message, fix.
const BUILTIN_RULE_DEFS: &[(&str, &str, &str, &str)] = &[(
    "no-build-with-theme",
    "build_with_theme(",
    "prefer RenderOnce over build_with_theme: the helper bypasses accessibility registration",
    "implement RenderOnce and call cx.register_accessible in render()",
)];

/// Built-in detection rules shipped with the CLI.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::builtin_rules;
///
/// assert!(builtin_rules().iter().any(|rule| rule.id == "no-build-with-theme"));
/// ```
pub fn builtin_rules() -> Vec<UpgradeRule> {
    BUILTIN_RULE_DEFS
        .iter()
        .map(|(id, pattern, message, fix)| UpgradeRule {
            id: (*id).to_owned(),
            pattern: (*pattern).to_owned(),
            message: (*message).to_owned(),
            fix: Some((*fix).to_owned()),
        })
        .collect()
}

/// One deprecated-pattern hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeFinding {
    /// File relative to the scanned directory.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// Firing rule id.
    pub rule: String,
    /// Why the pattern is deprecated.
    pub message: String,
    /// Remediation, omitted when the message suffices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// Detection payload for one directory scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeDetect {
    /// Scanned directory, as given.
    pub dir: String,
    /// Active rules: built-ins first, then `toolkit.toml` extras.
    pub rules: Vec<UpgradeRule>,
    /// Rust files found under the directory.
    pub scanned_files: usize,
    /// Files skipped (unreadable or non-UTF-8).
    pub skipped_files: usize,
    /// Findings, sorted by file, line, then rule.
    pub findings: Vec<UpgradeFinding>,
    /// Number of findings.
    pub count: usize,
}

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

/// Scans `dir` for deprecated patterns.
///
/// Walks Rust files recursively (hidden directories and `target/`
/// skipped, entries sorted for determinism) and matches each rule's
/// literal pattern against code lines. `//` comments are stripped with
/// string awareness, so `let url = "https://…"` never hides a real hit
/// on the same line; block comments are not stripped. `extra_rules`
/// (from `[upgrade.rules]`) run after the built-ins.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] when `dir` is not a
/// directory.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::upgrade_detect;
///
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(dir.path().join("lib.rs"), "fn f() {}\n").unwrap();
/// let report = upgrade_detect(dir.path(), &[]).unwrap();
/// assert_eq!(report.scanned_files, 1);
/// assert_eq!(report.count, 0);
/// ```
pub fn upgrade_detect(
    dir: &Path,
    extra_rules: &[UpgradeRule],
) -> Result<UpgradeDetect, ToolkitError> {
    if !dir.is_dir() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("directory '{}' does not exist", dir.display()),
        ));
    }
    let mut rules = builtin_rules();
    rules.extend(extra_rules.iter().cloned());
    let mut files = Vec::new();
    collect_rust_files(dir, &mut files);
    let mut findings = Vec::new();
    let mut skipped_files = 0;
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            skipped_files += 1;
            continue;
        };
        let rel = file
            .strip_prefix(dir)
            .unwrap_or(file)
            .to_string_lossy()
            .into_owned();
        for (index, line) in text.lines().enumerate() {
            let code = code_without_line_comment(line);
            for rule in &rules {
                if code.contains(&rule.pattern) {
                    findings.push(UpgradeFinding {
                        file: rel.clone(),
                        line: index + 1,
                        rule: rule.id.clone(),
                        message: rule.message.clone(),
                        fix: rule.fix.clone(),
                    });
                }
            }
        }
    }
    findings.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.line.cmp(&right.line))
            .then(left.rule.cmp(&right.rule))
    });
    Ok(UpgradeDetect {
        dir: dir.to_string_lossy().into_owned(),
        rules,
        scanned_files: files.len(),
        skipped_files,
        count: findings.len(),
        findings,
    })
}

/// Collects Rust files under `dir`, skipping hidden and `target/` dirs.
fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut sorted: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    sorted.sort();
    for path in sorted {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// Strips a trailing `//` comment, ignoring `//` inside strings.
///
/// Tracks double-quoted strings with backslash escapes; character
/// literals and block comments are out of scope.
fn code_without_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0;
    while index + 1 < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'/' && bytes[index + 1] == b'/' {
            return &line[..index];
        }
        index += 1;
    }
    line
}

/// Renders a detection report as human-readable text.
///
/// Dense output keeps one line per finding plus the counts; the
/// default rendering adds messages, fixes, and a closing verdict.
pub fn render_upgrade_detect_text(report: &UpgradeDetect, dense: bool) -> String {
    let mut text = String::new();
    for finding in &report.findings {
        if dense {
            text.push_str(&format!(
                "{}:{} {}\n",
                finding.file, finding.line, finding.rule
            ));
        } else {
            text.push_str(&format!(
                "{}:{} [{}] {}\n",
                finding.file, finding.line, finding.rule, finding.message
            ));
            if let Some(fix) = &finding.fix {
                text.push_str(&format!("  fix: {fix}\n"));
            }
        }
    }
    if dense {
        text.push_str(&format!(
            "scanned={} skipped={} findings={}\n",
            report.scanned_files, report.skipped_files, report.count
        ));
    } else if report.findings.is_empty() {
        text.push_str(&format!(
            "No deprecated patterns in {} ({} files scanned).\n",
            report.dir, report.scanned_files
        ));
    } else {
        text.push_str(&format!(
            "\n{} findings in {} ({} files scanned).\n",
            report.count, report.dir, report.scanned_files
        ));
    }
    text
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
