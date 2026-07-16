#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

const GPUI_VERSION: &str = "0.2.2";
const GPUI_ZED_TAG: &str = "v1.9.0";
const SCAFFOLD_TEMPLATE_VERSION: &str = "1";

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
    let names = AppNames::new(&options.name)?;
    let output_dir = options
        .output_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", options.output_dir.display()))?;
    let app_dir = output_dir.join(&names.directory_name);

    if app_dir.exists() {
        if options.force {
            ensure_directory_is_replaceable(&app_dir)?;
            if !options.dry_run {
                fs::remove_dir_all(&app_dir)
                    .with_context(|| format!("failed to remove {}", app_dir.display()))?;
            }
        } else {
            bail!(
                "{} already exists; pass --force to replace an empty scaffold",
                app_dir.display()
            );
        }
    }

    if options.dry_run {
        return Ok(ScaffoldedApp {
            app_dir,
            package_name: names.package_name,
            title: names.title,
        });
    }

    fs::create_dir_all(app_dir.join("src"))
        .with_context(|| format!("failed to create {}", app_dir.display()))?;
    fs::create_dir_all(app_dir.join("ios").join(&names.ios_source_dir))
        .with_context(|| format!("failed to create {}", app_dir.join("ios").display()))?;
    fs::create_dir_all(app_dir.join("ios/lib"))
        .with_context(|| format!("failed to create {}", app_dir.join("ios/lib").display()))?;
    fs::create_dir_all(app_dir.join("android/gradle/app/src/main/res/values"))
        .with_context(|| format!("failed to create {}", app_dir.join("android").display()))?;
    fs::create_dir_all(app_dir.join("android/gradle/app/src/main/jniLibs/arm64-v8a"))
        .with_context(|| format!("failed to create {}", app_dir.join("android").display()))?;

    let toolkit_root = toolkit_root()?;
    let miniapp_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-miniapp"));
    let ui_kit_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-ui-kit"));
    let ios_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-ios"));
    let android_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-android"));
    let block_path = relative_path(&app_dir, &toolkit_root.join("crates/3rdparties/block"));
    let zed_font_kit_path = relative_path(
        &app_dir,
        &toolkit_root.join("crates/3rdparties/zed-font-kit"),
    );

    write_file(
        &app_dir.join("Cargo.toml"),
        &cargo_toml(
            &names,
            &miniapp_path,
            &ui_kit_path,
            &ios_path,
            &android_path,
            &block_path,
            &zed_font_kit_path,
        ),
    )?;
    write_file(
        &app_dir.join("gpui-scaffold.toml"),
        &scaffold_metadata(&names),
    )?;
    write_file(&app_dir.join("Justfile"), &justfile(&names))?;
    write_file(&app_dir.join("README.md"), &readme(&names))?;
    write_file(&app_dir.join("src/app.rs"), &app_rs(&names))?;
    write_file(&app_dir.join("src/lib.rs"), &lib_rs(&names))?;
    write_file(&app_dir.join("src/main.rs"), &main_rs(&names))?;
    write_file(&app_dir.join("ios/project.yml"), &xcode_project_yml(&names))?;
    write_file(
        &app_dir
            .join("ios")
            .join(&names.ios_source_dir)
            .join("AppDelegate.swift"),
        &swift_app_delegate(&names),
    )?;
    write_file(
        &app_dir
            .join("ios")
            .join(&names.ios_source_dir)
            .join("BridgingHeader.h"),
        &bridging_header(&names),
    )?;
    write_file(
        &app_dir
            .join("ios")
            .join(&names.ios_source_dir)
            .join("Info.plist"),
        &info_plist(&names),
    )?;
    write_file(
        &app_dir
            .join("ios")
            .join(&names.ios_source_dir)
            .join("Entitlements.plist"),
        &entitlements_plist(),
    )?;
    write_file(
        &app_dir.join("android/gradle/settings.gradle.kts"),
        &android_settings_gradle(&names),
    )?;
    write_file(
        &app_dir.join("android/gradle/build.gradle.kts"),
        &android_root_build_gradle(),
    )?;
    write_file(
        &app_dir.join("android/gradle/gradle.properties"),
        &android_gradle_properties(),
    )?;
    write_file(
        &app_dir.join("android/gradle/app/build.gradle.kts"),
        &android_app_build_gradle(&names),
    )?;
    write_file(
        &app_dir.join("android/gradle/app/src/main/AndroidManifest.xml"),
        &android_manifest(),
    )?;
    write_file(
        &app_dir.join("android/gradle/app/src/main/res/values/strings.xml"),
        &android_strings(&names),
    )?;
    write_file(
        &app_dir.join("android/gradle/app/src/main/res/values/styles.xml"),
        &android_styles(),
    )?;

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
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn cargo_toml(
    names: &AppNames,
    miniapp_path: &Path,
    ui_kit_path: &Path,
    ios_path: &Path,
    android_path: &Path,
    block_path: &Path,
    zed_font_kit_path: &Path,
) -> String {
    format!(
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

[target.'cfg(any(target_os = "ios", target_os = "tvos"))'.dependencies]
gpui-ios = {{ path = "{ios_path}" }}

[target.'cfg(target_os = "android")'.dependencies]
android-activity = {{ version = "0.6", features = ["native-activity"] }}
android_logger = "0.15"
gpui-android = {{ path = "{android_path}" }}
log = "0.4"

[patch."https://github.com/zed-industries/font-kit"]
zed-font-kit = {{ path = "{zed_font_kit_path}" }}

[patch.crates-io]
block = {{ path = "{block_path}" }}
"#,
        package_name = toml_string(&names.package_name),
        library_name = toml_string(&names.library_name),
        gpui_version = GPUI_VERSION,
        gpui_zed_tag = GPUI_ZED_TAG,
        miniapp_path = cargo_path(miniapp_path),
        ui_kit_path = cargo_path(ui_kit_path),
        ios_path = cargo_path(ios_path),
        android_path = cargo_path(android_path),
        block_path = cargo_path(block_path),
        zed_font_kit_path = cargo_path(zed_font_kit_path),
    )
}

