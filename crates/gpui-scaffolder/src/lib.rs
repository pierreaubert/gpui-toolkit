use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ScaffoldOptions {
    pub name: String,
    pub output_dir: PathBuf,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldedApp {
    pub app_dir: PathBuf,
    pub package_name: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppNames {
    directory_name: String,
    package_name: String,
    title: String,
    view_name: String,
}

pub fn scaffold_app(options: &ScaffoldOptions) -> Result<ScaffoldedApp> {
    let names = AppNames::new(&options.name)?;
    let output_dir = options
        .output_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", options.output_dir.display()))?;
    let app_dir = output_dir.join(&names.directory_name);

    if app_dir.exists() {
        if options.force {
            ensure_directory_is_replaceable(&app_dir)?;
            fs::remove_dir_all(&app_dir)
                .with_context(|| format!("failed to remove {}", app_dir.display()))?;
        } else {
            bail!(
                "{} already exists; pass --force to replace an empty scaffold",
                app_dir.display()
            );
        }
    }

    fs::create_dir_all(app_dir.join("src"))
        .with_context(|| format!("failed to create {}", app_dir.display()))?;

    let toolkit_root = toolkit_root()?;
    let miniapp_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-miniapp"));

    write_file(
        &app_dir.join("Cargo.toml"),
        &cargo_toml(&names, &miniapp_path),
    )?;
    write_file(&app_dir.join("Justfile"), &justfile())?;
    write_file(&app_dir.join("README.md"), &readme(&names))?;
    write_file(&app_dir.join("src/main.rs"), &main_rs(&names))?;

    Ok(ScaffoldedApp {
        app_dir,
        package_name: names.package_name,
        title: names.title,
    })
}

fn ensure_directory_is_replaceable(path: &Path) -> Result<()> {
    if !path.is_dir() {
        bail!("{} exists and is not a directory", path.display());
    }

    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?;
    if entries.next().is_some() {
        bail!("{} is not empty; refusing to replace it", path.display());
    }

    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn toolkit_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .context("failed to resolve gpui-toolkit root")
        .and_then(|root| {
            root.canonicalize()
                .with_context(|| format!("failed to resolve {}", root.display()))
        })
}

impl AppNames {
    fn new(name: &str) -> Result<Self> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("app name cannot be empty");
        }
        validate_directory_name(trimmed)?;

        let package_name = package_name(trimmed);
        let title = title_case(trimmed);
        let view_name = format!("{}View", pascal_case(&package_name));

        Ok(Self {
            directory_name: trimmed.to_owned(),
            package_name,
            title,
            view_name,
        })
    }
}

fn validate_directory_name(name: &str) -> Result<()> {
    let path = Path::new(name);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => bail!("app name must be a single directory name"),
    }
}

fn package_name(name: &str) -> String {
    let mut package = separated_identifier(name, '-').to_ascii_lowercase();
    if package.is_empty() {
        package.push_str("gpui-app");
    }

    if !package
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
    {
        package.insert_str(0, "app-");
    }

    package
}

fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    for part in name.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if part.is_empty() {
            continue;
        }

        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars.map(|ch| ch.to_ascii_lowercase()));
        }
    }

    if out.is_empty() || !out.as_bytes()[0].is_ascii_alphabetic() {
        out.insert_str(0, "App");
    }

    out
}

fn title_case(name: &str) -> String {
    let mut words = Vec::new();
    for part in name.split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace()) {
        if part.is_empty() {
            continue;
        }

        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            let mut word = String::new();
            word.push(first.to_ascii_uppercase());
            word.extend(chars);
            words.push(word);
        }
    }

    if words.is_empty() {
        name.to_owned()
    } else {
        words.join(" ")
    }
}

fn separated_identifier(name: &str, separator: char) -> String {
    let mut out = String::new();
    let mut needs_separator = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_separator && !out.is_empty() {
                out.push(separator);
            }
            out.push(ch);
            needs_separator = false;
        } else {
            needs_separator = true;
        }
    }

    out.trim_matches(separator).to_owned()
}

