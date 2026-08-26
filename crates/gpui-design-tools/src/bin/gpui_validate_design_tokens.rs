use anyhow::Result;
use clap::Parser;
use gpui_design_tools::{
    DesignTokenFormat, ensure_passed, validate_current_design_tokens,
    validate_design_tokens_from_path,
};
use std::path::PathBuf;

use gpui_design_tools::write_text_atomically;

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

    let json_report = (args.json || args.report_json.is_some())
        .then(|| serde_json::to_string_pretty(&report))
        .transpose()?;

    if args.json {
        println!(
            "{}",
            json_report.as_deref().expect("JSON report is available")
        );
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

    let validation_result = ensure_passed(&report);

    if let Some(path) = args.report_json.as_deref() {
        write_report(
            path,
            json_report.as_deref().expect("JSON report is available"),
        )?;
    }
    if let Some(path) = args.report_markdown.as_deref() {
        let mut markdown = report.conformance_markdown;
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

    validation_result
}

fn write_report(path: &std::path::Path, body: impl AsRef<[u8]>) -> Result<()> {
    write_text_atomically(path, body)
}
