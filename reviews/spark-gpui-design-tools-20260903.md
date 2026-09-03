# Code Review: gpui-design-tools — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-design-tools` (~1.4k LOC)

## 1. Purpose / role
Style-Dictionary token CLI + handoff reports over `gpui-design`. Files: `lib.rs` (982), `bin/gpui_export_design_tokens.rs` (20), `bin/gpui_import_design_tokens.rs` (25), `bin/gpui_validate_design_tokens.rs` (101).

Public API: `DesignTokenFormat::parse` (`lib.rs:31`), `export_design_tokens` (`:216`), `import_design_tokens[_from_path]` (`:226,328`), `validate_design_tokens[_from_path]` (`:250,338`), `write_text_atomically` (`:300`), `DesignTokenValidationReport/ImportedDesignTokens` (`:43-59`), `DesignToolingHandoffReport/items/blocking_entries/to_markdown` (`:104-145`).

## 2. SOTA gap analysis (vs Figma Tokens Studio, Style Dictionary, Code Connect)
1. **Single wire format** — `StyleDictionaryJson` only, `parse` rejects rest (`:31-39`); no W3C DTCG, CSS vars, Tailwind, SVG.
2. **Shape-check-only import** (`inspect_token_value`, `:344`) — no type/color-science validation, no alias resolution.
3. **No Figma REST pull/push** — Code Connect is static `.md` (`:196-203`), live-preview is `ExternalGate` (`:204-212`).
4. **No diff/migration** between token versions.
5. **No watch mode** for designer iteration.
6. **All-or-nothing conformance** (`:263-269`), no per-component severity/baseline.
7. **Thin binary UX** — no shared `--format/--output` parity.

## 3. Performance evaluation
I/O-bound, no compute hotspots. `export` always pretty-prints all presets (`:216-222`) — no compact/streaming option. Import/validate parse to `Value` then walk twice (`from_str` `:232,257` + `inspect_token_value` `:344` + `all_presets()` `:265`). Every finding is owned `format!` (`:267,353,358,370-392,437`). `to_markdown` is `push_str(&format!())` per row (`:124-144`). `write_text_atomically` fsyncs every write (`:300-323`) — correct, slow for bulk; no opt-out.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | `--compact` export skipping pretty-print | S | bulk speed |
| 2 | Streaming/`&str`-borrow validation instead of double-walk + owned findings | M | invalid-doc cost |
| 3 | Second format (W3C DTCG JSON) to prove extensibility | M | interop |
| 4 | Token-diff command (`old.json new.json` → breaking/additive) | M | migration story |
| 5 | `--durable` flag gating `sync_all()` | S | bulk-export speed |

## 5. Verdict
Correct atomic writer + shape validation; needs DTCG/CSS export, alias resolution, and diff/watch to be designer-grade.
