#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const GPUI_VERSION: &str = "0.2.2";
const GPUI_ZED_TAG: &str = "v1.9.0";
const SCAFFOLD_TEMPLATE_VERSION: &str = "1";
/// Name of the only scaffold template shipped today. Custom templates are a
/// future extension; [`ScaffoldFlags`] reserves the selection slot already.
pub const DEFAULT_TEMPLATE: &str = "default";
const TOOLKIT_ROOT_ENV: &str = "GPUI_TOOLKIT_ROOT";
const SCAFFOLD_GITIGNORE: &str =
    "/target/\n/ios/build/\n/ios/lib/\n.gradle/\n**/local.properties\n";
/// Toolkit-relative path of the Android activity host source.
#[cfg(any(not(feature = "mobile"), test))]
const GPUI_ANDROID_ACTIVITY_JAVA_PATH: &str =
    "crates/gpui-android/android/src/main/java/dev/gpui/mobile/GpuiActivity.java";
/// Toolkit-relative path of the Android file-provider host source.
#[cfg(any(not(feature = "mobile"), test))]
const GPUI_ANDROID_FILE_PROVIDER_JAVA_PATH: &str =
    "crates/gpui-android/android/src/main/java/dev/gpui/mobile/GpuiFileProvider.java";

/// Embedded Android Java hosts. Gated behind the `mobile` feature (enabled by
/// default) so `--help`-only binaries can opt out with `--no-default-features`
/// and load the same sources from a toolkit checkout at runtime instead.
#[cfg(feature = "mobile")]
const GPUI_ANDROID_ACTIVITY_JAVA: &str =
    include_str!("../../gpui-android/android/src/main/java/dev/gpui/mobile/GpuiActivity.java");
#[cfg(feature = "mobile")]
const GPUI_ANDROID_FILE_PROVIDER_JAVA: &str =
    include_str!("../../gpui-android/android/src/main/java/dev/gpui/mobile/GpuiFileProvider.java");

#[derive(Debug, Clone)]
pub struct ScaffoldOptions {
    pub name: String,
    pub output_dir: PathBuf,
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldedApp {
    pub app_dir: PathBuf,
    pub package_name: String,
    pub title: String,
}

/// Non-mutating scaffold plan. Paths are the exact files that a subsequent
/// `scaffold_app` call with the same options and flags would create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldPreview {
    pub app: ScaffoldedApp,
    pub files: Vec<PathBuf>,
}

/// Template and platform selection for a scaffold.
///
/// `ScaffoldOptions` is intentionally unchanged so existing callers keep
/// compiling; pass these flags to [`scaffold_app_with_flags`] or
/// [`preview_scaffold_with_flags`] to customize the generated project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldFlags {
    /// Template name. Only [`DEFAULT_TEMPLATE`] (`"default"`) exists today;
    /// any other value is an error, reserving the slot for a future template
    /// registry.
    pub template: String,
    /// Skip the iOS host (`ios/`, the `gpui-ios` dependency, iOS Just recipes).
    pub no_ios: bool,
    /// Skip the Android host (`android/`, the `gpui-android` dependency,
    /// Android Just recipes).
    pub no_android: bool,
}

impl Default for ScaffoldFlags {
    fn default() -> Self {
        Self {
            template: DEFAULT_TEMPLATE.to_owned(),
            no_ios: false,
            no_android: false,
        }
    }
}

impl ScaffoldFlags {
    /// Reject unknown templates before touching the filesystem.
    pub fn validate(&self) -> Result<()> {
        if self.template != DEFAULT_TEMPLATE {
            bail!(
                "unknown template \"{}\"; only \"default\" is supported",
                self.template
            );
        }
        Ok(())
    }
}

/// Toolkit-relative dependency paths, derived from a single
/// [`relative_path`] ancestor walk (see [`toolkit_dependency_paths`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyPaths {
    miniapp: PathBuf,
    ui_kit: PathBuf,
    ios: PathBuf,
    android: PathBuf,
    block: PathBuf,
    zed_font_kit: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppNames {
    directory_name: String,
    package_name: String,
    library_name: String,
    ffi_start_symbol: String,
    xcode_target_name: String,
    ios_source_dir: String,
    bundle_identifier: String,
    title: String,
    view_name: String,
}

pub fn scaffold_app(options: &ScaffoldOptions) -> Result<ScaffoldedApp> {
    scaffold_app_with_flags(options, &ScaffoldFlags::default())
}

/// Scaffold a project with explicit template/platform flags; see
/// [`ScaffoldFlags`]. With `dry_run` the request is validated but nothing is
/// created, deleted, or modified.
pub fn scaffold_app_with_flags(
    options: &ScaffoldOptions,
    flags: &ScaffoldFlags,
) -> Result<ScaffoldedApp> {
    flags.validate()?;
    let names = AppNames::new(&options.name)?;
    let output_dir = options
        .output_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", options.output_dir.display()))?;
    let app_dir = output_dir.join(&names.directory_name);

    check_existing_app_dir(&app_dir, options.force, options.dry_run)?;

    // Single source of truth: the same plan backs dry-run validation,
    // `preview_scaffold`, and the files written below, so the three can never
    // diverge.
    let plan = build_scaffold_plan(&names, &app_dir, flags)?;

    if options.dry_run {
        return Ok(ScaffoldedApp {
            app_dir,
            package_name: names.package_name,
            title: names.title,
        });
    }

    create_scaffold_directories(&app_dir, &names, flags)?;
    for (path, contents) in &plan {
        write_file(path, contents)?;
    }

    Ok(ScaffoldedApp {
        app_dir,
        package_name: names.package_name,
        title: names.title,
    })
}

/// Validate a scaffold request and enumerate its generated files without
/// creating, deleting, or modifying any project files.
pub fn preview_scaffold(options: &ScaffoldOptions) -> Result<ScaffoldPreview> {
    preview_scaffold_with_flags(options, &ScaffoldFlags::default())
}

/// Preview a scaffold with explicit template/platform flags; see
/// [`ScaffoldFlags`]. Never touches the filesystem beyond resolving paths.
pub fn preview_scaffold_with_flags(
    options: &ScaffoldOptions,
    flags: &ScaffoldFlags,
) -> Result<ScaffoldPreview> {
    flags.validate()?;
    let names = AppNames::new(&options.name)?;
    let output_dir = options
        .output_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", options.output_dir.display()))?;
    let app_dir = output_dir.join(&names.directory_name);

    // Same collision rules as `scaffold_app` in dry-run mode: validate, never
    // delete.
    check_existing_app_dir(&app_dir, options.force, true)?;

    let plan = build_scaffold_plan(&names, &app_dir, flags)?;
    let app = ScaffoldedApp {
        app_dir,
        package_name: names.package_name,
        title: names.title,
    };
    Ok(ScaffoldPreview {
        files: plan.into_iter().map(|(path, _)| path).collect(),
        app,
    })
}

/// Enforce `--force` collision rules. `dry_run` only validates; otherwise an
/// existing replaceable directory is removed in one validation+removal pass.
fn check_existing_app_dir(app_dir: &Path, force: bool, dry_run: bool) -> Result<()> {
    if !app_dir.exists() {
        return Ok(());
    }
    if !force {
        bail!(
            "{} already exists; pass --force to replace an empty scaffold",
            app_dir.display()
        );
    }
    if dry_run {
        ensure_directory_is_replaceable(app_dir)?;
    } else {
        // `replace_directory` re-checks the listing itself, so files added
        // after validation are never removed (and the removal fails instead).
        replace_directory(app_dir)?;
    }
    Ok(())
}

/// Compute every file a scaffold with these names and flags generates.
/// Absolute paths are anchored at `app_dir`; this is the single plan consumed
/// by both `scaffold_app` and `preview_scaffold`.
fn build_scaffold_plan(
    names: &AppNames,
    app_dir: &Path,
    flags: &ScaffoldFlags,
) -> Result<Vec<(PathBuf, String)>> {
    let toolkit_root = toolkit_root()?;
    let dependencies = toolkit_dependency_paths(app_dir, &toolkit_root);
    // Lazily loaded only when the Android host is actually generated.
    let java = if flags.no_android {
        None
    } else {
        Some(android_java_sources(&toolkit_root)?)
    };

    let mut plan = vec![
        (
            app_dir.join("Cargo.toml"),
            cargo_toml(names, &dependencies, flags),
        ),
        (
            app_dir.join("gpui-scaffold.toml"),
            scaffold_metadata(names, flags),
        ),
        (app_dir.join(".gitignore"), SCAFFOLD_GITIGNORE.to_owned()),
        (app_dir.join("Justfile"), justfile(names, flags)),
        (app_dir.join("README.md"), readme(names, flags)),
        (app_dir.join("src/app.rs"), app_rs(names)),
        (app_dir.join("src/lib.rs"), lib_rs(names, flags)),
        (app_dir.join("src/main.rs"), main_rs(names)),
    ];

    if !flags.no_ios {
        let ios_source = app_dir.join("ios").join(&names.ios_source_dir);
        plan.extend([
            (app_dir.join("ios/project.yml"), xcode_project_yml(names)),
            (
                ios_source.join("AppDelegate.swift"),
                swift_app_delegate(names),
            ),
            (ios_source.join("BridgingHeader.h"), bridging_header(names)),
            (ios_source.join("Info.plist"), info_plist(names)),
            (ios_source.join("Entitlements.plist"), entitlements_plist()),
        ]);
    }

    if !flags.no_android {
        let (activity_java, file_provider_java) =
            java.context("android sources must be loaded when the android host is generated")?;
        plan.extend([
            (
                app_dir.join("android/gradle/settings.gradle.kts"),
                android_settings_gradle(names),
            ),
            (
                app_dir.join("android/gradle/build.gradle.kts"),
                android_root_build_gradle(),
            ),
            (
                app_dir.join("android/gradle/gradle.properties"),
                android_gradle_properties(),
            ),
            (
                app_dir.join("android/gradle/app/build.gradle.kts"),
                android_app_build_gradle(names),
            ),
            (
                app_dir.join("android/gradle/app/src/main/AndroidManifest.xml"),
                android_manifest(),
            ),
            (
                app_dir.join("android/gradle/app/src/main/res/values/strings.xml"),
                android_strings(names),
            ),
            (
                app_dir.join("android/gradle/app/src/main/res/values/styles.xml"),
                android_styles(),
            ),
            (
                app_dir.join("android/gradle/app/src/main/java/dev/gpui/mobile/GpuiActivity.java"),
                activity_java,
            ),
            (
                app_dir
                    .join("android/gradle/app/src/main/java/dev/gpui/mobile/GpuiFileProvider.java"),
                file_provider_java,
            ),
        ]);
    }

    Ok(plan)
}

