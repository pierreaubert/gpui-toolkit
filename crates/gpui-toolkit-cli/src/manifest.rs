//! Self-describing CLI capability manifest.
//!
//! [`build_manifest`] walks the clap command tree so mechanical facts
//! (names, descriptions, flags, arguments) cannot drift from the parser.
//! Hand-authored facts live in exactly one place: [`COMMANDS`], which
//! records per-command JSON support, response types, and usage examples.
//! A drift test asserts both directions — every clap subcommand appears
//! in [`COMMANDS`] with at least one example, and every table entry
//! names a real subcommand — so adding a command without documenting it
//! fails CI.

// Rust guideline compliant 2026-02-21

use crate::{
    COMPONENT_BATCH_TYPE, COMPONENT_DETAIL_TYPE, COMPONENT_LIST_TYPE, COMPONENT_PROPS_TYPE,
    DOCTOR_TYPE, EJECT_TYPE, GAP_REPORT_TYPE, INIT_TYPE, LAYOUT_CHECK_TYPE, LAYOUT_EXPAND_TYPE,
    LAYOUT_GRAMMAR_TYPE, SEARCH_TYPE, TEMPLATE_COPY_TYPE, TEMPLATE_LIST_TYPE, TEMPLATE_SHOW_TYPE,
    TEMPLATE_SKELETON_TYPE, THEME_BUILD_TYPE, THEME_CHECK_TYPE, THEME_LIST_TYPE,
    THEME_TARGETS_TYPE, UPGRADE_DETECT_TYPE, UPGRADE_LIST_TYPE,
};
use clap::{Arg, ArgAction, Command};
use serde::Serialize;

/// Discriminator for manifest payloads in JSON envelopes.
pub const MANIFEST_TYPE: &str = "manifest";

/// Hand-authored facts for one subcommand.
///
/// Mechanical facts (description, flags) derive from clap; this table
/// carries what clap cannot know: whether the command honors `--json`,
/// the response types it emits, and the examples agents copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandStatic {
    /// Subcommand name as spelled on the command line.
    pub name: &'static str,
    /// Whether `--json` yields a typed envelope for this command.
    pub json: bool,
    /// Response discriminators, referencing the `*_TYPE` constants, plus
    /// `"error"` for the failure envelope; empty for bare groups.
    pub responses: &'static [&'static str],
    /// Copy-pasteable invocations; never empty.
    pub examples: &'static [&'static str],
}

