//! trybuild-style negative tests without the trybuild crate.
//!
//! The `trybuild` crate cannot be fetched in this offline workspace, so this
//! module provides the same value with zero new dependencies: each fixture in
//! `tests/ui/*.rs` declares its derive macro (`// derive: <Macro>`) and the
//! diagnostics it must produce (`// expect-error: <substring>`). The harness
//! runs the derive implementation over the fixture source and asserts every
//! expected diagnostic appears in the emitted `compile_error!` output.
//!
//! To add a case, drop a new fixture file in `tests/ui/` — no harness changes
//! needed. Fixtures are never compiled as Rust; they are parsed as token
//! streams, exactly what the proc macro receives.

use super::derive::{derive_component_builder_impl, derive_component_theme_impl};
use super::variant::derive_component_variant_impl;

struct UiCase {
    file: &'static str,
    derive: &'static str,
    expected_errors: Vec<String>,
    source: String,
}

fn load_case(file: &std::path::Path) -> UiCase {
    let source = std::fs::read_to_string(file).expect("read ui fixture");
    let mut derive = None;
    let mut expected_errors = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed
            .strip_prefix("// derive:")
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            derive = Some(name.to_string());
        } else if let Some(message) = trimmed
            .strip_prefix("// expect-error:")
            .map(str::trim)
            .filter(|message| !message.is_empty())
        {
            expected_errors.push(message.to_string());
        }
    }
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    assert!(
        derive.is_some(),
        "{name}: fixture is missing `// derive: <Macro>` header"
    );
    assert!(
        !expected_errors.is_empty(),
        "{name}: fixture needs at least one `// expect-error: <substring>` header"
    );
    UiCase {
        file: Box::leak(name.to_string().into_boxed_str()),
        derive: Box::leak(derive.unwrap().into_boxed_str()),
        expected_errors,
        source,
    }
}

fn expand(case: &UiCase) -> String {
    let tokens: proc_macro2::TokenStream = case.source.parse().expect("fixture parses as tokens");
    match case.derive {
        "ComponentTheme" => derive_component_theme_impl(tokens).to_string(),
        "ComponentBuilder" | "FormField" => derive_component_builder_impl(tokens).to_string(),
        "ComponentVariant" => derive_component_variant_impl(tokens).to_string(),
        other => panic!("{}: unknown derive `{other}`", case.file),
    }
}

#[test]
fn ui_fixtures_emit_expected_errors() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui_dir = manifest.join("tests").join("ui");
    let mut entries: Vec<_> = std::fs::read_dir(&ui_dir)
        .expect("read tests/ui")
        .map(|entry| entry.expect("ui dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "tests/ui holds no fixtures");

    for path in entries {
        let case = load_case(&path);
        let output = expand(&case);
        assert!(
            output.contains("compile_error !"),
            "{}: expected a compile_error, got: {output}",
            case.file
        );
        for expected in &case.expected_errors {
            assert!(
                output.contains(expected.as_str()),
                "{}: expected diagnostic containing `{expected}`, got: {output}",
                case.file
            );
        }
    }
}