/// Create the scaffold directory tree (parents of every planned file plus the
/// empty `ios/lib` and JNI staging directories the Just recipes expect).
fn create_scaffold_directories(
    app_dir: &Path,
    names: &AppNames,
    flags: &ScaffoldFlags,
) -> Result<()> {
    fs::create_dir_all(app_dir.join("src"))
        .with_context(|| format!("failed to create {}", app_dir.display()))?;
    if !flags.no_ios {
        fs::create_dir_all(app_dir.join("ios").join(&names.ios_source_dir))
            .with_context(|| format!("failed to create {}", app_dir.join("ios").display()))?;
        fs::create_dir_all(app_dir.join("ios/lib"))
            .with_context(|| format!("failed to create {}", app_dir.join("ios").display()))?;
    }
    if !flags.no_android {
        for dir in [
            "android/gradle/app/src/main/res/values",
            "android/gradle/app/src/main/java/dev/gpui/mobile",
            "android/gradle/app/src/main/jniLibs/arm64-v8a",
        ] {
            fs::create_dir_all(app_dir.join(dir)).with_context(|| {
                format!("failed to create {}", app_dir.join("android").display())
            })?;
        }
    }
    Ok(())
}

/// Derive the six toolkit dependency paths from a single [`relative_path`]
/// ancestor walk: everything below the toolkit root hangs off that base, so
/// `relative_path(app, root.join(sub)) == relative_path(app, root).join(sub)`.
fn toolkit_dependency_paths(app_dir: &Path, toolkit_root: &Path) -> DependencyPaths {
    let base = relative_path(app_dir, toolkit_root);
    // Joining onto "." would produce "./crates/..." while the direct walk
    // produces "crates/..."; normalize so both spellings agree exactly.
    let join = |sub: &str| {
        if base == Path::new(".") {
            PathBuf::from(sub)
        } else {
            base.join(sub)
        }
    };
    DependencyPaths {
        miniapp: join("crates/gpui-miniapp"),
        ui_kit: join("crates/gpui-ui-kit"),
        ios: join("crates/gpui-ios"),
        android: join("crates/gpui-android"),
        block: join("crates/3rdparties/block"),
        zed_font_kit: join("crates/3rdparties/zed-font-kit"),
    }
}

/// Android Java host sources: embedded at compile time with the default
/// `mobile` feature, otherwise read from the surrounding toolkit checkout so
/// lean binaries pay no size cost.
#[cfg(feature = "mobile")]
fn android_java_sources(_toolkit_root: &Path) -> Result<(String, String)> {
    Ok((
        GPUI_ANDROID_ACTIVITY_JAVA.to_owned(),
        GPUI_ANDROID_FILE_PROVIDER_JAVA.to_owned(),
    ))
}

/// Android Java host sources: embedded at compile time with the default
/// `mobile` feature, otherwise read from the surrounding toolkit checkout so
/// lean binaries pay no size cost.
#[cfg(not(feature = "mobile"))]
fn android_java_sources(toolkit_root: &Path) -> Result<(String, String)> {
    let activity = toolkit_root.join(GPUI_ANDROID_ACTIVITY_JAVA_PATH);
    let provider = toolkit_root.join(GPUI_ANDROID_FILE_PROVIDER_JAVA_PATH);
    Ok((
        fs::read_to_string(&activity)
            .with_context(|| format!("failed to read {}", activity.display()))?,
        fs::read_to_string(&provider)
            .with_context(|| format!("failed to read {}", provider.display()))?,
    ))
}

fn ensure_directory_is_replaceable(path: &Path) -> Result<()> {
    replaceable_directory_entries(path).map(|_| ())
}

fn replace_directory(path: &Path) -> Result<()> {
    for entry in replaceable_directory_entries(path)? {
        fs::remove_file(&entry).with_context(|| format!("failed to remove {}", entry.display()))?;
    }
    fs::remove_dir(path).with_context(|| {
        format!(
            "failed to remove {}; it may have changed while scaffolding",
            path.display()
        )
    })
}

fn replaceable_directory_entries(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.is_dir() {
        bail!("{} exists and is not a directory", path.display());
    }

    let mut ignored_entries = Vec::new();
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = entry?;
        let entry_path = entry.path();
        if is_ignored_system_entry(&entry_path) {
            ignored_entries.push(entry_path);
        } else {
            bail!("{} is not empty; refusing to replace it", path.display());
        }
    }

    Ok(ignored_entries)
}

fn is_ignored_system_entry(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".DS_Store" | "Thumbs.db" | "desktop.ini")
    )
}

