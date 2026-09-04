//! Toolkit-owned design token import/export and conformance helpers.

use anyhow::{Context, Result, anyhow, bail};
use gpui_design::{DesignConformanceMatrix, DesignTokenExport};
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

/// Current schema version for `DesignTokenValidationReport` JSON output.
pub const DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION: u32 = 1;

/// Stable report discriminator for machine-readable validation output.
pub const DESIGN_TOKEN_VALIDATION_REPORT_TYPE: &str = "gpui-design-token-validation";

/// Current schema version for `DesignToolingHandoffReport` JSON output.
pub const DESIGN_TOOLING_HANDOFF_REPORT_SCHEMA_VERSION: u32 = 1;

/// Stable report discriminator for design handoff readiness output.
pub const DESIGN_TOOLING_HANDOFF_REPORT_TYPE: &str = "gpui-design-tooling-handoff";

/// Supported design token wire formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignTokenFormat {
    /// Style Dictionary-compatible JSON emitted by `gpui-design`.
    StyleDictionaryJson,
    /// W3C Design Tokens Community Group (DTCG) JSON with `$value`/`$type` leaves.
    W3cDtcgJson,
}

impl DesignTokenFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "json" | "style-dictionary-json" | "style_dictionary_json" => {
                Ok(Self::StyleDictionaryJson)
            }
            "dtcg" | "w3c-dtcg" | "w3c-dtcg-json" | "w3c_dtcg_json" => Ok(Self::W3cDtcgJson),
            other => bail!("unsupported design token format '{other}'"),
        }
    }

    /// Canonical CLI spelling for this format.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StyleDictionaryJson => "style-dictionary-json",
            Self::W3cDtcgJson => "w3c-dtcg-json",
        }
    }
}

/// Output options for [`export_design_tokens_with_options`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DesignTokenExportOptions {
    /// Emit compact JSON instead of pretty-printed JSON. The compact form is
    /// cheaper to write and parse for bulk exports; the token content is
    /// identical.
    pub compact: bool,
}

impl DesignTokenExportOptions {
    /// Pretty-printed output (default, human-readable).
    pub fn pretty() -> Self {
        Self { compact: false }
    }

    /// Compact single-line output for bulk exports.
    pub fn compact() -> Self {
        Self { compact: true }
    }
}

/// Append formatted text without the intermediate `String` that
/// `push_str(&format!(...))` allocates per row.
fn push_markdown(output: &mut String, args: std::fmt::Arguments<'_>) {
    let _ = std::fmt::Write::write_fmt(output, args);
}

/// Result of parsing an imported token document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedDesignTokens {
    pub preset_count: usize,
    pub token_count: usize,
    pub raw: Value,
}

/// Validation report for token documents and design conformance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignTokenValidationReport {
    pub schema_version: u32,
    pub report_type: Cow<'static, str>,
    pub passed: bool,
    pub findings: Vec<Cow<'static, str>>,
    pub preset_count: usize,
    pub token_count: usize,
    pub conformance_markdown: String,
}

/// Release-readiness status for a design tooling handoff row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesignToolingHandoffStatus {
    Implemented,
    Documented,
    ExternalGate,
}

impl DesignToolingHandoffStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Documented => "documented",
            Self::ExternalGate => "external-gate",
        }
    }

    pub fn is_release_blocking(self) -> bool {
        !matches!(self, Self::Implemented | Self::Documented)
    }
}

/// One design tooling handoff capability or artifact expected by release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DesignToolingHandoffItem {
    pub id: &'static str,
    pub title: &'static str,
    pub artifact_type: &'static str,
    pub path_or_command: &'static str,
    pub status: DesignToolingHandoffStatus,
    pub release_evidence: &'static str,
    pub remaining_gap: &'static str,
}

impl DesignToolingHandoffItem {
    pub fn is_release_blocking(&self) -> bool {
        self.status.is_release_blocking()
    }
}

/// Stable report describing design-token, Figma, and live-preview handoff state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignToolingHandoffReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub items: &'static [DesignToolingHandoffItem],
}

impl DesignToolingHandoffReport {
    pub fn blocking_entries(&self) -> Vec<&'static DesignToolingHandoffItem> {
        self.items
            .iter()
            .filter(|item| item.is_release_blocking())
            .collect()
    }

    pub fn item(&self, id: &str) -> Option<&'static DesignToolingHandoffItem> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# gpui-design-tools Handoff Readiness\n\n");
        push_markdown(
            &mut output,
            format_args!(
                "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n\n",
                self.schema_version, self.report_type, self.crate_name, self.crate_version
            ),
        );
        output
            .push_str("| id | status | artifact | path or command | evidence | remaining gap |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- |\n");
        for item in self.items {
            push_markdown(
                &mut output,
                format_args!(
                    "| `{}` | `{}` | {} | `{}` | {} | {} |\n",
                    item.id,
                    item.status.label(),
                    item.title,
                    item.path_or_command,
                    item.release_evidence,
                    item.remaining_gap,
                ),
            );
        }
        output
    }
}

pub fn design_tooling_handoff_report() -> DesignToolingHandoffReport {
    DesignToolingHandoffReport {
        schema_version: DESIGN_TOOLING_HANDOFF_REPORT_SCHEMA_VERSION,
        report_type: DESIGN_TOOLING_HANDOFF_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        items: DESIGN_TOOLING_HANDOFF_ITEMS,
    }
}