/// Hand-authored command facts, one entry per command path.
///
/// Top-level commands use their bare name; nested groups join with
/// spaces (`theme build`). Bare groups carry `json: false` since only
/// leaves run. This table is the only hand-maintained manifest input.
/// The drift test in `tests/cli.rs` fails when a clap command is missing
/// here, when an entry names no real command, or when `examples` is
/// empty.
pub const COMMANDS: &[CommandStatic] = &[
    CommandStatic {
        name: "manifest",
        json: true,
        responses: &[MANIFEST_TYPE, "error"],
        examples: &["gpui-toolkit manifest", "gpui-toolkit manifest --json"],
    },
    CommandStatic {
        name: "init",
        json: true,
        responses: &[INIT_TYPE, "error"],
        examples: &[
            "gpui-toolkit init --dir ./my-app",
            "gpui-toolkit init --dir ./my-app --json",
        ],
    },
    CommandStatic {
        name: "component",
        json: true,
        responses: &[
            COMPONENT_LIST_TYPE,
            COMPONENT_DETAIL_TYPE,
            COMPONENT_PROPS_TYPE,
            COMPONENT_BATCH_TYPE,
            "error",
        ],
        examples: &[
            "gpui-toolkit component --detail compact",
            "gpui-toolkit component ui-kit.button",
            "gpui-toolkit component ui-kit.button --props --json",
            "gpui-toolkit component ui-kit.button ui-kit.input --json",
        ],
    },
    CommandStatic {
        name: "search",
        json: true,
        responses: &[SEARCH_TYPE, "error"],
        examples: &[
            "gpui-toolkit search button",
            "gpui-toolkit search meter --limit 5 --json",
        ],
    },
    CommandStatic {
        name: "template",
        json: true,
        responses: &[
            TEMPLATE_LIST_TYPE,
            TEMPLATE_SHOW_TYPE,
            TEMPLATE_SKELETON_TYPE,
            TEMPLATE_COPY_TYPE,
            "error",
        ],
        examples: &[
            "gpui-toolkit template",
            "gpui-toolkit template settings-page --skeleton",
            "gpui-toolkit template dashboard-page --out ./src --json",
        ],
    },
    CommandStatic {
        name: "eject",
        json: true,
        responses: &[EJECT_TYPE, "error"],
        examples: &[
            "gpui-toolkit eject Button --into ./src/vendor",
            "gpui-toolkit eject NumberInput --into ./src/vendor --json",
        ],
    },
    CommandStatic {
        name: "theme",
        json: false,
        responses: &[],
        examples: &["gpui-toolkit theme list"],
    },
    CommandStatic {
        name: "theme list",
        json: true,
        responses: &[THEME_LIST_TYPE, "error"],
        examples: &["gpui-toolkit theme list", "gpui-toolkit theme list --json"],
    },
    CommandStatic {
        name: "theme build",
        json: true,
        responses: &[THEME_BUILD_TYPE, THEME_CHECK_TYPE, "error"],
        examples: &[
            "gpui-toolkit theme build dark --out ./tokens",
            "gpui-toolkit theme build dark --out ./tokens --check --json",
        ],
    },
    CommandStatic {
        name: "theme targets",
        json: true,
        responses: &[THEME_TARGETS_TYPE, "error"],
        examples: &[
            "gpui-toolkit theme targets dark",
            "gpui-toolkit theme targets nord --json",
        ],
    },
    CommandStatic {
        name: "doctor",
        json: true,
        responses: &[DOCTOR_TYPE, "error"],
        examples: &["gpui-toolkit doctor", "gpui-toolkit doctor --json"],
    },
    CommandStatic {
        name: "upgrade",
        json: true,
        responses: &[UPGRADE_LIST_TYPE, UPGRADE_DETECT_TYPE, "error"],
        examples: &[
            "gpui-toolkit upgrade",
            "gpui-toolkit upgrade --json",
            "gpui-toolkit upgrade --detect --dir ./src",
        ],
    },
    CommandStatic {
        name: "gap-report",
        json: true,
        responses: &[GAP_REPORT_TYPE, "error"],
        examples: &[
            "gpui-toolkit gap-report --area component DateRangePicker",
            "gpui-toolkit gap-report --area theme \"Dark sidebar\" --json",
        ],
    },
    CommandStatic {
        name: "layout",
        json: false,
        responses: &[],
        examples: &["gpui-toolkit layout check \"V > B\\\"Hi\\\"\""],
    },
    CommandStatic {
        name: "layout check",
        json: true,
        responses: &[LAYOUT_CHECK_TYPE, "error"],
        examples: &[
            "gpui-toolkit layout check \"V > B\\\"Hi\\\"\"",
            "gpui-toolkit layout check \"V > B\\\"Hi\\\"\" --json",
        ],
    },
    CommandStatic {
        name: "layout expand",
        json: true,
        responses: &[LAYOUT_EXPAND_TYPE, "error"],
        examples: &[
            "gpui-toolkit layout expand \"V > B\\\"Hi\\\"\"",
            "gpui-toolkit layout expand \"V > B\\\"Hi\\\"\" --out ./src --json",
        ],
    },
    CommandStatic {
        name: "layout grammar",
        json: true,
        responses: &[LAYOUT_GRAMMAR_TYPE, "error"],
        examples: &["gpui-toolkit layout grammar"],
    },
];

/// One manifest option or flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestOption {
    /// Flag spelling, such as `--json`.
    pub flag: String,
    /// Value kind: `boolean` for switches, `string` otherwise.
    #[serde(rename = "type")]
    pub kind: String,
    /// Help text from the parser, empty when undocumented.
    pub description: String,
}

/// One manifest positional argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestArgument {
    /// Argument name as spelled in help.
    pub name: String,
    /// Whether the parser requires this argument.
    pub required: bool,
    /// Help text from the parser, empty when undocumented.
    pub description: String,
}

/// Manifest entry describing one subcommand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCommand {
    /// Subcommand name as spelled on the command line.
    pub name: String,
    /// Short help from the parser.
    pub description: String,
    /// Positional arguments in declaration order.
    pub arguments: Vec<ManifestArgument>,
    /// Flags and options in declaration order.
    pub options: Vec<ManifestOption>,
    /// Whether `--json` yields a typed envelope.
    pub json: bool,
    /// Response discriminators from [`COMMANDS`].
    pub response_types: Vec<String>,
    /// Copy-pasteable invocations from [`COMMANDS`].
    pub examples: Vec<String>,
    /// Nested subcommands; empty for leaves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<ManifestCommand>,
}

/// One process exit code and its meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestExitCode {
    /// Numeric exit code.
    pub code: i32,
    /// What the code signals.
    pub meaning: String,
}

/// Full CLI capability manifest.
///
/// Agents read this instead of scraping `--help`: names, flags with
/// types, JSON support, response types, exit codes, and examples in one
/// typed payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// Binary name.
    pub name: String,
    /// Crate version.
    pub version: String,
    /// Short help from the parser.
    pub description: String,
    /// Flags accepted by every subcommand.
    pub global_options: Vec<ManifestOption>,
    /// One entry per subcommand, in declaration order.
    pub commands: Vec<ManifestCommand>,
    /// Names of commands honoring `--json`.
    pub json_supported: Vec<String>,
    /// Process exit codes shared by every command.
    pub exit_codes: Vec<ManifestExitCode>,
}

