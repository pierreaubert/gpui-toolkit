use anyhow::Result;
use clap::Parser;
use gpui_scaffolder::{ScaffoldOptions, scaffold_app};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "gpui-scaffolder",
    about = "Create a standalone GPUI mini-app project"
)]
struct Args {
    /// Directory and app name to create.
    name: String,

    /// Parent directory for the generated app.
    #[arg(long, value_name = "DIR", default_value = ".")]
    output_dir: PathBuf,

    /// Replace an existing empty directory.
    #[arg(long)]
    force: bool,

    /// Validate and print what would be created without writing files.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let scaffolded = scaffold_app(&ScaffoldOptions {
        name: args.name,
        output_dir: args.output_dir,
        force: args.force,
        dry_run: args.dry_run,
    })?;

    if args.dry_run {
        println!("Would create {}", scaffolded.app_dir.display());
        return Ok(());
    }

    println!("Created {}", scaffolded.app_dir.display());
    println!("Run it with:");
    println!("  cd {}", scaffolded.app_dir.display());
    println!("  cargo run");
    println!("  just run");

    Ok(())
}