/// Write a scaffold file atomically: the contents land in a same-directory
/// temporary file (fsynced) that is then renamed over the destination, so a
/// crash can never leave a half-written project behind.
fn write_file(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;

    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temporary file next to {}", path.display()))?;
    temporary
        .write_all(contents.as_bytes())
        .with_context(|| format!("failed to write temporary file for {}", path.display()))?;
    temporary
        .as_file()
        .sync_all()
        .with_context(|| format!("failed to sync temporary file for {}", path.display()))?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn toolkit_root() -> Result<PathBuf> {
    let configured_root = std::env::var_os(TOOLKIT_ROOT_ENV).map(PathBuf::from);
    let executable = std::env::current_exe().ok();
    toolkit_root_from(configured_root.as_deref(), executable.as_deref())
}

fn toolkit_root_from(configured_root: Option<&Path>, executable: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = configured_root {
        return canonical_toolkit_root(root).with_context(|| {
            format!(
                "{TOOLKIT_ROOT_ENV} must point to a gpui-toolkit workspace root (got {})",
                root.display()
            )
        });
    }

    if let Some(root) = executable.and_then(discover_toolkit_root) {
        return Ok(root);
    }

    bail!(
        "could not locate the gpui-toolkit workspace from the running executable; set {TOOLKIT_ROOT_ENV} to its root"
    )
}

fn discover_toolkit_root(executable: &Path) -> Option<PathBuf> {
    executable
        .ancestors()
        .find_map(|ancestor| canonical_toolkit_root(ancestor).ok())
}

fn canonical_toolkit_root(path: &Path) -> Result<PathBuf> {
    let root = path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    if root.join("crates/gpui-scaffolder").is_dir() && root.join("crates/gpui-miniapp").is_dir() {
        Ok(root)
    } else {
        bail!("{} is not a gpui-toolkit workspace root", root.display())
    }
}

impl AppNames {
    fn new(name: &str) -> Result<Self> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("app name cannot be empty");
        }
        if !trimmed.is_ascii() {
            bail!("app name must contain only ASCII characters");
        }
        validate_directory_name(trimmed)?;

        let package_name = package_name(trimmed);
        let library_name = package_name.replace('-', "_");
        let ffi_start_symbol = format!("{}_ios_start", library_name);
        let xcode_target_name = pascal_case(&package_name);
        let ios_source_dir = format!("{}App", xcode_target_name);
        let bundle_identifier = format!("com.example.{}", separated_identifier(&package_name, '.'));
        let title = title_case(trimmed);
        let view_name = format!("{}View", xcode_target_name);

        Ok(Self {
            directory_name: trimmed.to_owned(),
            package_name,
            library_name,
            ffi_start_symbol,
            xcode_target_name,
            ios_source_dir,
            bundle_identifier,
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
    let mut out = String::new();
    let mut first_word = true;
    for part in name.split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace()) {
        if part.is_empty() {
            continue;
        }

        if !first_word {
            out.push(' ');
        }
        first_word = false;

        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }

    if out.is_empty() { name.to_owned() } else { out }
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
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\u{0008}' => escaped.push_str("\\b"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\u{000C}' => escaped.push_str("\\f"),
            '\r' => escaped.push_str("\\r"),
            character if character <= '\u{001F}' || character == '\u{007F}' => {
                escaped.push_str(&format!("\\u{:04X}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn cargo_toml(names: &AppNames, dependencies: &DependencyPaths, flags: &ScaffoldFlags) -> String {
    let mut manifest = format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"

[lib]
name = "{library_name}"
crate-type = ["rlib", "staticlib", "cdylib"]

[workspace]
resolver = "3"

[dependencies]
gpui = {{ version = "{gpui_version}", git = "https://github.com/zed-industries/zed.git", tag = "{gpui_zed_tag}" }}
gpui-miniapp = {{ path = "{miniapp_path}" }}
gpui-ui-kit = {{ path = "{ui_kit_path}" }}
# zune-jpeg 0.5.15 accepts zune-core 0.5.2, but that combination does not
# compile without zune-core's optional log feature. Keep fresh scaffolds on the
# compatible release until the upstream decoder constrains or fixes the pair.
zune-core = "=0.5.1"
"#,
        package_name = toml_string(&names.package_name),
        library_name = toml_string(&names.library_name),
        gpui_version = GPUI_VERSION,
        gpui_zed_tag = GPUI_ZED_TAG,
        miniapp_path = cargo_path(&dependencies.miniapp),
        ui_kit_path = cargo_path(&dependencies.ui_kit),
    );

    if !flags.no_ios {
        manifest.push_str(&format!(
            r#"
[target.'cfg(any(target_os = "ios", target_os = "tvos"))'.dependencies]
gpui-ios = {{ path = "{ios_path}" }}
"#,
            ios_path = cargo_path(&dependencies.ios),
        ));
    }

    if !flags.no_android {
        manifest.push_str(&format!(
            r#"
[target.'cfg(target_os = "android")'.dependencies]
android-activity = {{ version = "0.6", features = ["native-activity"] }}
android_logger = "0.15"
gpui-android = {{ path = "{android_path}" }}
log = "0.4"
"#,
            android_path = cargo_path(&dependencies.android),
        ));
    }

    manifest.push_str(&format!(
        r#"
[patch."https://github.com/zed-industries/font-kit"]
zed-font-kit = {{ path = "{zed_font_kit_path}" }}

[patch.crates-io]
block = {{ path = "{block_path}" }}
"#,
        block_path = cargo_path(&dependencies.block),
        zed_font_kit_path = cargo_path(&dependencies.zed_font_kit),
    ));
    manifest
}

fn scaffold_metadata(names: &AppNames, flags: &ScaffoldFlags) -> String {
    format!(
        r#"[scaffold]
generator = "gpui-scaffolder"
generator_version = "{generator_version}"
template_version = "{template_version}"
template = "{template}"
no_ios = {no_ios}
no_android = {no_android}
package_name = "{package_name}"
title = "{title}"
gpui_version = "{gpui_version}"
gpui_zed_tag = "{gpui_zed_tag}"
"#,
        generator_version = env!("CARGO_PKG_VERSION"),
        template_version = SCAFFOLD_TEMPLATE_VERSION,
        template = toml_string(&flags.template),
        no_ios = flags.no_ios,
        no_android = flags.no_android,
        package_name = toml_string(&names.package_name),
        title = toml_string(&names.title),
        gpui_version = GPUI_VERSION,
        gpui_zed_tag = GPUI_ZED_TAG,
    )
}

fn justfile(names: &AppNames, flags: &ScaffoldFlags) -> String {
    let mut out = String::from(
        r#"default:
	just --list

run:
	cargo run

check:
	cargo check

"#,
    );

    if !flags.no_ios {
        out.push_str(&format!(
            r#"# Build the Rust static library for the iOS simulator and stage it for Xcode.
ios-rust-sim:
	rustup target add aarch64-apple-ios-sim
	cargo build --lib --target aarch64-apple-ios-sim --release
	mkdir -p ios/lib target/mobile/ios-sim
	cp target/aarch64-apple-ios-sim/release/lib{library_name}.a ios/lib/
	cp target/aarch64-apple-ios-sim/release/lib{library_name}.a target/mobile/ios-sim/
	@echo "Built ios/lib/lib{library_name}.a"

# Build the Rust static library for an iOS device and stage it for Xcode.
ios-rust:
	rustup target add aarch64-apple-ios
	cargo build --lib --target aarch64-apple-ios --release
	mkdir -p ios/lib target/mobile/ios
	cp target/aarch64-apple-ios/release/lib{library_name}.a ios/lib/
	cp target/aarch64-apple-ios/release/lib{library_name}.a target/mobile/ios/
	@echo "Built ios/lib/lib{library_name}.a"

alias ios-rust-device := ios-rust

# Generate the Xcode project used by the iOS recipes.
ios-xcodegen:
	cd ios && PATH="/opt/homebrew/bin:/usr/local/bin:$PATH" xcodegen generate

# Build the full iOS simulator app with Xcode.
ios-sim: ios-rust-sim ios-xcodegen
	cd ios && xcodebuild -project {xcode_target_name}.xcodeproj -scheme {xcode_target_name} -configuration Release -sdk iphonesimulator -arch arm64 SYMROOT=build -derivedDataPath build/DerivedData CODE_SIGNING_ALLOWED=NO build

# Build the full iOS device app with Xcode. Set IOS_DEVELOPMENT_TEAM and
# IOS_SIGN_IDENTITY when building for a physical device.
ios: ios-rust ios-xcodegen
	cd ios && xcodebuild -project {xcode_target_name}.xcodeproj -scheme {xcode_target_name} -configuration Release -sdk iphoneos -arch arm64 SYMROOT=build -derivedDataPath build/DerivedData CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM="${{IOS_DEVELOPMENT_TEAM:-}}" CODE_SIGN_IDENTITY="${{IOS_SIGN_IDENTITY:-Apple Development}}" build

# Sign a generated device app explicitly with the scaffolded entitlements.
ios-sign app_path="ios/build/Release-iphoneos/{xcode_target_name}.app":
	codesign --force --deep --sign "${{IOS_SIGN_IDENTITY:-Apple Development}}" --entitlements "ios/{ios_source_dir}/Entitlements.plist" "{{{{app_path}}}}"

alias ios-device := ios

# Build the Rust static library for the tvOS simulator.
# tvOS is a Tier 3 Rust target and requires nightly with rust-src:
#   rustup toolchain install nightly
#   rustup component add rust-src --toolchain nightly
tvos-sim:
	cargo +nightly build --lib --target aarch64-apple-tvos-sim --release -Zbuild-std
	mkdir -p target/mobile/tvos-sim
	cp target/aarch64-apple-tvos-sim/release/lib{library_name}.a target/mobile/tvos-sim/
	@echo "Built target/mobile/tvos-sim/lib{library_name}.a"

# Build the Rust static library for a tvOS device.
tvos:
	cargo +nightly build --lib --target aarch64-apple-tvos --release -Zbuild-std
	mkdir -p target/mobile/tvos
	cp target/aarch64-apple-tvos/release/lib{library_name}.a target/mobile/tvos/
	@echo "Built target/mobile/tvos/lib{library_name}.a"

alias tvos-device := tvos

"#,
            library_name = names.library_name,
            xcode_target_name = names.xcode_target_name,
            ios_source_dir = names.ios_source_dir,
        ));
    }

    if !flags.no_android {
        out.push_str(&format!(
            r#"# Build the Rust dynamic library for Android ARM64 and stage it for Gradle.
android-rust:
	rustup target add aarch64-linux-android
	cargo build --lib --target aarch64-linux-android --release
	mkdir -p android/gradle/app/src/main/jniLibs/arm64-v8a target/mobile/android-arm64
	cp target/aarch64-linux-android/release/lib{library_name}.so android/gradle/app/src/main/jniLibs/arm64-v8a/
	cp target/aarch64-linux-android/release/lib{library_name}.so target/mobile/android-arm64/
	@echo "Built android/gradle/app/src/main/jniLibs/arm64-v8a/lib{library_name}.so"

# Validate the generated Android Gradle host without requiring a device.
android-gradle-check:
	cd android/gradle && gradle :app:assembleDebug --dry-run

# Build a debug APK. Requires Android SDK/NDK plus Gradle on PATH.
android-apk: android-rust
	cd android/gradle && gradle :app:assembleDebug

alias android := android-apk
"#,
            library_name = names.library_name,
        ));
    }

    out
}

fn readme(names: &AppNames, flags: &ScaffoldFlags) -> String {
    let mut out = format!(
        "# {title}\n\nRun the desktop app with:\n\n```sh\ncargo run\n```\n\nOr, if you use `just`:\n\n```sh\njust run\n```\n",
        title = names.title,
    );

    if !flags.no_ios {
        out.push_str(&format!(
            "\nBuild the generated iOS app with:\n\n```sh\njust ios-sim\njust ios\n```\n\nBuild the generated tvOS Rust library with:\n\n```sh\njust tvos-sim\njust tvos\n```\n\nThe iOS host lives in `ios/`, uses XcodeGen, and includes `ios/{ios_source_dir}/Entitlements.plist`. To sign an already-built device app explicitly:\n\n```sh\nIOS_SIGN_IDENTITY=\"Apple Development: Your Name\" just ios-sign\n```\n",
            ios_source_dir = names.ios_source_dir,
        ));
    }

    if !flags.no_android {
        out.push_str(&format!(
            "\nBuild the generated Android Rust library and debug APK with:\n\n```sh\njust android-rust\njust android-apk\n```\n\nThe Android host lives in `android/gradle/`, uses Android `NativeActivity`, and loads `lib{library_name}.so` from `app/src/main/jniLibs/arm64-v8a/`. Install the Android SDK/NDK before running the Android recipes.\n",
            library_name = names.library_name,
        ));
    }

    out
}

fn app_rs(names: &AppNames) -> String {
    format!(
        r#"use gpui::*;
use gpui_miniapp::{{MiniApp, MiniAppConfig}};
use gpui_ui_kit::{{Button, ButtonVariant, Heading, Text, ThemeExt}};

pub struct {view_name};

impl {view_name} {{
    pub fn new(_: &mut Context<Self>) -> Self {{
        Self
    }}
}}

impl Render for {view_name} {{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {{
        let theme = cx.theme();

        div()
            .id("app-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(Heading::new("{title}"))
            .child(Text::new("A small app composed from toolkit crates."))
            .child(Button::new("refresh", "Refresh").variant(ButtonVariant::Primary))
    }}
}}

pub fn run_desktop() {{
    MiniApp::run(
        MiniAppConfig::new("{title}")
            .size(1200.0, 780.0)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new({view_name}::new),
    );
}}

pub fn open_app_window(cx: &mut App) {{
    let open_result = cx.open_window(
        WindowOptions {{
            window_bounds: None,
            ..Default::default()
        }},
        |_, cx| cx.new({view_name}::new),
    );

    if let Err(error) = open_result {{
        eprintln!("failed to open {title} window: {{error}}");
        return;
    }}

    cx.activate(true);
}}
"#,
        title = rust_string(&names.title),
        view_name = names.view_name,
    )
}

fn lib_rs(names: &AppNames, flags: &ScaffoldFlags) -> String {
    let mut out = format!(
        r#"mod app;

pub use app::{{open_app_window, run_desktop, {view_name}}};

"#,
        view_name = names.view_name,
    );

    if !flags.no_ios {
        out.push_str(&format!(
            r#"#[cfg(any(target_os = "ios", target_os = "tvos"))]
mod mobile {{
    use gpui::*;
    use gpui_ui_kit::accessibility::AccessibilityTree;
    use gpui_ui_kit::{{DesignSystemState, I18nState, ThemeState, ThemeVariant}};

    fn ffi_guard<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + std::panic::UnwindSafe,
        R: Default,
    {{
        match std::panic::catch_unwind(f) {{
            Ok(result) => result,
            Err(_) => {{
                eprintln!("gpui scaffold mobile startup panicked");
                R::default()
            }}
        }}
    }}

    #[unsafe(no_mangle)]
    pub extern "C" fn {ffi_start_symbol}() {{
        ffi_guard(|| {{
            gpui_ios::ios::ffi::set_app_callback(Box::new(|cx: &mut App| {{
                cx.set_global(ThemeState::with_variant(ThemeVariant::default()));
                cx.set_global(DesignSystemState::new());
                cx.set_global(AccessibilityTree::new());
                cx.set_global(I18nState::new());
                crate::app::open_app_window(cx);
            }}));

            gpui_ios::ios::ffi::run_app();
        }});
    }}
}}

"#,
            ffi_start_symbol = names.ffi_start_symbol,
        ));
    }

    if !flags.no_android {
        out.push_str(&format!(
            r#"#[cfg(target_os = "android")]
mod android {{
    use gpui::{{App, AppContext, Application}};
    use gpui_ui_kit::accessibility::AccessibilityTree;
    use gpui_ui_kit::{{DesignSystemState, I18nState, ThemeState, ThemeVariant}};

    #[unsafe(no_mangle)]
    pub fn android_main(app: android_activity::AndroidApp) {{
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("{library_name}"),
        );
        gpui_android::android::jni::install_panic_hook();

        let _platform = gpui_android::android::jni::init_platform(&app);
        let Some(shared_platform) = gpui_android::android::jni::shared_platform() else {{
            log::error!("android_main: shared_platform() returned None");
            return;
        }};

        Application::with_platform(shared_platform.into_rc()).run(|cx: &mut App| {{
            cx.set_global(ThemeState::with_variant(ThemeVariant::default()));
            cx.set_global(DesignSystemState::new());
            cx.set_global(AccessibilityTree::new());
            cx.set_global(I18nState::new());
            crate::app::open_app_window(cx);
        }});
    }}
}}
"#,
            library_name = rust_string(&names.library_name),
        ));
    }

    out
}

fn main_rs(names: &AppNames) -> String {
    format!(
        "fn main() {{\n    {library_name}::run_desktop();\n}}\n",
        library_name = names.library_name,
    )
}

fn xcode_project_yml(names: &AppNames) -> String {
    format!(
        r#"name: {xcode_target_name}
options:
  bundleIdPrefix: com.example
  deploymentTarget:
    iOS: "15.0"

settings:
  base:
    SWIFT_OBJC_BRIDGING_HEADER: {ios_source_dir}/BridgingHeader.h
    EXCLUDED_ARCHS[sdk=iphonesimulator*]: x86_64

targets:
  {xcode_target_name}:
    type: application
    platform: iOS
    deploymentTarget: "15.0"
    sources:
      - path: {ios_source_dir}
        excludes:
          - Entitlements.plist
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: {bundle_identifier}
        INFOPLIST_FILE: {ios_source_dir}/Info.plist
        CODE_SIGN_ENTITLEMENTS: {ios_source_dir}/Entitlements.plist
        SWIFT_VERSION: "5.10"
        LIBRARY_SEARCH_PATHS: "$(PROJECT_DIR)/lib"
        ENABLE_USER_SCRIPT_SANDBOXING: false
        DEAD_CODE_STRIPPING: false
        OTHER_LDFLAGS:
          - "-L$(PROJECT_DIR)/lib"
          - "-force_load"
          - "$(PROJECT_DIR)/lib/lib{library_name}.a"
          - "-framework"
          - "Metal"
          - "-framework"
          - "MetalKit"
          - "-framework"
          - "QuartzCore"
          - "-framework"
          - "CoreText"
          - "-framework"
          - "CoreGraphics"
          - "-framework"
          - "CoreFoundation"
          - "-framework"
          - "UIKit"
          - "-framework"
          - "Foundation"
          - "-lc++"
"#,
        xcode_target_name = names.xcode_target_name,
        ios_source_dir = names.ios_source_dir,
        bundle_identifier = names.bundle_identifier,
        library_name = names.library_name,
    )
}

fn swift_app_delegate(names: &AppNames) -> String {
    format!(
        r#"import UIKit
import QuartzCore

@main
class AppDelegate: UIResponder, UIApplicationDelegate {{
    var window: UIWindow?
    private var displayLink: CADisplayLink?

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {{
        {ffi_start_symbol}()

        displayLink = CADisplayLink(target: self, selector: #selector(renderFrame))
        displayLink?.add(to: .main, forMode: .common)

        return true
    }}

    @objc private func renderFrame() {{
        gpui_ios_request_current_frame()
    }}

    func applicationWillEnterForeground(_ application: UIApplication) {{
        gpui_ios_will_enter_foreground(nil)
    }}

    func applicationDidBecomeActive(_ application: UIApplication) {{
        gpui_ios_did_become_active(nil)
    }}

    func applicationWillResignActive(_ application: UIApplication) {{
        gpui_ios_will_resign_active(nil)
    }}

    func applicationDidEnterBackground(_ application: UIApplication) {{
        gpui_ios_did_enter_background(nil)
    }}

    func applicationWillTerminate(_ application: UIApplication) {{
        displayLink?.invalidate()
    }}
}}
"#,
        ffi_start_symbol = names.ffi_start_symbol,
    )
}

fn bridging_header(names: &AppNames) -> String {
    format!(
        r#"#ifndef {header_guard}
#define {header_guard}

#include <stdint.h>

void {ffi_start_symbol}(void);

void gpui_ios_request_current_frame(void);
void gpui_ios_will_enter_foreground(void *app);
void gpui_ios_did_become_active(void *app);
void gpui_ios_will_resign_active(void *app);
void gpui_ios_did_enter_background(void *app);

#endif
"#,
        ffi_start_symbol = names.ffi_start_symbol,
        header_guard = format!(
            "{}_BRIDGING_HEADER_H",
            names.library_name.to_ascii_uppercase()
        ),
    )
}

fn info_plist(names: &AppNames) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>$(DEVELOPMENT_LANGUAGE)</string>
	<key>CFBundleDisplayName</key>
	<string>{title}</string>
	<key>CFBundleExecutable</key>
	<string>$(EXECUTABLE_NAME)</string>
	<key>CFBundleIdentifier</key>
	<string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$(PRODUCT_NAME)</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>LSRequiresIPhoneOS</key>
	<true/>
	<key>UIRequiredDeviceCapabilities</key>
	<array>
		<string>metal</string>
	</array>
	<key>UISupportedInterfaceOrientations</key>
	<array>
		<string>UIInterfaceOrientationPortrait</string>
		<string>UIInterfaceOrientationLandscapeLeft</string>
		<string>UIInterfaceOrientationLandscapeRight</string>
	</array>
	<key>UISupportedInterfaceOrientations~ipad</key>
	<array>
		<string>UIInterfaceOrientationPortrait</string>
		<string>UIInterfaceOrientationPortraitUpsideDown</string>
		<string>UIInterfaceOrientationLandscapeLeft</string>
		<string>UIInterfaceOrientationLandscapeRight</string>
	</array>
</dict>
</plist>
"#,
        title = xml_string(&names.title),
    )
}

fn entitlements_plist() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
</dict>
</plist>
"#
    .to_owned()
}

fn android_settings_gradle(names: &AppNames) -> String {
    format!(
        r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}

dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}

rootProject.name = "{project_name}Android"
include(":app")
"#,
        project_name = toml_string(&names.xcode_target_name),
    )
}

