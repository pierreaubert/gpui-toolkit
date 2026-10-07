//! Command-line parser and dispatch.
//!
//! [`Cli`] is the clap-derived parser; [`run_command`] executes one
//! subcommand and renders both human-readable text and the typed JSON
//! envelope. Keeping the parser in the library (instead of the binary)
//! lets tests walk the real command tree for manifest and drift checks.

// Rust guideline compliant 2026-02-21

use crate::{
    COMPONENT_DETAIL_TYPE, COMPONENT_LIST_TYPE, COMPONENT_PROPS_TYPE, DEFAULT_LAYOUT_FN,
    DOCTOR_TYPE, EJECT_TYPE, ErrorCode, GAP_REPORT_TYPE, InitOptions, LAYOUT_CHECK_TYPE,
    LAYOUT_EXPAND_TYPE, LAYOUT_GRAMMAR_TYPE, LayoutExpand, SEARCH_TYPE, TEMPLATE_COPY_TYPE,
    TEMPLATE_LIST_TYPE, TEMPLATE_SHOW_TYPE, TEMPLATE_SKELETON_TYPE, THEME_BUILD_TYPE,
    THEME_CHECK_TYPE, THEME_LIST_TYPE, THEME_TARGETS_TYPE, ToolkitError, UPGRADE_LIST_TYPE,
    build_manifest, component_detail, component_list, component_props, eject_component, gap_report,
    layout_check, layout_expand, layout_expand_to_file, layout_grammar, render_detail_text,
    render_doctor_text, render_eject_text, render_gap_text, render_json, render_json_compact,
    render_layout_check_text, render_layout_expand_text, render_list_text, render_props_text,
    render_search_text, render_template_copy_text, render_template_list_text,
    render_theme_build_text, render_theme_check_text, render_theme_list_text,
    render_theme_targets_text, render_upgrade_list_text, run_doctor, run_init, search,
    success_envelope, template_copy, template_list, template_show, template_skeleton, theme_build,
    theme_check, theme_list, theme_targets, upgrade_list,
};
use crate::{DEFAULT_SEARCH_LIMIT, DetailLevel};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

/// Unified agent-ready CLI for gpui-toolkit.
#[derive(Debug, Parser)]
#[command(
    name = "gpui-toolkit",
    version,
    about = "Unified agent-ready CLI for gpui-toolkit"
)]
pub struct Cli {
    /// Print machine-readable typed JSON envelopes.
    #[arg(long, global = true)]
    pub json: bool,

    /// Token-efficient output: terse text, single-line JSON.
    #[arg(long, global = true)]
    pub dense: bool,

    /// List detail level; single-item views always return everything.
    #[arg(long, global = true, value_enum, default_value_t = DetailArg::Brief)]
    pub detail: DetailArg,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// List detail levels accepted on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DetailArg {
    /// Names only.
    Brief,
    /// Names plus one-line titles.
    Compact,
    /// Names, titles, and descriptions.
    Full,
}