pub const DESIGN_TOOLING_HANDOFF_ITEMS: &[DesignToolingHandoffItem] = &[
    DesignToolingHandoffItem {
        id: "token-export",
        title: "Style Dictionary token export",
        artifact_type: "cli-command",
        path_or_command: "cargo run -p gpui-design-tools --bin gpui-export-design-tokens -- --format style-dictionary-json",
        status: DesignToolingHandoffStatus::Implemented,
        release_evidence: "DesignTokenExport::for_all_presets() serializes built-in presets with token names, paths, values, and token types.",
        remaining_gap: "none for token handoff",
    },
    DesignToolingHandoffItem {
        id: "token-import-validation",
        title: "Imported token shape validation",
        artifact_type: "cli-command",
        path_or_command: "cargo run -p gpui-design-tools --bin gpui-import-design-tokens -- --input tokens.json",
        status: DesignToolingHandoffStatus::Implemented,
        release_evidence: "import_design_tokens() rejects missing presets, token arrays, names, paths, values, and token types.",
        remaining_gap: "none for current Style Dictionary JSON shape",
    },
    DesignToolingHandoffItem {
        id: "conformance-report",
        title: "Machine-readable design conformance report",
        artifact_type: "cli-command",
        path_or_command: "cargo run -p gpui-design-tools --bin gpui-validate-design-tokens -- --report-json target/gpui-conformance/design-tokens.json",
        status: DesignToolingHandoffStatus::Implemented,
        release_evidence: "DesignTokenValidationReport exposes schema_version, report_type, pass/fail, findings, token counts, and optional Markdown.",
        remaining_gap: "none for release-token validation",
    },
    DesignToolingHandoffItem {
        id: "component-lab-preview",
        title: "Responsive component preview handoff",
        artifact_type: "companion-tool",
        path_or_command: "cargo run -p gpui-component-lab",
        status: DesignToolingHandoffStatus::Documented,
        release_evidence: "gpui-component-lab owns responsive story metadata and visual manifest generation for design review.",
        remaining_gap: "not a live Figma plugin or in-canvas data editor",
    },
    DesignToolingHandoffItem {
        id: "figma-code-connect",
        title: "Figma Code Connect mapping files",
        artifact_type: "repository-artifact",
        path_or_command: "figma/CODE_CONNECT_MAPPINGS.md",
        status: DesignToolingHandoffStatus::Implemented,
        release_evidence: "figma/CODE_CONNECT_MAPPINGS.md records schema-versioned component, token, and QA artifact mappings for static Figma handoff.",
        remaining_gap: "external Figma Code Connect publication is still a release-runner artifact, not a missing repository mapping",
    },
    DesignToolingHandoffItem {
        id: "live-preview-plugin",
        title: "Live preview and token data editing",
        artifact_type: "future-integration",
        path_or_command: "external design-tool plugin or live preview bridge",
        status: DesignToolingHandoffStatus::ExternalGate,
        release_evidence: "Token CLI and component-lab previews provide static handoff but no live bidirectional design-tool session.",
        remaining_gap: "implement Figma/live-preview bridge if Slint-level workflow parity is a release promise",
    },
];

/// Export built-in `DesignSystem` presets as design tokens.
///
/// The default form is pretty-printed JSON. Use
/// [`export_design_tokens_with_options`] with
/// [`DesignTokenExportOptions::compact`] for bulk exports.
pub fn export_design_tokens(format: DesignTokenFormat) -> Result<String> {
    export_design_tokens_with_options(format, DesignTokenExportOptions::pretty())
}

/// Export built-in `DesignSystem` presets with explicit output options.
pub fn export_design_tokens_with_options(
    format: DesignTokenFormat,
    options: DesignTokenExportOptions,
) -> Result<String> {
    let context = "serialize design token export";
    match format {
        DesignTokenFormat::StyleDictionaryJson => {
            let export = DesignTokenExport::for_all_presets();
            if options.compact {
                serde_json::to_string(&export).context(context)
            } else {
                serde_json::to_string_pretty(&export).context(context)
            }
        }
        DesignTokenFormat::W3cDtcgJson => {
            let value = dtcg_export_value();
            if options.compact {
                serde_json::to_string(&value).context(context)
            } else {
                serde_json::to_string_pretty(&value).context(context)
            }
        }
    }
}

/// Import and validate a design token document.
pub fn import_design_tokens(
    input: &str,
    format: DesignTokenFormat,
) -> Result<ImportedDesignTokens> {
    let raw: Value = serde_json::from_str(input).context("parse design token JSON")?;
    let (preset_count, token_count, findings) = match format {
        DesignTokenFormat::StyleDictionaryJson => inspect_token_value(&raw),
        DesignTokenFormat::W3cDtcgJson => inspect_dtcg_value(&raw),
    };
    if !findings.is_empty() {
        bail!("invalid design token JSON: {}", findings.join("; "));
    }
    Ok(ImportedDesignTokens {
        preset_count,
        token_count,
        raw,
    })
}

/// Validate a token document and current design conformance matrix.
///
/// Set `render_markdown` to `false` when only JSON output is needed to avoid the
/// cost of building the markdown table.
pub fn validate_design_tokens(
    input: &str,
    format: DesignTokenFormat,
    render_markdown: bool,
) -> Result<DesignTokenValidationReport> {
    let raw: Value = serde_json::from_str(input).context("parse design token JSON")?;
    let (preset_count, token_count, findings) = match format {
        DesignTokenFormat::StyleDictionaryJson => inspect_token_value(&raw),
        DesignTokenFormat::W3cDtcgJson => inspect_dtcg_value(&raw),
    };
    validate_with_counts(preset_count, token_count, findings, render_markdown)
}

fn validate_with_counts(
    preset_count: usize,
    token_count: usize,
    mut findings: Vec<Cow<'static, str>>,
    render_markdown: bool,
) -> Result<DesignTokenValidationReport> {
    let matrix = DesignConformanceMatrix::all_presets();
    for (case, finding) in matrix.findings() {
        findings.push(Cow::Owned(format!(
            "{}:{}:{}",
            case.preset_id,
            case.motion_label(),
            finding.id
        )));
    }

    let conformance_markdown = if render_markdown {
        matrix.to_markdown_table()
    } else {
        String::new()
    };

    Ok(DesignTokenValidationReport {
        schema_version: DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION,
        report_type: Cow::Borrowed(DESIGN_TOKEN_VALIDATION_REPORT_TYPE),
        passed: findings.is_empty(),
        findings,
        preset_count,
        token_count,
        conformance_markdown,
    })
}

/// Export tokens to a path.
pub fn export_design_tokens_to_path(path: &Path, format: DesignTokenFormat) -> Result<()> {
    export_design_tokens_to_path_with_options(
        path,
        format,
        DesignTokenExportOptions::pretty(),
        true,
    )
}

/// Export tokens to a path with explicit output options.
///
/// `durable` controls whether the temporary file is fsynced before the atomic
/// rename; pass `false` for bulk exports where throughput matters more than
/// crash durability.
pub fn export_design_tokens_to_path_with_options(
    path: &Path,
    format: DesignTokenFormat,
    options: DesignTokenExportOptions,
    durable: bool,
) -> Result<()> {
    let output = export_design_tokens_with_options(format, options)?;
    write_text_atomically_with_durability(path, output, durable)
}

/// Write a text report through a same-directory temporary file before replacing
/// the destination, so an interrupted write cannot leave a truncated report.
pub fn write_text_atomically(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    write_text_atomically_with_durability(path, contents, true)
}