fn relative_path(from_dir: &Path, to_path: &Path) -> PathBuf {
    let from_components = normal_components(from_dir);
    let to_components = normal_components(to_path);
    let common_len = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(left, right)| left == right)
        .count();

    if common_len == 0 {
        return to_path.to_path_buf();
    }

    let mut path = PathBuf::new();
    for _ in common_len..from_components.len() {
        path.push("..");
    }
    for component in &to_components[common_len..] {
        path.push(component);
    }

    if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    }
}

fn normal_components(path: &Path) -> Vec<OsString> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_os_string()),
            Component::RootDir => Some(OsString::from(std::path::MAIN_SEPARATOR.to_string())),
            Component::Normal(part) => Some(part.to_os_string()),
            Component::CurDir | Component::ParentDir => None,
        })
        .collect()
}

fn cargo_path(path: &Path) -> String {
    toml_string(&path.to_string_lossy().replace('\\', "/"))
}

fn toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn cargo_toml(names: &AppNames, miniapp_path: &Path) -> String {
    format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"

[workspace]
resolver = "3"

[dependencies]
gpui = {{ version = "0.2.2", git = "https://github.com/zed-industries/zed.git", tag = "v1.0.0" }}
gpui-miniapp = {{ path = "{miniapp_path}" }}
"#,
        package_name = toml_string(&names.package_name),
        miniapp_path = cargo_path(miniapp_path),
    )
}

fn justfile() -> String {
    "default:\n\tjust --list\n\nrun:\n\tcargo run\n".to_owned()
}

fn readme(names: &AppNames) -> String {
    format!(
        "# {title}\n\nRun the app with:\n\n```sh\ncargo run\n```\n\nOr, if you use `just`:\n\n```sh\njust run\n```\n",
        title = names.title,
    )
}

fn main_rs(names: &AppNames) -> String {
    format!(
        r#"use gpui::*;
use gpui_miniapp::{{MiniApp, MiniAppConfig}};

struct {view_name};

impl Render for {view_name} {{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {{
        div()
            .id("app-root")
            .size_full()
            .bg(rgb(0xf8fafc))
            .text_color(rgb(0x111827))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(div().text_3xl().child("{title}"))
            .child(div().text_sm().text_color(rgb(0x64748b)).child("Ready"))
    }}
}}

fn main() {{
    MiniApp::run(
        MiniAppConfig::new("{title}")
            .size(720.0, 480.0)
            .scrollable(false)
            .with_theme(true),
        |cx| cx.new(|_cx| {view_name}),
    );
}}
"#,
        title = rust_string(&names.title),
        view_name = names.view_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn normalizes_names_for_cargo_and_rust() -> Result<()> {
        let names = AppNames::new("My Little App")?;

        assert_eq!(names.directory_name, "My Little App");
        assert_eq!(names.package_name, "my-little-app");
        assert_eq!(names.title, "My Little App");
        assert_eq!(names.view_name, "MyLittleAppView");

        Ok(())
    }

    #[test]
    fn rejects_paths_as_names() {
        assert!(AppNames::new("../outside").is_err());
        assert!(AppNames::new("nested/app").is_err());
        assert!(AppNames::new("").is_err());
    }

    #[test]
    fn creates_a_standalone_app_directory() -> Result<()> {
        let dir = tempdir()?;
        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "demo-app".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
        })?;

        assert_eq!(scaffolded.package_name, "demo-app");
        assert!(scaffolded.app_dir.join("Cargo.toml").is_file());
        assert!(scaffolded.app_dir.join("Justfile").is_file());
        assert!(scaffolded.app_dir.join("README.md").is_file());
        assert!(scaffolded.app_dir.join("src/main.rs").is_file());

        let manifest = fs::read_to_string(scaffolded.app_dir.join("Cargo.toml"))?;
        assert!(manifest.contains("[workspace]"));
        assert!(manifest.contains("gpui-miniapp"));

        let app = fs::read_to_string(scaffolded.app_dir.join("src/main.rs"))?;
        assert!(app.contains("MiniApp::run"));
        assert!(app.contains("Demo App"));

        Ok(())
    }
}
