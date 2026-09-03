use anyhow::Result;
use clap::Parser;
use gpui_design_tools::{
    DesignTokenExportOptions, DesignTokenFormat, export_design_tokens_to_path_with_options,
};
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
    #[arg(short, long, default_value = "design-tokens/gpui-tokens.json")]
    output: PathBuf,
    #[arg(long, help = "Emit compact JSON instead of pretty-printed JSON")]
    compact: bool,
    #[arg(
        long,
        help = "fsync the temporary file before replacing the destination (slower, crash-durable)"
    )]
    durable: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format = DesignTokenFormat::parse(&args.format)?;
    let options = if args.compact {
        DesignTokenExportOptions::compact()
    } else {
        DesignTokenExportOptions::pretty()
    };
    export_design_tokens_to_path_with_options(&args.output, format, options, args.durable)?;
    println!("Wrote {}", args.output.display());
    Ok(())
}
