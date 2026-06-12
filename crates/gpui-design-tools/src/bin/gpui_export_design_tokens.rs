use anyhow::Result;
use clap::Parser;
use gpui_design_tools::{DesignTokenFormat, export_design_tokens_to_path};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "style-dictionary-json")]
    format: String,
    #[arg(short, long, default_value = "design-tokens/gpui-tokens.json")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format = DesignTokenFormat::parse(&args.format)?;
    export_design_tokens_to_path(&args.output, format)?;
    println!("Wrote {}", args.output.display());
    Ok(())
}
