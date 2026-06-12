use anyhow::{Context, Result};
use clap::Parser;
use gpui_component_lab::lab_ui::{LabAppConfig, run_lab_app};
use gpui_component_lab::{
    ComponentLabConformanceReport, builtin_story_registry, ensure_component_lab_conformance_passed,
    latest_rust_source_modified, load_story_documents, validate_component_lab_conformance,
};
use gpui_design_tools::{
    DesignTokenFormat, DesignTokenValidationReport, validate_current_design_tokens,
    validate_design_tokens_from_path,
};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

#[derive(Parser)]
struct Args {
    /// Directory containing `*.story.json` designer state.
    #[arg(long, default_value = "crates/gpui-toolkit/stories")]
    stories_dir: PathBuf,
    /// Watch story/token state and print reload events.
    #[arg(long)]
    watch: bool,
    /// Token JSON files to reload while watching.
    #[arg(long = "token")]
    tokens: Vec<PathBuf>,
    /// Watch Rust sources and relaunch a supervised child process.
    #[arg(long)]
    supervise_rust: bool,
    /// Root scanned by `--supervise-rust`.
    #[arg(long, default_value = "crates/gpui-toolkit")]
    rust_watch_root: PathBuf,
    /// Child command relaunched by `--supervise-rust`.
    #[arg(long)]
    child_command: Option<String>,
    /// Emit the built-in registry as JSON.
    #[arg(long)]
    json: bool,
    /// Validate design conformance before starting.
    #[arg(long)]
    conformance: bool,
    /// Emit conformance as JSON instead of Markdown.
    #[arg(long)]
    conformance_json: bool,
    /// Write conformance report JSON.
    #[arg(long)]
    report_json: Option<PathBuf>,
    /// Write conformance report Markdown.
    #[arg(long)]
    report_markdown: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.supervise_rust {
        return supervise_rust_source(&args.rust_watch_root, args.child_command.as_deref());
    }

    if args.conformance
        || args.conformance_json
        || args.report_json.is_some()
        || args.report_markdown.is_some()
    {
        let report = run_conformance(&args.stories_dir, &args.tokens)?;
        emit_conformance_report(
            &report,
            args.conformance_json,
            args.report_json.as_deref(),
            args.report_markdown.as_deref(),
        )?;
        return ensure_component_lab_conformance_passed(&report);
    }

    if args.json {
        let registry = builtin_story_registry()?;
        println!("{}", serde_json::to_string_pretty(&registry)?);
        return Ok(());
    }

    run_lab_app(LabAppConfig::new(args.stories_dir, args.tokens).with_watch(args.watch))
}

fn run_conformance(
    stories_dir: &Path,
    tokens: &[PathBuf],
) -> Result<ComponentLabConformanceReport> {
    let registry = builtin_story_registry()?;
    let docs = load_story_documents(stories_dir)?;
    let token_report = validate_conformance_tokens(tokens)?;
    Ok(validate_component_lab_conformance(
        &registry,
        &docs,
        &token_report,
    ))
}

fn validate_conformance_tokens(tokens: &[PathBuf]) -> Result<DesignTokenValidationReport> {
    if tokens.is_empty() {
        return validate_current_design_tokens();
    }

    let mut combined: Option<DesignTokenValidationReport> = None;
    for token in tokens {
        let report =
            validate_design_tokens_from_path(token, DesignTokenFormat::StyleDictionaryJson)
                .with_context(|| format!("validate {}", token.display()))?;
        if let Some(combined) = combined.as_mut() {
            combined.passed &= report.passed;
            combined.preset_count += report.preset_count;
            combined.token_count += report.token_count;
            combined.findings.extend(
                report
                    .findings
                    .into_iter()
                    .map(|finding| format!("{}: {finding}", token.display())),
            );
            combined
                .conformance_markdown
                .push_str(&format!("\n\n### {}\n\n", token.display()));
            combined
                .conformance_markdown
                .push_str(&report.conformance_markdown);
        } else {
            combined = Some(report);
        }
    }

    combined.context("no token reports produced")
}

fn emit_conformance_report(
    report: &ComponentLabConformanceReport,
    json_stdout: bool,
    report_json: Option<&Path>,
    report_markdown: Option<&Path>,
) -> Result<()> {
    if json_stdout {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{}", report.to_markdown());
    }

    if let Some(path) = report_json {
        write_report(path, serde_json::to_string_pretty(report)?)?;
    }
    if let Some(path) = report_markdown {
        write_report(path, report.to_markdown())?;
    }

    Ok(())
}

fn write_report(path: &Path, body: String) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, body).with_context(|| format!("write {}", path.display()))
}

fn supervise_rust_source(root: &Path, child_command: Option<&str>) -> Result<()> {
    let command = child_command.context("--supervise-rust requires --child-command")?;
    println!(
        "Watching {} for Rust source changes; relaunching child command safely",
        root.display()
    );
    let mut child = Some(spawn_child(command)?);
    let mut last_seen = latest_rust_source_modified(root)?;

    loop {
        std::thread::sleep(Duration::from_millis(1000));
        if let Some(running) = child.as_mut()
            && running.try_wait()?.is_some()
        {
            child = None;
        }

        let next = latest_rust_source_modified(root)?;
        if next > last_seen {
            if let Some(mut running) = child.take() {
                let _ = running.kill();
                let _ = running.wait();
            }
            child = Some(spawn_child(command)?);
            last_seen = next;
        }
    }
}

fn spawn_child(command: &str) -> Result<Child> {
    let mut parts = command.split_whitespace();
    let program = parts.next().context("child command must not be empty")?;
    Command::new(program)
        .args(parts)
        .spawn()
        .with_context(|| format!("spawn child command '{command}'"))
}
