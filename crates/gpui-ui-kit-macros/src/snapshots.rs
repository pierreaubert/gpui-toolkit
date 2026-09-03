//! Expansion snapshots: `cargo expand` in test form.
//!
//! Each input in `tests/snapshots/*.rs` declares its derive macro with a
//! `// derive: <Macro>` header. The harness expands it and diffs the result
//! against the checked-in `<name>.expanded.rs` file, so macro-output growth
//! (the downstream monomorphization cost called out in the review) is visible
//! in code review before it lands in hot `render()` paths.
//!
//! To bless new output after an intentional macro change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p gpui-ui-kit-macros snapshots
//! ```
//!
//! For full downstream expansion of a real component, use `cargo expand`
//! (install with `cargo install cargo-expand`, nightly required):
//!
//! ```sh
//! cargo expand -p gpui-ui-kit button::types
//! ```

use super::derive::{derive_component_builder_impl, derive_component_theme_impl};
use super::variant::derive_component_variant_impl;

fn expand_snapshot(source: &str) -> (String, String) {
    let derive = source
        .lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("// derive:")
                .map(str::trim)
                .filter(|name| !name.is_empty())
        })
        .expect("snapshot input is missing `// derive: <Macro>` header");
    let name = match derive {
        "ComponentTheme" | "ComponentBuilder" | "FormField" | "ComponentVariant" => derive,
        other => panic!("unknown derive `{other}`"),
    };
    let tokens: proc_macro2::TokenStream = source.parse().expect("snapshot parses as tokens");
    let output = match name {
        "ComponentTheme" => derive_component_theme_impl(tokens).to_string(),
        "ComponentVariant" => derive_component_variant_impl(tokens).to_string(),
        _ => derive_component_builder_impl(tokens).to_string(),
    };
    (name.to_string(), output)
}

#[test]
fn expansions_match_checked_in_snapshots() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshots_dir = manifest.join("tests").join("snapshots");
    let mut entries: Vec<_> = std::fs::read_dir(&snapshots_dir)
        .expect("read tests/snapshots")
        .map(|entry| entry.expect("snapshots dir entry").path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "rs")
                && !path
                    .file_stem()
                    .is_some_and(|stem| stem.to_string_lossy().ends_with(".expanded"))
        })
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "tests/snapshots holds no inputs");

    let bless = std::env::var("UPDATE_SNAPSHOTS").is_ok();
    for input_path in entries {
        let source = std::fs::read_to_string(&input_path).expect("read snapshot input");
        let (_, output) = expand_snapshot(&source);
        let expected_path = input_path.with_extension("expanded.rs");
        if bless {
            std::fs::write(&expected_path, &output).expect("bless snapshot");
            continue;
        }
        let expected = std::fs::read_to_string(&expected_path).unwrap_or_else(|_| {
            panic!(
                "missing snapshot {}; bless with UPDATE_SNAPSHOTS=1",
                expected_path.display()
            )
        });
        assert!(
            output == expected,
            "expansion drift in {}: bless with UPDATE_SNAPSHOTS=1 after reviewing the diff",
            input_path.display()
        );
        // Spot markers so a passing test still proves the interesting impls exist.
        assert!(output.contains("automatically_derived"), "marker check");
    }
}
