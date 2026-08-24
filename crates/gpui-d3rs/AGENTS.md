# gpui-d3rs (lib: `d3rs`, version: 0.7.0)

D3.js-inspired GPU-accelerated plotting library for GPUI.

## Key Features

- 2D and 3D GPU-accelerated charts
- Scales (linear, log, band, time, color)
- Axes, contours, shapes
- Unstructured triangle meshes (`mesh` module): validation, topology, marching-triangles contours, picking index, revolve — feature-independent, no GPUI/wgpu
- Spinorama speaker measurement visualization
- Delaunay triangulation
- Force-directed graphs
- Read `GPUI.md` at the project root before working on GPUI code

## Features

- `gpui` (default) - GPUI rendering integration
- `gpu-2d` (default) - 2D GPU-accelerated rendering
- `gpu-3d` - 3D surface rendering
- `spinorama` - Spinorama API integration for speaker data
- `vello` - Vello-backed 2D chart scene layer (`vello2d` module) + CPU rasterizer fallback (vello_cpu)
- `vello-gpui` - `vello` + GPUI/wgpu element rendering (`VelloChartElement`, zero-copy `WgpuCustomDraw`)

## Demo apps (example targets)

- `d3rs-showcase` - Chart gallery — `cargo run -p gpui-d3rs --example d3rs-showcase --release`
- `d3rs-spinorama` - Spinorama visualization demo — `cargo run -p gpui-d3rs --example d3rs-spinorama --features spinorama --release`

The demos are `[[example]]` targets (sources under `bin/`) so they can use the
dev-dependency shell crates (`gpui-ui-kit`, `gpui-miniapp`, `gpui-builder`).
Keeping those crates out of `[dependencies]` is what allows `gpui-ui-kit` to
depend on `gpui-d3rs` for the vello2d painter without a package cycle.

## Examples

15+ examples: scale, color, contour, delaunay, force, line, bar, scatter, mesh contours (`mesh_contour_demo`, compute-only), etc.

## Testing

```bash
cargo test -p gpui-d3rs --no-default-features --tests   # Golden + unit tests
cargo test -p gpui-d3rs --no-default-features --features vello --tests   # + vello2d scene/CPU rasterizer tests
cargo test -p gpui-d3rs --lib                            # Unit tests only
cargo check -p gpui-d3rs && cargo clippy -p gpui-d3rs
```

Test targets that use feature-gated modules declare `required-features` in
`Cargo.toml` (`glyph_text_tests` → `gpui`, `gpu2d_line_tests` → `gpu-2d`,
`gpu3d_camera_tests` / `surface3d_cache_tests` → `gpu-3d`), so the bare
`--no-default-features` command skips them and stays green. Any new test
target against a gated module must follow the same pattern.

## Example Architecture (Zero Duplication)

Each Observable example has three layers, all sharing the same compute code:

```
src/examples/pie_chart.rs    ← Pure computation (data → scales → layout → paths)
                               No GPUI, no test harness. Also serves as documentation.

tests/golden_tests.rs        ← Calls examples::pie_chart::compute(), validates vs golden JSON

bin/showcase/d3_examples/    ← Calls examples::pie_chart::compute(), renders via GPUI
```

**Available `src/examples/` modules:**

| Module | Observable Source | d3rs APIs Used |
|--------|-----------------|----------------|
| `hexbin` | `@d3/hexbin` | LogScale, Hexbin |
| `pie_chart` | `@d3/pie-chart` | Pie, Arc |
| `donut_chart` | `@d3/donut-chart` | Pie (inner radius), Arc |
| `line_chart` | `@d3/line-chart` | LinearScale, Curve (7 types) |
| `stacked_bar` | `@d3/stacked-bar-chart` | BandScale, Stack (Diverging) |
| `stacked_area` | `@d3/stacked-area-chart` | Stack, LinearScale, Curve |
| `streamgraph` | `@d3/streamgraph` | Stack (Wiggle + InsideOut) |
| `box_plot` | `@d3/box-plot` | BandScale, quantile stats |
| `chord` | `@d3/chord-diagram` | ChordLayout |
| `force_directed` | `@d3/force-directed-graph` | Simulation, ForceCenter, ForceManyBody |