/// Same as [`write_text_atomically`], but `durable` gates the `sync_all()` call.
///
/// Pass `durable = false` for bulk exports where throughput matters more than
/// crash durability; the rename is still atomic, so readers never observe a
/// truncated file. Release artifacts should keep `durable = true`.
pub fn write_text_atomically_with_durability(
    path: &Path,
    contents: impl AsRef<[u8]>,
    durable: bool,
) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;

    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create temporary file next to {}", path.display()))?;
    temporary
        .write_all(contents.as_ref())
        .with_context(|| format!("write temporary file for {}", path.display()))?;
    if durable {
        temporary
            .as_file()
            .sync_all()
            .with_context(|| format!("sync temporary file for {}", path.display()))?;
    }
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

/// Import tokens from a path.
pub fn import_design_tokens_from_path(
    path: &Path,
    format: DesignTokenFormat,
) -> Result<ImportedDesignTokens> {
    let input =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    import_design_tokens(&input, format)
}

/// Validate tokens from a path.
pub fn validate_design_tokens_from_path(
    path: &Path,
    format: DesignTokenFormat,
    render_markdown: bool,
) -> Result<DesignTokenValidationReport> {
    let input =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    validate_design_tokens(&input, format, render_markdown)
}

fn inspect_token_value(raw: &Value) -> (usize, usize, Vec<Cow<'static, str>>) {
    let mut findings: Vec<Cow<'static, str>> = Vec::new();
    let Some(presets) = raw.get("presets").and_then(Value::as_array) else {
        return (0, 0, vec![Cow::Borrowed("root.presets must be an array")]);
    };

    let mut token_count = 0usize;
    for (preset_index, preset) in presets.iter().enumerate() {
        if preset.get("preset_id").and_then(Value::as_str).is_none() {
            findings.push(Cow::Owned(format!(
                "presets[{preset_index}].preset_id must be a string"
            )));
        }
        let Some(tokens) = preset.get("tokens").and_then(Value::as_array) else {
            findings.push(Cow::Owned(format!(
                "presets[{preset_index}].tokens must be an array"
            )));
            continue;
        };
        token_count += tokens.len();
        // Single borrowed pass per token: fully valid tokens allocate nothing
        // and each invalid token formats its prefix at most once.
        for (token_index, token) in tokens.iter().enumerate() {
            let name_ok = token.get("name").and_then(Value::as_str).is_some();
            let path_ok = token
                .get("path")
                .and_then(Value::as_array)
                .is_some_and(|path| path.iter().all(Value::is_string));
            let value_ok = token.get("value").and_then(Value::as_str).is_some();
            let type_ok = token.get("token_type").and_then(Value::as_str).is_some();
            if name_ok && path_ok && value_ok && type_ok {
                continue;
            }
            let prefix = format!("presets[{preset_index}].tokens[{token_index}]");
            if !name_ok {
                findings.push(Cow::Owned(format!("{prefix}.name must be a string")));
            }
            if !path_ok {
                findings.push(Cow::Owned(format!(
                    "{prefix}.path must be an array of strings"
                )));
            }
            if !value_ok {
                findings.push(Cow::Owned(format!("{prefix}.value must be a string")));
            }
            if !type_ok {
                findings.push(Cow::Owned(format!("{prefix}.token_type must be a string")));
            }
        }
    }

    if presets.is_empty() {
        findings.push(Cow::Borrowed("root.presets must not be empty"));
    }
    if token_count == 0 {
        findings.push(Cow::Borrowed(
            "token export must contain at least one token",
        ));
    }

    (presets.len(), token_count, findings)
}

/// Export current tokens and validate them with the conformance matrix.
///
/// Set `render_markdown` to `false` when only JSON output is needed.
pub fn validate_current_design_tokens(
    render_markdown: bool,
) -> Result<DesignTokenValidationReport> {
    let export = DesignTokenExport::for_all_presets();
    validate_design_token_export(&export, render_markdown)
}

fn validate_design_token_export(
    export: &DesignTokenExport,
    render_markdown: bool,
) -> Result<DesignTokenValidationReport> {
    let preset_count = export.presets.len();
    let token_count = export.presets.iter().map(|p| p.tokens.len()).sum();
    let mut findings: Vec<Cow<'static, str>> = Vec::new();
    if preset_count == 0 {
        findings.push(Cow::Borrowed("root.presets must not be empty"));
    }
    if token_count == 0 {
        findings.push(Cow::Borrowed(
            "token export must contain at least one token",
        ));
    }

    let matrix = DesignConformanceMatrix::all_presets();
    for (case, finding) in matrix.findings() {
        findings.push(Cow::Owned(format!(
            "{}:{}:{}",
            case.preset_id,
            case.motion_label(),
            finding.id
        )));
    }

    let conformance_markdown = if render_markdown {
        matrix.to_markdown_table()
    } else {
        String::new()
    };

    Ok(DesignTokenValidationReport {
        schema_version: DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION,
        report_type: Cow::Borrowed(DESIGN_TOKEN_VALIDATION_REPORT_TYPE),
        passed: findings.is_empty(),
        findings,
        preset_count,
        token_count,
        conformance_markdown,
    })
}

/// Build the W3C DTCG JSON value for the built-in presets.
///
/// Presets become top-level groups, each token path becomes nested groups, and
/// each leaf carries `$value`/`$type`. A node that is both a leaf and a group
/// keeps its `$value`/`$type` alongside child groups so no token is lost.
fn dtcg_export_value() -> Value {
    let export = DesignTokenExport::for_all_presets();
    let mut root = serde_json::Map::new();
    for preset in &export.presets {
        let group = root
            .entry(preset.preset_id.to_owned())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        let group = group.as_object_mut().expect("preset group is an object");
        for token in &preset.tokens {
            insert_dtcg_leaf(group, &token.path, &token.value, token.token_type);
        }
    }
    Value::Object(root)
}

fn insert_dtcg_leaf(
    group: &mut serde_json::Map<String, Value>,
    path: &[&str],
    value: &str,
    token_type: &str,
) {
    let mut current = group;
    for segment in path {
        let next = current
            .entry((*segment).to_owned())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if !next.is_object() {
            *next = Value::Object(serde_json::Map::new());
        }
        current = next.as_object_mut().expect("DTCG group is an object");
    }
    current.insert("$value".to_owned(), Value::String(value.to_owned()));
    current.insert("$type".to_owned(), Value::String(token_type.to_owned()));
}

/// Single-pass borrowed validation for W3C DTCG JSON documents.
fn inspect_dtcg_value(raw: &Value) -> (usize, usize, Vec<Cow<'static, str>>) {
    let Some(root) = raw.as_object() else {
        return (
            0,
            0,
            vec![Cow::Borrowed("root must be a JSON object of preset groups")],
        );
    };
    if root.is_empty() {
        return (
            0,
            0,
            vec![
                Cow::Borrowed("root must contain at least one preset group"),
                Cow::Borrowed("token export must contain at least one token"),
            ],
        );
    }
    let mut findings = Vec::new();
    let mut token_count = 0usize;
    for (preset_id, group) in root {
        inspect_dtcg_node(group, preset_id.clone(), &mut token_count, &mut findings);
    }
    if token_count == 0 {
        findings.push(Cow::Borrowed(
            "token export must contain at least one token",
        ));
    }
    (root.len(), token_count, findings)
}