fn scaffold_metadata(names: &AppNames) -> String {
    format!(
        r#"[scaffold]
generator = "gpui-scaffolder"
generator_version = "{generator_version}"
template_version = "{template_version}"
package_name = "{package_name}"
title = "{title}"
gpui_version = "{gpui_version}"
gpui_zed_tag = "{gpui_zed_tag}"
"#,
        generator_version = env!("CARGO_PKG_VERSION"),
        template_version = SCAFFOLD_TEMPLATE_VERSION,
        package_name = toml_string(&names.package_name),
        title = toml_string(&names.title),
        gpui_version = GPUI_VERSION,
        gpui_zed_tag = GPUI_ZED_TAG,
    )
}

fn justfile(names: &AppNames) -> String {
    format!(
        r#"default:
	just --list

run:
	cargo run

check:
	cargo check

# Build the Rust static library for the iOS simulator and stage it for Xcode.
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

# Build the Rust dynamic library for Android ARM64 and stage it for Gradle.
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
        xcode_target_name = names.xcode_target_name,
        ios_source_dir = names.ios_source_dir,
    )
}

fn readme(names: &AppNames) -> String {
    format!(
        "# {title}\n\nRun the desktop app with:\n\n```sh\ncargo run\n```\n\nOr, if you use `just`:\n\n```sh\njust run\n```\n\nBuild the generated iOS app with:\n\n```sh\njust ios-sim\njust ios\n```\n\nBuild the generated tvOS Rust library with:\n\n```sh\njust tvos-sim\njust tvos\n```\n\nBuild the generated Android Rust library and debug APK with:\n\n```sh\njust android-rust\njust android-apk\n```\n\nThe iOS host lives in `ios/`, uses XcodeGen, and includes `ios/{ios_source_dir}/Entitlements.plist`. To sign an already-built device app explicitly:\n\n```sh\nIOS_SIGN_IDENTITY=\"Apple Development: Your Name\" just ios-sign\n```\n\nThe Android host lives in `android/gradle/`, uses Android `NativeActivity`, and loads `lib{library_name}.so` from `app/src/main/jniLibs/arm64-v8a/`. Install the Android SDK/NDK before running the Android recipes.\n",
        title = names.title,
        ios_source_dir = names.ios_source_dir,
        library_name = names.library_name,
    )
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

fn lib_rs(names: &AppNames) -> String {
    format!(
        r#"mod app;

pub use app::{{open_app_window, run_desktop, {view_name}}};

#[cfg(any(target_os = "ios", target_os = "tvos"))]
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
            Err(_) => R::default(),
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

#[cfg(target_os = "android")]
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
        ffi_start_symbol = names.ffi_start_symbol,
        library_name = rust_string(&names.library_name),
        view_name = names.view_name,
    )
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
            android:name="android.app.NativeActivity"
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
        assert!(android_manifest.contains("android.app.NativeActivity"));
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

        run_scaffolded_cargo_check_with_toolchain(
            &scaffolded.app_dir.join("Cargo.toml"),
            &dir.path().join("target-tvos"),
            Some("nightly"),
            &["--lib", "--target", "aarch64-apple-tvos-sim", "-Zbuild-std"],
            "scaffolded tvOS simulator library failed cargo check",
        )?;

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
    fn generated_gpui_tag_matches_workspace_dependency() -> Result<()> {
        let manifest: toml::Value = toml::from_str(include_str!("../../../Cargo.toml"))?;
        let workspace_tag = manifest
            .get("workspace")
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(|dependencies| dependencies.get("gpui"))
            .and_then(|gpui| gpui.get("tag"))
            .and_then(toml::Value::as_str)
            .context("workspace gpui dependency must have a tag")?;

        assert_eq!(GPUI_ZED_TAG, workspace_tag);
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
    fn string_escaping_helpers() {
        assert_eq!(toml_string(r#"a\"b"#), r#"a\\\"b"#);
        assert_eq!(rust_string(r#"a\"b"#), r#"a\\\"b"#);
        assert_eq!(cargo_path(Path::new(r"C:\path")), "C:/path");
    }

    #[test]
    fn cargo_toml_escapes_package_name_and_path() {
        let names = AppNames::new("demo-app").unwrap();
        let toml = cargo_toml(
            &names,
            Path::new("/tmp/miniapp"),
            Path::new("/tmp/ui-kit"),
            Path::new("/tmp/ios"),
            Path::new("/tmp/android"),
            Path::new("/tmp/block"),
            Path::new("/tmp/zed-font-kit"),
        );
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
}
