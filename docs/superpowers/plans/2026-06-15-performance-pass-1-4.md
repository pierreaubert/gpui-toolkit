# Performance Pass Items 1-4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the remaining per-frame / per-render allocations and recomputation identified in `performance2.md` items 1-4.

**Architecture:** Convert interactive UI components to persistent `Render` entities with stable handlers, move expensive simulation/data generation out of `Render` into model/app state with incremental cache keys, and replace remaining temporary `Vec`/`HashMap`/`String` allocations in layout and bidi paths with reusable scratch buffers or borrowed views.

**Tech Stack:** Rust, GPUI, `gpui-ui-kit`, `gpui-d3rs`, `gpui-px`, `gpui-builder`, `gpui-pretext`.

---

## Task 1: Persistent entity wrappers and stable handlers for `Tabs`, `Input`, `NumberInput`, `Slider`

**Files:**
- Modify: `crates/gpui-ui-kit/src/tabs.rs`
- Modify: `crates/gpui-ui-kit/src/input.rs`
- Modify: `crates/gpui-ui-kit/src/number_input.rs`
- Modify: `crates/gpui-ui-kit/src/slider.rs`
- Test: `cargo check -p gpui-ui-kit`, `cargo check --bin gpui-component-lab`, `cargo check --bin d3rs-showcase`

`Input`, `NumberInput`, and `Slider` already have thread-local `WeakEntity` caches (`INPUT_ENTITIES`, `NUMBER_INPUT_ENTITIES`, `SLIDER_ENTITIES`). They still allocate fresh closures and clone values every render. `Tabs` does not have an entity cache at all and builds a fresh `TabsEntity` each render.

### Step 1.1: Convert `Tabs` to a cached `Render` entity

- [ ] Read `crates/gpui-ui-kit/src/tabs.rs`. Locate `Tabs::render` and `TabsEntity`.
- [ ] Add a thread-local `TABS_ENTITIES: RefCell<HashMap<String, WeakEntity<TabsEntity>>>` (or use the existing id type) keyed by the tabs element id, mirroring the pattern in `input.rs` around lines 946-977.
- [ ] In `Tabs::render`, look up `TABS_ENTITIES` by id. If the entity is alive, update its state (`self.clone()` or needed fields) and return the entity as a child. If missing or dead, create a new `TabsEntity`, store its `WeakEntity`, and return it.
- [ ] Move all per-tab `on_mouse_down` and hover handlers from the ephemeral `render` closure into `TabsEntity` using `cx.listener` on the entity. Store a `WeakEntity<Tabs>` parent reference in `TabsEntity` so listeners can call back without capturing new closures.
- [ ] Remove any `format!("nav-{:?}", section)` id formatting from the hot render path; precompute `ElementId`s when the entity is constructed or when the tab list changes.

### Step 1.2: Remove per-render closures and clones in `Input`

- [ ] Read `crates/gpui-ui-kit/src/input.rs`. Locate the cached entity (`InputEntity`) and its `Render` impl.
- [ ] Replace the fresh `hover` closure (`input.rs:442`) with a `cx.listener` that reads state from the entity model and applies the hover style via GPUI's `.when` or `.hover` with a stable listener.
- [ ] Replace fresh `on_mouse_down` / `on_mouse_move` / `on_mouse_up` / `on_key_down` closures (`input.rs:471, 532, 550, 569`) with `cx.listener` handlers stored once in the entity.
- [ ] Avoid `let current_value = props.value.clone();` (`input.rs:332`) each render: keep `current_value` as a `SharedString` in the entity model and update it only when the prop changes.
- [ ] Avoid `current_value.to_string()` when not editing (`input.rs:376`): render directly from the `SharedString` or cache the display `String` only on edit-state transitions.
- [ ] Run `cargo check -p gpui-ui-kit`. Fix any borrow/lifetime issues by passing `&Theme` and `&SharedString` where needed.

### Step 1.3: Remove per-render closures and clones in `NumberInput`

- [ ] Read `crates/gpui-ui-kit/src/number_input.rs`. Locate the cached entity.
- [ ] Replace fresh `hover_button` / `active_button` closures (`number_input.rs:477-478`) and mouse/key handlers (`number_input.rs:500, 576, 611, 818`) with `cx.listener` handlers attached once.
- [ ] Avoid `let label = props.label.clone();` each render by storing `label` in the entity model.
- [ ] Avoid splitting text into owned `before`/`after` `String`s each render (`number_input.rs:487-491`): render the cursor as an overlay using the cached text and cursor index.
- [ ] Run `cargo check -p gpui-ui-kit`.

### Step 1.4: Remove per-render closures and clones in `Slider`

