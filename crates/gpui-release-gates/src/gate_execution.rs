//! Execute recorded release-gate commands so CI refreshes evidence instead of
//! trusting stale `evidence` strings.
//!
//! Only the allowlisted `cargo publish --dry-run --locked -p <crate>` shape is
//! executable; manual-action rows (installers, archives, external gates) parse
//! to `None` and must stay human-attested. Execution never uses a shell: the
//! recorded command is split into argv and spawned directly.

use std::io;
use std::process::Command;

use crate::release_packaging_entries;

/// A recorded gate command parsed into executable argv (no shell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableGateCommand {
    /// Binary to spawn (always `cargo` for allowlisted rows).
    pub program: String,
    /// Arguments without the program name.
    pub args: Vec<String>,
}

/// Outcome of executing one gate command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateExecutionOutput {
    /// Command that was spawned, joined for logs.
    pub command: String,
    /// Whether the process exited successfully.
    pub success: bool,
    /// Process exit code, when available.
    pub code: Option<i32>,
    /// Captured stdout (lossy).
    pub stdout: String,
    /// Captured stderr (lossy).
    pub stderr: String,
}

/// Split a command line into argv honoring single/double quotes (no shell
/// expansion, no escapes beyond `\\` inside double quotes).
fn split_argv(command: &str) -> Vec<String> {
    let mut argv = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut in_word = false;

    for c in command.chars() {
        match (quote, c) {
            (None, '\'') => {
                quote = Some('\'');
                in_word = true;
            }
            (None, '"') => {
                quote = Some('"');
                in_word = true;
            }
            (Some(q), c) if c == q => quote = None,
            (None, c) if c.is_whitespace() => {
                if in_word {
                    argv.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            (_, c) => {
                current.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        argv.push(current);
    }
    argv
}

/// Parse a recorded `command_or_action` into executable argv when it matches
/// the allowlisted `cargo publish --dry-run --locked -p <crate>` shape.
/// Returns `None` for manual actions, so callers fail closed instead of
/// executing prose.
pub fn parse_executable_gate_command(command: &str) -> Option<ExecutableGateCommand> {
    let argv = split_argv(command);
    let (program, args) = argv.split_first()?;
    if program != "cargo" {
        return None;
    }
    if !args.contains(&"--dry-run".to_string()) {
        return None;
    }
    let mut crate_name = None;
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "-p" || arg == "--package" {
            crate_name = iter.next().cloned();
        }
    }
    crate_name?;
    Some(ExecutableGateCommand {
        program: program.clone(),
        args: args.to_vec(),
    })
}

/// Run previously parsed argv without a shell, capturing output.
pub fn execute_gate_argv(command: &ExecutableGateCommand) -> io::Result<GateExecutionOutput> {
    let output = Command::new(&command.program)
        .args(&command.args)
        .output()?;
    let rendered = std::iter::once(command.program.clone())
        .chain(command.args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(GateExecutionOutput {
        command: rendered,
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Find the packaging entry `id` and execute it when its recorded command is
/// allowlisted. Returns `Ok(None)` for unknown ids and for manual-action rows.
pub fn execute_packaging_entry(id: &str) -> io::Result<Option<GateExecutionOutput>> {
    let entry = release_packaging_entries()
        .iter()
        .find(|entry| entry.id == id);
    let Some(entry) = entry else {
        return Ok(None);
    };
    let Some(command) = parse_executable_gate_command(entry.command_or_action) else {
        return Ok(None);
    };
    execute_gate_argv(&command).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_rows_parse_to_cargo_argv() {
        for id in [
            "gpui-design-dry-run",
            "gpui-profiler-dry-run",
            "gpui-pretext-dry-run",
            "gpui-builder-dry-run",
            "gpui-ui-kit-macros-dry-run",
        ] {
            let entry = release_packaging_entries()
                .iter()
                .find(|entry| entry.id == id)
                .expect("packaging row exists");
            let parsed = parse_executable_gate_command(entry.command_or_action)
                .expect("dry-run row is executable");
            assert_eq!(parsed.program, "cargo");
            assert!(parsed.args.contains(&"--dry-run".to_string()));
            assert!(parsed.args.contains(&"-p".to_string()));
        }
    }

    #[test]
    fn manual_action_rows_are_not_executable() {
        for id in [
            "platform-installers",
            "internal-aggregate-and-apps",
            "vendored-patches",
            "beta-visualization-dry-runs",
        ] {
            let entry = release_packaging_entries()
                .iter()
                .find(|entry| entry.id == id)
                .expect("packaging row exists");
            assert!(
                parse_executable_gate_command(entry.command_or_action).is_none(),
                "{id} must stay human-attested"
            );
        }
    }

    #[test]
    fn non_cargo_commands_are_rejected() {
        assert!(parse_executable_gate_command("make install").is_none());
        assert!(parse_executable_gate_command("cargo build -p foo").is_none());
        assert!(parse_executable_gate_command("").is_none());
    }

    #[test]
    fn argv_runner_reports_hermetic_binary() {
        let exe = std::env::current_exe().expect("test binary path");
        let command = ExecutableGateCommand {
            program: exe.to_string_lossy().into_owned(),
            args: vec!["--list".to_string()],
        };
        let output = execute_gate_argv(&command).expect("spawn test binary");
        assert!(output.success);
        assert!(output.command.contains("--list"));
    }

    #[test]
    fn unknown_entry_id_executes_to_none() {
        let output = execute_packaging_entry("no-such-row").expect("unknown id is not an io error");
        assert!(output.is_none());
    }

    #[test]
    fn manual_entry_id_executes_to_none() {
        let output =
            execute_packaging_entry("platform-installers").expect("manual row is not an io error");
        assert!(output.is_none());
    }
}
