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
    fs::create_dir_all(app_dir.join("ios").join(&names.ios_source_dir))
        .with_context(|| format!("failed to create {}", app_dir.join("ios").display()))?;
    fs::create_dir_all(app_dir.join("ios/lib"))
        .with_context(|| format!("failed to create {}", app_dir.join("ios/lib").display()))?;

    let toolkit_root = toolkit_root()?;
    let miniapp_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-miniapp"));
    let ui_kit_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-ui-kit"));
    let ios_path = relative_path(&app_dir, &toolkit_root.join("crates/gpui-ios"));

    write_file(
        &app_dir.join("Cargo.toml"),
        &cargo_toml(&names, &miniapp_path, &ui_kit_path, &ios_path),
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
gpui = {{ version = "0.2.2", git = "https://github.com/zed-industries/zed.git", tag = "v1.0.0" }}
gpui-miniapp = {{ path = "{miniapp_path}" }}
gpui-ui-kit = {{ path = "{ui_kit_path}" }}

[target.'cfg(any(target_os = "ios", target_os = "tvos"))'.dependencies]
gpui-ios = {{ path = "{ios_path}" }}
"#,
        package_name = toml_string(&names.package_name),
        library_name = toml_string(&names.library_name),
        miniapp_path = cargo_path(miniapp_path),
        ui_kit_path = cargo_path(ui_kit_path),
        ios_path = cargo_path(ios_path),
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
"#,
        library_name = names.library_name,
        xcode_target_name = names.xcode_target_name,
        ios_source_dir = names.ios_source_dir,
    )
}

fn readme(names: &AppNames) -> String {
    format!(
        "# {title}\n\nRun the desktop app with:\n\n```sh\ncargo run\n```\n\nOr, if you use `just`:\n\n```sh\njust run\n```\n\nBuild the generated iOS app with:\n\n```sh\njust ios-sim\njust ios\n```\n\nThe iOS host lives in `ios/`, uses XcodeGen, and includes `ios/{ios_source_dir}/Entitlements.plist`. To sign an already-built device app explicitly:\n\n```sh\nIOS_SIGN_IDENTITY=\"Apple Development: Your Name\" just ios-sign\n```\n",
        title = names.title,
        ios_source_dir = names.ios_source_dir,
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
"#,
        ffi_start_symbol = names.ffi_start_symbol,
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
        })?;

        assert_eq!(scaffolded.package_name, "demo-app");
        assert!(scaffolded.app_dir.join("Cargo.toml").is_file());
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

        let manifest = fs::read_to_string(scaffolded.app_dir.join("Cargo.toml"))?;
        assert!(manifest.contains("[workspace]"));
        assert!(manifest.contains("crate-type = [\"rlib\", \"staticlib\", \"cdylib\"]"));
        assert!(manifest.contains("gpui-miniapp"));
        assert!(manifest.contains("gpui-ui-kit"));
        assert!(manifest.contains("gpui-ios"));

        let justfile = fs::read_to_string(scaffolded.app_dir.join("Justfile"))?;
        assert!(justfile.contains("ios-sim:"));
        assert!(justfile.contains("ios:"));
        assert!(justfile.contains("ios-rust-sim:"));
        assert!(justfile.contains("ios-xcodegen:"));
        assert!(justfile.contains("xcodebuild"));
        assert!(justfile.contains("ios-sign"));
        assert!(justfile.contains("tvos-sim:"));
        assert!(justfile.contains("tvos:"));

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

        let app = fs::read_to_string(scaffolded.app_dir.join("src/main.rs"))?;
        assert!(app.contains("demo_app::run_desktop"));

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
        );
        assert!(toml.contains("name = \"demo-app\""));
        assert!(toml.contains("name = \"demo_app\""));
        assert!(toml.contains("/tmp/miniapp"));
        assert!(toml.contains("/tmp/ui-kit"));
        assert!(toml.contains("/tmp/ios"));
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
            })
            .is_err()
        );

        Ok(())
    }
}
