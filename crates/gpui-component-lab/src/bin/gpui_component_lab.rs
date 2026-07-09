use anyhow::{Context, Result};
use clap::Parser;
use gpui_component_lab::lab_ui::{LabAppConfig, run_lab_app};
use gpui_component_lab::{
    ComponentLabConformanceReport, ComponentLabVisualDiffReport, ComponentLabVisualManifest,
    builtin_story_registry, builtin_story_renderers, ensure_component_lab_conformance_passed,
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
    /// Emit the visual-regression screenshot manifest as JSON.
    #[arg(long)]
    visual_manifest: bool,
    /// Root used for generated baseline/actual/diff screenshot paths.
    #[arg(long, default_value = "target/gpui-component-lab/visual")]
    visual_output_root: PathBuf,
    /// Write visual-regression manifest JSON.
    #[arg(long)]
    visual_manifest_json: Option<PathBuf>,
    /// Write visual-regression manifest Markdown.
    #[arg(long)]
    visual_manifest_markdown: Option<PathBuf>,
    /// Compare baseline and actual screenshots from the visual manifest.
    #[arg(long)]
    visual_diff: bool,
    /// Maximum changed pixels allowed per capture.
    #[arg(long, default_value_t = 0)]
    visual_diff_max_changed_pixels: u64,
    /// Emit visual-regression diff report as JSON.
    #[arg(long)]
    visual_diff_json: Option<PathBuf>,
    /// Emit visual-regression diff report as Markdown.
    #[arg(long)]
    visual_diff_markdown: Option<PathBuf>,
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

    if args.visual_manifest
        || args.visual_manifest_json.is_some()
        || args.visual_manifest_markdown.is_some()
        || args.visual_diff
        || args.visual_diff_json.is_some()
        || args.visual_diff_markdown.is_some()
    {
        let manifest = run_visual_manifest(&args.visual_output_root)?;
        if args.visual_manifest
            || args.visual_manifest_json.is_some()
            || args.visual_manifest_markdown.is_some()
        {
            emit_visual_manifest(
                &manifest,
                args.visual_manifest,
                args.visual_manifest_json.as_deref(),
                args.visual_manifest_markdown.as_deref(),
            )?;
        }
        if args.visual_diff
            || args.visual_diff_json.is_some()
            || args.visual_diff_markdown.is_some()
        {
            let report = manifest.diff_captures(args.visual_diff_max_changed_pixels);
            emit_visual_diff(
                &report,
                args.visual_diff,
                args.visual_diff_json.as_deref(),
                args.visual_diff_markdown.as_deref(),
            )?;
            if !report.passed {
                anyhow::bail!(
                    "visual diff failed: {} of {} cases failed",
                    report.failed_count,
                    report.case_count
                );
            }
        }
        return Ok(());
    }

    if args.json {
        let registry = builtin_story_registry()?;
        println!("{}", serde_json::to_string_pretty(&registry)?);
        return Ok(());
    }

    run_lab_app(LabAppConfig::new(args.stories_dir, args.tokens).with_watch(args.watch))
}

fn run_visual_manifest(output_root: &Path) -> Result<ComponentLabVisualManifest> {
    let stories = builtin_story_registry()?;
    let renderers = builtin_story_renderers()?;
    Ok(ComponentLabVisualManifest::from_registries(
        &stories,
        &renderers,
        output_root,
    ))
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
        return validate_current_design_tokens(true);
    }

    let mut combined: Option<DesignTokenValidationReport> = None;
    for token in tokens {
        let report =
            validate_design_tokens_from_path(token, DesignTokenFormat::StyleDictionaryJson, true)
                .with_context(|| format!("validate {}", token.display()))?;
        if let Some(combined) = combined.as_mut() {
            combined.passed &= report.passed;
            combined.preset_count += report.preset_count;
            combined.token_count += report.token_count;
            combined.findings.extend(
                report.findings.into_iter().map(|finding| {
                    std::borrow::Cow::Owned(format!("{}: {finding}", token.display()))
                }),
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

fn emit_visual_manifest(
    manifest: &ComponentLabVisualManifest,
    json_stdout: bool,
    report_json: Option<&Path>,
    report_markdown: Option<&Path>,
) -> Result<()> {
    if json_stdout {
        println!("{}", serde_json::to_string_pretty(manifest)?);
    } else {
        println!("{}", manifest.to_markdown_table());
    }

    if let Some(path) = report_json {
        write_report(path, serde_json::to_string_pretty(manifest)?)?;
    }
    if let Some(path) = report_markdown {
        write_report(path, manifest.to_markdown_table())?;
    }

    Ok(())
}

fn emit_visual_diff(
    report: &ComponentLabVisualDiffReport,
    json_stdout: bool,
    report_json: Option<&Path>,
    report_markdown: Option<&Path>,
) -> Result<()> {
    if json_stdout {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{}", report.to_markdown_table());
    }

    if let Some(path) = report_json {
        write_report(path, serde_json::to_string_pretty(report)?)?;
    }
    if let Some(path) = report_markdown {
        write_report(path, report.to_markdown_table())?;
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
