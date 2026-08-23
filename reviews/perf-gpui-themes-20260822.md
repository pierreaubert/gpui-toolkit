# Perf review: gpui-themes

Date: 2026-08-22

## Role and hot paths

`gpui-themes` (crates/gpui-themes) provides the `EditorTheme` data model
(~90 `Color` fields + serde JSON import/export), built-in presets, and two
binaries: `theme-editor` (color editing with preview/export tabs) and
`theme-showcase` (preset gallery). All drawing is plain GPUI `div()`/text —
there is no custom rasterization, wgpu, or compute anywhere in the crate.

Actual hot paths, by frequency:

- `Render for ThemeEditor` (src/editor/theme_editor.rs:960) — rebuilds the
  whole editor element tree on every `cx.notify()` (clicks, selection,
  color edits). Element IDs and labels are already cached in `OnceLock`
  maps (theme_editor.rs:55-126) — a good existing optimization.
- `ThemeEditor::update_color` (theme_editor.rs:223) — per color edit.
- `Render for ComponentShowcase` (src/showcase/component_showcase.rs:427) —
  full component-gallery rebuild; only in the tree when the Preview tab is
  active.
- Theme parse/serialize (`to_json`/`from_json`, src/theme/editor_theme.rs:961-968;
  `to_rust_code`, editor_theme.rs:971-1207) — cold, export/import paths only.

Note on the GPUI view cache (verified in the vendored fork): a child view
re-renders when it is dirty **or** when any ancestor re-renders
(`window.refreshing` cascade, crates/3rdparties/gpui/src/view.rs:160,172).
So the showcase re-renders whenever the editor re-renders while the Preview
tab is visible. There is no per-frame update loop, no timers, no animation
code in this crate (grep: no `Timer`/`interval`/`animate`).

## Findings

1. **[Alloc] Unconditional full-theme JSON re-serialization on every color
   edit** — `update_color` calls `refresh_export_cache()`
   (theme_editor.rs:238), which runs `serde_json::to_string_pretty` on the
   entire theme (theme_editor.rs:205-212, editor_theme.rs:961) even when
   the Export tab is not visible. ~2 KB String plus per-field serde work per
   edit. Wasted work in the common case; should be lazy (dirty flag, recompute
   in `render_export_tab`). Impact: small per edit, trivially fixable.

2. **[Alloc] `Arc::make_mut` deep-clones the theme on every edit** —
   theme_editor.rs:229. The `Arc<EditorTheme>` is always shared (showcase
   clone at theme_editor.rs:240-243, `cached_theme` at :204), so refcount ≥ 3
   and every `update_color` clones the whole `EditorTheme` (3 Strings +
   `Vec<Color>` + ~400 B of Copy colors). Cheap in absolute terms (~µs), but
   pure churn: COW never wins here because a clone always exists. Impact:
   negligible latency, but it's the kind of per-event allocation the campaign
   targets; consider publishing a fresh `Arc` instead of mutating in place.

3. **[Alloc] Per-render string formatting in the color editor panel** —
   theme_editor.rs:491 (`to_hex_string()` → `SharedString`, not using the
   `hex_cache` that the color list uses at :433), :500-503 and :510-515
   (`format!` for RGBA and HSL labels, plus `to_hsl()` float math).
   ~4 String allocs per render of the Colors tab even when nothing changed.
   `render_export_tab` also does `export_format.clone()` (:588, String) and
   `theme.name.clone()` (:605) per render. Impact: minor; easy to cache
   alongside the existing `hex_cache`.

4. **[Alloc] `ThemeGallery::from_built_ins` builds 8 full `EditorTheme`s to
   read one luminance value** — src/theme/theme_gallery.rs:17-24 constructs
   every preset (each with Strings + `band_colors` Vec, e.g.
   editor_theme.rs:277-415) only to call `theme.appearance()`. One-time cost
   per gallery build; a `BuiltInThemePreset::appearance()` constant or
   background-color lookup would remove ~8 theme constructions. Impact: minor.

5. **[Roundtrip] None.** No `map_async`/`read_texture`/`device.poll`/
   `pollster` anywhere in the crate (grep verified). No offscreen-render
   patterns. Nothing to fix.

6. **[GPU] No missed GPU opportunity.** All rendering is GPUI quads/text,
   already GPU-composited; the only per-pixel math is `Color::to_rgba()`
   (crates/gpui-ui-kit/src/color.rs:89, trivial float division) and HSL
   conversions in the editor panel. Note: `Cargo.toml:16-17` declares
   `gpu-2d`/`gpu-3d` features that are never referenced in the crate
   (grep: only Cargo.toml hits) — dead features worth deleting to avoid
   implying a GPU path exists.

7. **[Alloc] Showcase live-preview update does not notify the showcase
   entity** — `showcase.update(cx, |showcase, _| showcase.set_theme(...))`
   at theme_editor.rs:241-243 and src/bin/theme_showcase.rs:38-40 never calls
   `cx.notify()` for the showcase. It happens to refresh anyway via the
   ancestor-refresh cascade (finding context above), but this is fragile:
   embedded in any context where the ancestor is not re-rendered, the preview
   goes stale. Not a perf bug today; one-line hardening (`cx.notify()` inside
   the update closure) also documents the intent.

8. **[Alloc] Export codegen allocates ~100 short Strings per export** —
   `to_rust_code` (editor_theme.rs:971-1207) calls `color_to_rust` (~90
   `format!` Strings) then one giant `format!`. Cold path (button click), so
   listed for completeness only. No change recommended.

Existing perf hygiene: no TODO/FIXME in the crate, no criterion benches, no
allocation-count tests; `tests/render_tests.rs` is render smoke only.
`qa/perf/` has no gpui-themes references. The `OnceLock`-cached element
IDs/labels (theme_editor.rs:55-126) and `hex_cache` (:51, :189) show the
hot render path was already partially de-allocated.

## Recommendations

| # | Action | Finding | Effort | Expected payoff |
|---|--------|---------|--------|-----------------|
| 1 | Make export cache lazy: dirty flag in `update_color`, recompute only in `render_export_tab` | 1 | S | Removes a full serde serialize per color edit |
| 2 | Cache RGBA/HSL strings (and reuse `hex_cache`) in `render_color_editor`; drop per-render `String` clones in export tab | 3 | S | ~6 allocs/render saved on Colors tab |
| 3 | Replace `Arc::make_mut` edit with build-new-`Arc` publish (theme is small; COW always clones anyway) | 2 | S | Removes hidden per-edit deep clone; clearer ownership |
| 4 | Add `BuiltInThemePreset::appearance()` without constructing a theme | 4 | S | Gallery build avoids 8 theme constructions |
| 5 | Add `cx.notify()` inside the showcase `update` closures | 7 | S | Robustness; prevents stale preview |
| 6 | Delete unused `gpu-2d`/`gpu-3d` features from Cargo.toml | 6 | S | Hygiene; no false GPU expectations |

No GPU or roundtrip work is warranted for this crate; its cost profile is
small-scale CPU allocation churn on interaction events. If any finding needs
quantification, wrap `update_color` with `gpui-profiler` allocation counters
(needs profiling) — expected but unmeasured: a few KB per edit.

## Quick wins

- Lazy export cache (rec 1) — a dirty bool plus moving the refresh into
  `render_export_tab`; <30 min.
- `cx.notify()` in the two showcase `update` closures (rec 5) — 2 lines.
- Drop dead `gpu-2d`/`gpu-3d` features (rec 6) — 2 lines.
- Use `hex_cache` in `render_color_editor` hex display (part of rec 2) —
  reuse the cache the list view already maintains.