- [ ] Read `crates/gpui-ui-kit/src/slider.rs`. Locate the cached entity.
- [ ] Replace fresh `hover_thumb` closure (`slider.rs:310`) and all mouse/scroll/keyboard/click handlers (`slider.rs:423, 432, 446, 466, 476, 485, 499, 534`) with `cx.listener` handlers attached once.
- [ ] Cache the formatted `value_label` as a `SharedString` in the entity model; update only when `value` changes.
- [ ] Run `cargo check -p gpui-ui-kit`.

### Step 1.5: Verify integration

- [ ] Run `cargo check --bin gpui-component-lab`.
- [ ] Run `cargo check --bin d3rs-showcase`.
- [ ] If either fails, fix the public API usage in those bins (should be none if internal changes are contained).

---

## Task 2: Move force simulation and `surface_plots` data generation out of `Render`

**Files:**
- Modify: `crates/gpui-d3rs/bin/showcase/main/showcase_app.rs`
- Modify: `crates/gpui-d3rs/bin/showcase/showcase_modules/force.rs`
- Modify: `crates/gpui-d3rs/bin/showcase/showcase_modules/surface_plots.rs`
- Test: `cargo check --bin d3rs-showcase`

### Step 2.1: Move force simulation ticks out of render

- [ ] Read `crates/gpui-d3rs/bin/showcase/showcase_modules/force.rs`.
- [ ] Change the module's `render` function so it no longer calls `app.force_simulation.tick(5)`. Instead, it reads node positions from `app.force_node_positions` (already a `Rc<RefCell<Vec<(f32, f32)>>>`).
- [ ] In `showcase_app.rs`, add a method `tick_force_simulation(&mut self, cx: &mut Context<ShowcaseApp>)` that runs `self.force_simulation.tick(5)` and copies the new positions into `self.force_node_positions`.
- [ ] Spawn a repeating timer/background task (e.g., `cx.spawn` with a 16 ms timer) that calls `tick_force_simulation` and `cx.notify()` while the force demo is visible and `force_running` is true. Stop the timer when the section changes or `force_running` is false.
- [ ] Ensure the render path only draws the cached positions; no per-render simulation work.
- [ ] Run `cargo check --bin d3rs-showcase`.

### Step 2.2: Move `surface_plots` data generation out of render

- [ ] Read `crates/gpui-d3rs/bin/showcase/showcase_modules/surface_plots.rs`.
- [ ] Identify the `SurfaceData` objects generated from z-function closures (lines 11-72).
- [ ] In `showcase_app.rs`, add a `surface_plot_cache: Option<SurfacePlotCache>` field. The cache key should be `(grid_size, peak1_x, peak1_y, peak2_x, peak2_y, ...)` or a small struct.
- [ ] Add a method `ensure_surface_plot_cache(&mut self)` that computes the three `SurfaceData` objects when the key changes and stores them in `surface_plot_cache`.
- [ ] Call `ensure_surface_plot_cache` on `DemoSection::SurfacePlots` transitions or when the relevant parameters change (e.g., in `render` before rendering, guarded by the key).
- [ ] Change `surface_plots.rs::render` to take the precomputed `SurfaceData` from the cache instead of generating them.
- [ ] Run `cargo check --bin d3rs-showcase`.

---

## Task 3: Cache 3D surface per-paint projection, isolines, and tick labels

**Files:**
- Modify: `crates/gpui-d3rs/src/surface/render/surface_element.rs`
- Modify: `crates/gpui-d3rs/src/gpu3d/element/surface3_delement.rs`
- Test: `cargo check -p gpui-d3rs`, `cargo check --bin d3rs-showcase`

### Step 3.1: Cache axis projection and tick labels in `surface_element.rs`

- [ ] Read `crates/gpui-d3rs/src/surface/render/surface_element.rs`.
- [ ] Add a cache struct (e.g., `SurfacePaintCache`) holding projected axis endpoints and pre-formatted tick label strings, keyed by `(camera, bounds_size, data_generation)`.
- [ ] In `prepaint`, compute or invalidate the cache when the key changes. Move depth-sort/triangle color/lighting work that is already cached into the same generation check.
- [ ] In `paint`, read projected axis points and label strings from the cache instead of recomputing projections and calling `format!`.
- [ ] Run `cargo check -p gpui-d3rs`.

### Step 3.2: Cache isolines, depth buffer, and axis/colorbar labels in `surface3_delement.rs`

- [ ] Read `crates/gpui-d3rs/src/gpu3d/element/surface3_delement.rs`.
- [ ] Extend `SurfaceTextureCache` (or add a sibling `SurfaceGeometryCache`) to store:
  - projected contour/isoline segments,
  - the depth buffer,
  - pre-formatted axis tick labels,
  - pre-formatted colorbar tick labels.
  Key by `(camera, data, config, size)`.
