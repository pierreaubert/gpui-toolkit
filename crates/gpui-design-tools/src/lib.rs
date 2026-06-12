//! Toolkit-owned design token import/export and conformance helpers.

use anyhow::{Context, Result, anyhow, bail};
use gpui_design::{DesignConformanceMatrix, DesignTokenExport};
use serde::Serialize;
use serde_json::Value;
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
    pub findings: Vec<String>,
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
pub fn validate_design_tokens(
    input: &str,
    format: DesignTokenFormat,
) -> Result<DesignTokenValidationReport> {
    let raw: Value = match format {
        DesignTokenFormat::StyleDictionaryJson => {
            serde_json::from_str(input).context("parse design token JSON")?
        }
    };
    let (preset_count, token_count, mut findings) = inspect_token_value(&raw);
    let matrix = DesignConformanceMatrix::all_presets();
    for (case, finding) in matrix.findings() {
        findings.push(format!(
            "{}:{}:{}",
            case.preset_id,
            case.motion_label(),
            finding.id
        ));
    }

    Ok(DesignTokenValidationReport {
        passed: findings.is_empty(),
        findings,
        preset_count,
        token_count,
        conformance_markdown: matrix.to_markdown_table(),
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
) -> Result<DesignTokenValidationReport> {
    let input =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    validate_design_tokens(&input, format)
}

fn inspect_token_value(raw: &Value) -> (usize, usize, Vec<String>) {
    let mut findings = Vec::new();
    let Some(presets) = raw.get("presets").and_then(Value::as_array) else {
        return (0, 0, vec!["root.presets must be an array".to_string()]);
    };

    let mut token_count = 0usize;
    for (preset_index, preset) in presets.iter().enumerate() {
        if preset.get("preset_id").and_then(Value::as_str).is_none() {
            findings.push(format!(
                "presets[{preset_index}].preset_id must be a string"
            ));
        }
        let Some(tokens) = preset.get("tokens").and_then(Value::as_array) else {
            findings.push(format!("presets[{preset_index}].tokens must be an array"));
            continue;
        };
        token_count += tokens.len();
        for (token_index, token) in tokens.iter().enumerate() {
            let prefix = format!("presets[{preset_index}].tokens[{token_index}]");
            if token.get("name").and_then(Value::as_str).is_none() {
                findings.push(format!("{prefix}.name must be a string"));
            }
            if token.get("path").and_then(Value::as_array).is_none() {
                findings.push(format!("{prefix}.path must be an array"));
            }
            if token.get("value").and_then(Value::as_str).is_none() {
                findings.push(format!("{prefix}.value must be a string"));
            }
            if token.get("token_type").and_then(Value::as_str).is_none() {
                findings.push(format!("{prefix}.token_type must be a string"));
            }
        }
    }

    if presets.is_empty() {
        findings.push("root.presets must not be empty".to_string());
    }
    if token_count == 0 {
        findings.push("token export must contain at least one token".to_string());
    }

    (presets.len(), token_count, findings)
}

/// Export current tokens and validate them with the conformance matrix.
pub fn validate_current_design_tokens() -> Result<DesignTokenValidationReport> {
    let output = export_design_tokens(DesignTokenFormat::StyleDictionaryJson)?;
    validate_design_tokens(&output, DesignTokenFormat::StyleDictionaryJson)
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
        let report = validate_design_tokens("{}", DesignTokenFormat::StyleDictionaryJson).unwrap();
        assert!(!report.passed);
        assert!(report.findings.iter().any(|f| f.contains("root.presets")));
    }
}
