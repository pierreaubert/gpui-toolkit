# gpui-design-tools

Toolkit-owned design token tooling backed by `gpui_design::DesignSystem`.

## Commands

```bash
cargo run -p gpui-design-tools --bin gpui-export-design-tokens
cargo run -p gpui-design-tools --bin gpui-import-design-tokens -- --input tokens.json
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens
```

All commands accept `--format style-dictionary-json`; the aliases
`style_dictionary_json` and `json` are also accepted.

Export tokens:

```bash
cargo run -p gpui-design-tools --bin gpui-export-design-tokens -- \
  --format style-dictionary-json \
  --output target/design/gpui-tokens.json
```

Import and shape-check tokens:

```bash
cargo run -p gpui-design-tools --bin gpui-import-design-tokens -- \
  --format style-dictionary-json \
  --input target/design/gpui-tokens.json
```

`gpui-validate-design-tokens` supports CI report output:

```bash
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens -- \
  --report-json target/gpui-conformance/design-tokens.json \
  --report-markdown target/gpui-conformance/design-tokens.md
```

Use `--json` to print the machine-readable report to stdout instead of the
markdown table:

```bash
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens -- --json
```

## Machine-readable report contract

`gpui-validate-design-tokens --json` and `--report-json` emit a stable JSON
object with this schema:

```json
{
  "schema_version": 1,
  "report_type": "gpui-design-token-validation",
  "passed": true,
  "findings": [],
  "preset_count": 4,
  "token_count": 128,
  "conformance_markdown": ""
}
```

Field meanings:

- `schema_version`: integer report schema version. Version `1` is the current
  contract.
- `report_type`: stable discriminator, always
  `gpui-design-token-validation`.
- `passed`: `true` when no token-shape or conformance findings were produced.
- `findings`: stable string identifiers or validation messages suitable for CI
  logs.
- `preset_count`: number of design-system presets inspected.
- `token_count`: total number of tokens inspected across all presets.
- `conformance_markdown`: markdown table text when markdown rendering is
  requested; otherwise an empty string.

Compatibility policy: additive fields require a schema-version bump, and
existing fields keep their names and JSON types within a schema version.

## Design Handoff Readiness

`design_tooling_handoff_report()` exposes a schema-versioned release artifact
for design-tool maturity checks. It records which handoff pieces are implemented
locally, which companion workflows are documented, which static Figma Code
Connect mappings are repository artifacts, and which live-preview rows are
still external release gates.

```rust
use gpui_design_tools::design_tooling_handoff_report;

let report = design_tooling_handoff_report();
assert!(report.item("token-export").is_some());
assert!(report.item("figma-code-connect").is_some());
assert!(
    report
        .blocking_entries()
        .iter()
        .any(|item| item.id == "live-preview-plugin")
);
```

The JSON contract uses `schema_version = 1` and
`report_type = "gpui-design-tooling-handoff"`. Rows include `id`, `title`,
`artifact_type`, `path_or_command`, `status`, `release_evidence`, and
`remaining_gap`. Status values are `implemented`, `documented`, and
`external-gate`.

Static Figma Code Connect handoff lives in `figma/CODE_CONNECT_MAPPINGS.md`.
That artifact maps toolkit component sets to GPUI APIs, token sources, and
visual QA artifacts. Live Figma publication, bidirectional token editing, and
in-canvas preview sessions remain separate external release gates.

This crate is generic toolkit infrastructure and must not depend on
`sotf-gpui`.