fn android_root_build_gradle() -> String {
    r#"plugins {
    id("com.android.application") version "8.7.3" apply false
}
"#
    .to_owned()
}

fn android_gradle_properties() -> String {
    r#"android.useAndroidX=true
android.nonTransitiveRClass=true
org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
"#
    .to_owned()
}

fn android_app_build_gradle(names: &AppNames) -> String {
    format!(
        r#"plugins {{
    id("com.android.application")
}}

android {{
    namespace = "{application_id}"
    compileSdk = 35

    defaultConfig {{
        applicationId = "{application_id}"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {{
            abiFilters += listOf("arm64-v8a")
        }}

        manifestPlaceholders["nativeLibraryName"] = "{library_name}"
    }}

    buildTypes {{
        debug {{
            isDebuggable = true
            isJniDebuggable = true
        }}
        release {{
            isMinifyEnabled = false
        }}
    }}

    sourceSets {{
        getByName("main") {{
            jniLibs.srcDirs("src/main/jniLibs")
        }}
    }}

    packaging {{
        jniLibs {{
            keepDebugSymbols += listOf("*/arm64-v8a/lib{library_name}.so")
        }}
    }}
}}
"#,
        application_id = toml_string(&names.bundle_identifier),
        library_name = toml_string(&names.library_name),
    )
}

