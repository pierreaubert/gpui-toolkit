use anyhow::{Context, Result};
use clap::Parser;
use gpui_design_tools::{
    DesignTokenFormat, ensure_passed, validate_current_design_tokens,
    validate_design_tokens_from_path,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "gpui-validate-design-tokens",
    about = "Validate GPUI design tokens and emit stable CI reports.",
    long_about = "Validate GPUI design tokens in Style Dictionary JSON format. The JSON report uses the stable gpui-design-token-validation schema and is intended for CI, release gates, and downstream tooling."
)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "style-dictionary-json",
        help = "Token wire format: style-dictionary-json, style_dictionary_json, or json"
    )]
    format: String,
    #[arg(
        short,
        long,
        help = "Validate this token JSON file instead of the built-in DesignSystem export"
    )]
    input: Option<PathBuf>,
    #[arg(long, help = "Print the stable machine-readable JSON report to stdout")]
    json: bool,
    #[arg(
        long,
        help = "Write the stable machine-readable JSON report to this path"
    )]
    report_json: Option<PathBuf>,
    #[arg(long, help = "Write a human-readable markdown report to this path")]
    report_markdown: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format = DesignTokenFormat::parse(&args.format)?;

    // Avoid building the markdown table when the CLI only needs JSON output.
    let need_markdown = !args.json || args.report_markdown.is_some();
    let report = if let Some(input) = args.input.as_deref() {
        validate_design_tokens_from_path(input, format, need_markdown)?
    } else {
        validate_current_design_tokens(need_markdown)?
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.conformance_markdown);
        if report.findings.is_empty() {
            println!("Design token validation passed.");
        } else {
            println!("Findings:");
            for finding in &report.findings {
                println!("- {finding}");
            }
        }
    }

    if let Some(path) = args.report_json.as_deref() {
        write_report(path, serde_json::to_string_pretty(&report)?)?;
    }
    if let Some(path) = args.report_markdown.as_deref() {
        let mut markdown = report.conformance_markdown.clone();
        if report.findings.is_empty() {
            markdown.push_str("\n\nDesign token validation passed.\n");
        } else {
            markdown.push_str("\n\n## Findings\n");
            for finding in &report.findings {
                markdown.push_str(&format!("- {finding}\n"));
            }
        }
        write_report(path, markdown)?;
    }

    ensure_passed(&report)
}

fn write_report(path: &std::path::Path, body: String) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
