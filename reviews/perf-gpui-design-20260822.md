# Perf review: gpui-design

Date: 2026-08-22

## Role and hot paths

`gpui-design` (~2,600 LOC) is a **pure-data crate**: platform-adaptive design
tokens (corner radii, spacing, interaction, elevation, animation durations,
typography, layout thresholds, audio-control geometry) for seven presets
(Neutral, Apple HIG, Material 3, Fluent, Adwaita, Breeze, Carbon). Per its own
AGENTS.md: "Pure data types only — no rendering code, no framework
dependencies."

There is **no paint/layout/event-loop code in the crate itself**. The closest
things to hot paths are:

1. `DesignExt::design()` (`src/design_system_state.rs:37-49`) — per-render
   global lookup returning `Arc<DesignSystem>`. Called from every UI-kit
   component paint via `resolve_design` (8 call sites, e.g.
   `crates/gpui-ui-kit/src/button.rs:339`, `slider.rs:328`, `table.rs:716`)
   and from showcase render paths (9 sites in
   `crates/gpui-d3rs/bin/spinorama_demo/app/spinorama_app.rs`).
2. `DesignSystem::motion_spec()` (`src/design_system.rs:944-962`) — per-call
   `Copy` struct, no allocation. Animation **interpolation itself does not
   live here**; the crate only stores duration/spring constants.
3. `style_dictionary_tokens()` / `conformance_report()` / documentation and
   release-presentation report builders — export/CI-time only, invoked once
   per preset.

Everything below is therefore allocation/API-shape work; the crate has **no
GPU or roundtrip surface at all** (no wgpu usage, no readbacks, nothing to
offload — corner radii and shadows are consumed by GPUI's own GPU renderer,
not rasterized here).

## Findings

1. **[Alloc] `Clone for DesignSystem` silently drops the token cache** —
   the manual `Clone` impl re-initializes `cached_tokens: OnceLock::new()`
   (`src/design_system.rs:77`), so any cloned `DesignSystem` that later calls
   `style_dictionary_tokens_ref()` rebuilds all ~37 tokens from scratch
   (`build_style_dictionary_tokens`, `src/design_system.rs:634-757`). Each
   token costs a `Vec<&str>` + two `String`s (`src/design_token.rs:37-43`),
   so a rebuild is ~110 small allocations. Impact: low — tokens are
   export/CI-time and presets are normally shared via `Arc` — but the cache
   invalidation is invisible at the call site. Fix by cloning the `Arc` out
   of the `OnceLock` when present.

2. **[Alloc] `token()` splits then re-joins the same string** —
   `src/design_token.rs:37-38`: `path.split('.').collect::<Vec<_>>()` followed
   by `path_vec.join(".")` reproduces `path` byte-for-byte, so `name` could be
   `path.to_string()` directly (and could even be `&'static str`, since both
   `path` segments and the joined form are `'static`). One wasted `Vec` and
   one extra pass per token; ~37 tokens × 7 presets at export time. Trivial.

3. **[Alloc] Per-render `Arc<DesignSystem>` clones from `cx.design()`** —
   `DesignExt::design()` (`src/design_system_state.rs:37-49`) returns an owned
   `Arc` (atomic refcount bump + `try_global` lookup) on every call, and UI
   components call it once per paint (see hot path #1). The fallback path is
   already a `static OnceLock` (good). Cost per call is nanoseconds, so this
   is an API-shape note, not a bottleneck: a borrowing accessor
   (`fn design(&self) -> &DesignSystem`, possible since `App::global` returns
   `&G`) would eliminate the refcount churn but is a breaking change across
   gpui-ui-kit and showcase crates. (Needs profiling to justify the churn;
   likely not worth it.)

4. **[Alloc] `DesignSystemState::new()` fallback builds `platform_default()`
   per call in `ui-kit`** — `crates/gpui-ui-kit/src/design.rs:14-21`
   (`neutral_design()` / `platform_design()`) allocate a full `DesignSystem`
   (~200-byte struct + `Arc`) on every call. Grep shows current callers are
   tests only (`crates/gpui-ui-kit/tests/coverage.rs`), so this is latent, not
   live — but if these helpers ever migrate into render paths they should
   share the same `static OnceLock<Arc<DesignSystem>>` pattern already used in
   `design_system_state.rs:42-47`.

5. **[Alloc] Report builders use per-row `format!` + `push_str`** —
   `design_documentation_report.rs:81`, `design_conformance_matrix.rs:60`,
   `design_release_presentation.rs:145,159` build markdown with one `format!`
   allocation per table row. These run once per release/CI invocation over
   ≤14 rows — negligible; listed only for completeness.

6. **No GPU / Roundtrip findings.** Searched the crate: no wgpu, no
   `map_async`/`read_texture`/`device.poll`/`pollster`, no offscreen patterns.
   `CornerRadiusStyle::Continuous` (squircle, `src/types.rs:6-11`) is a hint
   consumed by renderers; any squircle tessellation cost belongs to the
   consuming crate (gpui/ui-kit), not here.

## Existing perf infrastructure

- No TODO/FIXME/PERF markers anywhere in the crate (grep clean).
- No criterion benches, no allocation-count tests. `src/tests.rs:258` asserts
  the token cache is reused (`as_ptr()` equality) — the only
  allocation-relevant test.
- `qa/perf/baseline.json` has no gpui-design entry; no docs reference its perf.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Preserve `cached_tokens` in `Clone` (clone the `Arc` if initialized) | 1 | S | Removes a hidden rebuild trap; keeps cache semantics honest |
| 2 | Simplify `token()`: `name: path.to_string()`, drop split/join; consider `name: &'static str` | 2 | S | ~37 fewer allocs per preset export; cleaner code |
| 3 | Cache `neutral_design()`/`platform_design()` in a `OnceLock<Arc<_>>` before they get used in render paths | 4 | S | Prevents a future per-frame full-struct alloc |
| 4 | (Optional) Add a borrowing `design_ref()` accessor alongside `design()`; migrate hot components later | 3 | M | Removes per-paint refcount churn (needs profiling to confirm it matters) |
| 5 | Nothing on GPU/roundtrips — out of scope for this crate | 6 | — | — |

## Quick wins

- Fix `Clone for DesignSystem` to carry the token cache (finding 1) — a few
  lines in `src/design_system.rs:62-80`, covered by existing cache tests.
- Collapse the split/join in `token()` (finding 2) — one-line change in
  `src/design_token.rs:37-38`.
- Wrap `ui-kit`'s `neutral_design()`/`platform_design()` bodies in a
  `static OnceLock` (finding 4) — same pattern already proven in
  `design_system_state.rs:42-47`.

Overall: this crate's runtime perf footprint is already close to minimal
(`Copy` rule structs, `Cow<'static, str>` font names, `OnceLock` caches, an
`Arc`-shared global). The realistic upside is a handful of allocation cleanups
plus guarding the `Clone`-drops-cache trap, not architectural work.
