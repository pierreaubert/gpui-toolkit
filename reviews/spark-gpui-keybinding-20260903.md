# Code Review: gpui-keybinding — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-keybinding` (~2.9k LOC)

## 1. Purpose / role
Reusable keybinding framework: presets, provider/registry aggregation, conflict detection, platform labels, palette/which-key discovery. Files: `discovery.rs` (893), `registry.rs` (200), `platform.rs` (212), `conflict.rs` (147), `provider.rs` (124), `preset.rs` (94), `presets/*` (5× ~112-131), `lib.rs` (33), `benches/discovery.rs` (162).

Public API: `KeymapPreset`, `KeybindingProvider` + `DocumentedKeybinding` + `KeybindingCategory`, `KeybindingRegistry::{register,get_bindings,get_documented,detect_conflicts}` (`registry.rs:54-97`), `CommandPaletteEntry::from_binding` (`discovery.rs:33`), `command_palette_entries/search_command_palette[_cached]` (`:214-338`), `keybinding_hints[_cached]` (`:348-406`), `NavigationAction/navigation_key/mappings`, `format_key_label/platform_modifier(_symbol)`, `detect_conflicts`.

## 2. SOTA gap analysis (vs VSCode keybindings)
1. **No `when`-clause/context evaluator** — dedup only (executable contexts explicitly out of scope).
2. **No user-remapping/persistence** (no `keybindings.json` load/save).
3. **No chord timeout/state machine** — static prefix splits (`discovery.rs:410-415`).
4. **No multi-keystroke sequences** beyond whitespace split.
5. **No locale/layout awareness** (platform labels only).
6. **No fuzzy scoring** — `score_entry` (`:568`) is substring + sort, no typo/frecency.
7. **Presets cover navigation only** (`presets/*.rs`), not full editor command sets.

## 3. Performance evaluation
Low complexity; highest `search_command_palette_cached` score 83 (`discovery.rs:274`). `from_binding` clones 4 strings + 5 lowercase copies per binding (`:33-58`). Uncached `search` does `entries.to_vec()` on empty query (`:243-244`) + full scan + `clone()` per hit (`:261-264`). Cached path duplicates logic (`:295-315`), stores `bindings.to_vec()` snapshots (bounded 16×64, `:279-280`) with clear-all eviction (`:319-321`). `build_keybinding_hints` uses `BTreeMap<&str>` + `split_whitespace` per prefix (`:411-417`). `format_single_key` falls back to `Cow::Owned(to_string())` (`platform.rs:66`).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Steer hot paths to `_cached` variants or remove uncached twins | S | scan cost |
| 2 | Intern keys (`Rc<str>`) to kill 5× lowercase copies | S | registry-build allocs |
| 3 | LRU (not clear-all) per query map | S | cache churn |
| 4 | Add `when`-context field to `DocumentedKeybinding` (first VSCode-parity step) | M | context parity |
| 5 | JSON serialize/deserialize for user overrides | M | remapping story |

## 5. Verdict
Good discovery/registry core; executable contexts + persistence + fuzzy scoring are the SOTA gates. Perf is string-clone hygiene.
