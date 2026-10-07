//! `gpui-toolkit` command-line interface.
//!
//! Thin wrapper over [`gpui_toolkit_cli`]: parses arguments, runs one
//! library call, then prints human-readable text or a typed JSON
//! envelope. Exit codes: 0 on success, 1 on failure, 2 on command-line
//! usage errors (from clap).

// Rust guideline compliant 2026-02-21

use clap::Parser;
use gpui_toolkit_cli::{Cli, error_envelope, render_json, render_json_compact, run_command};

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    match run_command(&cli) {
        Ok(output) => {
            if json {
                println!("{}", output.json);
            } else {
                println!("{}", output.text);
            }
            if output.exit_code != 0 {
                std::process::exit(output.exit_code);
            }
        }
        Err(error) => {
            if json {
                let envelope = error_envelope(&error);
                let rendered = if cli.dense {
                    render_json_compact(&envelope)
                } else {
                    render_json(&envelope)
                };
                match rendered {
                    Ok(text) => println!("{text}"),
                    Err(render) => {
                        eprintln!("failed to render error envelope: {render}");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("{error}");
            }
            std::process::exit(1);
        }
    }
}