## Golden Test Infrastructure

Three levels of D3.js compatibility testing in `golden/`:

### 1. Unit-level (`generate.js`)
Tests individual D3 primitives: scales, shapes, arrays, colors, interpolation.

### 2. Feature-level (`generate_examples.js`)
Tests D3 features in isolation: force simulation, hierarchy layouts, chord diagrams, etc.

### 3. Observable examples (`generate_observable_examples.js`) — NEW
Tests **complete visualization pipelines** from real Observable notebooks.
Each test captures the full chain: data -> scales -> layout -> color -> axes.

**Available examples:**

| Example | Observable URL | D3 Modules Tested | d3rs Modules Verified |
|---------|---------------|-------------------|----------------------|
| hexbin | `@d3/hexbin` | d3-hexbin, d3-scale | LogScale, Hexbin |
| streamgraph | `@d3/streamgraph` | d3-shape | Stack (Wiggle+InsideOut) |
| ortho_to_equirect | `@d3/orthographic-to-equirectangular` | d3-geo | Orthographic, Equirectangular |
| circle_packing | `@d3/zoomable-circle-packing` | d3-hierarchy | structure validation* |
| sunburst | `@d3/sunburst` | d3-hierarchy | structure validation* |
| versor | `@d3/versor-dragging` | d3-geo | Orthographic, quaternion math |
| box_plot | `@d3/box-plot` | d3-array, d3-scale | BandScale, statistics |
| force | `@d3/force-directed-graph` | d3-force | structure validation |
| sankey | `@d3/sankey` | d3-sankey | structure validation |
| chord | `@d3/chord-diagram` | d3-chord | structure validation |
| stacked_bar | `@d3/stacked-bar-chart` | d3-shape, d3-scale | BandScale, LinearScale |
| stacked_area | `@d3/stacked-area-chart` | d3-shape | Stack, area paths |
| line | `@d3/line-chart` | d3-shape | LinearScale, 7 curve types |
| pie | `@d3/pie-chart` | d3-shape | Pie, Arc |
| donut | `@d3/donut-chart` | d3-shape | Pie (inner radius), Arc |
| parallel_sets | `@d3/parallel-sets` | d3-sankey | flow conservation |

\* = d3rs does not yet implement pack/partition layouts; test validates golden file structure.

### Pipeline: Porting Observable Examples (8 Steps)

Every Observable example follows this disciplined pipeline. The key insight from
experience: **golden tests must validate numerical output BEFORE writing any
rendering code**. Visual bugs are expensive to debug; numerical bugs are cheap to
catch with golden data.

#### Step 1: Study the Observable Source

- Fetch the Observable notebook URL
- **Read the D3.js source line by line** — don't skim, don't guess
- Extract the **exact D3 API calls**, **parameter values**, and **defaults**:
  - What are the D3 defaults for this API? (e.g., `geoConicEqualArea` defaults to
    `center([0, 33.6442])` — missing this caused a 86px y-offset bug)
  - What is the pipeline order? (e.g., D3 rotates THEN centers — not center then rotate)
  - What data transforms happen? (group, rollup, sort, filter)
- Note the exact dataset (CSV/JSON), column names, and row counts

**Common traps:**
- D3 defaults that aren't documented (check the source, not the docs)
- D3's `recenter()` mechanism for projections (center offsets output, not input)
- D3's `rotation.js` uses Cartesian sphere rotation, not coordinate offsets
- D3 `forceLink` strength is degree-dependent, not constant

#### Step 2: Generate Golden Data FIRST

**File**: `golden/generate_observable_examples.js`

This is the most critical step. Generate golden data **before writing any Rust code**.
The golden file IS the specification.