fn android_manifest() -> String {
    r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-feature
        android:name="android.hardware.vulkan.level"
        android:required="false"
        android:version="0" />
    <uses-feature
        android:name="android.hardware.vulkan.version"
        android:required="false"
        android:version="0x00401000" />
    <uses-feature android:glEsVersion="0x00030000" android:required="true" />
    <uses-feature android:name="android.hardware.touchscreen" android:required="false" />

    <application
        android:allowBackup="false"
        android:hardwareAccelerated="true"
        android:label="@string/app_name"
        android:supportsRtl="true"
        android:theme="@style/GpuiTheme">
        <activity
            android:name="dev.gpui.mobile.GpuiActivity"
            android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboardHidden|keyboard|navigation|uiMode|density"
            android:exported="true"
            android:launchMode="singleTask"
            android:screenOrientation="unspecified">
            <meta-data
                android:name="android.app.lib_name"
                android:value="${nativeLibraryName}" />

            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
        <provider
            android:name="dev.gpui.mobile.GpuiFileProvider"
            android:authorities="${applicationId}.gpui.fileprovider"
            android:exported="false"
            android:grantUriPermissions="true" />
    </application>
</manifest>
"#
    .to_owned()
}

fn android_strings(names: &AppNames) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{title}</string>
</resources>
"#,
        title = xml_string(&names.title),
    )
}

fn android_styles() -> String {
    r##"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="GpuiTheme" parent="@android:style/Theme.Material.NoActionBar">
        <item name="android:windowNoTitle">true</item>
        <item name="android:windowActionBar">false</item>
        <item name="android:windowFullscreen">true</item>
        <item name="android:windowDrawsSystemBarBackgrounds">true</item>
        <item name="android:navigationBarColor">#202020</item>
        <item name="android:statusBarColor">#202020</item>
    </style>
</resources>
"##
    .to_owned()
}