impl Display for DetailArg {
    /// Renders the flag spelling consumed by clap.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Brief => "brief",
            Self::Compact => "compact",
            Self::Full => "full",
        };
        f.write_str(text)
    }
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print the CLI capability manifest.
    Manifest,
    /// Write the managed component catalog into agent docs.
    Init {
        /// Project directory receiving the catalog.
        #[arg(long, value_name = "DIR", default_value = ".")]
        dir: PathBuf,
        /// Agent doc file name inside the directory.
        #[arg(long, value_name = "FILE", default_value = "AGENTS.md")]
        agents_file: String,
    },
    /// List components or print one component document.
    Component {
        /// Story id or title; omit to list components.
        name: Option<String>,
        /// Print only the props table; requires a name.
        #[arg(long, requires = "name")]
        props: bool,
    },
    /// Search components by free text.
    Search {
        /// Query matched against ids, titles, and descriptions.
        query: String,
        /// Maximum hits returned.
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
    },
    /// List templates, or show and copy one template.
    Template {
        /// Template id; omit to list templates.
        id: Option<String>,
        /// Print the structural skeleton instead of the source.
        #[arg(long, conflicts_with = "out")]
        skeleton: bool,
        /// Write the source into this directory instead of printing it.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// Output file name; defaults to `<id>.rs`.
        #[arg(long, value_name = "FILE", requires = "out")]
        file: Option<String>,
    },
    /// Copy component source into a directory to own it.
    Eject {
        /// Component name, such as `Button`.
        component: String,
        /// Directory receiving the ejected sources.
        #[arg(long, value_name = "DIR")]
        into: PathBuf,
    },
    /// Work with theme presets.
    Theme {
        /// Theme action to run.
        #[command(subcommand)]
        action: ThemeAction,
    },
    /// Run read-only health checks; exits 1 on failure.
    Doctor {
        /// Directory to diagnose; defaults to the working directory.
        #[arg(long, value_name = "DIR")]
        dir: Option<PathBuf>,
    },
    /// List registered migration notes.
    Upgrade,
    /// Render a prefilled missing-capability report.
    GapReport {
        /// Report area: component, template, theme, layout, cli, other.
        #[arg(long, value_name = "AREA")]
        area: String,
        /// Issue title.
        title: String,
    },
    /// Work with layout expressions.
    Layout {
        /// Layout action to run.
        #[command(subcommand)]
        action: LayoutAction,
    },
}

/// Available layout actions.
#[derive(Debug, Subcommand)]
pub enum LayoutAction {
    /// Validate an expression and echo its canonical form.
    Check {
        /// Layout expression to validate.
        expr: String,
    },
    /// Expand an expression to a Rust unit.
    Expand {
        /// Layout expression to expand.
        expr: String,
        /// Generated function name.
        #[arg(long, value_name = "NAME")]
        function: Option<String>,
        /// Write the unit into this directory instead of printing it.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// Output file name; defaults to `<function>.rs`.
        #[arg(long, value_name = "FILE", requires = "out")]
        file: Option<String>,
    },
    /// Print the normative grammar reference.
    Grammar,
}

/// Available theme actions.
#[derive(Debug, Subcommand)]
pub enum ThemeAction {
    /// List available theme presets.
    List,
    /// Export a preset to Style Dictionary JSON.
    Build {
        /// Preset id, such as `dark`.
        preset: String,
        /// Directory receiving the export.
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
        /// Output file name; defaults to `<preset>.tokens.json`.
        #[arg(long, value_name = "FILE")]
        file: Option<String>,
        /// Check freshness without writing anything.
        #[arg(long)]
        check: bool,
    },
    /// List the token surface of a preset.
    Targets {
        /// Preset id, such as `dark`.
        preset: String,
    },
}

/// Human-readable and JSON renderings of one command result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Terminal-oriented rendering.
    pub text: String,
    /// Typed JSON envelope rendering.
    pub json: String,
    /// Process exit code; nonzero only for failing reports.
    pub exit_code: i32,
}

impl CommandOutput {
    /// Successful result exiting 0.
    fn ok(text: String, json: String) -> Self {
        Self {
            text,
            json,
            exit_code: 0,
        }
    }

    /// Reported result with an explicit exit code.
    fn with_code(text: String, json: String, exit_code: i32) -> Self {
        Self {
            text,
            json,
            exit_code,
        }
    }
}

/// Converts a command-line detail flag into a list level.
fn detail_level(arg: DetailArg) -> DetailLevel {
    match arg {
        DetailArg::Brief => DetailLevel::Brief,
        DetailArg::Compact => DetailLevel::Compact,
        DetailArg::Full => DetailLevel::Full,
    }
}

/// Renders a payload envelope, honoring dense output.
fn render_typed<T: serde::Serialize>(
    kind: &str,
    data: &T,
    dense: bool,
) -> Result<String, ToolkitError> {
    let envelope = success_envelope(kind, data);
    let render = if dense {
        render_json_compact(&envelope)
    } else {
        render_json(&envelope)
    };
    render.map_err(|error| {
        ToolkitError::new(
            ErrorCode::Unknown,
            format!("failed to render {kind}: {error}"),
        )
    })
}

