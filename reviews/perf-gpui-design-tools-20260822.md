# Perf review: gpui-design-tools

Date: 2026-08-22

## Role and hot paths

`gpui-design-tools` (`crates/gpui-design-tools`, v0.9.4) is **batch CLI tooling**:
design-token import/export/validation plus conformance/handoff reporting. It has
no GPUI dependency beyond the pure-data `gpui-design` crate (see
`crates/gpui-design/AGENTS.md`: "Pure data types only — no rendering code").
Total source: 880-line `src/lib.rs` plus three thin clap bins
(`src/bin/gpui_{export,import,validate}_design_tokens.rs`, 20/25/91 lines).

There is **no runtime/UI surface**: no per-frame paint or layout, no event
handlers, no update loops. Execution model is a one-shot CLI invocation over a
small JSON document (built-in export is ~4 presets / ~128 tokens per the README
contract example at `crates/gpui-design-tools/README.md:58-59`):

- `export_design_tokens` — `DesignTokenExport::for_all_presets()` +
  `serde_json::to_string_pretty` (`src/lib.rs:215-222`).
- `import_design_tokens` / `validate_design_tokens` — one `serde_json::from_str`
  into `Value` plus a shape walk (`src/lib.rs:225-260`, `inspect_token_value`
  at `src/lib.rs:318-377`).
- `validate_current_design_tokens` — `DesignConformanceMatrix::all_presets()`
  (2 cases per preset: `crates/gpui-design/src/design_conformance_matrix.rs:13-27`)
  plus optional markdown table (`src/lib.rs:382-430`).

Declared features (`autoeq`, `gpu-2d`, `gpu-3d`, `showcase`, `tokio`, …) exist
in `Cargo.toml:11-20` but map to no code in this crate — they appear to be
surface-parity stubs, not perf-relevant paths.

## Findings

1. **[GPU|Roundtrip] No GPU or readback surface — nothing to do.**
   The crate never touches wgpu/vello/GPUI rendering; its only dependencies are
   `anyhow`, `clap`, `gpui-design`, `serde`, `serde_json`
   (`crates/gpui-design-tools/Cargo.toml:22-27`). No `map_async`,
   `device.poll`, offscreen render, or image paint anywhere. GPU campaign goals
   1–2 do not apply to this crate.

2. **[Alloc] Per-invocation allocations are already minimized where it matters.**
   `inspect_token_value` lazily builds the error-path prefix `String` only when
   a finding is emitted (`src/lib.rs:339-362`, covered by tests
   `inspect_token_value_lazy_prefix*` at `src/lib.rs:513-523,852-879`), and uses
   `Cow<'static, str>` for findings so passing runs allocate almost nothing.
   The validate CLI explicitly skips building the markdown table when only JSON
   is needed (`src/lib.rs:247-248`, `src/bin/gpui_validate_design_tokens.rs:44-45`).
   At ~128-token input scale, total work is a few hundred small allocations per
   process run — not a meaningful target.

3. **[Alloc] Full `serde_json::Value` DOM parse for import/validate.**
   `serde_json::from_str` into `Value` (`src/lib.rs:231`, `src/lib.rs:254-258`)
   materializes the entire document; `import_design_tokens` also returns that
   `Value` in `ImportedDesignTokens.raw` (`src/lib.rs:41-46,236-241`). Fine at
   current scale; if token files ever grow large, a typed/streaming
   deserialization would cut peak memory. **(needs profiling — only relevant if
   input size grows beyond the current built-in export scale.)**

4. **[Alloc] Trivial avoidable clone in the validate CLI.**
   `report.conformance_markdown.clone()` before appending a findings footer
   (`src/bin/gpui_validate_design_tokens.rs:70`) — could take ownership of
   `report` or push into a new `String` built via `push_str(&report.conformance_markdown)`.
   Impact: one copy of a few-KB string once per process run. Cosmetic.

5. **[Alloc] Markdown/JSON built via `format!` push loops.**
   `DesignToolingHandoffReport::to_markdown` (`src/lib.rs:123-144`) and
   `DesignConformanceMatrix::to_markdown_table`
   (`crates/gpui-design/src/design_conformance_matrix.rs:43-70`) build output
   row-by-row with `format!`. Standard practice, ~7 handoff items / ~8 matrix
   rows; not worth touching.

No TODO/FIXME markers in the crate, no criterion benches, no
allocation-count tests. `qa/perf/` has no gpui-design-tools references.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | **Exclude from the perf campaign** — no GPU, roundtrip, or per-frame allocation surface | 1 | S | Avoids wasted effort; correct scoping |
| 2 | Replace the `conformance_markdown.clone()` with ownership/`push_str` | 4 | S | Cosmetic only |
| 3 | Revisit DOM parse only if token files grow | 3 | M | None today (needs profiling) |

## Quick wins

- None required. The crate is already allocation-conscious (lazy error
  prefixes, `Cow` findings, skip-markdown flag). Finding 4's one-line clone
  removal is the only landable tweak, and it is below the noise floor.