fn xml_string(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn normalizes_names_for_cargo_and_rust() -> Result<()> {
        let names = AppNames::new("My Little App")?;

        assert_eq!(names.directory_name, "My Little App");
        assert_eq!(names.package_name, "my-little-app");
        assert_eq!(names.library_name, "my_little_app");
        assert_eq!(names.ffi_start_symbol, "my_little_app_ios_start");
        assert_eq!(names.xcode_target_name, "MyLittleApp");
        assert_eq!(names.ios_source_dir, "MyLittleAppApp");
        assert_eq!(names.bundle_identifier, "com.example.my.little.app");
        assert_eq!(names.title, "My Little App");
        assert_eq!(names.view_name, "MyLittleAppView");

        Ok(())
    }

    #[test]
    fn title_cases_names() {
        assert_eq!(title_case("my-little-app"), "My Little App");
        assert_eq!(title_case("my_little_app"), "My Little App");
        assert_eq!(title_case("my little app"), "My Little App");
        assert_eq!(title_case("My-Little-App"), "My Little App");
        assert_eq!(title_case("single"), "Single");
        assert_eq!(title_case(""), "");
        assert_eq!(title_case("--__  "), "--__  ");
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
            dry_run: false,
        })?;

        assert_eq!(scaffolded.package_name, "demo-app");
        assert!(scaffolded.app_dir.join("Cargo.toml").is_file());
        assert!(scaffolded.app_dir.join("gpui-scaffold.toml").is_file());
        assert!(scaffolded.app_dir.join("Justfile").is_file());
        assert!(scaffolded.app_dir.join("README.md").is_file());
        assert!(scaffolded.app_dir.join("src/app.rs").is_file());
        assert!(scaffolded.app_dir.join("src/lib.rs").is_file());
        assert!(scaffolded.app_dir.join("src/main.rs").is_file());
        assert!(scaffolded.app_dir.join("ios/project.yml").is_file());
        assert!(
            scaffolded
                .app_dir
                .join("ios/DemoAppApp/AppDelegate.swift")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("ios/DemoAppApp/BridgingHeader.h")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("ios/DemoAppApp/Info.plist")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("ios/DemoAppApp/Entitlements.plist")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("android/gradle/settings.gradle.kts")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("android/gradle/app/build.gradle.kts")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/AndroidManifest.xml")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/res/values/strings.xml")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/java/dev/gpui/mobile/GpuiActivity.java")
                .is_file()
        );
        assert!(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/java/dev/gpui/mobile/GpuiFileProvider.java")
                .is_file()
        );

        let android_manifest = fs::read_to_string(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/AndroidManifest.xml"),
        )?;
        assert!(android_manifest.contains("dev.gpui.mobile.GpuiActivity"));
        assert!(android_manifest.contains("dev.gpui.mobile.GpuiFileProvider"));

        let manifest = fs::read_to_string(scaffolded.app_dir.join("Cargo.toml"))?;
        let _: toml::Value = toml::from_str(&manifest)?;
        assert!(manifest.contains("[workspace]"));
        assert!(manifest.contains("crate-type = [\"rlib\", \"staticlib\", \"cdylib\"]"));
        assert!(manifest.contains(r#"tag = "v1.9.0""#));
        assert!(manifest.contains("gpui-miniapp"));
        assert!(manifest.contains("gpui-ui-kit"));
        assert!(manifest.contains("gpui-ios"));
        assert!(manifest.contains("gpui-android"));
        assert!(manifest.contains("android-activity"));
        assert!(manifest.contains("android_logger"));
        assert!(manifest.contains("[patch.\"https://github.com/zed-industries/font-kit\"]"));
        assert!(manifest.contains("zed-font-kit"));
        assert!(manifest.contains("crates/3rdparties/zed-font-kit"));
        assert!(manifest.contains("[patch.crates-io]"));
        assert!(manifest.contains("zune-core = \"=0.5.1\""));
        assert!(manifest.contains("block"));
        assert!(manifest.contains("crates/3rdparties/block"));

        let metadata: toml::Value = toml::from_str(&fs::read_to_string(
            scaffolded.app_dir.join("gpui-scaffold.toml"),
        )?)?;
        let metadata = metadata
            .get("scaffold")
            .context("gpui-scaffold.toml must contain [scaffold]")?;
        assert_eq!(
            metadata.get("generator").and_then(toml::Value::as_str),
            Some("gpui-scaffolder")
        );
        assert_eq!(
            metadata
                .get("generator_version")
                .and_then(toml::Value::as_str),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            metadata
                .get("template_version")
                .and_then(toml::Value::as_str),
            Some(SCAFFOLD_TEMPLATE_VERSION)
        );
        assert_eq!(
            metadata.get("package_name").and_then(toml::Value::as_str),
            Some("demo-app")
        );
        assert_eq!(
            metadata.get("gpui_zed_tag").and_then(toml::Value::as_str),
            Some(GPUI_ZED_TAG)
        );

        let justfile = fs::read_to_string(scaffolded.app_dir.join("Justfile"))?;
        assert!(justfile.contains("ios-sim:"));
        assert!(justfile.contains("ios:"));
        assert!(justfile.contains("ios-rust-sim:"));
        assert!(justfile.contains("ios-xcodegen:"));
        assert!(justfile.contains("xcodebuild"));
        assert!(justfile.contains("ios-sign"));
        assert!(justfile.contains("tvos-sim:"));
        assert!(justfile.contains("tvos:"));
        assert!(justfile.contains("android-rust:"));
        assert!(justfile.contains("android-gradle-check:"));
        assert!(justfile.contains("android-apk:"));

        let project = fs::read_to_string(scaffolded.app_dir.join("ios/project.yml"))?;
        assert!(project.contains("name: DemoApp"));
        assert!(project.contains("CODE_SIGN_ENTITLEMENTS: DemoAppApp/Entitlements.plist"));
        assert!(project.contains("$(PROJECT_DIR)/lib/libdemo_app.a"));

        let app_delegate =
            fs::read_to_string(scaffolded.app_dir.join("ios/DemoAppApp/AppDelegate.swift"))?;
        assert!(app_delegate.contains("demo_app_ios_start()"));
        assert!(app_delegate.contains("gpui_ios_request_current_frame()"));

        let app = fs::read_to_string(scaffolded.app_dir.join("src/app.rs"))?;
        assert!(app.contains("MiniApp::run"));
        assert!(app.contains("Demo App"));
        assert!(app.contains("gpui_ui_kit"));

        let lib = fs::read_to_string(scaffolded.app_dir.join("src/lib.rs"))?;
        assert!(lib.contains("demo_app_ios_start"));
        assert!(lib.contains("gpui_ios::ios::ffi::run_app"));
        assert!(lib.contains("pub fn android_main(app: android_activity::AndroidApp)"));
        assert!(lib.contains("gpui_android::android::jni::init_platform"));

        let app = fs::read_to_string(scaffolded.app_dir.join("src/main.rs"))?;
        assert!(app.contains("demo_app::run_desktop"));

        let readme = fs::read_to_string(scaffolded.app_dir.join("README.md"))?;
        assert!(readme.contains("just tvos-sim"));
        assert!(readme.contains("just tvos"));
        assert!(readme.contains("just android-rust"));
        assert!(readme.contains("android/gradle"));

        let android_settings = fs::read_to_string(
            scaffolded
                .app_dir
                .join("android/gradle/settings.gradle.kts"),
        )?;
        assert!(android_settings.contains("rootProject.name = \"DemoAppAndroid\""));

        let android_build = fs::read_to_string(
            scaffolded
                .app_dir
                .join("android/gradle/app/build.gradle.kts"),
        )?;
        assert!(android_build.contains("namespace = \"com.example.demo.app\""));
        assert!(android_build.contains("applicationId = \"com.example.demo.app\""));
        assert!(
            android_build.contains("manifestPlaceholders[\"nativeLibraryName\"] = \"demo_app\"")
        );
        assert!(android_build.contains("arm64-v8a"));

        let android_manifest = fs::read_to_string(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/AndroidManifest.xml"),
        )?;
        assert!(android_manifest.contains("dev.gpui.mobile.GpuiActivity"));
        assert!(android_manifest.contains("dev.gpui.mobile.GpuiFileProvider"));
        assert!(android_manifest.contains("android.app.lib_name"));
        assert!(android_manifest.contains("${nativeLibraryName}"));

        let android_strings = fs::read_to_string(
            scaffolded
                .app_dir
                .join("android/gradle/app/src/main/res/values/strings.xml"),
        )?;
        assert!(android_strings.contains("<string name=\"app_name\">Demo App</string>"));

        Ok(())
    }

    #[test]
    fn scaffolded_project_passes_cargo_check() -> Result<()> {
        let dir = tempdir()?;
        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "compile-smoke".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: false,
        })?;
        patch_scaffold_for_local_workspace(&scaffolded.app_dir.join("Cargo.toml"))?;

        run_scaffolded_cargo_check(
            &scaffolded.app_dir.join("Cargo.toml"),
            &dir.path().join("target"),
            &["--all-targets"],
            "scaffolded project failed cargo check",
        )?;

        Ok(())
    }

    #[test]
    fn scaffolded_project_passes_ios_simulator_cargo_check() -> Result<()> {
        let dir = tempdir()?;
        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "ios-smoke".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: false,
        })?;
        patch_scaffold_for_local_workspace(&scaffolded.app_dir.join("Cargo.toml"))?;

        run_scaffolded_cargo_check(
            &scaffolded.app_dir.join("Cargo.toml"),
            &dir.path().join("target-ios"),
            &["--lib", "--target", "aarch64-apple-ios-sim"],
            "scaffolded iOS simulator library failed cargo check",
        )?;

        Ok(())
    }

    #[test]
    fn scaffolded_project_passes_tvos_simulator_cargo_check() -> Result<()> {
        let dir = tempdir()?;
        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "tvos-smoke".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: false,
        })?;
        patch_scaffold_for_local_workspace(&scaffolded.app_dir.join("Cargo.toml"))?;

        run_scaffolded_cargo_check_with_toolchain(
            &scaffolded.app_dir.join("Cargo.toml"),
            &dir.path().join("target-tvos"),
            Some("nightly"),
            &["--lib", "--target", "aarch64-apple-tvos-sim", "-Zbuild-std"],
            "scaffolded tvOS simulator library failed cargo check",
        )?;

        Ok(())
    }

    /// Keep compile-smoke tests offline without changing the published
    /// scaffold template: generated apps intentionally point at the public
    /// Zed tag, while this test-only replacement redirects that dependency to
    /// the workspace's namespaced vendored GPUI crate.
    fn patch_scaffold_for_local_workspace(manifest_path: &Path) -> Result<()> {
        let toolkit_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?;
        let workspace_manifest: toml::Value = toml::from_str(include_str!("../../../Cargo.toml"))?;
        let crates_io_patches = workspace_manifest
            .get("patch")
            .and_then(|patch| patch.get("crates-io"))
            .and_then(toml::Value::as_table)
            .context("workspace must define crates.io patches")?;

        let mut manifest = fs::read_to_string(manifest_path)?;
        let generated_gpui = format!(
            r#"gpui = {{ version = "{GPUI_VERSION}", git = "https://github.com/zed-industries/zed.git", tag = "{GPUI_ZED_TAG}" }}"#
        );
        let local_gpui = toolkit_root.join("crates/3rdparties/gpui").canonicalize()?;
        let local_gpui = format!(
            r#"gpui = {{ package = "gpui-toolkit-gpui", path = "{}" }}"#,
            cargo_path(&local_gpui),
        );
        if !manifest.contains(&generated_gpui) {
            bail!("generated scaffold must contain the expected GPUI Git dependency");
        }
        manifest = manifest.replace(&generated_gpui, &local_gpui);

        let generated_manifest: toml::Value = toml::from_str(&manifest)?;
        let existing_patches = generated_manifest
            .get("patch")
            .and_then(|patch| patch.get("crates-io"))
            .and_then(toml::Value::as_table)
            .context("generated scaffold must define crates.io patches")?;
        manifest.push_str("\n# Test-only patches inherited from the local workspace.\n");
        for (name, dependency) in crates_io_patches {
            if existing_patches.contains_key(name) {
                continue;
            }

            if let Some(relative_path) = dependency.get("path").and_then(toml::Value::as_str) {
                let path = toolkit_root.join(relative_path).canonicalize()?;
                manifest.push_str(&format!(
                    "{name} = {{ path = \"{}\" }}\n",
                    cargo_path(&path),
                ));
                continue;
            }

            let git = dependency
                .get("git")
                .and_then(toml::Value::as_str)
                .with_context(|| {
                    format!("workspace crates.io patch {name} must have a path or git")
                })?;
            let rev = dependency
                .get("rev")
                .and_then(toml::Value::as_str)
                .with_context(|| format!("workspace crates.io Git patch {name} must have a rev"))?;
            manifest.push_str(&format!(
                "{name} = {{ git = \"{}\", rev = \"{}\" }}\n",
                toml_string(git),
                toml_string(rev),
            ));
        }
        fs::write(manifest_path, manifest)?;
        Ok(())
    }

    fn run_scaffolded_cargo_check(
        manifest_path: &Path,
        target_dir: &Path,
        extra_args: &[&str],
        failure_context: &str,
    ) -> Result<()> {
        run_scaffolded_cargo_check_with_toolchain(
            manifest_path,
            target_dir,
            None,
            extra_args,
            failure_context,
        )
    }

    fn run_scaffolded_cargo_check_with_toolchain(
        manifest_path: &Path,
        target_dir: &Path,
        toolchain: Option<&str>,
        extra_args: &[&str],
        failure_context: &str,
    ) -> Result<()> {
        let mut command = if let Some(toolchain) = toolchain {
            let mut command = Command::new("rustup");
            command.args(["run", toolchain, "cargo"]);
            command
        } else {
            Command::new(env!("CARGO"))
        };
        command
            .arg("check")
            .arg("--manifest-path")
            .arg(manifest_path)
            .env("CARGO_TARGET_DIR", target_dir);
        if std::env::var_os("GPUI_SCAFFOLDER_OFFLINE").is_some() {
            command.arg("--offline");
        }
        command.args(extra_args);

        let output = command
            .output()
            .with_context(|| format!("failed to run cargo check: {failure_context}"))?;

        if !output.status.success() {
            bail!(
                "{failure_context}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        Ok(())
    }

    #[test]
    fn generated_gpui_tag_matches_vendored_revision() -> Result<()> {
        let vendored = include_str!("../../3rdparties/gpui/VENDORED.md");
        let vendored_ref = vendored
            .lines()
            .find_map(|line| line.strip_prefix("- Base ref: "))
            .context("vendored GPUI provenance must declare a base ref")?;

        assert_eq!(GPUI_ZED_TAG, vendored_ref);
        Ok(())
    }

    #[test]
    fn package_name_normalizes_edge_cases() {
        assert_eq!(package_name("my app"), "my-app");
        assert_eq!(package_name("123-app"), "app-123-app");
        assert_eq!(package_name(""), "gpui-app");
        assert_eq!(package_name("app"), "app");
    }

    #[test]
    fn pascal_case_normalizes_edge_cases() {
        assert_eq!(pascal_case("my-app"), "MyApp");
        assert_eq!(pascal_case("123"), "App123");
        assert_eq!(pascal_case(""), "App");
    }

    #[test]
    fn title_case_numbers_and_symbols() {
        assert_eq!(title_case("123-app"), "123 App");
        assert_eq!(title_case("under_score"), "Under Score");
    }

    #[test]
    fn separated_identifier_respects_separator() {
        assert_eq!(separated_identifier("my app", '_'), "my_app");
        assert_eq!(separated_identifier("a--b", '-'), "a-b");
    }

    #[test]
    fn relative_path_handles_edge_cases() {
        let base = Path::new("/a/b/c");
        let target = Path::new("/a/b/c");
        assert_eq!(relative_path(base, target), PathBuf::from("."));

        let target = Path::new("/x/y/z");
        assert_eq!(relative_path(base, target), PathBuf::from("../../../x/y/z"));
    }

    #[test]
    fn toolkit_root_discovery_uses_runtime_executable_location() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().join("moved-gpui-toolkit");
        fs::create_dir_all(root.join("crates/gpui-scaffolder"))?;
        fs::create_dir_all(root.join("crates/gpui-miniapp"))?;
        let executable = root.join("target/debug/gpui-scaffolder");
        fs::create_dir_all(executable.parent().context("executable has no parent")?)?;
        fs::write(&executable, "")?;

        assert_eq!(
            toolkit_root_from(None, Some(&executable))?,
            root.canonicalize()?
        );
        Ok(())
    }

    #[test]
    fn toolkit_root_configuration_overrides_executable_location() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().join("configured-gpui-toolkit");
        fs::create_dir_all(root.join("crates/gpui-scaffolder"))?;
        fs::create_dir_all(root.join("crates/gpui-miniapp"))?;

        assert_eq!(
            toolkit_root_from(Some(&root), Some(Path::new("/not/a/toolkit/binary")))?,
            root.canonicalize()?
        );
        Ok(())
    }

    #[test]
    fn string_escaping_helpers() {
        assert_eq!(toml_string(r#"a\"b"#), r#"a\\\"b"#);
        assert_eq!(rust_string(r#"a\"b"#), r#"a\\\"b"#);
        assert_eq!(cargo_path(Path::new(r"C:\path")), "C:/path");
    }

    #[test]
    fn toml_string_escapes_control_characters() -> Result<()> {
        let value = "line one\nline two\t\u{0008}\u{000C}\r\u{0001}\u{007F}";
        let manifest = format!("value = \"{}\"", toml_string(value));
        let parsed: toml::Value = toml::from_str(&manifest)?;

        assert_eq!(parsed["value"].as_str(), Some(value));
        Ok(())
    }

    #[test]
    fn cargo_toml_escapes_package_name_and_path() {
        let names = AppNames::new("demo-app").unwrap();
        let dependencies = DependencyPaths {
            miniapp: Path::new("/tmp/miniapp").to_path_buf(),
            ui_kit: Path::new("/tmp/ui-kit").to_path_buf(),
            ios: Path::new("/tmp/ios").to_path_buf(),
            android: Path::new("/tmp/android").to_path_buf(),
            block: Path::new("/tmp/block").to_path_buf(),
            zed_font_kit: Path::new("/tmp/zed-font-kit").to_path_buf(),
        };
        let toml = cargo_toml(&names, &dependencies, &ScaffoldFlags::default());
        assert!(toml.contains("name = \"demo-app\""));
        assert!(toml.contains("name = \"demo_app\""));
        assert!(toml.contains("/tmp/miniapp"));
        assert!(toml.contains("/tmp/ui-kit"));
        assert!(toml.contains("/tmp/ios"));
        assert!(toml.contains("/tmp/android"));
        assert!(toml.contains("/tmp/block"));
        assert!(toml.contains("/tmp/zed-font-kit"));
    }

    #[test]
    fn app_names_reject_non_ascii_input() {
        let error = AppNames::new("café").expect_err("non-ASCII app name must fail");
        assert!(error.to_string().contains("only ASCII"));
    }

    #[test]
    fn generated_mobile_ffi_guard_reports_panics() -> Result<()> {
        let names = AppNames::new("mobile-guard")?;
        let source = lib_rs(&names, &ScaffoldFlags::default());

        assert!(source.contains("gpui scaffold mobile startup panicked"));
        Ok(())
    }

    #[test]
    fn app_names_with_numeric_start_get_prefix() {
        let names = AppNames::new("123-demo").unwrap();
        assert_eq!(names.package_name, "app-123-demo");
        assert_eq!(names.view_name, "App123DemoView");
    }

    #[test]
    fn scaffold_refuses_existing_non_empty_directory() -> Result<()> {
        let dir = tempdir()?;
        let output = dir.path().join("existing");
        fs::create_dir(&output)?;
        fs::write(output.join("file.txt"), "x")?;

        assert!(
            scaffold_app(&ScaffoldOptions {
                name: "existing".to_owned(),
                output_dir: dir.path().to_path_buf(),
                force: false,
                dry_run: false,
            })
            .is_err()
        );

        Ok(())
    }

    #[test]
    fn scaffold_force_replaces_empty_directory() -> Result<()> {
        let dir = tempdir()?;
        let output = dir.path().join("empty");
        fs::create_dir(&output)?;

        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "empty".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: true,
            dry_run: false,
        })?;

        assert!(scaffolded.app_dir.join("Cargo.toml").is_file());
        Ok(())
    }

    #[test]
    fn scaffold_force_replaces_system_metadata_only_directory() -> Result<()> {
        let dir = tempdir()?;
        let output = dir.path().join("finder-touched");
        fs::create_dir(&output)?;
        fs::write(output.join(".DS_Store"), "metadata")?;

        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "finder-touched".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: true,
            dry_run: false,
        })?;

        assert!(scaffolded.app_dir.join("Cargo.toml").is_file());
        assert!(!scaffolded.app_dir.join(".DS_Store").exists());
        Ok(())
    }

    #[test]
    fn replacement_never_removes_files_added_after_validation() -> Result<()> {
        let dir = tempdir()?;
        let output = dir.path().join("changed");
        fs::create_dir(&output)?;
        ensure_directory_is_replaceable(&output)?;
        let added_file = output.join("added-after-validation.txt");
        fs::write(&added_file, "keep")?;

        assert!(replace_directory(&output).is_err());
        assert!(added_file.is_file());
        Ok(())
    }

    #[test]
    fn scaffold_force_refuses_file_collision() -> Result<()> {
        let dir = tempdir()?;
        let output = dir.path().join("existing");
        fs::write(&output, "x")?;

        assert!(
            scaffold_app(&ScaffoldOptions {
                name: "existing".to_owned(),
                output_dir: dir.path().to_path_buf(),
                force: true,
                dry_run: false,
            })
            .is_err()
        );

        Ok(())
    }

    #[test]
    fn scaffold_dry_run_does_not_write_files() -> Result<()> {
        let dir = tempdir()?;
        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "preview".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: true,
        })?;

        assert_eq!(scaffolded.package_name, "preview");
        assert_eq!(
            scaffolded.app_dir,
            dir.path().canonicalize()?.join("preview")
        );
        assert!(!scaffolded.app_dir.exists());
        assert!(!scaffolded.app_dir.join("gpui-scaffold.toml").exists());

        Ok(())
    }

    #[test]
    fn preview_enumerates_exact_generated_files_without_writing() -> Result<()> {
        let dir = tempdir()?;
        let preview = preview_scaffold(&ScaffoldOptions {
            name: "preview-app".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: false,
        })?;

        assert_eq!(preview.app.package_name, "preview-app");
        assert!(
            preview
                .files
                .iter()
                .any(|path| path.ends_with("Cargo.toml"))
        );
        assert!(
            preview
                .files
                .iter()
                .any(|path| path.ends_with("ios/PreviewAppApp/AppDelegate.swift"))
        );
        assert!(
            preview
                .files
                .iter()
                .any(|path| path.ends_with("android/gradle/app/build.gradle.kts"))
        );
        assert!(preview.files.iter().all(|path| !path.exists()));

        Ok(())
    }

    #[test]
    fn preview_matches_the_complete_generated_file_set() -> Result<()> {
        let dir = tempdir()?;
        let options = ScaffoldOptions {
            name: "preview-complete".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: false,
        };
        let preview = preview_scaffold(&options)?;
        let scaffolded = scaffold_app(&options)?;
        let expected = preview
            .files
            .iter()
            .map(|path| {
                path.strip_prefix(&scaffolded.app_dir)
                    .map(Path::to_path_buf)
            })
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        let actual = generated_file_paths(&scaffolded.app_dir, &scaffolded.app_dir)?;

        assert_eq!(actual, expected);
        Ok(())
    }

    fn generated_file_paths(
        root: &Path,
        directory: &Path,
    ) -> Result<std::collections::BTreeSet<PathBuf>> {
        let mut paths = std::collections::BTreeSet::new();
        let mut directories = vec![directory.to_path_buf()];
        while let Some(directory) = directories.pop() {
            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    directories.push(path);
                } else if entry.file_type()?.is_file() {
                    paths.insert(path.strip_prefix(root)?.to_path_buf());
                }
            }
        }
        Ok(paths)
    }

    #[test]
    fn scaffold_dry_run_force_keeps_empty_directory() -> Result<()> {
        let dir = tempdir()?;
        let output = dir.path().join("empty");
        fs::create_dir(&output)?;

        let scaffolded = scaffold_app(&ScaffoldOptions {
            name: "empty".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: true,
            dry_run: true,
        })?;

        assert_eq!(scaffolded.app_dir, output.canonicalize()?);
        assert!(output.is_dir());
        assert!(fs::read_dir(&output)?.next().is_none());

        Ok(())
    }

    #[test]
    fn scaffold_dry_run_preserves_collision_rules() -> Result<()> {
        let dir = tempdir()?;
        let empty = dir.path().join("empty");
        fs::create_dir(&empty)?;

        assert!(
            scaffold_app(&ScaffoldOptions {
                name: "empty".to_owned(),
                output_dir: dir.path().to_path_buf(),
                force: false,
                dry_run: true,
            })
            .is_err()
        );

        let non_empty = dir.path().join("non-empty");
        fs::create_dir(&non_empty)?;
        fs::write(non_empty.join("file.txt"), "x")?;

        assert!(
            scaffold_app(&ScaffoldOptions {
                name: "non-empty".to_owned(),
                output_dir: dir.path().to_path_buf(),
                force: true,
                dry_run: true,
            })
            .is_err()
        );
        assert!(non_empty.join("file.txt").is_file());

        Ok(())
    }

    fn options_with_flags(
        dir: &Path,
        name: &str,
        flags: ScaffoldFlags,
    ) -> (ScaffoldOptions, ScaffoldFlags) {
        (
            ScaffoldOptions {
                name: name.to_owned(),
                output_dir: dir.to_path_buf(),
                force: false,
                dry_run: false,
            },
            flags,
        )
    }

    #[test]
    fn unknown_template_is_rejected_without_writing() -> Result<()> {
        let dir = tempdir()?;
        let flags = ScaffoldFlags {
            template: "tabs".to_owned(),
            ..ScaffoldFlags::default()
        };
        let (options, flags) = options_with_flags(dir.path(), "bad-template", flags);

        let scaffold_error =
            scaffold_app_with_flags(&options, &flags).expect_err("unknown template must fail");
        assert!(scaffold_error.to_string().contains("unknown template"));
        assert!(!dir.path().join("bad-template").exists());

        let preview_error = preview_scaffold_with_flags(&options, &flags)
            .expect_err("unknown template must fail preview");
        assert!(preview_error.to_string().contains("unknown template"));

        Ok(())
    }

    #[test]
    fn platform_flags_control_the_generated_project() -> Result<()> {
        for (case, flags, expected_files) in [
            (
                "desktop-only",
                ScaffoldFlags {
                    no_ios: true,
                    no_android: true,
                    ..ScaffoldFlags::default()
                },
                8,
            ),
            (
                "no-ios",
                ScaffoldFlags {
                    no_ios: true,
                    ..ScaffoldFlags::default()
                },
                17,
            ),
            (
                "no-android",
                ScaffoldFlags {
                    no_android: true,
                    ..ScaffoldFlags::default()
                },
                13,
            ),
            ("full", ScaffoldFlags::default(), 22),
        ] {
            let dir = tempdir()?;
            let name = format!("flagged-{case}");
            let (options, flags) = options_with_flags(dir.path(), &name, flags);
            let preview = preview_scaffold_with_flags(&options, &flags)?;
            assert_eq!(preview.files.len(), expected_files, "case {case}");
            assert!(
                preview.files.iter().all(|path| !path.exists()),
                "case {case}: preview must not write files"
            );

            let scaffolded = scaffold_app_with_flags(&options, &flags)?;
            let actual = generated_file_paths(&scaffolded.app_dir, &scaffolded.app_dir)?;
            let expected = preview
                .files
                .iter()
                .map(|path| {
                    path.strip_prefix(&scaffolded.app_dir)
                        .map(Path::to_path_buf)
                })
                .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
            assert_eq!(actual, expected, "case {case}: preview must equal apply");

            // Match on app-relative paths: the app name itself may contain
            // substrings like "ios" (e.g. "flagged-no-ios").
            let relative: Vec<String> = preview
                .files
                .iter()
                .map(|path| {
                    path.strip_prefix(&scaffolded.app_dir)
                        .map(|relative| relative.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .collect();
            let has_ios = relative.iter().any(|path| path.contains("ios"));
            let has_android = relative.iter().any(|path| path.contains("android"));
            assert_eq!(has_ios, !flags.no_ios, "case {case}");
            assert_eq!(has_android, !flags.no_android, "case {case}");

            let manifest = fs::read_to_string(scaffolded.app_dir.join("Cargo.toml"))?;
            let _: toml::Value = toml::from_str(&manifest)?;
            assert_eq!(manifest.contains("gpui-ios"), !flags.no_ios, "case {case}");
            assert_eq!(
                manifest.contains("gpui-android"),
                !flags.no_android,
                "case {case}"
            );
            // Desktop core is unconditional.
            assert!(manifest.contains("gpui-miniapp"), "case {case}");
            assert!(manifest.contains("gpui-ui-kit"), "case {case}");

            let lib = fs::read_to_string(scaffolded.app_dir.join("src/lib.rs"))?;
            assert_eq!(
                lib.contains("gpui_ios::ios::ffi::run_app"),
                !flags.no_ios,
                "case {case}"
            );
            assert_eq!(
                lib.contains("android_main"),
                !flags.no_android,
                "case {case}"
            );

            let justfile = fs::read_to_string(scaffolded.app_dir.join("Justfile"))?;
            assert!(justfile.contains("cargo run"), "case {case}");
            assert_eq!(justfile.contains("ios-sim:"), !flags.no_ios, "case {case}");
            assert_eq!(
                justfile.contains("android-apk:"),
                !flags.no_android,
                "case {case}"
            );

            let readme = fs::read_to_string(scaffolded.app_dir.join("README.md"))?;
            assert_eq!(
                readme.contains("just ios-sim"),
                !flags.no_ios,
                "case {case}"
            );
            assert_eq!(
                readme.contains("just android-apk"),
                !flags.no_android,
                "case {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn scaffold_metadata_records_template_and_platform_flags() -> Result<()> {
        let dir = tempdir()?;
        let flags = ScaffoldFlags {
            no_android: true,
            ..ScaffoldFlags::default()
        };
        let (options, flags) = options_with_flags(dir.path(), "meta-app", flags);
        let scaffolded = scaffold_app_with_flags(&options, &flags)?;
        let metadata: toml::Value = toml::from_str(&fs::read_to_string(
            scaffolded.app_dir.join("gpui-scaffold.toml"),
        )?)?;
        let metadata = metadata
            .get("scaffold")
            .context("gpui-scaffold.toml must contain [scaffold]")?;
        assert_eq!(
            metadata.get("template").and_then(toml::Value::as_str),
            Some("default")
        );
        assert_eq!(
            metadata.get("no_ios").and_then(toml::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            metadata.get("no_android").and_then(toml::Value::as_bool),
            Some(true)
        );
        Ok(())
    }

    #[test]
    fn default_flag_output_matches_unflagged_api() -> Result<()> {
        let dir = tempdir()?;
        let options = ScaffoldOptions {
            name: "parity-app".to_owned(),
            output_dir: dir.path().to_path_buf(),
            force: false,
            dry_run: false,
        };
        let preview = preview_scaffold(&options)?;
        let (flagged_options, flags) =
            options_with_flags(dir.path(), "parity-app", ScaffoldFlags::default());
        let flagged_preview = preview_scaffold_with_flags(&flagged_options, &flags)?;
        assert_eq!(preview, flagged_preview);
        Ok(())
    }

    #[test]
    fn write_file_creates_parents_and_replaces_atomically() -> Result<()> {
        let dir = tempdir()?;
        let target = dir.path().join("nested").join("deep").join("file.txt");

        write_file(&target, "first")?;
        assert_eq!(fs::read_to_string(&target)?, "first");

        write_file(&target, "second")?;
        assert_eq!(fs::read_to_string(&target)?, "second");

        // No same-directory temporary files may survive the rename.
        let leftovers = fs::read_dir(target.parent().context("target has no parent")?)?
            .collect::<std::io::Result<Vec<_>>>()?;
        assert_eq!(leftovers.len(), 1);
        Ok(())
    }

    #[test]
    fn toolkit_dependency_paths_match_direct_relative_paths() {
        for (from, root) in [
            ("/toolkit/out/my-app", "/toolkit"),
            ("/other/out/my-app", "/toolkit"),
            ("/toolkit", "/toolkit"),
        ] {
            let from = Path::new(from);
            let root = Path::new(root);
            let dependencies = toolkit_dependency_paths(from, root);
            assert_eq!(
                dependencies.miniapp,
                relative_path(from, &root.join("crates/gpui-miniapp"))
            );
            assert_eq!(
                dependencies.ui_kit,
                relative_path(from, &root.join("crates/gpui-ui-kit"))
            );
            assert_eq!(
                dependencies.ios,
                relative_path(from, &root.join("crates/gpui-ios"))
            );
            assert_eq!(
                dependencies.android,
                relative_path(from, &root.join("crates/gpui-android"))
            );
            assert_eq!(
                dependencies.block,
                relative_path(from, &root.join("crates/3rdparties/block"))
            );
            assert_eq!(
                dependencies.zed_font_kit,
                relative_path(from, &root.join("crates/3rdparties/zed-font-kit"))
            );
        }
    }

    #[test]
    fn android_java_sources_match_toolkit_checkout() -> Result<()> {
        let root = toolkit_root()?;
        let (activity, provider) = android_java_sources(&root)?;
        assert!(activity.contains("GpuiActivity"));
        assert!(provider.contains("GpuiFileProvider"));

        // Whatever the `mobile` feature selects, the content must equal the
        // checkout files so embedded and runtime payloads cannot diverge.
        assert_eq!(
            activity,
            fs::read_to_string(root.join(GPUI_ANDROID_ACTIVITY_JAVA_PATH))?
        );
        assert_eq!(
            provider,
            fs::read_to_string(root.join(GPUI_ANDROID_FILE_PROVIDER_JAVA_PATH))?
        );
        Ok(())
    }
}