/// Runs the parsed subcommand and renders both outputs.
///
/// # Errors
///
/// Returns the subcommand's [`ToolkitError`] unchanged; callers decide
/// how to present it (human text on stderr or a JSON error envelope).
///
/// # Examples
///
/// ```rust
/// use clap::Parser;
/// use gpui_toolkit_cli::{Cli, run_command};
///
/// let cli = Cli::try_parse_from(["gpui-toolkit", "manifest"]).unwrap();
/// let output = run_command(&cli).unwrap();
/// assert!(output.json.contains("\"type\": \"manifest\""));
/// ```
pub fn run_command(cli: &Cli) -> Result<CommandOutput, ToolkitError> {
    match &cli.command {
        Commands::Manifest => {
            let command = Cli::command();
            let manifest = build_manifest(&command);
            let mut text = format!(
                "{} {} — {}\n",
                manifest.name, manifest.version, manifest.description
            );
            for entry in &manifest.commands {
                text.push_str(&format!("\n{} — {}\n", entry.name, entry.description));
                for example in &entry.examples {
                    text.push_str(&format!("  {example}\n"));
                }
            }
            let json = render_typed(crate::MANIFEST_TYPE, &manifest, cli.dense)?;
            Ok(CommandOutput::ok(text, json))
        }
        Commands::Init { dir, agents_file } => {
            let receipt = run_init(&InitOptions {
                dir: dir.clone(),
                agents_file: agents_file.clone(),
            })?;
            let action = if receipt.created {
                "created"
            } else if receipt.updated {
                "refreshed managed block in"
            } else {
                "appended catalog to"
            };
            let text = format!(
                "Wrote {} stories to {} ({})",
                receipt.stories,
                receipt.path.display(),
                action,
            );
            let json = render_typed(crate::INIT_TYPE, &receipt, cli.dense)?;
            Ok(CommandOutput::ok(text, json))
        }
        Commands::Component { name, props } => match name {
            None => {
                let list = component_list(detail_level(cli.detail))?;
                let text = render_list_text(&list, cli.dense);
                let json = render_typed(COMPONENT_LIST_TYPE, &list, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
            Some(name) if *props => {
                let props = component_props(name)?;
                let text = render_props_text(&props, cli.dense);
                let json = render_typed(COMPONENT_PROPS_TYPE, &props, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
            Some(name) => {
                let detail = component_detail(name)?;
                let text = render_detail_text(&detail, cli.dense);
                let json = render_typed(COMPONENT_DETAIL_TYPE, &detail, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
        },
        Commands::Search { query, limit } => {
            let results = search(query, limit.unwrap_or(DEFAULT_SEARCH_LIMIT))?;
            let text = render_search_text(&results, cli.dense);
            let json = render_typed(SEARCH_TYPE, &results, cli.dense)?;
            Ok(CommandOutput::ok(text, json))
        }
        Commands::Template {
            id,
            skeleton,
            out,
            file,
        } => match (id, out) {
            (None, None) => {
                let list = template_list();
                let text = render_template_list_text(&list, cli.dense);
                let json = render_typed(TEMPLATE_LIST_TYPE, &list, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
            (None, Some(_)) => Err(ToolkitError::new(
                ErrorCode::InvalidArgument,
                "template --out requires a template id",
            )),
            (Some(id), Some(out)) => {
                let copy = template_copy(id, out, file.as_deref())?;
                let text = render_template_copy_text(&copy);
                let json = render_typed(TEMPLATE_COPY_TYPE, &copy, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
            (Some(id), None) if *skeleton => {
                let skeleton = template_skeleton(id)?;
                let json = render_typed(TEMPLATE_SKELETON_TYPE, &skeleton, cli.dense)?;
                Ok(CommandOutput::ok(skeleton.skeleton.clone(), json))
            }
            (Some(id), None) => {
                let shown = template_show(id)?;
                let json = render_typed(TEMPLATE_SHOW_TYPE, &shown, cli.dense)?;
                Ok(CommandOutput::ok(shown.source.clone(), json))
            }
        },
        Commands::Eject { component, into } => {
            let receipt = eject_component(component, into)?;
            let text = render_eject_text(&receipt, cli.dense);
            let json = render_typed(EJECT_TYPE, &receipt, cli.dense)?;
            Ok(CommandOutput::ok(text, json))
        }
        Commands::Theme { action } => match action {
            ThemeAction::List => {
                let list = theme_list();
                let text = render_theme_list_text(&list, cli.dense);
                let json = render_typed(THEME_LIST_TYPE, &list, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
            ThemeAction::Build {
                preset,
                out,
                file,
                check,
            } => {
                if *check {
                    let checked = theme_check(preset, out, file.as_deref())?;
                    let text = render_theme_check_text(&checked);
                    let json = render_typed(THEME_CHECK_TYPE, &checked, cli.dense)?;
                    Ok(CommandOutput::ok(text, json))
                } else {
                    let build = theme_build(preset, out, file.as_deref())?;
                    let text = render_theme_build_text(&build);
                    let json = render_typed(THEME_BUILD_TYPE, &build, cli.dense)?;
                    Ok(CommandOutput::ok(text, json))
                }
            }
            ThemeAction::Targets { preset } => {
                let targets = theme_targets(preset)?;
                let text = render_theme_targets_text(&targets, cli.dense);
                let json = render_typed(THEME_TARGETS_TYPE, &targets, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
        },
        Commands::Doctor { dir } => {
            let dir = match dir {
                Some(dir) => dir.clone(),
                None => std::env::current_dir().map_err(|error| {
                    ToolkitError::new(
                        ErrorCode::Unknown,
                        format!("cannot determine working directory: {error}"),
                    )
                })?,
            };
            let report = run_doctor(&dir);
            let text = render_doctor_text(&report, cli.dense);
            let json = render_typed(DOCTOR_TYPE, &report, cli.dense)?;
            let exit_code = i32::from(report.failed());
            Ok(CommandOutput::with_code(text, json, exit_code))
        }
        Commands::Upgrade => {
            let list = upgrade_list();
            let text = render_upgrade_list_text(&list, cli.dense);
            let json = render_typed(UPGRADE_LIST_TYPE, &list, cli.dense)?;
            Ok(CommandOutput::ok(text, json))
        }
        Commands::GapReport { area, title } => {
            let report = gap_report(area, title)?;
            let text = render_gap_text(&report, cli.dense);
            let json = render_typed(GAP_REPORT_TYPE, &report, cli.dense)?;
            Ok(CommandOutput::ok(text, json))
        }
        Commands::Layout { action } => match action {
            LayoutAction::Check { expr } => {
                let checked = layout_check(expr)?;
                let text = render_layout_check_text(&checked, cli.dense);
                let json = render_typed(LAYOUT_CHECK_TYPE, &checked, cli.dense)?;
                Ok(CommandOutput::ok(text, json))
            }
            LayoutAction::Expand {
                expr,
                function,
                out,
                file,
            } => {
                if let Some(out) = out {
                    let expanded =
                        layout_expand_to_file(expr, function.as_deref(), out, file.as_deref())?;
                    let text = render_layout_expand_text(&expanded);
                    let json = render_typed(LAYOUT_EXPAND_TYPE, &expanded, cli.dense)?;
                    Ok(CommandOutput::ok(text, json))
                } else {
                    let rust = layout_expand(expr, function.as_deref())?;
                    let expanded = LayoutExpand {
                        expr: expr.clone(),
                        function: function
                            .clone()
                            .unwrap_or_else(|| DEFAULT_LAYOUT_FN.to_owned()),
                        bytes: rust.len(),
                        rust: rust.clone(),
                        path: None,
                    };
                    let json = render_typed(LAYOUT_EXPAND_TYPE, &expanded, cli.dense)?;
                    Ok(CommandOutput::ok(rust, json))
                }
            }
            LayoutAction::Grammar => {
                let grammar = layout_grammar();
                let json = render_typed(LAYOUT_GRAMMAR_TYPE, &grammar, cli.dense)?;
                Ok(CommandOutput::ok(grammar.grammar.clone(), json))
            }
        },
    }
}