fn inspect_dtcg_node(
    node: &Value,
    path: String,
    token_count: &mut usize,
    findings: &mut Vec<Cow<'static, str>>,
) {
    let Some(object) = node.as_object() else {
        findings.push(Cow::Owned(format!("{path} must be a token group object")));
        return;
    };
    if let Some(value) = object.get("$value") {
        *token_count += 1;
        if value.as_str().is_none() {
            findings.push(Cow::Owned(format!("{path}.$value must be a string")));
        }
        if object.get("$type").and_then(Value::as_str).is_none() {
            findings.push(Cow::Owned(format!("{path}.$type must be a string")));
        }
    }
    for (key, child) in object {
        if key.starts_with('$') {
            continue;
        }
        inspect_dtcg_node(child, format!("{path}.{key}"), token_count, findings);
    }
}

/// One token whose value or type changed between two token documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignTokenValueChange {
    pub id: String,
    pub old_value: String,
    pub new_value: String,
    pub old_type: String,
    pub new_type: String,
}

/// Additive/breaking diff between two token documents.
///
/// `added` tokens are safe for downstream consumers, while `removed` tokens
/// and `changed` values/types are breaking.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DesignTokenDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<DesignTokenValueChange>,
}

impl DesignTokenDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// Removed tokens and changed values/types break downstream Style
    /// Dictionary / DTCG consumers; purely additive diffs are safe.
    pub fn is_breaking(&self) -> bool {
        !self.removed.is_empty() || !self.changed.is_empty()
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Design Token Diff\n\n");
        push_markdown(
            &mut output,
            format_args!(
                "- added: {}\n- removed: {}\n- changed: {}\n- breaking: {}\n\n",
                self.added.len(),
                self.removed.len(),
                self.changed.len(),
                self.is_breaking(),
            ),
        );
        for section in [("Added", &self.added), ("Removed", &self.removed)] {
            push_markdown(&mut output, format_args!("## {}\n\n", section.0));
            if section.1.is_empty() {
                output.push_str("(none)\n\n");
            } else {
                for id in section.1 {
                    push_markdown(&mut output, format_args!("- `{id}`\n"));
                }
                output.push('\n');
            }
        }
        output.push_str("## Changed\n\n");
        if self.changed.is_empty() {
            output.push_str("(none)\n\n");
        } else {
            for change in &self.changed {
                push_markdown(
                    &mut output,
                    format_args!(
                        "- `{}`: `{}` -> `{}` (type `{}` -> `{}`)\n",
                        change.id,
                        change.old_value,
                        change.new_value,
                        change.old_type,
                        change.new_type,
                    ),
                );
            }
            output.push('\n');
        }
        output
    }
}

/// Diff two token documents in the same wire format.
///
/// Returns the additive/breaking change set; use
/// [`DesignTokenDiff::is_breaking`] to gate releases.
pub fn diff_design_tokens(
    old: &str,
    new: &str,
    format: DesignTokenFormat,
) -> Result<DesignTokenDiff> {
    let old_raw: Value = serde_json::from_str(old).context("parse old design token JSON")?;
    let new_raw: Value = serde_json::from_str(new).context("parse new design token JSON")?;
    let (old_map, new_map) = match format {
        DesignTokenFormat::StyleDictionaryJson => (
            flatten_style_dictionary(&old_raw)?,
            flatten_style_dictionary(&new_raw)?,
        ),
        DesignTokenFormat::W3cDtcgJson => (flatten_dtcg(&old_raw)?, flatten_dtcg(&new_raw)?),
    };
    Ok(diff_token_maps(&old_map, &new_map))
}

/// Diff two token files in the same wire format.
pub fn diff_design_tokens_from_paths(
    old: &Path,
    new: &Path,
    format: DesignTokenFormat,
) -> Result<DesignTokenDiff> {
    let old_input =
        std::fs::read_to_string(old).with_context(|| format!("read {}", old.display()))?;
    let new_input =
        std::fs::read_to_string(new).with_context(|| format!("read {}", new.display()))?;
    diff_design_tokens(&old_input, &new_input, format)
}

