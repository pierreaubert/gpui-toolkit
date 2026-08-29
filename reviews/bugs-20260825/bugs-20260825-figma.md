# Bug Review: figma (directory crates/figma — review any code/scripts there; if it is documentation-only, say so in a short report) — 2026-08-25

Scope: `crates/figma/` contains no Rust crate and no executable code — it is
three files (~1,725 lines total): `DESIGN_SYSTEM_RULES.md` (189 lines),
`CODE_CONNECT_MAPPINGS.md` (204 lines), and `component-library.html`
(1,332 lines of static HTML/CSS plus one external Figma capture `<script>`,
no inline JS or event handlers). Because these docs are the specification an
agent follows when translating Figma nodes into Rust via the Figma MCP
(`get_design_context` / `add_code_connect_map`), documentation errors here
directly produce non-compiling generated code and broken file references, so
I reviewed them with the same rigor as source: I checked every referenced
file path against the actual tree and every Rust code example against the
real component signatures in `crates/gpui-ui-kit`, `crates/gpui-px`, and
`crates/gpui-audio-kit`.

## Findings

Ranked by severity. All findings are documentation-accuracy bugs; there is no
runtime code in this directory to harbor correctness, memory, threading, or
GPU bugs.

### High

- **H1 — `Toggle` example uses a constructor signature that does not exist.**
  `crates/figma/DESIGN_SYSTEM_RULES.md:140` and
  `crates/figma/CODE_CONNECT_MAPPINGS.md:82` show
  `Toggle::new("enable-toggle", is_checked)`, but the real constructor is
  `Toggle::new(id: impl Into<ElementId>)` — one argument
  (`crates/gpui-ui-kit/src/toggle.rs:45`). Any Code Connect snippet generated
  from this spec fails to compile. Fix: change the examples to
  `Toggle::new("enable-toggle").checked(is_checked)` (verify the checked-state
  setter name against `toggle.rs` before publishing).

- **H2 — `Toggle` example chains a nonexistent `.on_toggle(...)` handler.**
  `crates/figma/DESIGN_SYSTEM_RULES.md:144` shows
  `.on_toggle(|checked, cx| { /* handler */ })`; the actual builder method is
  `.on_change(...)` (`crates/gpui-ui-kit/src/toggle.rs:104`), and the handler
  type is `Fn(bool, &mut Window, &mut App)` — three parameters, not two. Fix:
  rewrite the example as `.on_change(|checked, _window, cx| { ... })`.

- **H3 — `Select` example passes options to the constructor.**
  `crates/figma/CODE_CONNECT_MAPPINGS.md:115` shows
  `Select::new("id", vec![SelectOption::new(...)])`, but
  `Select::new(id: impl Into<ElementId>)` takes only the id
  (`crates/gpui-ui-kit/src/select.rs:68`); options are attached via the
  `.options(Vec<SelectOption>)` builder method
  (`crates/gpui-ui-kit/src/select.rs:111`). Fix: change the example to
  `Select::new("id").options(vec![SelectOption::new("opt1", "Option One"), ...])`.

### Medium

- **M1 — Every source path carries a stale `crates/gpui-toolkit/` prefix.**
  Both docs reference files as `crates/gpui-toolkit/gpui-ui-kit/src/button.rs`,
  `crates/gpui-toolkit/gpui-design/src/lib.rs`, etc.
  (`crates/figma/CODE_CONNECT_MAPPINGS.md:12-61`, ~40 rows;
  `crates/figma/DESIGN_SYSTEM_RULES.md:7,12,44,149,168,177`). The actual
  workspace layout is `crates/gpui-ui-kit/src/button.rs` — `crates/gpui-toolkit`
  exists but is a small facade crate containing only `Cargo.toml` and `src/`,
  and the parent directory `all_of_sotf/` has no `crates/` directory at all,
  so the prefix is wrong from every plausible root. I spot-verified that the
  de-prefixed paths do exist (`crates/gpui-px/src/scatter.rs`,
  `crates/gpui-audio-kit/src/audio/potentiometer.rs`,
  `crates/gpui-ui-kit/src/button_set.rs`, and others all resolve). Impact:
  the `Source File` column of `CODE_CONNECT_MAPPINGS.md` is fed verbatim to
  Figma's `add_code_connect_map`, so every mapped component points at a
  nonexistent file. Fix: strip the `gpui-toolkit/` path segment throughout,
  e.g. `crates/gpui-ui-kit/src/button.rs`.

