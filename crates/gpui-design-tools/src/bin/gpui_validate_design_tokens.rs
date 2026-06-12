use anyhow::Result;
use clap::Parser;
use gpui_design_tools::{
    DesignTokenFormat, ensure_passed, validate_current_design_tokens,
    validate_design_tokens_from_path,
};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "style-dictionary-json")]
    format: String,
    #[arg(short, long)]
    input: Option<PathBuf>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    report_json: Option<PathBuf>,
    #[arg(long)]
    report_markdown: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format = DesignTokenFormat::parse(&args.format)?;
    let report = if let Some(input) = args.input.as_deref() {
        validate_design_tokens_from_path(input, format)?
    } else {
        validate_current_design_tokens()?
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
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    Ok(())
}
