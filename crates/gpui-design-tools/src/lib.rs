//! Toolkit-owned design token import/export and conformance helpers.

use anyhow::{Context, Result, anyhow, bail};
use gpui_design::{DesignConformanceMatrix, DesignTokenExport};
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
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
}

impl DesignTokenFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "json" | "style-dictionary-json" | "style_dictionary_json" => {
                Ok(Self::StyleDictionaryJson)
            }
            other => bail!("unsupported design token format '{other}'"),
        }
    }
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
        output.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n\n",
            self.schema_version, self.report_type, self.crate_name, self.crate_version
        ));
        output
            .push_str("| id | status | artifact | path or command | evidence | remaining gap |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- |\n");
        for item in self.items {
            output.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` | {} | {} |\n",
                item.id,
                item.status.label(),
                item.title,
                item.path_or_command,
                item.release_evidence,
                item.remaining_gap,
            ));
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
pub fn export_design_tokens(format: DesignTokenFormat) -> Result<String> {
    match format {
        DesignTokenFormat::StyleDictionaryJson => {
            let export = DesignTokenExport::for_all_presets();
            serde_json::to_string_pretty(&export).context("serialize design token export")
        }
    }
}

/// Import and validate a design token document.
pub fn import_design_tokens(
    input: &str,
    format: DesignTokenFormat,
) -> Result<ImportedDesignTokens> {
    match format {
        DesignTokenFormat::StyleDictionaryJson => {
            let raw: Value = serde_json::from_str(input).context("parse design token JSON")?;
            let (preset_count, token_count, findings) = inspect_token_value(&raw);
            if !findings.is_empty() {
                bail!("invalid design token JSON: {}", findings.join("; "));
            }
            Ok(ImportedDesignTokens {
                preset_count,
                token_count,
                raw,
            })
        }
    }
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
    let raw: Value = match format {
        DesignTokenFormat::StyleDictionaryJson => {
            serde_json::from_str(input).context("parse design token JSON")?
        }
    };
    validate_raw_tokens(&raw, render_markdown)
}

fn validate_raw_tokens(raw: &Value, render_markdown: bool) -> Result<DesignTokenValidationReport> {
    let (preset_count, token_count, mut findings) = inspect_token_value(raw);
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
    let output = export_design_tokens(format)?;
    std::fs::write(path, output).with_context(|| format!("write {}", path.display()))
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
        for (token_index, token) in tokens.iter().enumerate() {
            let mut prefix: Option<String> = None;
            if token.get("name").and_then(Value::as_str).is_none() {
                let p = prefix.get_or_insert_with(|| {
                    format!("presets[{preset_index}].tokens[{token_index}]")
                });
                findings.push(Cow::Owned(format!("{p}.name must be a string")));
            }
            if token.get("path").and_then(Value::as_array).is_none() {
                let p = prefix.get_or_insert_with(|| {
                    format!("presets[{preset_index}].tokens[{token_index}]")
                });
                findings.push(Cow::Owned(format!("{p}.path must be an array")));
            }
            if token.get("value").and_then(Value::as_str).is_none() {
                let p = prefix.get_or_insert_with(|| {
                    format!("presets[{preset_index}].tokens[{token_index}]")
                });
                findings.push(Cow::Owned(format!("{p}.value must be a string")));
            }
            if token.get("token_type").and_then(Value::as_str).is_none() {
                let p = prefix.get_or_insert_with(|| {
                    format!("presets[{preset_index}].tokens[{token_index}]")
                });
                findings.push(Cow::Owned(format!("{p}.token_type must be a string")));
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
        assert!(json["findings"].as_array().unwrap().len() >= 1);
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
}