- **M2 — Section 8 "Project Structure" describes a tree that does not exist.**
  `crates/figma/DESIGN_SYSTEM_RULES.md:176-188` shows `crates/gpui-toolkit/`
  containing `gpui-design/`, `gpui-ui-kit/src/`, etc., plus a sibling
  `crates/app-gpui/`. Neither matches: the crates live directly under
  `crates/` in this workspace, and no `app-gpui` directory exists anywhere
  reachable from this repository (the monorepo root `all_of_sotf/` contains
  `gpui-toolkit/` at top level, not under `crates/`). Fix: redraw the tree
  from the actual `crates/` listing and either drop `app-gpui` or give its
  real location in the enclosing monorepo.

### Low

- **L1 — References a `gpui-icons` crate that does not exist.**
  `crates/figma/DESIGN_SYSTEM_RULES.md:168` (Icon System section) and
  `:183` (project tree) cite `crates/gpui-toolkit/gpui-icons/`. There is no
  icons crate in the workspace (`ls crates/` shows none; the only icon-related
  code is `crates/gpui-ui-kit/src/icon_button.rs`). An agent following this
  rule will hunt for a nonexistent crate. Fix: either point at the real icon
  mechanism (how `IconButton` sources its glyphs) or remove the section until
  the crate exists.

- **L2 — `component-library.html` hardcodes a full hex palette while the
  rules forbid hardcoded hex.** The HTML defines ~30 literal colors as CSS
  variables (`crates/figma/component-library.html:10-40`) while rule 5 of the
  same directory's spec says "Map Figma fills to `DesignSystem` color tokens,
  never hardcode hex" (`crates/figma/DESIGN_SYSTEM_RULES.md:161`). For a
  Figma capture page hex is unavoidable, but the values are a hand-maintained
  copy of the token palette and will silently drift from
  `gpui_design::DesignSystem`. Fix (optional): generate the CSS variable block
  from `gpui-export-design-tokens` output so the reference page tracks the
  token source of truth.

## Clean bill

- No Rust/Python/JS logic exists in `crates/figma/` — correctness,
  allocation, threading, and GPU categories are not applicable and nothing
  was invented to fill them.
- The remaining code examples I verified are accurate:
  `Button::new(id, label)` (`button.rs:56`) and its `on_click` handler shape
  (`button.rs:136`), `Input::new(id).placeholder(...)`
  (`input.rs:190,232`), `Badge::new(label)` (`badge.rs:75`),
  `Alert::new(id, message)` (`alert.rs:63`), `Tabs::new(id)`
  (`tabs.rs:49`), `VolumeKnob::new().id(...)` (`volume_knob.rs:71,113`),
  `render_horizontal_meter_bar(label, value, &TickConfig, theme)`
  (`meter.rs:148`), `render_spectrum_frequency_axis(f32, f32, theme)`
  (`spectrum/render.rs:8`), and the `gpui-px` free functions
  `line(&x, &y).title(...).fill().min_size(...).aspect_ratio(...).build()`
  (`line/chart_theme.rs:58`, `line/line_chart.rs:477,595,601,609,1106`).
- The gpui-px file references in the mappings table
  (`scatter.rs`, `line.rs`, `isoline.rs`, …) still exist as shim files over
  the new per-chart module directories, so those rows are only wrong in their
  `crates/gpui-toolkit/` prefix (M1), not in the filename.
- `component-library.html` loads exactly one external script (Figma's
  official `capture.js`, async) and contains no inline JS, so there are no
  script-side bugs to report.

## Resolution status

- [x] H1–H2. **Toggle examples** (2026-08-26): all examples now use the verified `Toggle::new(id).checked(bool).on_change(bool, Window, App)` builder API.
- [x] H3. **Select example** (2026-08-26): the snippet now uses the verified `Select::new(id).options(Vec<SelectOption>)` API.
- [x] M1. **Code Connect paths** (2026-08-26): removed the nonexistent `crates/gpui-toolkit/` prefix from all mapping and rule-document paths.
- [x] M2/L1. **Project and icon guidance** (2026-08-26): corrected the workspace tree, removed the nonexistent `app-gpui` and `gpui-icons` entries, and documented the supported `IconButton` ownership model.
- Verified by a zero-result scan for obsolete constructor calls, `on_toggle`, and the old path prefix in `crates/figma/`.

## Follow-up regression evidence

- `reviewed_figma_examples_keep_current_component_apis_and_paths` now pins the reviewed Toggle and Select builder forms plus the corrected crate-path/icon guidance. It runs in `cargo test -p gpui-design-tools`.