- [ ] Move `paint_projected_isolines` recomputation into a cache-invalidation check.
- [ ] Replace per-paint `format!` calls for axis/colorbar labels with cached strings.
- [ ] Run `cargo check -p gpui-d3rs`.

### Step 3.3: Integration check

- [ ] Run `cargo check --bin d3rs-showcase`.

---

## Task 4: Remove per-layout `String`/`HashMap`/`Vec` allocations in `gpui-builder` and `gpui-pretext` bidi

**Files:**
- Modify: `crates/gpui-builder/src/solver/misc.rs`
- Modify: `crates/gpui-builder/src/solver/solve.rs`
- Modify: `crates/gpui-builder/src/solved/solved_tree.rs`
- Modify: `crates/gpui-pretext/src/bidi/mod.rs`
- Modify: `crates/gpui-pretext/src/line_break/types.rs`
- Modify: `crates/gpui-px/src/line/misc.rs`
- Test: `cargo test -p gpui-builder`, `cargo test -p gpui-pretext`, `cargo test -p gpui-px`

### Step 4.1: `gpui-builder` cache-key and temporary container fixes

- [ ] Read `crates/gpui-builder/src/solver/misc.rs`. Locate `compute_text_size`.
- [ ] Change the cache key from `input.text.to_string()` to a borrowed key or hashed view. If the cache map requires an owned key, use a stack-allocated short-string path or change the map key to `(usize, ...)` where the usize is a hash of the text plus style parameters. Ensure cache lookup remains correct and fast.
- [ ] Read `crates/gpui-builder/src/solver/solve.rs`. Locate `solve_tree_with_cache`.
- [ ] Replace the fresh `nodes: Vec<...>` and `index: HashMap<...>` allocations with reusable scratch buffers stored in `TextMeasureCache` or a thread-local pool. Clear and reuse them each solve instead of dropping and reallocating.
- [ ] Read `crates/gpui-builder/src/solved/solved_tree.rs`. Locate `as_map`.
- [ ] Store the computed `HashMap` inside `SolvedTree` (e.g., `map: OnceLock<HashMap<String, usize>>`) and return a reference or clone-on-write. Alternatively, change all callers of `as_map` to use the existing flat index (`find`/index) and remove the hot-path `HashMap` rebuild.
- [ ] Run `cargo test -p gpui-builder`.

### Step 4.2: `gpui-pretext` bidi and line-break scratch

- [ ] Read `crates/gpui-pretext/src/bidi/mod.rs`. Locate `compute_bidi_levels` / `compute_segment_levels`.
- [ ] Replace `Vec<char>` materialization by iterating `char_indices()` on the input string. Use thread-local scratch buffers (`BIDI_CHARS_SCRATCH` etc.) only for the level/type arrays, not for character storage.
- [ ] Replace `Vec<usize>` char-starts allocation with `char_indices()` iteration or a thread-local scratch.
- [ ] Keep the normalized-text level cache intact.
- [ ] Read `crates/gpui-pretext/src/line_break/types.rs`. Locate the local `to_deactivate: Vec<usize>`.
- [ ] Replace it with a thread-local scratch buffer (`KP_DEACTIVATE_SCRATCH`) and clear/reuse it per chunk.
- [ ] Run `cargo test -p gpui-pretext`.

### Step 4.3: `gpui-px` log-tick formatting

- [ ] Read `crates/gpui-px/src/line/misc.rs`. Locate `format_log_tick` and `generate_log_ticks`.
- [ ] Change `format_log_tick` to write into a caller-provided `&mut String` (add `format_log_tick_into`) and return `&str` from the thread-local buffer. Update all callers to pass a reusable buffer.
- [ ] Change `generate_log_ticks` to return `Cow<[f64]>` (or `&[f64]` where lifetimes allow) so it does not clone the cached `Vec<f64>` on every call.
- [ ] Run `cargo test -p gpui-px`.

---

## Verification (run after all tasks)

- [ ] `cargo check -p gpui-ui-kit`
- [ ] `cargo check -p gpui-d3rs`
- [ ] `cargo check -p gpui-builder`
- [ ] `cargo check -p gpui-pretext`
- [ ] `cargo check -p gpui-px`
- [ ] `cargo check --bin gpui-component-lab`
- [ ] `cargo check --bin d3rs-showcase`
- [ ] `cargo test -p gpui-builder`
- [ ] `cargo test -p gpui-pretext`
- [ ] `cargo test -p gpui-px`

If any check fails, fix the offending task before moving to the next.
