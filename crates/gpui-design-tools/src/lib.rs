//! Toolkit-owned design token import/export and conformance helpers.

use anyhow::{Context, Result, anyhow, bail};
use gpui_design::{DesignConformanceMatrix, DesignTokenExport};
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use std::path::Path;

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
    pub passed: bool,
    pub findings: Vec<Cow<'static, str>>,
    pub preset_count: usize,
    pub token_count: usize,
    pub conformance_markdown: String,
}

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

fn validate_raw_tokens(
    raw: &Value,
    render_markdown: bool,
) -> Result<DesignTokenValidationReport> {
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
        findings.push(Cow::Borrowed("token export must contain at least one token"));
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
        findings.push(Cow::Borrowed("token export must contain at least one token"));
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
    fn validate_from_path_round_trip() {
        let json = export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap();
        let path = std::env::temp_dir()
            .join(format!("gpui-design-tools-test-{}.json", std::process::id()));
        std::fs::write(&path, &json).unwrap();
        let report = validate_design_tokens_from_path(
            &path,
            DesignTokenFormat::StyleDictionaryJson,
            true,
        )
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
            passed: true,
            findings: Vec::new(),
            preset_count: 1,
            token_count: 1,
            conformance_markdown: String::new(),
        };
        assert!(ensure_passed(&passing).is_ok());

        let failing = DesignTokenValidationReport {
            passed: false,
            findings: vec![Cow::Borrowed("bad")],
            preset_count: 0,
            token_count: 0,
            conformance_markdown: String::new(),
        };
        assert!(ensure_passed(&failing).is_err());
    }
}
