use anyhow::Result;
use clap::Parser;
use gpui_design_tools::{DesignTokenFormat, diff_design_tokens_from_paths};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "gpui-diff-design-tokens",
    about = "Diff two GPUI design token documents and report additive vs breaking changes.",
    long_about = "Diff two token JSON files in the same wire format (Style Dictionary or W3C DTCG). Added tokens are safe; removed tokens and changed values or types are breaking and fail the command for CI gating."
)]
struct Args {
    #[arg(help = "Old (baseline) token JSON file")]
    old: PathBuf,
    #[arg(help = "New (candidate) token JSON file")]
    new: PathBuf,
    #[arg(
        short,
        long,
        default_value = "style-dictionary-json",
        help = "Token wire format: style-dictionary-json or w3c-dtcg-json"
    )]
    format: String,
    #[arg(long, help = "Print the machine-readable JSON diff to stdout")]
    json: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format = DesignTokenFormat::parse(&args.format)?;
    let diff = diff_design_tokens_from_paths(&args.old, &args.new, format)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&diff)?);
    } else {
        print!("{}", diff.to_markdown());
    }
    if diff.is_breaking() {
        anyhow::bail!(
            "breaking design token changes: {} removed, {} changed, {} added",
            diff.removed.len(),
            diff.changed.len(),
            diff.added.len()
        );
    }
    println!("No breaking design token changes ({} added).", diff.added.len());
    Ok(())
}