/// Looks up hand-authored facts for a command path.
///
/// Paths join nesting with spaces (`theme build`); top-level commands
/// use their bare name.
///
/// # Panics
///
/// Panics when `path` has no [`COMMANDS`] entry. That is a programming
/// bug — clap exposes a command the manifest table does not know —
/// and the drift test catches it before release.
pub fn command_static(path: &str) -> &'static CommandStatic {
    match COMMANDS.iter().find(|entry| entry.name == path) {
        Some(entry) => entry,
        None => unreachable!("manifest table is missing command '{path}'"),
    }
}

/// Builds a manifest by walking the clap command tree.
///
/// Names, descriptions, flags, and arguments derive from `command`, so
/// they cannot drift from the real parser; JSON support and examples
/// come from [`COMMANDS`].
///
/// # Examples
///
/// ```rust
/// use clap::Command;
/// use gpui_toolkit_cli::build_manifest;
///
/// let command = Command::new("demo").subcommand(Command::new("init"));
/// let manifest = build_manifest(&command);
/// assert_eq!(manifest.commands.len(), 1);
/// assert_eq!(manifest.commands[0].name, "init");
/// ```
pub fn build_manifest(command: &Command) -> Manifest {
    let global_options = command
        .get_arguments()
        .filter(|arg| !arg.is_positional())
        .map(describe_option)
        .collect();
    let mut commands = Vec::new();
    let mut json_supported = Vec::new();
    for subcommand in command.get_subcommands() {
        commands.push(build_command(
            subcommand,
            subcommand.get_name().to_owned(),
            &mut json_supported,
        ));
    }
    Manifest {
        name: command.get_name().to_owned(),
        version: command
            .get_version()
            .map(ToString::to_string)
            .unwrap_or_default(),
        description: command
            .get_about()
            .map(ToString::to_string)
            .unwrap_or_default(),
        global_options,
        commands,
        json_supported,
        exit_codes: exit_codes(),
    }
}

/// Process exit codes shared by every command.
///
/// Mirrors `src/main.rs` plus clap usage errors: 0 succeeds, 1 fails
/// (including doctor reports with failures), 2 rejects the invocation.
fn exit_codes() -> Vec<ManifestExitCode> {
    [
        (0, "success"),
        (1, "command failed, or doctor reported failures"),
        (2, "usage error: invalid flags or arguments"),
    ]
    .into_iter()
    .map(|(code, meaning)| ManifestExitCode {
        code,
        meaning: meaning.to_owned(),
    })
    .collect()
}

/// Builds one manifest entry, recursing into nested groups.
///
/// `path` joins nesting with spaces for [`COMMANDS`] lookup; commands
/// honoring `--json` append their path to `json_supported`.
fn build_command(
    command: &Command,
    path: String,
    json_supported: &mut Vec<String>,
) -> ManifestCommand {
    let facts = command_static(&path);
    let arguments = command
        .get_arguments()
        .filter(|arg| arg.is_positional())
        .map(|arg| ManifestArgument {
            name: arg.get_id().to_string(),
            required: arg.is_required_set(),
            description: arg.get_help().map(ToString::to_string).unwrap_or_default(),
        })
        .collect();
    let options = command
        .get_arguments()
        .filter(|arg| !arg.is_positional())
        .map(describe_option)
        .collect();
    let subcommands = command
        .get_subcommands()
        .map(|subcommand| {
            build_command(
                subcommand,
                format!("{path} {}", subcommand.get_name()),
                json_supported,
            )
        })
        .collect();
    if facts.json {
        json_supported.push(path);
    }
    ManifestCommand {
        name: command.get_name().to_owned(),
        description: command
            .get_about()
            .map(ToString::to_string)
            .unwrap_or_default(),
        arguments,
        options,
        json: facts.json,
        response_types: facts.responses.iter().map(ToString::to_string).collect(),
        examples: facts.examples.iter().map(ToString::to_string).collect(),
        subcommands,
    }
}

/// Describes one clap flag or option for the manifest.
fn describe_option(arg: &Arg) -> ManifestOption {
    let flag = match arg.get_long() {
        Some(long) => format!("--{long}"),
        None => match arg.get_short() {
            Some(short) => format!("-{short}"),
            None => format!("--{}", arg.get_id()),
        },
    };
    let kind = match arg.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count => "boolean",
        ArgAction::Set
        | ArgAction::Append
        | ArgAction::Help
        | ArgAction::HelpShort
        | ArgAction::HelpLong
        | ArgAction::Version => "string",
        // ArgAction is non-exhaustive: a future clap action must fail
        // loudly here so the manifest learns its type explicitly.
        _ => panic!("unsupported clap ArgAction in manifest"),
    };
    ManifestOption {
        flag,
        kind: kind.to_owned(),
        description: arg.get_help().map(ToString::to_string).unwrap_or_default(),
    }
}
