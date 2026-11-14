use anyhow::Result;
use clap::Parser;
use gpui_scaffolder::{
    ScaffoldFlags, ScaffoldOptions, preview_scaffold_with_flags, scaffold_app_with_flags,
};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "gpui-scaffolder",
    about = "Create a standalone GPUI mini-app project",
    after_long_help = "Set GPUI_TOOLKIT_ROOT when the executable is not located inside a gpui-toolkit checkout."
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

    /// Scaffold template to use: "default" (native Rust app) or
    /// "python-mobile" (Python-authored iOS/Android app).
    #[arg(long, value_name = "TEMPLATE", default_value = "default")]
    template: String,

    /// Skip the iOS host (ios/, gpui-ios dependency, iOS Just recipes).
    #[arg(long)]
    no_ios: bool,

    /// Skip the Android host (android/, gpui-android dependency, Android Just recipes).
    #[arg(long)]
    no_android: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let is_python_mobile = args.template == gpui_scaffolder::PYTHON_MOBILE_TEMPLATE;
    let options = ScaffoldOptions {
        name: args.name,
        output_dir: args.output_dir,
        force: args.force,
        dry_run: args.dry_run,
    };
    let flags = ScaffoldFlags {
        template: args.template,
        no_ios: args.no_ios,
        no_android: args.no_android,
    };

    if args.dry_run {
        let preview = preview_scaffold_with_flags(&options, &flags)?;
        println!(
            "Would create {} ({} files):",
            preview.app.app_dir.display(),
            preview.files.len()
        );
        for file in &preview.files {
            match file.strip_prefix(&preview.app.app_dir) {
                Ok(relative) => println!("  {}", relative.display()),
                Err(_) => println!("  {}", file.display()),
            }
        }
        return Ok(());
    }

    let scaffolded = scaffold_app_with_flags(&options, &flags)?;

    println!("Created {}", scaffolded.app_dir.display());
    println!("Run it with:");
    println!("  cd {}", scaffolded.app_dir.display());
    if is_python_mobile {
        println!("  just preflight");
        println!("  just ios-sim");
        println!("  just android-apk");
    } else {
        println!("  cargo run");
        println!("  just run");
    }

    Ok(())
}
