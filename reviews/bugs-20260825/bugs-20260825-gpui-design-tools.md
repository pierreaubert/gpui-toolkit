# Bug Review: gpui-design-tools — 2026-08-25

Scope: `gpui-design-tools` is the design-token CLI tooling crate — a
880-line `src/lib.rs` (of which ~440 lines are tests), three small clap
binaries (`gpui_export_design_tokens.rs`, `gpui_import_design_tokens.rs`,
`gpui_validate_design_tokens.rs`, 20–93 lines each), plus `Cargo.toml`,
`README.md`, `TUTORIAL.md`, and `CHANGELOG.md`. I read every tracked file end
to end. The crate is pure offline JSON tooling: no GPUI rendering, no wgpu,
no threading, no async, no unsafe. Its only dependency of note is
`gpui-design` (`DesignTokenExport`, `DesignConformanceMatrix`); I spot-checked
those definitions in `crates/gpui-design/src/` to confirm the wire shapes the
validator assumes. `cargo test -p gpui-design-tools` passes: 31/31 lib tests
(the only test target with tests).

## Findings

No Critical or High findings. One Medium, three Low.

- **Medium — Export CLI fails whenever the output directory does not already
  exist, including its own default path.**
  `crates/gpui-design-tools/src/lib.rs:292-295`
  (`export_design_tokens_to_path` calls `std::fs::write` directly) combined
  with `crates/gpui-design-tools/src/bin/gpui_export_design_tokens.rs:10`
  (default `--output design-tokens/gpui-tokens.json` — a directory that does
  not exist in a fresh clone) and the README's example `--output
  target/design/gpui-tokens.json` (README.md:20-21). I confirmed empirically:
  running the binary with `--output /tmp/dt-review-check/sub/tokens.json`
  fails with `No such file or directory (os error 2)`. The sibling validate
  binary does this correctly — `write_report`
  (`crates/gpui-design-tools/src/bin/gpui_validate_design_tokens.rs:87-92`)
  calls `std::fs::create_dir_all(parent)` first. Impact: the documented,
  default invocation of the export command errors out on first use; CI jobs
  writing into a fresh `target/` tree hit the same wall. Fix: hoist the
  `create_dir_all(parent)` guard from `write_report` into
  `export_design_tokens_to_path` (or a shared helper in the lib) so all
  three binaries behave identically.

- **Low — `path` array elements are not validated as strings, and duplicate
  tokens/presets are not detected.**
  `crates/gpui-design-tools/src/lib.rs:346-351` — `inspect_token_value` only
  checks `token.path` is an array; an element list like `[1, 2]` passes
  import validation even though the upstream type
  (`crates/gpui-design/src/design_token.rs:7`, `path: Vec<&'static str>`)
  and the Style Dictionary format require string segments. Similarly there
  is no check for duplicate `preset_id`s or duplicate token names/paths
  within a preset, so a hand-edited document with two colliding tokens
  imports cleanly and the collision surfaces nowhere. Fix: extend the `path`
  check to `as_array().is_some_and(|a| a.iter().all(|v| v.is_string()))` and,
  if the contract cares, collect `(preset_id, token.name)` pairs in a
  `HashSet` during the walk and flag duplicates.

- **Low — The JSON report is serialized twice when `--json` and
  `--report-json` are combined.**
  `crates/gpui-design-tools/src/bin/gpui_validate_design_tokens.rs:53` and
  `:69` — `serde_json::to_string_pretty(&report)` runs once for stdout and
  again for the file. Reports are small (4 presets, ~128 tokens), so this is
  cosmetic, but it is also a divergence hazard: if one call site is ever
  changed (e.g. compact vs pretty) the two outputs silently differ. Fix:
  `let json = serde_json::to_string_pretty(&report)?;` once, then
  `println!("{json}")` / `write_report(path, json)`.

- **Low — Report writes are non-atomic.**
  `crates/gpui-design-tools/src/bin/gpui_validate_design_tokens.rs:91` and
  `crates/gpui-design-tools/src/lib.rs:294` — `std::fs::write` truncates in
  place, so a crash or full disk mid-write leaves a truncated report at the
  target path that a CI artifact collector could pick up as a valid result.
  Low likelihood for ~KB-sized files, hence Low. Fix if it ever matters:
  write to `<path>.tmp` in the same directory and `std::fs::rename` over the
  target.

Notes on things that look like bugs but are not:

- `write_report` calling `path.parent()` on a bare filename yields
  `Some("")`, and `create_dir_all("")` is a no-op success via the std
  fast-path for empty paths — no bug (`gpui_validate_design_tokens.rs:88-90`).
- `need_markdown = !args.json || args.report_markdown.is_some()`
  (`gpui_validate_design_tokens.rs:45`) looks convoluted but is correct for
  all four flag combinations: markdown is skipped only when output is
  exclusively JSON, matching the documented contract that
  `conformance_markdown` is `""` when rendering is not requested.
- The conformance-finding formatting block is duplicated verbatim between
  `validate_raw_tokens` (lib.rs:264-272) and
  `validate_design_token_export` (lib.rs:405-413). Duplication, not a bug;
  worth a shared helper only if a third caller appears.
- `import_design_tokens` bails on shape findings while
  `validate_design_tokens` reports them in `findings` — an intentional
  import-vs-validate distinction, consistent with the README.

## Clean bill

- No threading, locks, channels, `RefCell`, async, or unsafe anywhere;
  nothing to deadlock, race, or borrow-panic.
- Every `unwrap`/`expect` is inside `#[cfg(test)]`; all production error
  paths go through `anyhow` with context. No panic paths reachable from the
  CLIs beyond clap's own exits.
- No allocation concerns: `inspect_token_value` deliberately builds its
  per-token error prefix lazily and reuses it across the four field checks
  (lib.rs:339-363, covered by two dedicated tests); findings are bounded by
  input size; markdown rendering is skipped when not requested.
- JSON report contracts (`schema_version`/`report_type`/key sets for both
  reports) are pinned by stability tests (lib.rs:551-585, 658-693), and the
  `figma/CODE_CONNECT_MAPPINGS.md` artifact is asserted via `include_str!`
  so it cannot silently go missing.
- No GPU code (no wgpu/vello), no UI rendering — the GPU/CPU data-flow and
  UI/UX sections are omitted as inapplicable.

## Resolution — 2026-08-25

- Fixed the confirmed export-path bug: `export_design_tokens_to_path` now creates a non-empty missing parent directory before writing. Added a nested-output regression test; verified with `cargo test -p gpui-design-tools export_design_tokens_to_path`.
- Fixed duplicate JSON serialization: the validation CLI now serializes the report once and shares that string between `--json` stdout and `--report-json`. Verified the binary with both flags; the two reports differ only by stdout's terminating newline.
- Fixed non-atomic report writes: token exports and validator reports now write and sync a same-directory temporary file before replacing the destination. Added an existing-output replacement regression test; verified with `cargo test -p gpui-design-tools export_design_tokens_to_path`.
- Fixed malformed token paths: import validation now rejects any `path` array containing a non-string segment, with a focused regression test verified by `cargo test -p gpui-design-tools inspect_token_value`.
- Verified duplicate preset IDs/token names are not a current data-loss bug: `ImportedDesignTokens` retains the source JSON without building a keyed map or overwriting a collision, and the documented wire contract has no uniqueness invariant. No stricter schema rule was added.