Rules:
- **Use the exact same D3 API calls as the Observable notebook** — copy the JS code
- **Match all D3 defaults** — don't set parameters the Observable doesn't set
  (e.g., if Observable doesn't call `.center()`, don't add it — use D3's default)
- Data must be **deterministic** (`Math.sin(i*k)` or embed real data, never `Math.random()`)
- Capture **ALL intermediate outputs** — not just final paths:
  - Scale domains, ranges, and sample input→output pairs
  - Layout coordinates (every node/bin/slice position with x, y, width, height)
  - Generated SVG path strings
  - Axis tick values and formatted labels
  - Color assignments per data item
  - **Projection outputs for a grid of test points** (for geo examples)
- For algorithms with parameters (projections, layouts): test **multiple configurations**
  (e.g., different rotation angles, different parallels)
- Run: `cd golden && node generate_observable_examples.js <name>`

#### Step 3: Write the Compute Module

**File**: `src/examples/<name>.rs`

Rules:
- **Pure computation only** — no GPUI, no test harness, no rendering
- Use **d3rs APIs exclusively** — never hand-roll what d3rs provides
- **Match D3 defaults exactly** — if D3's API has a default value, d3rs must match it
- Return a **result struct** with all computed geometry
- Include `default_data()` or `load_csv()`/`load_json()` for the real dataset
- Register in `src/examples/mod.rs`

**If d3rs is missing an API**: implement it in the library first, with its own
golden test. Do not approximate or hand-roll the algorithm in the example.

#### Step 4: Write the Golden Test — NUMERICAL VALIDATION

**File**: `tests/golden_tests.rs` — add `test_observable_<name>()`

This is where bugs get caught cheaply. **Every numerical output must match D3.**

Rules:
- Load golden JSON file
- Call `examples::<name>::compute()` with the **same inputs** the golden JS used
- **Match ALL D3 defaults**: if the golden JS uses `d3.geoConicEqualArea()` without
  calling `.center()`, the Rust test must use `ConicEqualArea::new()` without
  calling `.center()` either
- Assert **every intermediate value** against the golden data:
  - Scale outputs (input→output samples) — tolerance 1e-6
  - Layout positions (x, y per element) — tolerance 0.5px
  - Path structure (starts with M, correct length)
  - For projections: test a grid of (lon, lat) points with multiple rotations
- Report **pass rate** (e.g., "455/455 = 100%") for large test grids
- For non-deterministic algorithms (force): verify convergence, not exact positions
- **Do NOT proceed to Step 5 until >95% of golden assertions pass**

**Debugging with golden data**: When a test fails, the golden data tells you
exactly what D3 produces. Compare field by field. Common root causes:
- Wrong pipeline order (rotate vs center)
- Missing D3 default (center, parallels, clip angle)
- Sign convention mismatch (D3 doesn't negate rotation angles)
- Clipping that D3 doesn't do (spurious theta clip in conic)

#### Step 5: Write the Showcase Renderer

**File**: `bin/showcase/showcase_modules/d3_examples/<name>.rs`

Only start this AFTER Step 4 passes. If the numbers are right, rendering bugs
are limited to GPUI path conversion and coordinate transforms.

Rules:
- Call `examples::<name>::compute()` — **never duplicate computation logic**
- Use **d3rs path types** → `d3rs_path_to_gpui_simple()` for rendering
- All paths must be **closed** (use `.close_path()`) — open paths get filled as
  triangles by GPUI (the histogram black-triangle bug)
- For axis/grid lines, use thin closed rectangles (width=1px), not open line paths
- Use **d3rs color schemes** (not hardcoded hex arrays)
- Include proper **axes**, **title**, **source URL**, **legend**
- Register in `d3_examples/mod.rs` and `main.rs`

#### Step 6: Visual Verification

After Step 5, run the showcase and compare visually against the Observable original.

If the visual doesn't match:
1. Check if the golden test passes (Step 4) — if not, fix the compute
2. If golden passes but visual is wrong → the bug is in rendering (Step 5)
3. If golden data itself is wrong → re-read the Observable source (Step 1)

**Do not "fix" the rendering by tweaking numbers.** The golden data is the
specification. If the rendering disagrees with it, fix the rendering.

#### Step 7: Review Checklist

- [ ] Golden JSON captures ALL D3.js outputs (scales, paths, colors, ticks)
- [ ] Golden JS uses exact same D3 defaults as the Observable notebook
- [ ] `src/examples/<name>.rs` uses only d3rs APIs
- [ ] Golden test asserts numerical values, not just structure
- [ ] Golden test pass rate >95%
- [ ] Showcase calls `compute()` exclusively
- [ ] All paths are closed (no fill-triangle artifacts)
- [ ] `cargo test -p gpui-d3rs --no-default-features --tests` — all pass
- [ ] `cargo clippy -p gpui-d3rs` — no new warnings
- [ ] Visual output matches the Observable example

#### Step 8: Update Documentation

- Update AGENTS.md table with new example
- Update `overview.rs` with clickable nav item
- If new d3rs API was added, update the gap analysis table

#### Key Principles

1. **Golden data is the specification** — if d3rs disagrees with D3.js, fix d3rs
2. **No rendering before numerical validation** — Step 4 must pass before Step 5
3. **Match D3 defaults exactly** — read D3 source code, not just docs
4. **Test with multiple configurations** — one rotation angle isn't enough
5. **Debug numerically, not visually** — golden tests pinpoint the bug instantly
6. **Use d3rs or implement in d3rs** — if an API is missing, add it to the library
7. **One example at a time** — complete all 8 steps before starting the next

#### Lessons Learned (Anti-Patterns to Avoid)

| Anti-Pattern | Consequence | Correct Approach |
|-------------|-------------|-----------------|
| Writing showcase before golden test | Visual bugs with no way to diagnose | Golden test first, showcase last |
| Guessing D3 defaults | Constant pixel offsets (86px conic center bug) | Read D3 source for exact defaults |
| Applying center before rotation | All rotated projections wrong | D3 pipeline: rotate → center → project |
| Negating rotation angles | 100% projection failure | D3 passes angles directly, no negation |
| Open path for axis lines | Black triangle fill artifacts in GPUI | Always close paths for filled rendering |
| Structure-only golden tests | "Tests pass but visual is wrong" | Assert numerical values per-point |
| Writing 10 examples at once | Shallow bugs in all, deep bugs in none | Complete each example end-to-end |

#### Files Modified Per Example

| Step | File | Purpose |
|------|------|---------|
| 2 | `golden/generate_observable_examples.js` | JS generator |
| 2 | `golden/observable/<name>.json` | Golden data |
| 3 | `src/examples/<name>.rs` | Compute module |
| 3 | `src/examples/mod.rs` | Module registration |
| 4 | `tests/golden_tests.rs` | Golden test |
| 5 | `bin/showcase/.../<name>.rs` | Showcase renderer |
| 5 | `bin/showcase/.../d3_examples/mod.rs` | Module registration |
| 5 | `bin/showcase/main.rs` | DemoSection + menu |
| 8 | `AGENTS.md` | Documentation |
| 8 | `bin/showcase/.../overview.rs` | Clickable nav |

### Known Discrepancies

| Area | Issue | Status |
|------|-------|--------|
| Pie padAngle | d3rs distributes padding differently than D3.js | Test uses width tolerance |
| Stack InsideOut | Ordering algorithm differs slightly | Test verifies widths not positions |
| Hierarchy pack/partition | Simplified algorithms (not full D3 front-chain/partition) | Golden structure validation |
| Force simulation | Non-deterministic initial positions | Verify convergence, not positions |
| Line chart curves | D3.js emits native SVG curves (C/S), d3rs interpolates to L commands | Path structure validated, not exact strings |
| Line chart .nice() | D3.js uses .nice() domains, d3rs compute uses raw extent | Scale samples validated separately |
| Stereographic clip edge | ~10% of points at 142° clip boundary disagree | Core projection math is correct (100% for non-edge points) |
| Projection rotation | SphereRotation matches D3 for all tested rotations | Ortho 100%, Stereo 89.5%, Conic 100% |

### Regenerating Golden Files

```bash
cd golden
npm install                              # First time only
npm run generate                         # Regenerate ALL golden files
node generate_observable_examples.js     # Observable examples only
node generate_observable_examples.js pie # Single example
```

## Notes

- API design inspired by D3.js but adapted for Rust and GPU rendering
- Used by `app-gpui` for real-time audio visualization
- Golden tests ensure 1:1 compatibility with D3.js v7.9.0
