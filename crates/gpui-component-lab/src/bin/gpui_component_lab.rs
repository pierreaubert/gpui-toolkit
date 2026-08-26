use anyhow::{Context, Result};
use clap::Parser;
#[cfg(feature = "visual-capture")]
use gpui_component_lab::lab_ui::{ComponentLabCaptureReport, capture_component_lab_cases};
use gpui_component_lab::lab_ui::{LabAppConfig, run_lab_app};
use gpui_component_lab::{
    ComponentLabConformanceReport, ComponentLabVisualDiffReport, ComponentLabVisualManifest,
    builtin_story_registry, builtin_story_renderers, ensure_component_lab_conformance_passed,
    generate_component_lab_gallery, latest_rust_source_modified, load_story_documents,
    promote_component_lab_baselines, validate_component_lab_conformance,
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
    /// Shell-style child command relaunched by `--supervise-rust`.
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
    /// Renderer namespace used for baseline, actual, and diff artifacts.
    #[arg(long)]
    visual_renderer: Option<String>,
    /// Logical-to-device pixel scale encoded in capture dimensions.
    #[arg(long)]
    visual_pixel_scale: Option<u32>,
    /// Restrict capture/diff to exact manifest IDs; repeat for multiple cases.
    #[arg(long = "visual-case")]
    visual_cases: Vec<String>,
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
    /// Maximum diff cases; zero checks the full manifest.
    #[arg(long, default_value_t = 200)]
    visual_diff_limit: usize,
    /// Zero-based shard selected from the diff subset.
    #[arg(long, default_value_t = 0)]
    visual_diff_shard_index: usize,
    /// Number of deterministic diff shards.
    #[arg(long, default_value_t = 1)]
    visual_diff_shard_count: usize,
    /// Render actual PNG pixels for a deterministic subset of the manifest.
    #[arg(long)]
    visual_capture: bool,
    /// Maximum capture cases; zero captures the full manifest.
    #[arg(long, default_value_t = 200)]
    visual_capture_limit: usize,
    /// Zero-based shard selected from the capture subset.
    #[arg(long, default_value_t = 0)]
    visual_capture_shard_index: usize,
    /// Number of deterministic capture shards.
    #[arg(long, default_value_t = 1)]
    visual_capture_shard_count: usize,
    /// Write renderer-capture report JSON.
    #[arg(long)]
    visual_capture_json: Option<PathBuf>,
    /// Write renderer-capture report Markdown.
    #[arg(long)]
    visual_capture_markdown: Option<PathBuf>,
    /// Explicitly promote successful actual captures to renderer baselines.
    #[arg(long)]
    visual_update_baselines: bool,
    /// Override the baseline index JSON path.
    #[arg(long)]
    visual_baseline_index: Option<PathBuf>,
    /// Generate PNG contact sheets and gallery indexes from successful captures.
    #[arg(long)]
    visual_gallery: bool,
    /// Override the gallery output directory.
    #[arg(long)]
    visual_gallery_root: Option<PathBuf>,
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
        || args.visual_capture
        || args.visual_capture_json.is_some()
        || args.visual_capture_markdown.is_some()
        || args.visual_update_baselines
        || args.visual_baseline_index.is_some()
        || args.visual_gallery
        || args.visual_gallery_root.is_some()
    {
        let renderer_id = args
            .visual_renderer
            .clone()
            .unwrap_or_else(default_visual_renderer);
        let pixel_scale = args
            .visual_pixel_scale
            .unwrap_or_else(default_visual_pixel_scale);
        let manifest = run_visual_manifest(&args.visual_output_root, &renderer_id, pixel_scale)?;
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
        if args.visual_baseline_index.is_some() && !args.visual_update_baselines {
            anyhow::bail!("--visual-baseline-index requires --visual-update-baselines");
        }
        let renderer_capture_requested = args.visual_capture
            || args.visual_capture_json.is_some()
            || args.visual_capture_markdown.is_some();
        if (args.visual_update_baselines
            || args.visual_gallery
            || args.visual_gallery_root.is_some())
            && !renderer_capture_requested
        {
            anyhow::bail!(
                "baseline promotion and gallery generation require --visual-capture in the same run"
            );
        }
        if renderer_capture_requested {
            let subset = select_visual_cases(
                &manifest,
                args.visual_capture_limit,
                args.visual_capture_shard_index,
                args.visual_capture_shard_count,
                "capture",
                &args.visual_cases,
            )?;
            let report =
                run_renderer_capture(&renderer_id, &subset, &args.stories_dir, &args.tokens)?;
            emit_capture_report(
                &report,
                args.visual_capture,
                args.visual_capture_json.as_deref(),
                args.visual_capture_markdown.as_deref(),
            )?;
            if !report.passed {
                anyhow::bail!(
                    "component-lab renderer capture failed for {} of {} cases",
                    report.failed_count,
                    report.requested_count
                );
            }
            if args.visual_update_baselines {
                let index_path = args.visual_baseline_index.clone().unwrap_or_else(|| {
                    args.visual_output_root
                        .join(&renderer_id)
                        .join("baseline")
                        .join("index.json")
                });
                let index = promote_component_lab_baselines(&renderer_id, &subset, &index_path)?;
                let markdown_path = index_path.with_extension("md");
                write_report(&markdown_path, index.to_markdown_table())?;
            }
            if args.visual_gallery || args.visual_gallery_root.is_some() {
                let gallery_root = args
                    .visual_gallery_root
                    .clone()
                    .unwrap_or_else(|| args.visual_output_root.join(&renderer_id).join("gallery"));
                generate_component_lab_gallery(&renderer_id, &subset, &gallery_root)?;
            }
        }
        if args.visual_diff
            || args.visual_diff_json.is_some()
            || args.visual_diff_markdown.is_some()
        {
            let subset = select_visual_cases(
                &manifest,
                args.visual_diff_limit,
                args.visual_diff_shard_index,
                args.visual_diff_shard_count,
                "diff",
                &args.visual_cases,
            )?;
            let report =
                manifest.diff_selected_captures(&subset, args.visual_diff_max_changed_pixels);
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

fn run_visual_manifest(
    output_root: &Path,
    renderer_id: &str,
    pixel_scale: u32,
) -> Result<ComponentLabVisualManifest> {
    let stories = builtin_story_registry()?;
    let renderers = builtin_story_renderers()?;
    Ok(ComponentLabVisualManifest::from_registries_for_renderer(
        &stories,
        &renderers,
        output_root,
        renderer_id,
        pixel_scale,
    ))
}

fn default_visual_renderer() -> String {
    if cfg!(target_os = "macos") {
        "metal".to_string()
    } else if cfg!(target_os = "windows") {
        "directx".to_string()
    } else if cfg!(target_os = "linux") {
        "wgpu-linux".to_string()
    } else {
        "unsupported".to_string()
    }
}

fn select_visual_cases(
    manifest: &ComponentLabVisualManifest,
    limit: usize,
    shard_index: usize,
    shard_count: usize,
    operation: &str,
    requested_ids: &[String],
) -> Result<Vec<gpui_component_lab::ComponentLabVisualCase>> {
    if shard_count == 0 || shard_index >= shard_count {
        anyhow::bail!(
            "visual {operation} shard index {shard_index} is invalid for {shard_count} shards"
        );
    }
    if !requested_ids.is_empty() {
        let selected = requested_ids
            .iter()
            .map(|capture_id| {
                manifest
                    .cases
                    .iter()
                    .find(|case| case.capture_id == *capture_id)
                    .cloned()
                    .with_context(|| format!("unknown visual case `{capture_id}`"))
            })
            .collect::<Result<Vec<_>>>()?;
        return Ok(selected);
    }
    Ok(manifest
        .representative_cases(limit)
        .into_iter()
        .enumerate()
        .filter_map(|(index, case)| (index % shard_count == shard_index).then_some(case))
        .collect())
}

const fn default_visual_pixel_scale() -> u32 {
    if cfg!(target_os = "macos") { 2 } else { 1 }
}

#[cfg(feature = "visual-capture")]
fn run_renderer_capture(
    renderer_id: &str,
    cases: &[gpui_component_lab::ComponentLabVisualCase],
    stories_dir: &Path,
    token_paths: &[PathBuf],
) -> Result<ComponentLabCaptureReport> {
    capture_component_lab_cases(renderer_id, cases, stories_dir, token_paths)
}

#[cfg(not(feature = "visual-capture"))]
fn run_renderer_capture(
    _renderer_id: &str,
    _cases: &[gpui_component_lab::ComponentLabVisualCase],
    _stories_dir: &Path,
    _token_paths: &[PathBuf],
) -> Result<NeverCaptureReport> {
    anyhow::bail!("--visual-capture requires the gpui-component-lab visual-capture feature")
}

#[cfg(not(feature = "visual-capture"))]
struct NeverCaptureReport {
    passed: bool,
    failed_count: usize,
    requested_count: usize,
}

#[cfg(feature = "visual-capture")]
fn emit_capture_report(
    report: &ComponentLabCaptureReport,
    markdown_stdout: bool,
    report_json: Option<&Path>,
    report_markdown: Option<&Path>,
) -> Result<()> {
    if markdown_stdout {
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

#[cfg(not(feature = "visual-capture"))]
fn emit_capture_report(
    _report: &NeverCaptureReport,
    _markdown_stdout: bool,
    _report_json: Option<&Path>,
    _report_markdown: Option<&Path>,
) -> Result<()> {
    Ok(())
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
    let (program, args) = parse_child_command(command)?;
    Command::new(program)
        .args(args)
        .spawn()
        .with_context(|| format!("spawn child command '{command}'"))
}

fn parse_child_command(command: &str) -> Result<(String, Vec<String>)> {
    let parts =
        shlex::split(command).context("child command must use valid shell-style quoting")?;
    let (program, args) = parts
        .split_first()
        .context("child command must not be empty")?;
    Ok((program.clone(), args.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::parse_child_command;

    #[test]
    fn child_command_preserves_quoted_arguments() {
        let (program, args) = parse_child_command(
            "cargo run -p gpui-component-lab -- --stories-dir 'stories with spaces'",
        )
        .expect("quoted child command");

        assert_eq!(program, "cargo");
        assert_eq!(
            args,
            [
                "run",
                "-p",
                "gpui-component-lab",
                "--",
                "--stories-dir",
                "stories with spaces",
            ]
        );
    }

    #[test]
    fn child_command_rejects_unclosed_quote() {
        assert!(parse_child_command("cargo run 'unterminated").is_err());
    }
}