/// Flatten a Style Dictionary document to `preset/name -> (value, type)`.
fn flatten_style_dictionary(raw: &Value) -> Result<BTreeMap<String, (String, String)>> {
    let presets = raw
        .get("presets")
        .and_then(Value::as_array)
        .context("diff requires Style Dictionary JSON with a root.presets array")?;
    let mut map = BTreeMap::new();
    for preset in presets {
        let preset_id = preset
            .get("preset_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let tokens = preset
            .get("tokens")
            .and_then(Value::as_array)
            .with_context(|| {
                format!("diff requires preset '{preset_id}' to have a tokens array")
            })?;
        for token in tokens {
            let name = token.get("name").and_then(Value::as_str).with_context(|| {
                format!("diff requires every token in preset '{preset_id}' to have a string name")
            })?;
            let value = token
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let token_type = token
                .get("token_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            map.insert(
                format!("{preset_id}/{name}"),
                (value.to_owned(), token_type.to_owned()),
            );
        }
    }
    Ok(map)
}

/// Flatten a W3C DTCG document to `preset/dotted.path -> (value, type)`.
fn flatten_dtcg(raw: &Value) -> Result<BTreeMap<String, (String, String)>> {
    let root = raw
        .as_object()
        .context("diff requires a W3C DTCG JSON object of preset groups")?;
    let mut map = BTreeMap::new();
    for (preset_id, group) in root {
        flatten_dtcg_node(group, preset_id, &mut map)?;
    }
    Ok(map)
}

fn flatten_dtcg_node(
    node: &Value,
    path: &str,
    map: &mut BTreeMap<String, (String, String)>,
) -> Result<()> {
    let object = node
        .as_object()
        .with_context(|| format!("diff requires DTCG group '{path}' to be an object"))?;
    if let Some(value) = object.get("$value") {
        let value = value
            .as_str()
            .with_context(|| format!("diff requires DTCG leaf '{path}.$value' to be a string"))?;
        let token_type = object
            .get("$type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        map.insert(path.to_owned(), (value.to_owned(), token_type.to_owned()));
    }
    for (key, child) in object {
        if key.starts_with('$') {
            continue;
        }
        flatten_dtcg_node(child, &format!("{path}.{key}"), map)?;
    }
    Ok(())
}

fn diff_token_maps(
    old: &BTreeMap<String, (String, String)>,
    new: &BTreeMap<String, (String, String)>,
) -> DesignTokenDiff {
    let mut diff = DesignTokenDiff::default();
    for (id, (old_value, old_type)) in old {
        match new.get(id) {
            None => diff.removed.push(id.clone()),
            Some((new_value, new_type)) if new_value != old_value || new_type != old_type => {
                diff.changed.push(DesignTokenValueChange {
                    id: id.clone(),
                    old_value: old_value.clone(),
                    new_value: new_value.clone(),
                    old_type: old_type.clone(),
                    new_type: new_type.clone(),
                });
            }
            Some(_) => {}
        }
    }
    for id in new.keys() {
        if !old.contains_key(id) {
            diff.added.push(id.clone());
        }
    }
    diff
}

pub fn ensure_passed(report: &DesignTokenValidationReport) -> Result<()> {
    if report.passed {
        Ok(())
    } else {
        Err(anyhow!(
            "design token validation failed: {}",
            report.findings.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_tokens_round_trip() {
        let json = export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap();
        let imported = import_design_tokens(&json, DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert!(imported.preset_count > 0);
        assert!(imported.token_count >= imported.preset_count);
    }

    #[test]
    fn validation_reports_bad_shape() {
        let report =
            validate_design_tokens("{}", DesignTokenFormat::StyleDictionaryJson, false).unwrap();
        assert!(!report.passed);
        assert!(report.findings.iter().any(|f| f.contains("root.presets")));
    }

    #[test]
    fn validation_skips_markdown_when_not_requested() {
        let report =
            validate_design_tokens("{}", DesignTokenFormat::StyleDictionaryJson, false).unwrap();
        assert!(report.conformance_markdown.is_empty());
    }

    #[test]
    fn validation_renders_markdown_when_requested() {
        let report = validate_design_tokens(
            r#"{"presets": [{"preset_id": "test", "tokens": []}]}"#,
            DesignTokenFormat::StyleDictionaryJson,
            true,
        )
        .unwrap();
        assert!(!report.conformance_markdown.is_empty());
    }

    #[test]
    fn current_tokens_validate() {
        let report = validate_current_design_tokens(true).unwrap();
        assert!(report.preset_count > 0);
        assert!(report.token_count >= report.preset_count);
        assert!(!report.conformance_markdown.is_empty());
    }

    #[test]
    fn current_tokens_validate_skips_markdown() {
        let report = validate_current_design_tokens(false).unwrap();
        assert!(report.preset_count > 0);
        assert!(report.conformance_markdown.is_empty());
    }

    #[test]
    fn validate_from_path_round_trip() {
        let json = export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap();
        let path = std::env::temp_dir().join(format!(
            "gpui-design-tools-test-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, &json).unwrap();
        let report =
            validate_design_tokens_from_path(&path, DesignTokenFormat::StyleDictionaryJson, true)
                .unwrap();
        assert!(report.preset_count > 0);
        assert!(report.token_count >= report.preset_count);
        assert!(!report.conformance_markdown.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn inspect_token_value_lazy_prefix() {
        let raw = serde_json::from_str(
            r#"{"presets": [{"preset_id": "test", "tokens": [{"name": "ok", "path": [], "value": "v", "token_type": "t"}]}]}"#,
        )
        .unwrap();
        let (presets, tokens, findings) = inspect_token_value(&raw);
        assert_eq!(presets, 1);
        assert_eq!(tokens, 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn ensure_passed_respects_report_status() {
        let passing = DesignTokenValidationReport {
            schema_version: DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION,
            report_type: Cow::Borrowed(DESIGN_TOKEN_VALIDATION_REPORT_TYPE),
            passed: true,
            findings: Vec::new(),
            preset_count: 1,
            token_count: 1,
            conformance_markdown: String::new(),
        };
        assert!(ensure_passed(&passing).is_ok());

        let failing = DesignTokenValidationReport {
            schema_version: DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION,
            report_type: Cow::Borrowed(DESIGN_TOKEN_VALIDATION_REPORT_TYPE),
            passed: false,
            findings: vec![Cow::Borrowed("bad")],
            preset_count: 0,
            token_count: 0,
            conformance_markdown: String::new(),
        };
        assert!(ensure_passed(&failing).is_err());
    }

    #[test]
    fn validation_report_json_contract_is_stable() {
        let report =
            validate_design_tokens("{}", DesignTokenFormat::StyleDictionaryJson, false).unwrap();
        let json = serde_json::to_value(&report).unwrap();
        let object = json.as_object().unwrap();
        let keys: std::collections::BTreeSet<_> = object.keys().map(String::as_str).collect();

        assert_eq!(
            keys,
            [
                "schema_version",
                "report_type",
                "passed",
                "findings",
                "preset_count",
                "token_count",
                "conformance_markdown",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            json["schema_version"].as_u64(),
            Some(DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION as u64)
        );
        assert_eq!(
            json["report_type"].as_str(),
            Some(DESIGN_TOKEN_VALIDATION_REPORT_TYPE)
        );
        assert_eq!(json["passed"].as_bool(), Some(false));
        assert!(!json["findings"].as_array().unwrap().is_empty());
        assert_eq!(json["preset_count"].as_u64(), Some(0));
        assert_eq!(json["token_count"].as_u64(), Some(0));
        assert_eq!(json["conformance_markdown"].as_str(), Some(""));
    }

    #[test]
    fn design_tooling_handoff_report_has_stable_contract() {
        let report = design_tooling_handoff_report();

        assert_eq!(
            report.schema_version,
            DESIGN_TOOLING_HANDOFF_REPORT_SCHEMA_VERSION
        );
        assert_eq!(report.report_type, DESIGN_TOOLING_HANDOFF_REPORT_TYPE);
        assert_eq!(report.crate_name, "gpui-design-tools");
        assert!(report.items.len() >= 6);
        assert!(report.item("token-export").is_some());
        assert!(report.item("figma-code-connect").is_some());

        let mut ids = std::collections::HashSet::new();
        assert!(report.items.iter().all(|item| ids.insert(item.id)));
    }

    #[test]
    fn design_tooling_handoff_report_marks_local_and_external_gates() {
        let report = design_tooling_handoff_report();

        for id in [
            "token-export",
            "token-import-validation",
            "conformance-report",
            "figma-code-connect",
        ] {
            let item = report.item(id).expect("local handoff item should exist");
            assert_eq!(item.status, DesignToolingHandoffStatus::Implemented);
            assert!(!item.is_release_blocking());
        }

        let preview = report
            .item("component-lab-preview")
            .expect("component-lab preview row should exist");
        assert_eq!(preview.status, DesignToolingHandoffStatus::Documented);
        assert!(!preview.is_release_blocking());

        let blocking_ids: std::collections::BTreeSet<_> = report
            .blocking_entries()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(blocking_ids, ["live-preview-plugin"].into_iter().collect());
    }

    #[test]
    fn design_tooling_handoff_markdown_is_release_attachable() {
        let markdown = design_tooling_handoff_report().to_markdown();

        assert!(markdown.contains(DESIGN_TOOLING_HANDOFF_REPORT_TYPE));
        assert!(markdown.contains("token-export"));
        assert!(markdown.contains("figma-code-connect"));
        assert!(markdown.contains("figma/CODE_CONNECT_MAPPINGS.md"));
        assert!(markdown.contains("external-gate"));
        assert!(markdown.contains("gpui-component-lab"));
    }

    #[test]
    fn figma_code_connect_mapping_artifact_is_present() {
        let mapping = include_str!("../../../figma/CODE_CONNECT_MAPPINGS.md");

        assert!(mapping.contains("gpui-toolkit-figma-code-connect-mappings"));
        assert!(mapping.contains("gpui_ui_kit::{Button, IconButton}"));
        assert!(mapping.contains("gpui_design::DesignTokenExport"));
        assert!(mapping.contains("ui_kit_visual_regression_manifest"));
        assert!(mapping.contains("chart_visual_regression_manifest"));
        assert!(mapping.contains("separate gates"));
    }

    #[test]
    fn reviewed_figma_examples_keep_current_component_apis_and_paths() {
        // GitHub's Windows checkout may present these tracked Markdown files
        // with CRLF line endings.  The snippets below intentionally include
        // newlines, so compare their normalized text rather than making the
        // documentation contract host-line-ending dependent.
        let rules = include_str!("../../figma/DESIGN_SYSTEM_RULES.md").replace("\r\n", "\n");
        let mappings = include_str!("../../figma/CODE_CONNECT_MAPPINGS.md").replace("\r\n", "\n");

        assert!(rules.contains("Toggle::new(\"enable-toggle\")\n    .checked(is_checked)"));
        assert!(mappings.contains("Toggle::new(\"id\")\n    .checked(checked)"));
        assert!(mappings.contains("Select::new(\"id\").options(vec!["));
        assert!(rules.contains("crates/gpui-ui-kit/"));
        assert!(!rules.contains("crates/gpui-toolkit/"));
        assert!(!rules.contains("crates/gpui-icons/"));
    }

    #[test]
    fn design_tooling_handoff_json_contract_is_stable() {
        let report = design_tooling_handoff_report();
        let json = serde_json::to_value(&report).unwrap();
        let object = json.as_object().unwrap();
        let keys: std::collections::BTreeSet<_> = object.keys().map(String::as_str).collect();

        assert_eq!(
            keys,
            [
                "schema_version",
                "report_type",
                "crate_name",
                "crate_version",
                "items",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            json["schema_version"].as_u64(),
            Some(DESIGN_TOOLING_HANDOFF_REPORT_SCHEMA_VERSION as u64)
        );
        assert_eq!(
            json["report_type"].as_str(),
            Some(DESIGN_TOOLING_HANDOFF_REPORT_TYPE)
        );
        assert_eq!(json["items"][0]["status"].as_str(), Some("implemented"));
        assert!(
            json["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["status"].as_str() == Some("external-gate"))
        );
    }

    #[test]
    fn format_parse_accepts_aliases() {
        for value in ["json", "style-dictionary-json", "style_dictionary_json"] {
            assert_eq!(
                DesignTokenFormat::parse(value).unwrap(),
                DesignTokenFormat::StyleDictionaryJson
            );
        }
    }

    #[test]
    fn format_parse_rejects_unknown() {
        let err = DesignTokenFormat::parse("yaml").unwrap_err();
        assert!(err.to_string().contains("unsupported design token format"));
    }

    #[test]
    fn import_design_tokens_reports_invalid_shape() {
        let err = import_design_tokens("{}", DesignTokenFormat::StyleDictionaryJson).unwrap_err();
        assert!(err.to_string().contains("root.presets"));
    }

    #[test]
    fn export_design_tokens_to_path_round_trip() {
        let path = std::env::temp_dir().join(format!(
            "gpui-design-tools-export-test-{}.json",
            std::process::id()
        ));
        export_design_tokens_to_path(&path, DesignTokenFormat::StyleDictionaryJson).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("presets"));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn export_design_tokens_to_path_creates_missing_parent_directories() {
        let root = std::env::temp_dir().join(format!(
            "gpui-design-tools-export-parent-test-{}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("nested/gpui-tokens.json");

        export_design_tokens_to_path(&path, DesignTokenFormat::StyleDictionaryJson).unwrap();

        assert!(path.is_file());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn export_design_tokens_to_path_replaces_existing_output() {
        let root = std::env::temp_dir().join(format!(
            "gpui-design-tools-export-replace-test-{}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("gpui-tokens.json");
        std::fs::write(&path, "incomplete output").unwrap();

        export_design_tokens_to_path(&path, DesignTokenFormat::StyleDictionaryJson).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("presets"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_design_tokens_from_path_round_trip() {
        let json = export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap();
        let path = std::env::temp_dir().join(format!(
            "gpui-design-tools-import-test-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, &json).unwrap();
        let imported =
            import_design_tokens_from_path(&path, DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert!(imported.preset_count > 0);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn import_design_tokens_from_path_missing_file() {
        let result = import_design_tokens_from_path(
            Path::new("/nonexistent/gpui-design-tools-test.json"),
            DesignTokenFormat::StyleDictionaryJson,
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_design_tokens_from_path_missing_file() {
        let result = validate_design_tokens_from_path(
            Path::new("/nonexistent/gpui-design-tools-test.json"),
            DesignTokenFormat::StyleDictionaryJson,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn inspect_token_value_presets_not_array() {
        let raw = serde_json::json!({"presets": "nope"});
        let (presets, tokens, findings) = inspect_token_value(&raw);
        assert_eq!(presets, 0);
        assert_eq!(tokens, 0);
        assert!(findings.iter().any(|f| f.contains("root.presets")));
    }

    #[test]
    fn inspect_token_value_empty_presets() {
        let raw = serde_json::json!({"presets": []});
        let (presets, tokens, findings) = inspect_token_value(&raw);
        assert_eq!(presets, 0);
        assert_eq!(tokens, 0);
        assert!(findings.iter().any(|f| f.contains("must not be empty")));
        assert!(findings.iter().any(|f| f.contains("at least one token")));
    }

    #[test]
    fn inspect_token_value_missing_preset_id() {
        let raw = serde_json::json!({
            "presets": [{"tokens": [{"name": "n", "path": [], "value": "v", "token_type": "t"}]}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("preset_id must be a string"))
        );
    }

    #[test]
    fn inspect_token_value_tokens_not_array() {
        let raw = serde_json::json!({
            "presets": [{"preset_id": "test", "tokens": "nope"}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("tokens must be an array"))
        );
        assert!(findings.iter().any(|f| f.contains("at least one token")));
    }

    #[test]
    fn inspect_token_value_missing_name() {
        let raw = serde_json::json!({
            "presets": [{"preset_id": "test", "tokens": [{"path": [], "value": "v", "token_type": "t"}]}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        assert!(findings.iter().any(|f| f.contains("name must be a string")));
    }

    #[test]
    fn inspect_token_value_missing_path() {
        let raw = serde_json::json!({
            "presets": [{"preset_id": "test", "tokens": [{"name": "n", "value": "v", "token_type": "t"}]}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        assert!(findings.iter().any(|f| f.contains("path must be an array")));
    }

    #[test]
    fn inspect_token_value_rejects_non_string_path_segments() {
        let raw = serde_json::json!({
            "presets": [{
                "preset_id": "test",
                "tokens": [{
                    "name": "color.primary",
                    "path": ["color", 1],
                    "value": "#ffffff",
                    "token_type": "color"
                }]
            }]
        });

        let (_, _, findings) = inspect_token_value(&raw);

        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("path must be an array of strings"))
        );
    }

    #[test]
    fn inspect_token_value_missing_value() {
        let raw = serde_json::json!({
            "presets": [{"preset_id": "test", "tokens": [{"name": "n", "path": [], "token_type": "t"}]}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("value must be a string"))
        );
    }

    #[test]
    fn inspect_token_value_missing_token_type() {
        let raw = serde_json::json!({
            "presets": [{"preset_id": "test", "tokens": [{"name": "n", "path": [], "value": "v"}]}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("token_type must be a string"))
        );
    }

    #[test]
    fn inspect_token_value_lazy_prefix_reused() {
        let raw = serde_json::json!({
            "presets": [{"preset_id": "test", "tokens": [{"name": 1, "path": 2, "value": 3, "token_type": 4}]}]
        });
        let (_, _, findings) = inspect_token_value(&raw);
        let prefix = "presets[0].tokens[0]";
        assert!(
            findings
                .iter()
                .any(|f| f.contains(&format!("{prefix}.name must be a string")))
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains(&format!("{prefix}.path must be an array")))
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains(&format!("{prefix}.value must be a string")))
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains(&format!("{prefix}.token_type must be a string")))
        );
    }

    #[test]
    fn format_parse_accepts_dtcg_aliases() {
        for value in ["dtcg", "w3c-dtcg", "w3c-dtcg-json", "w3c_dtcg_json"] {
            assert_eq!(
                DesignTokenFormat::parse(value).unwrap(),
                DesignTokenFormat::W3cDtcgJson
            );
        }
        assert_eq!(DesignTokenFormat::W3cDtcgJson.as_str(), "w3c-dtcg-json");
        assert_eq!(
            DesignTokenFormat::StyleDictionaryJson.as_str(),
            "style-dictionary-json"
        );
    }

    #[test]
    fn compact_export_matches_pretty_content_and_is_smaller() {
        for format in [
            DesignTokenFormat::StyleDictionaryJson,
            DesignTokenFormat::W3cDtcgJson,
        ] {
            let pretty =
                export_design_tokens_with_options(format, DesignTokenExportOptions::pretty())
                    .unwrap();
            let compact =
                export_design_tokens_with_options(format, DesignTokenExportOptions::compact())
                    .unwrap();
            assert!(compact.len() < pretty.len(), "compact should be smaller");
            assert!(!compact.contains('\n'));
            let pretty_value: Value = serde_json::from_str(&pretty).unwrap();
            let compact_value: Value = serde_json::from_str(&compact).unwrap();
            assert_eq!(pretty_value, compact_value);
        }
    }

    #[test]
    fn dtcg_export_import_round_trip_matches_preset_counts() {
        let style = import_design_tokens(
            &export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap(),
            DesignTokenFormat::StyleDictionaryJson,
        )
        .unwrap();
        let dtcg = import_design_tokens(
            &export_design_tokens(DesignTokenFormat::W3cDtcgJson).unwrap(),
            DesignTokenFormat::W3cDtcgJson,
        )
        .unwrap();
        assert_eq!(dtcg.preset_count, style.preset_count);
        assert_eq!(dtcg.token_count, style.token_count);

        let compact = export_design_tokens_with_options(
            DesignTokenFormat::W3cDtcgJson,
            DesignTokenExportOptions::compact(),
        )
        .unwrap();
        let imported = import_design_tokens(&compact, DesignTokenFormat::W3cDtcgJson).unwrap();
        assert_eq!(imported.token_count, style.token_count);
    }

    #[test]
    fn dtcg_export_uses_value_and_type_leaves() {
        let json = export_design_tokens(DesignTokenFormat::W3cDtcgJson).unwrap();
        let raw: Value = serde_json::from_str(&json).unwrap();
        let root = raw.as_object().unwrap();
        assert!(!root.is_empty());
        assert!(json.contains("$value"));
        assert!(json.contains("$type"));
        assert!(!json.contains("token_type"));
    }

    #[test]
    fn dtcg_validation_reports_bad_shapes() {
        let empty = validate_design_tokens("{}", DesignTokenFormat::W3cDtcgJson, false).unwrap();
        assert!(!empty.passed);
        assert!(empty.findings.iter().any(|f| f.contains("preset group")));
        assert!(
            empty
                .findings
                .iter()
                .any(|f| f.contains("at least one token"))
        );

        let array = validate_design_tokens("[]", DesignTokenFormat::W3cDtcgJson, false).unwrap();
        assert!(!array.passed);
        assert!(array.findings.iter().any(|f| f.contains("preset groups")));

        let bad_leaf = validate_design_tokens(
            r#"{"a": {"color": {"$value": 1}}}"#,
            DesignTokenFormat::W3cDtcgJson,
            false,
        )
        .unwrap();
        assert!(!bad_leaf.passed);
        assert!(
            bad_leaf
                .findings
                .iter()
                .any(|f| f.contains("$value must be a string"))
        );
        assert!(
            bad_leaf
                .findings
                .iter()
                .any(|f| f.contains("$type must be a string"))
        );

        let bad_group =
            validate_design_tokens(r#"{"a": 1}"#, DesignTokenFormat::W3cDtcgJson, false).unwrap();
        assert!(!bad_group.passed);
        assert!(
            bad_group
                .findings
                .iter()
                .any(|f| f.contains("must be a token group object"))
        );

        let err = import_design_tokens("{}", DesignTokenFormat::W3cDtcgJson).unwrap_err();
        assert!(err.to_string().contains("preset group"));
    }

    #[test]
    fn dtcg_validate_from_path_round_trip() {
        let json = export_design_tokens(DesignTokenFormat::W3cDtcgJson).unwrap();
        let path = std::env::temp_dir().join(format!(
            "gpui-design-tools-dtcg-validate-test-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, &json).unwrap();
        let report =
            validate_design_tokens_from_path(&path, DesignTokenFormat::W3cDtcgJson, false).unwrap();
        assert!(report.preset_count > 0);
        assert!(report.token_count >= report.preset_count);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn diff_is_empty_for_identical_documents() {
        let json = export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap();
        let diff =
            diff_design_tokens(&json, &json, DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert!(diff.is_empty());
        assert!(!diff.is_breaking());

        let dtcg = export_design_tokens(DesignTokenFormat::W3cDtcgJson).unwrap();
        let dtcg_diff = diff_design_tokens(&dtcg, &dtcg, DesignTokenFormat::W3cDtcgJson).unwrap();
        assert!(dtcg_diff.is_empty());
    }

    #[test]
    fn diff_reports_added_removed_and_changed() {
        let old = r#"{"presets": [
            {"preset_id": "a", "tokens": [
                {"name": "keep", "path": ["keep"], "value": "1", "token_type": "string"},
                {"name": "gone", "path": ["gone"], "value": "x", "token_type": "string"},
                {"name": "tweak", "path": ["tweak"], "value": "old", "token_type": "color"}
            ]}
        ]}"#;
        let new = r#"{"presets": [
            {"preset_id": "a", "tokens": [
                {"name": "keep", "path": ["keep"], "value": "1", "token_type": "string"},
                {"name": "fresh", "path": ["fresh"], "value": "2", "token_type": "string"},
                {"name": "tweak", "path": ["tweak"], "value": "new", "token_type": "color"}
            ]}
        ]}"#;
        let diff = diff_design_tokens(old, new, DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert_eq!(diff.added, vec!["a/fresh".to_owned()]);
        assert_eq!(diff.removed, vec!["a/gone".to_owned()]);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].id, "a/tweak");
        assert_eq!(diff.changed[0].old_value, "old");
        assert_eq!(diff.changed[0].new_value, "new");
        assert!(diff.is_breaking());
        assert!(!diff.is_empty());

        let markdown = diff.to_markdown();
        assert!(markdown.contains("a/fresh"));
        assert!(markdown.contains("a/gone"));
        assert!(markdown.contains("a/tweak"));

        let json = serde_json::to_value(&diff).unwrap();
        assert_eq!(json["added"].as_array().unwrap().len(), 1);
        assert_eq!(json["removed"].as_array().unwrap().len(), 1);
        assert_eq!(json["changed"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn diff_purely_additive_is_not_breaking() {
        let old = r#"{"presets": [
            {"preset_id": "a", "tokens": [
                {"name": "keep", "path": ["keep"], "value": "1", "token_type": "string"}
            ]}
        ]}"#;
        let new = r#"{"presets": [
            {"preset_id": "a", "tokens": [
                {"name": "keep", "path": ["keep"], "value": "1", "token_type": "string"},
                {"name": "fresh", "path": ["fresh"], "value": "2", "token_type": "string"}
            ]}
        ]}"#;
        let diff = diff_design_tokens(old, new, DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert!(!diff.is_breaking());
        assert!(!diff.is_empty());
        assert_eq!(diff.added.len(), 1);
    }

    #[test]
    fn diff_rejects_invalid_documents() {
        let ok = export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert!(
            diff_design_tokens("not json", &ok, DesignTokenFormat::StyleDictionaryJson).is_err()
        );
        assert!(diff_design_tokens("{}", &ok, DesignTokenFormat::StyleDictionaryJson).is_err());
        assert!(
            diff_design_tokens_from_paths(
                Path::new("/nonexistent/gpui-design-tools-diff-old.json"),
                Path::new("/nonexistent/gpui-design-tools-diff-new.json"),
                DesignTokenFormat::StyleDictionaryJson,
            )
            .is_err()
        );
    }

    #[test]
    fn diff_from_paths_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "gpui-design-tools-diff-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let old_path = dir.join("old.json");
        let new_path = dir.join("new.json");
        std::fs::write(&old_path, r#"{"presets": []}"#).unwrap();
        std::fs::write(&new_path, r#"{"presets": []}"#).unwrap();
        let diff = diff_design_tokens_from_paths(
            &old_path,
            &new_path,
            DesignTokenFormat::StyleDictionaryJson,
        )
        .unwrap();
        assert!(diff.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn atomic_write_durability_flag_writes_identical_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "gpui-design-tools-durable-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let durable_path = dir.join("durable.txt");
        let fast_path = dir.join("fast.txt");
        write_text_atomically_with_durability(&durable_path, "payload", true).unwrap();
        write_text_atomically_with_durability(&fast_path, "payload", false).unwrap();
        assert_eq!(
            std::fs::read(&durable_path).unwrap(),
            std::fs::read(&fast_path).unwrap()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn export_to_path_with_options_compact_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "gpui-design-tools-compact-export-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pretty_path = dir.join("pretty.json");
        let compact_path = dir.join("compact.json");
        export_design_tokens_to_path_with_options(
            &pretty_path,
            DesignTokenFormat::StyleDictionaryJson,
            DesignTokenExportOptions::pretty(),
            false,
        )
        .unwrap();
        export_design_tokens_to_path_with_options(
            &compact_path,
            DesignTokenFormat::StyleDictionaryJson,
            DesignTokenExportOptions::compact(),
            false,
        )
        .unwrap();
        let pretty: Value =
            serde_json::from_str(&std::fs::read_to_string(&pretty_path).unwrap()).unwrap();
        let compact: Value =
            serde_json::from_str(&std::fs::read_to_string(&compact_path).unwrap()).unwrap();
        assert_eq!(pretty, compact);
        assert!(
            std::fs::metadata(&compact_path).unwrap().len()
                < std::fs::metadata(&pretty_path).unwrap().len()
        );
        let imported =
            import_design_tokens_from_path(&compact_path, DesignTokenFormat::StyleDictionaryJson)
                .unwrap();
        assert!(imported.preset_count > 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
