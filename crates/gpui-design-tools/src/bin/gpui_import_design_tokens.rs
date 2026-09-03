use anyhow::Result;
use clap::Parser;
use gpui_design_tools::{DesignTokenFormat, import_design_tokens_from_path};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "style-dictionary-json",
        help = "Token wire format: style-dictionary-json or w3c-dtcg-json"
    )]
    format: String,
    #[arg(short, long)]
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format = DesignTokenFormat::parse(&args.format)?;
    let imported = import_design_tokens_from_path(&args.input, format)?;
    println!(
        "Imported {} presets and {} tokens from {}",
        imported.preset_count,
        imported.token_count,
        args.input.display()
    );
    Ok(())
}
