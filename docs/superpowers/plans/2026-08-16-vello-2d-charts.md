# Vello 2D Charts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render d3rs/px 2D charts through vello (GPU, zero-copy via GPUI's `WgpuCustomDraw` seam) with a `vello_cpu` fallback for non-wgpu backends, and port the scatter chart end-to-end as the proving case.

**Architecture:** Charts emit draw commands into a backend-neutral `ChartScene` IR (kurbo paths + peniko brushes), built at paint time in the element's actual bounds so resizing stays correct. Two replayers consume the IR: `to_vello_scene()` → `vello::Scene` rendered by a shared `vello::Renderer` into an offscreen `Rgba8Unorm` texture, composited into the GPUI frame by a small premultiplied-alpha pipeline inside `draw_wgpu`; and `CpuRasterizer` → `vello_cpu` pixmap painted via `window.paint_image`. Backend is chosen per element (`Auto` probes a new capability flag set by `WgpuRenderer`).

**Spec:** `docs/superpowers/specs/2026-08-16-vello-2d-charts-design.md`

**Scope of THIS plan:** infrastructure (deps, IR, GPU + CPU backends, element) + scatter port + QA/perf. Porting line/area/bar/boxplot/treemap/pie is a follow-up plan once the pattern is proven.

**Tech Stack:** vello 0.10.0, vello_cpu 0.2.0, peniko 0.6.1, kurbo 0.13.1, wgpu 29.0.3 (Zed fork rev `357a0c56e0070480ad9daea5d2eaa83150b79e88`), vendored GPUI fork (`crates/3rdparties/gpui*`).

## Global Constraints

- vello 0.10.0 requires crates-io `wgpu = "29.0.3"`; the workspace pins Zed's wgpu fork which reports exactly 29.0.3 and is upstream v29.0.3 + one internal EGL patch (verified). Unify via `[patch.crates-io]` — never add a second wgpu source.
- Toolchain: stable rustc 1.97.1 (≥ vello MSRV 1.88); workspace edition 2024.
- Feature layout in `gpui-d3rs`: `vello` = IR + both replayers (NO gpui/wgpu needed); `vello-gpui` = `vello` + `gpui` + `gpu-2d` (element + custom draw). `gpui-px` gets a `vello` forwarding feature.
- Repo test rule (crates/gpui-d3rs/AGENTS.md): any new `[[test]]` target against feature-gated modules declares `required-features`; `cargo test -p gpui-d3rs --no-default-features --tests` must stay green.
- Integration tests in `crates/gpui-d3rs/tests/` cannot see d3rs's optional deps; they import kurbo/peniko via `d3rs::vello2d::{kurbo, peniko}` re-exports.
- Never panic in `paint()`; render errors → `log::error!` + skip frame; renderer creation failure → poison flag + permanent skip for that element.
- The `WgpuCustomDraw` dispatch (`gpui_wgpu/src/wgpu_renderer.rs:1214-1246`) passes scene bounds already in physical pixels with `scale_factor = 1.0`. Keep using the passed values; don't re-scale.
- vello's fine-pass target must be `wgpu::TextureFormat::Rgba8Unorm` with `STORAGE_BINDING`; GPUI's frame texture lacks it, hence offscreen + composite. vello output is premultiplied RGBA → composite with `BlendState::PREMULTIPLIED_ALPHA_BLENDING`.
- Vendored-crate changes (gpui, gpui_wgpu) must be recorded in that crate's `PATCHES.md`.
- Before touching GPUI-facing code, read the project-root `../../GPUI.md` conventions (repo rule from AGENTS.md files).
- Git: commit per task; `docs/` is gitignored — specs/plans under it are force-added (`git add -f`), everything else uses plain `git add`.

## File Structure

New module `d3rs::vello2d` (crate `gpui-d3rs`, lib name `d3rs`):

- `src/vello2d/mod.rs` — module wiring + re-exports (`ChartScene`, `ChartCmd`, `to_vello_scene`, `CpuRasterizer`, `kurbo`, `peniko`; element types under `vello-gpui`).
- `src/vello2d/scene.rs` — `ChartScene` IR + builder helpers. No GPU/GPUI types. Feature: `vello`.
- `src/vello2d/gpu_scene.rs` — `to_vello_scene(&ChartScene) -> vello::Scene`. Feature: `vello`.
- `src/vello2d/cpu.rs` — `CpuRasterizer` (vello_cpu replay → premultiplied RGBA bytes). Feature: `vello`.
- `src/vello2d/element.rs` — `VelloChartElement`, `RasterBackend`, lazy backend resolution, CPU paint path. Feature: `vello-gpui`.
- `src/vello2d/wgpu_draw.rs` — `WgpuVelloDraw` + composite pipeline. Feature: `vello-gpui`.

Vendored patches:

- `crates/3rdparties/gpui/src/custom_draw.rs` — `wgpu_custom_draw_available()` probe flag.
- `crates/3rdparties/gpui_wgpu/src/custom.rs` — add `target_format` param to `draw_wgpu`.
- `crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs` — set the probe flag on renderer init; pass `surface_config.format` at the dispatch site.
- `crates/gpui-d3rs/src/mesh/gpu/wgpu_backend.rs` — update `WgpuMeshDraw::draw_wgpu` signature.

Chart port:

- `crates/gpui-d3rs/src/shape/scatter.rs` — `scatter_chart_scene()` (feature `vello`) + `render_scatter_vello()` (feature `vello-gpui`).
- `crates/gpui-px/src/scatter/scatter_chart.rs` — `raster_backend` toggle.

QA:

- `crates/gpui-d3rs/tests/vello2d_scene_tests.rs`, `vello2d_cpu_tests.rs`, `vello2d_element_tests.rs`
- `crates/gpui-d3rs/benches/vello2d_bench.rs`
- `qa/visual/wasm/baselines/` — new px vello scatter baseline (via `just wasm-visual ... record`)

---

### Task 1: Dependency unification — wgpu patch + vello workspace deps + features

**Files:**
- Modify: `Cargo.toml` (`[workspace.dependencies]` near line 126, `[patch.crates-io]` near line 323)
- Modify: `crates/gpui-d3rs/Cargo.toml` (features at :16-29, deps at :31-56, test targets at :216-239)
- Modify: `crates/gpui-px/Cargo.toml` (features + d3rs dep at :16-17)

**Interfaces:**
- Consumes: existing `wgpu = { version = "29.0.0", git = ".../zed-industries/wgpu.git", rev = "357a0c56..." }` workspace dep.
- Produces: features `gpui-d3rs/vello`, `gpui-d3rs/vello-gpui`, `gpui-px/vello`; workspace deps `vello`, `vello_cpu`, `peniko`, `kurbo`.

- [ ] **Step 1: Add the crates-io patch so vello shares Zed's wgpu fork**

In root `Cargo.toml`, inside the existing `[patch.crates-io]` section (after the `async-task` line, ~line 329):

```toml
# vello 0.10 requires crates-io wgpu/naga 29.0.3; unify with Zed's fork
# (fork = upstream v29.0.3 + one internal wgpu-hal EGL patch, version-identical).
wgpu = { git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c56e0070480ad9daea5d2eaa83150b79e88" }
wgpu-core = { git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c56e0070480ad9daea5d2eaa83150b79e88" }
wgpu-hal = { git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c56e0070480ad9daea5d2eaa83150b79e88" }
wgpu-types = { git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c56e0070480ad9daea5d2eaa83150b79e88" }
naga = { git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c56e0070480ad9daea5d2eaa83150b79e88" }
```

- [ ] **Step 2: Add vello workspace deps**

In root `Cargo.toml` `[workspace.dependencies]`, right after the existing `wgpu = { ... }` line (~:126):

```toml
vello = { version = "0.10.0", default-features = false, features = ["wgpu"] }
vello_cpu = { version = "0.2.0" }
peniko = { version = "0.6.1" }
kurbo = { version = "0.13.1" }
```

- [ ] **Step 3: Add d3rs features, deps, and test-target registrations**

In `crates/gpui-d3rs/Cargo.toml`:

```toml
# in [features], after the `gpu-2d = ...` line (:26):
vello = ["dep:vello", "dep:vello_cpu", "dep:peniko", "dep:kurbo", "dep:bytemuck"]
vello-gpui = ["vello", "gpui", "gpu-2d"]

# in [dependencies], after `pollster = ...` (:53):
vello = { workspace = true, optional = true }
vello_cpu = { workspace = true, optional = true }
peniko = { workspace = true, optional = true }
kurbo = { workspace = true, optional = true }

# at the end of the [[test]] block area (after mesh_compute_diff_tests, :236-239):
[[test]]
name = "vello2d_scene_tests"
path = "tests/vello2d_scene_tests.rs"
required-features = ["vello"]

[[test]]
name = "vello2d_cpu_tests"
path = "tests/vello2d_cpu_tests.rs"
required-features = ["vello"]

[[test]]
name = "vello2d_element_tests"
path = "tests/vello2d_element_tests.rs"
required-features = ["vello-gpui"]

[[bench]]
name = "vello2d_bench"
path = "benches/vello2d_bench.rs"
harness = false
required-features = ["vello"]
```

- [ ] **Step 4: Add the px forwarding feature**

In `crates/gpui-px/Cargo.toml` (read it first; the d3rs dep is at :16-17):

```toml
# in [features]:
vello = ["gpui-d3rs/vello-gpui"]
```

Do NOT add `vello` to px's `default` features.

- [ ] **Step 5: Create stub files so the registered targets exist**

```bash
mkdir -p crates/gpui-d3rs/tests crates/gpui-d3rs/benches
printf '// populated by Task 2\n' > crates/gpui-d3rs/tests/vello2d_scene_tests.rs
printf '// populated by Task 4\n' > crates/gpui-d3rs/tests/vello2d_cpu_tests.rs
printf '// populated by Task 6\n' > crates/gpui-d3rs/tests/vello2d_element_tests.rs
printf '// populated by Task 10\nfn main() {}\n' > crates/gpui-d3rs/benches/vello2d_bench.rs
```

- [ ] **Step 6: Verify unification and compilation**

```bash
cargo check -p gpui-d3rs --no-default-features --features vello
cargo tree -p gpui-d3rs --no-default-features --features vello -i wgpu
cargo tree -p gpui-d3rs --no-default-features --features vello -i naga
cargo check -p gpui-d3rs --features vello-gpui
cargo check -p gpui-px --features vello
cargo test -p gpui-d3rs --no-default-features --tests
```

Expected: all checks pass; both `cargo tree -i` outputs show a SINGLE wgpu/naga, sourced from `git+https://github.com/zed-industries/wgpu.git?...357a0c56`, version 29.0.3. If vello fails on missing `wgsl`/`std` wgpu features, add `features = ["wgsl"]` to the workspace `wgpu` dep (line ~126) — feature union then covers vello. If cargo errors that the patch version doesn't match, STOP and re-check the patch rev's package versions against 29.0.3.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/gpui-d3rs/Cargo.toml crates/gpui-px/Cargo.toml crates/gpui-d3rs/tests/vello2d_scene_tests.rs crates/gpui-d3rs/tests/vello2d_cpu_tests.rs crates/gpui-d3rs/tests/vello2d_element_tests.rs crates/gpui-d3rs/benches/vello2d_bench.rs
git commit -m "feat(d3rs): add vello 0.10 deps unified on Zed wgpu 29.0.3 fork"
```

---

### Task 2: ChartScene IR (scene.rs)

**Files:**
- Create: `crates/gpui-d3rs/src/vello2d/mod.rs`
- Create: `crates/gpui-d3rs/src/vello2d/scene.rs`
- Test: `crates/gpui-d3rs/tests/vello2d_scene_tests.rs`
- Modify: `crates/gpui-d3rs/src/lib.rs` (add `#[cfg(feature = "vello")] pub mod vello2d;` — find the other `pub mod` declarations and place with them)

**Interfaces:**
- Produces (relied on by Tasks 3, 4, 6, 7, 8):
  - `pub enum ChartCmd { Fill { path: kurbo::BezPath, fill: peniko::Fill, brush: peniko::Brush }, Stroke { path: kurbo::BezPath, stroke: kurbo::Stroke, brush: peniko::Brush } }`
  - `pub struct ChartScene { cmds: Vec<ChartCmd> }`
  - `ChartScene::new() -> Self`, `fn fill_path(&mut self, path: kurbo::BezPath, brush: peniko::Brush)`, `fn fill_circle(&mut self, cx: f64, cy: f64, radius: f64, brush: peniko::Brush)`, `fn fill_rect(&mut self, rect: kurbo::Rect, brush: peniko::Brush)`, `fn stroke_path(&mut self, path: kurbo::BezPath, stroke: kurbo::Stroke, brush: peniko::Brush)`, `fn stroke_polyline(&mut self, points: &[(f64, f64)], stroke: kurbo::Stroke, brush: peniko::Brush)`, `fn commands(&self) -> &[ChartCmd]`, `fn len(&self) -> usize`, `fn is_empty(&self) -> bool`
  - Re-exports from `d3rs::vello2d`: `kurbo`, `peniko` (for integration tests).

- [ ] **Step 1: Write the failing tests**

`crates/gpui-d3rs/tests/vello2d_scene_tests.rs`:

```rust
use d3rs::vello2d::kurbo::{Circle, Rect, Stroke};
use d3rs::vello2d::peniko::{Brush, Color, Fill};
use d3rs::vello2d::{ChartCmd, ChartScene};

fn red() -> Brush {
    Brush::Solid(Color::from_rgb8(255, 0, 0))
}

#[test]
fn new_scene_is_empty() {
    let scene = ChartScene::new();
    assert!(scene.is_empty());
    assert_eq!(scene.len(), 0);
}

#[test]
fn fill_circle_appends_fill_cmd_with_circle_path() {
    let mut scene = ChartScene::new();
    scene.fill_circle(10.0, 20.0, 3.0, red());
    assert_eq!(scene.len(), 1);
    let ChartCmd::Fill { path, fill, .. } = &scene.commands()[0] else {
        panic!("expected Fill command");
    };
    assert_eq!(*fill, Fill::NonZero);
    // Circle::to_path emits move_to + 4 cubic segments + close = 6 elements.
    assert_eq!(path.elements().len(), 6);
}

#[test]
fn fill_rect_uses_rect_shape() {
    let mut scene = ChartScene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 5.0, 7.0), red());
    let ChartCmd::Fill { path, .. } = &scene.commands()[0] else {
        panic!("expected Fill command");
    };
    let els = path.elements();
    assert_eq!(els.len(), 5); // M + 3 L + close
    assert_eq!(els[0], kurbo::PathEl::MoveTo((0.0, 0.0).into()));
}

#[test]
fn stroke_polyline_builds_single_open_path() {
    let mut scene = ChartScene::new();
    scene.stroke_polyline(&[(0.0, 0.0), (1.0, 2.0), (3.0, 4.0)], Stroke::new(2.0), red());
    let ChartCmd::Stroke { path, stroke, .. } = &scene.commands()[0] else {
        panic!("expected Stroke command");
    };
    assert_eq!(stroke.width, 2.0);
    let els = path.elements();
    assert_eq!(els.len(), 3); // M + 2 L, NOT closed
    assert!(!matches!(els.last(), Some(kurbo::PathEl::ClosePath)));
}

#[test]
fn stroke_polyline_with_fewer_than_two_points_is_noop() {
    let mut scene = ChartScene::new();
    scene.stroke_polyline(&[(1.0, 1.0)], Stroke::new(1.0), red());
    assert!(scene.is_empty());
}

#[test]
fn circle_helper_matches_kurbo_circle_path() {
    let mut a = ChartScene::new();
    a.fill_circle(5.0, 5.0, 2.0, red());
    let ChartCmd::Fill { path, .. } = &a.commands()[0] else {
        panic!("expected Fill command");
    };
    let expected = Circle::new((5.0, 5.0), 2.0).to_path(0.1);
    assert_eq!(path.elements(), expected.elements());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_scene_tests`
Expected: FAIL — `d3rs::vello2d` does not exist (compile error).

- [ ] **Step 3: Implement mod.rs, scene.rs, and lib.rs wiring**

`crates/gpui-d3rs/src/vello2d/mod.rs`:

```rust
//! vello-backed 2D chart rendering.
//!
//! Charts emit draw commands into the backend-neutral [`ChartScene`] IR.
//! `gpu_scene` replays it into a `vello::Scene`; `cpu` replays it through
//! `vello_cpu`. The GPUI element and wgpu custom draw live behind the
//! `vello-gpui` feature.

mod cpu;
mod gpu_scene;
mod scene;

#[cfg(feature = "vello-gpui")]
mod element;
#[cfg(feature = "vello-gpui")]
mod wgpu_draw;

// Re-exported so integration tests and downstream crates (gpui-px) use the
// exact kurbo/peniko versions vello is compiled against.
pub use kurbo;
pub use peniko;

pub use cpu::CpuRasterizer;
pub use gpu_scene::to_vello_scene;
pub use scene::{ChartCmd, ChartScene};

#[cfg(feature = "vello-gpui")]
pub use element::{RasterBackend, VelloChartElement};
#[cfg(feature = "vello-gpui")]
pub use wgpu_draw::physical_size as wgpu_draw_physical_size;
```

`crates/gpui-d3rs/src/vello2d/scene.rs`:

```rust
//! Backend-neutral chart scene: kurbo geometry + peniko brushes.

use kurbo::{BezPath, Circle, PathEl, Rect, Stroke};
use peniko::{Brush, Fill};

/// One draw command in a [`ChartScene`].
#[derive(Clone, Debug)]
pub enum ChartCmd {
    /// Fill `path` with `brush` using `fill` rule.
    Fill {
        path: BezPath,
        fill: Fill,
        brush: Brush,
    },
    /// Stroke `path` with `stroke` style and `brush`.
    Stroke {
        path: BezPath,
        stroke: Stroke,
        brush: Brush,
    },
}

/// Ordered list of chart draw commands, replayed by GPU or CPU backends.
#[derive(Clone, Debug, Default)]
pub struct ChartScene {
    cmds: Vec<ChartCmd>,
}

impl ChartScene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fill an arbitrary path with the non-zero winding rule.
    pub fn fill_path(&mut self, path: BezPath, brush: Brush) {
        self.cmds.push(ChartCmd::Fill {
            path,
            fill: Fill::NonZero,
            brush,
        });
    }

    /// Fill a circle (the scatter-marker primitive).
    pub fn fill_circle(&mut self, cx: f64, cy: f64, radius: f64, brush: Brush) {
        if radius <= 0.0 {
            return;
        }
        self.fill_path(Circle::new((cx, cy), radius).to_path(0.1), brush);
    }

    /// Fill an axis-aligned rectangle (the bar primitive).
    pub fn fill_rect(&mut self, rect: Rect, brush: Brush) {
        self.fill_path(rect.to_path(0.1), brush);
    }

    /// Stroke an arbitrary path.
    pub fn stroke_path(&mut self, path: BezPath, stroke: Stroke, brush: Brush) {
        self.cmds.push(ChartCmd::Stroke {
            path,
            stroke,
            brush,
        });
    }

    /// Stroke a polyline (the line-chart primitive). Fewer than two points
    /// emit nothing.
    pub fn stroke_polyline(&mut self, points: &[(f64, f64)], stroke: Stroke, brush: Brush) {
        if points.len() < 2 {
            return;
        }
        let mut path = BezPath::new();
        path.push(PathEl::MoveTo(points[0].into()));
        for &p in &points[1..] {
            path.push(PathEl::LineTo(p.into()));
        }
        self.stroke_path(path, stroke, brush);
    }

    pub fn commands(&self) -> &[ChartCmd] {
        &self.cmds
    }

    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }
}
```

In `crates/gpui-d3rs/src/lib.rs`, with the other module declarations:

```rust
#[cfg(feature = "vello")]
pub mod vello2d;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_scene_tests`
Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gpui-d3rs/src/vello2d/mod.rs crates/gpui-d3rs/src/vello2d/scene.rs crates/gpui-d3rs/src/lib.rs crates/gpui-d3rs/tests/vello2d_scene_tests.rs
git commit -m "feat(d3rs): ChartScene IR for vello chart rendering"
```

---

### Task 3: GPU replay — `to_vello_scene`

**Files:**
- Create: `crates/gpui-d3rs/src/vello2d/gpu_scene.rs`
- Test: `crates/gpui-d3rs/tests/vello2d_scene_tests.rs` (append)

**Interfaces:**
- Consumes: `ChartScene`, `ChartCmd` (Task 2).
- Produces: `pub fn to_vello_scene(scene: &ChartScene) -> vello::Scene` — consumed by `WgpuVelloDraw` (Task 7).

- [ ] **Step 1: Write the failing test (append to tests/vello2d_scene_tests.rs)**

vello's `Scene::encoding()` exposes `vello_encoding::Encoding` with public `draw_tags` / `path_data` vecs — one `DrawTag` per fill/stroke command. (Field names checked against vello 0.10 docs; if the compiler disagrees, adjust to the actual public fields of `Encoding`.)

```rust
use d3rs::vello2d::to_vello_scene;

#[test]
fn gpu_replay_emits_one_draw_per_command() {
    let mut scene = ChartScene::new();
    scene.fill_circle(10.0, 10.0, 2.0, red());
    scene.stroke_polyline(&[(0.0, 0.0), (5.0, 5.0)], Stroke::new(1.0), red());
    let vello_scene = to_vello_scene(&scene);
    let encoding = vello_scene.encoding();
    assert_eq!(encoding.draw_tags.len(), 2);
    assert!(!encoding.path_data.is_empty());
}

#[test]
fn gpu_replay_of_empty_scene_has_no_draws() {
    let vello_scene = to_vello_scene(&ChartScene::new());
    assert_eq!(vello_scene.encoding().draw_tags.len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_scene_tests gpu_replay`
Expected: FAIL — `to_vello_scene` not found.

- [ ] **Step 3: Implement gpu_scene.rs**

```rust
//! Replay a [`ChartScene`] into a `vello::Scene` for GPU rendering.

use crate::vello2d::{ChartCmd, ChartScene};
use vello::kurbo::Affine;

/// Build a fresh `vello::Scene` from the IR. Rebuilt per frame in
/// `draw_wgpu`; encoding is cheap relative to rasterization.
pub fn to_vello_scene(scene: &ChartScene) -> vello::Scene {
    let mut out = vello::Scene::new();
    for cmd in scene.commands() {
        match cmd {
            ChartCmd::Fill { path, fill, brush } => {
                out.fill(*fill, Affine::IDENTITY, brush, None, path);
            }
            ChartCmd::Stroke {
                path,
                stroke,
                brush,
            } => {
                out.stroke(stroke, Affine::IDENTITY, brush, None, path);
            }
        }
    }
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_scene_tests`
Expected: 8 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gpui-d3rs/src/vello2d/gpu_scene.rs crates/gpui-d3rs/tests/vello2d_scene_tests.rs
git commit -m "feat(d3rs): replay ChartScene into vello::Scene"
```

---

### Task 4: CPU replay — `CpuRasterizer`

**Files:**
- Create: `crates/gpui-d3rs/src/vello2d/cpu.rs`
- Test: `crates/gpui-d3rs/tests/vello2d_cpu_tests.rs`

**Interfaces:**
- Consumes: `ChartScene`, `ChartCmd` (Task 2).
- Produces: `pub struct CpuRasterizer` with `new(width: u16, height: u16) -> Self` and `rasterize(&mut self, scene: &ChartScene, width: u16, height: u16) -> Vec<u8>` (premultiplied RGBA8, row-major, `width*4` stride) — consumed by the element's CPU paint path (Task 6) and the QA oracle (Tasks 8, 10).

- [ ] **Step 1: Write the failing tests**

`crates/gpui-d3rs/tests/vello2d_cpu_tests.rs`:

```rust
use d3rs::vello2d::kurbo::{Circle, Rect, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, CpuRasterizer};

fn px(buf: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * w + x) * 4;
    [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
}

#[test]
fn filled_rect_paints_interior_and_leaves_exterior_transparent() {
    let mut scene = ChartScene::new();
    scene.fill_rect(
        Rect::new(10.0, 10.0, 50.0, 50.0),
        Brush::Solid(Color::from_rgb8(255, 0, 0)),
    );
    let mut rast = CpuRasterizer::new(100, 100);
    let buf = rast.rasterize(&scene, 100, 100);
    assert_eq!(buf.len(), 100 * 100 * 4);
    let [r, g, b, a] = px(&buf, 100, 30, 30);
    assert!(r > 200 && g < 40 && b < 40 && a > 200, "interior: {r},{g},{b},{a}");
    assert_eq!(px(&buf, 100, 2, 2)[3], 0, "corner must stay transparent");
}

#[test]
fn stroked_circle_paints_ring() {
    let mut scene = ChartScene::new();
    scene.stroke_path(
        Circle::new((50.0, 50.0), 20.0).to_path(0.1),
        Stroke::new(4.0),
        Brush::Solid(Color::from_rgb8(0, 0, 255)),
    );
    let mut rast = CpuRasterizer::new(100, 100);
    let buf = rast.rasterize(&scene, 100, 100);
    // Point on the ring (rightmost): opaque blue.
    let ring = px(&buf, 100, 70, 50);
    assert!(ring[2] > 150 && ring[3] > 150, "ring: {ring:?}");
    // Center: transparent.
    assert_eq!(px(&buf, 100, 50, 50)[3], 0);
}

#[test]
fn resize_reallocates_and_clears() {
    let mut scene = ChartScene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 20.0, 20.0), Brush::Solid(Color::from_rgb8(0, 255, 0)));
    let mut rast = CpuRasterizer::new(32, 32);
    let _ = rast.rasterize(&scene, 32, 32);
    let buf = rast.rasterize(&ChartScene::new(), 64, 48);
    assert_eq!(buf.len(), 64 * 48 * 4);
    assert!(buf.iter().all(|&b| b == 0), "empty scene must clear the buffer");
}

#[test]
fn deterministic_for_fixed_input() {
    // QA-oracle property: same scene -> identical bytes across runs.
    let mut scene = ChartScene::new();
    scene.fill_circle(25.0, 25.0, 10.0, Brush::Solid(Color::from_rgba8(10, 200, 100, 128)));
    scene.stroke_polyline(
        &[(0.0, 0.0), (49.0, 49.0)],
        Stroke::new(1.5),
        Brush::Solid(Color::from_rgb8(1, 2, 3)),
    );
    let mut a = CpuRasterizer::new(50, 50);
    let mut b = CpuRasterizer::new(50, 50);
    assert_eq!(a.rasterize(&scene, 50, 50), b.rasterize(&scene, 50, 50));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_cpu_tests`
Expected: FAIL — `CpuRasterizer` not found.

- [ ] **Step 3: Implement cpu.rs**

vello_cpu 0.2 API (verified on docs.rs): `RenderContext::new(u16, u16)`, `set_paint(impl Into<Paint>)`, `set_stroke(kurbo::Stroke)`, `fill_path(&BezPath)`, `stroke_path(&BezPath)`, `flush()`, `render(&mut Pixmap, &mut Resources)`, `reset()`; `Pixmap::new(u16, u16)` with `data() -> &[PremulRgba8]` (fields `.r/.g/.b/.a`, premultiplied). vello_cpu re-exports `kurbo` and `peniko` — use those re-exports below so types always match its own version (they unify with the direct kurbo/peniko deps from Task 1 anyway).

```rust
//! CPU replay of a [`ChartScene`] via vello_cpu's sparse-strips rasterizer.
//!
//! Universal fallback (Metal renderer, missing wgpu hook) and the
//! deterministic QA oracle for GPU output. Output is premultiplied RGBA8,
//! matching what `gpu2d/element.rs` hands to `window.paint_image`.

use crate::vello2d::{ChartCmd, ChartScene};
use vello_cpu::peniko::Brush;
use vello_cpu::{Pixmap, RenderContext, Resources};

/// Reusable vello_cpu rasterizer; recreates its context only on resize.
pub struct CpuRasterizer {
    ctx: RenderContext,
    resources: Resources,
    size: (u16, u16),
}

impl CpuRasterizer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            ctx: RenderContext::new(width, height),
            resources: Resources::new(),
            size: (width, height),
        }
    }

    /// Rasterize `scene` at `width`x`height`; returns premultiplied RGBA8
    /// bytes (`width*4` row stride). An empty scene yields a cleared buffer.
    pub fn rasterize(&mut self, scene: &ChartScene, width: u16, height: u16) -> Vec<u8> {
        if self.size != (width, height) {
            *self = Self::new(width, height);
        } else {
            self.ctx.reset();
        }
        for cmd in scene.commands() {
            match cmd {
                ChartCmd::Fill { path, brush, .. } => {
                    apply_paint(&mut self.ctx, brush);
                    self.ctx.fill_path(path);
                }
                ChartCmd::Stroke {
                    path,
                    stroke,
                    brush,
                } => {
                    apply_paint(&mut self.ctx, brush);
                    self.ctx.set_stroke(stroke.clone());
                    self.ctx.stroke_path(path);
                }
            }
        }
        self.ctx.flush();
        let mut pixmap = Pixmap::new(width, height);
        self.ctx.render(&mut pixmap, &mut self.resources);
        pixmap
            .data()
            .iter()
            .flat_map(|p| [p.r, p.g, p.b, p.a])
            .collect()
    }
}

fn apply_paint(ctx: &mut RenderContext, brush: &Brush) {
    match brush {
        Brush::Solid(color) => ctx.set_paint(*color),
        Brush::Gradient(gradient) => ctx.set_paint(gradient.clone()),
        // Charts never paint images; vello_cpu image paints are out of scope.
        Brush::Image(_) => log::warn!("vello2d: image brush unsupported on CPU backend, skipped"),
    }
}
```

If `vello_cpu` doesn't re-export `peniko` (compile error on `use vello_cpu::peniko::Brush`), switch to the direct `peniko::Brush` dep. If d3rs lacks a `log` dependency (check `[dependencies]`), use `eprintln!` instead of adding one.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --tests`
Expected: all PASS (4 new + 8 scene tests). If the exact pixel assertions are off by antialiasing at edges, adjust the sampled pixel toward the shape center — the tests assert interior/ring/exterior, not edge coverage.

- [ ] **Step 5: Commit**

```bash
git add crates/gpui-d3rs/src/vello2d/cpu.rs crates/gpui-d3rs/tests/vello2d_cpu_tests.rs
git commit -m "feat(d3rs): vello_cpu replay of ChartScene (CpuRasterizer)"
```

---

### Task 5: Vendored GPUI patches — capability probe + `target_format`

**Files:**
- Modify: `crates/3rdparties/gpui/src/custom_draw.rs`
- Modify: `crates/3rdparties/gpui_wgpu/src/custom.rs:19-27`
- Modify: `crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs:1214-1246` (dispatch) + renderer init (grep `fn new` in that file; `new_from_canvas` is at :192-204)
- Modify: `crates/gpui-d3rs/src/mesh/gpu/wgpu_backend.rs:285-294` (signature update)
- Modify: `crates/3rdparties/gpui_wgpu/PATCHES.md` and `crates/3rdparties/gpui/PATCHES.md` (check both exist; match their format)

**Interfaces:**
- Produces: `gpui::wgpu_custom_draw_available() -> bool` + `#[doc(hidden)] gpui::set_wgpu_custom_draw_available(bool)`; `WgpuCustomDraw::draw_wgpu(&self, ctx, encoder, target, target_format: wgpu::TextureFormat, target_size, bounds, scale_factor)` (new param after `target`).
- Consumed by Tasks 6, 7.

- [ ] **Step 1: Write the failing test (append to the `#[cfg(test)] mod tests` in `crates/3rdparties/gpui/src/custom_draw.rs`)**

```rust
    #[test]
    fn wgpu_custom_draw_flag_roundtrip() {
        assert!(!wgpu_custom_draw_available());
        set_wgpu_custom_draw_available(true);
        assert!(wgpu_custom_draw_available());
        set_wgpu_custom_draw_available(false);
        assert!(!wgpu_custom_draw_available());
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p gpui custom_draw`
Expected: FAIL — functions don't exist.

- [ ] **Step 3: Implement the probe in custom_draw.rs**

Add to `crates/3rdparties/gpui/src/custom_draw.rs`:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by the wgpu renderer (`gpui_wgpu`) when it initializes, meaning
/// `WgpuCustomDraw` primitives registered in this process will actually be
/// dispatched. Chart elements probe this to pick GPU vs CPU rasterization.
static WGPU_CUSTOM_DRAW_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Whether the active renderer dispatches `WgpuCustomDraw` primitives.
pub fn wgpu_custom_draw_available() -> bool {
    WGPU_CUSTOM_DRAW_AVAILABLE.load(Ordering::Acquire)
}

/// Called by `gpui_wgpu::WgpuRenderer` on init. Not app API.
#[doc(hidden)]
pub fn set_wgpu_custom_draw_available(available: bool) {
    WGPU_CUSTOM_DRAW_AVAILABLE.store(available, Ordering::Release);
}
```

Check that both are re-exported at the gpui crate root (`crates/3rdparties/gpui/src/gpui.rs` or wherever `register_custom_draw` is re-exported — grep for `pub use.*custom_draw`).

- [ ] **Step 4: Extend `draw_wgpu` with `target_format`**

In `crates/3rdparties/gpui_wgpu/src/custom.rs`, change the trait method to:

```rust
    /// Record a render pass against `target` without submitting `encoder`.
    ///
    /// `target_format` is the texture format of `target` (needed to build
    /// compositing pipelines). Implementations should use `bounds` as their
    /// scissor rectangle. The bounds are in GPUI pixels and `scale_factor`
    /// is the scale to use when converting them to device pixels.
    /// `target_size` is the physical extent of `target`.
    fn draw_wgpu(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: [u32; 2],
        bounds: Bounds<Pixels>,
        scale_factor: f32,
    );
```

At the dispatch site in `crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs:1238-1245`, pass the surface format:

```rust
                                        wgpu_draw.0.draw_wgpu(
                                            context,
                                            &mut encoder,
                                            &frame_view,
                                            self.surface_config.format,
                                            [self.surface_config.width, self.surface_config.height],
                                            bounds,
                                            1.0,
                                        );
```

Update `WgpuMeshDraw::draw_wgpu` in `crates/gpui-d3rs/src/mesh/gpu/wgpu_backend.rs:285-294`: add `_target_format: wgpu::TextureFormat,` after `target`, body unchanged.

- [ ] **Step 5: Set the probe flag in the wgpu renderer init**

In `crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs`, find the `WgpuRenderer` constructors (grep `fn new`; `new_from_canvas` is at :192-204). Wherever the `WgpuContext` has been successfully created and stored into `self.context` (grep for `self.context =`), add immediately after:

```rust
        gpui::set_wgpu_custom_draw_available(true);
```

Add it to every successful-init path (or once in a shared context-init helper if one exists — read the file and follow its structure).

- [ ] **Step 6: Run the new test + full check**

```bash
cargo test -p gpui custom_draw
cargo check -p gpui-d3rs --features gpu-2d
cargo check -p gpui-d3rs --features vello-gpui
```

Expected: flag test PASSES; everything compiles (MeshPlot signature updated).

- [ ] **Step 7: Document the patches and commit**

Append to `crates/3rdparties/gpui_wgpu/PATCHES.md` (read it first, match its format): an entry describing the `target_format` addition and the `set_wgpu_custom_draw_available(true)` call on init. Same in `crates/3rdparties/gpui/PATCHES.md` for the probe flag.

```bash
git add crates/3rdparties/gpui/src/custom_draw.rs crates/3rdparties/gpui_wgpu/src/custom.rs crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs crates/3rdparties/gpui_wgpu/PATCHES.md crates/3rdparties/gpui/PATCHES.md crates/gpui-d3rs/src/mesh/gpu/wgpu_backend.rs
git commit -m "feat(gpui): wgpu custom-draw capability probe + target_format param"
```

---

### Task 6: `VelloChartElement` + CPU paint path

**Files:**
- Create: `crates/gpui-d3rs/src/vello2d/element.rs`
- Test: `crates/gpui-d3rs/tests/vello2d_element_tests.rs`

**Interfaces:**
- Consumes: `ChartScene` (Task 2), `CpuRasterizer` (Task 4), `gpui::wgpu_custom_draw_available` / `register_custom_draw` / `unregister_custom_draw` / `Window::paint_custom` (Task 5 + existing), `WgpuVelloDraw` (Task 7 — implement its Step-3 skeleton first if landing this task standalone, or land 6+7 in one branch).
- Produces:
  - `pub enum RasterBackend { Auto, Wgpu, Cpu }`
  - `pub struct VelloChartElement` with `pub fn new(scene: ChartScene) -> Self`, `pub fn with_builder(builder: impl Fn(f32, f32) -> ChartScene + 'static) -> Self`, `pub fn backend(mut self, backend: RasterBackend) -> Self`, `pub fn absolute(mut self) -> Self`; implements `IntoElement + Element` (layout mirrors `Chart2DElement`: `relative(1.0)` size, `.absolute()` opt-in).
  - `with_builder` regenerates the scene when paint-time bounds change size — this is what keeps charts correct across window resizes (the legacy `canvas` charts do the same inside their paint closure).

- [ ] **Step 1: Write the failing tests**

`crates/gpui-d3rs/tests/vello2d_element_tests.rs`:

```rust
use d3rs::vello2d::kurbo::Rect;
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, RasterBackend, VelloChartElement};

fn sample_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 4.0, 4.0), Brush::Solid(Color::from_rgb8(9, 9, 9)));
    scene
}

#[test]
fn default_backend_is_auto() {
    let element = VelloChartElement::new(ChartScene::new());
    assert!(format!("{element:?}").contains("Auto"));
}

#[test]
fn explicit_cpu_backend_shows_in_debug() {
    let element = VelloChartElement::new(sample_scene()).backend(RasterBackend::Cpu);
    assert!(format!("{element:?}").contains("Cpu"));
}

#[test]
fn builder_supplies_scene_lazily() {
    // Scene starts empty; the builder fills it at first paint with real bounds.
    let element = VelloChartElement::with_builder(|w, h| {
        let mut scene = ChartScene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, w as f64, h as f64), Brush::Solid(Color::from_rgb8(1, 2, 3)));
        scene
    });
    assert!(format!("{element:?}").contains("builder"));
}
```

(Paint-path behavior is verified by the CPU golden tests in Task 8 and visually in Task 9; GPUI's headless `TestAppContext` cannot exercise a real renderer, so unit tests cover construction/resolution only.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello-gpui --test vello2d_element_tests`
Expected: FAIL — `VelloChartElement` not found.

- [ ] **Step 3: Implement element.rs**

Model the `Element` impl on `gpu2d/element.rs:124-190` (style with `relative(1.0)` / absolute opt-in, no-op prepaint) and the image paint on `gpu2d/element.rs:317-331`. Read `../../GPUI.md` first per repo rule. Note: `Frame` and `RenderImage` come from `gpui`, `RgbaImage` from the `image` crate (already an optional dep enabled by the `gpui` feature).

```rust
//! GPUI element that paints a [`ChartScene`] via vello (GPU) or vello_cpu.

use crate::vello2d::wgpu_draw::WgpuVelloDraw;
use crate::vello2d::{ChartScene, CpuRasterizer};
use gpui::{
    App, Bounds, Corners, CustomDrawId, Edges, Element, ElementId, Frame, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Position, RenderImage, Size, Style,
    Window, px, relative,
};
use image::RgbaImage;
use std::cell::RefCell;
use std::panic::Location;
use std::rc::Rc;
use std::sync::Arc;

/// Which rasterizer paints the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterBackend {
    /// Probe `gpui::wgpu_custom_draw_available()` at first paint.
    Auto,
    /// Zero-copy GPU path through `WgpuCustomDraw` (requires the wgpu renderer).
    Wgpu,
    /// `vello_cpu` pixmap + `paint_image`. Works on every renderer.
    Cpu,
}

type SceneBuilder = Rc<dyn Fn(f32, f32) -> ChartScene>;

enum BackendState {
    Wgpu {
        custom_id: CustomDrawId,
        shared: Rc<RefCell<ChartScene>>,
    },
    Cpu(CpuRasterizer),
}

/// Element painting a [`ChartScene`]. Build it in the chart's render method;
/// `Drop` unregisters the custom draw. With `with_builder`, the scene is
/// (re)generated whenever paint bounds change size.
pub struct VelloChartElement {
    scene: ChartScene,
    builder: Option<SceneBuilder>,
    scene_size: Option<(f32, f32)>,
    backend_pref: RasterBackend,
    state: Option<BackendState>,
    absolute: bool,
}

impl std::fmt::Debug for VelloChartElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("VelloChartElement");
        d.field("backend_pref", &self.backend_pref)
            .field("scene_commands", &self.scene.len())
            .field(
                "resolved",
                &match &self.state {
                    None => "no",
                    Some(BackendState::Wgpu { .. }) => "Wgpu",
                    Some(BackendState::Cpu(_)) => "Cpu",
                },
            );
        if self.builder.is_some() {
            d.field("builder", &true);
        }
        d.finish()
    }
}

impl VelloChartElement {
    /// Static scene, baked in the coordinates it will be painted at. The
    /// caller must rebuild the element when the chart's pixel size changes.
    pub fn new(scene: ChartScene) -> Self {
        Self {
            scene,
            builder: None,
            scene_size: None,
            backend_pref: RasterBackend::Auto,
            state: None,
            absolute: false,
        }
    }

    /// Scene is (re)built at paint time from the actual bounds size
    /// (`builder(width, height)` in element-local pixels).
    pub fn with_builder(builder: impl Fn(f32, f32) -> ChartScene + 'static) -> Self {
        Self {
            scene: ChartScene::new(),
            builder: Some(Rc::new(builder)),
            scene_size: None,
            backend_pref: RasterBackend::Auto,
            state: None,
            absolute: false,
        }
    }

    pub fn backend(mut self, backend: RasterBackend) -> Self {
        self.backend_pref = backend;
        self
    }

    pub fn absolute(mut self) -> Self {
        self.absolute = true;
        self
    }

    /// Resolve the backend on first paint and, for wgpu, register the draw.
    fn resolve(&mut self) {
        if self.state.is_some() {
            return;
        }
        let backend = match self.backend_pref {
            RasterBackend::Auto => {
                if gpui::wgpu_custom_draw_available() {
                    RasterBackend::Wgpu
                } else {
                    RasterBackend::Cpu
                }
            }
            explicit => explicit,
        };
        self.state = Some(match backend {
            RasterBackend::Wgpu => {
                let shared = Rc::new(RefCell::new(self.scene.clone()));
                let draw = WgpuVelloDraw::new(Rc::clone(&shared));
                let custom_id = gpui::register_custom_draw(draw.into_custom_draw());
                BackendState::Wgpu { custom_id, shared }
            }
            RasterBackend::Cpu | RasterBackend::Auto => {
                BackendState::Cpu(CpuRasterizer::new(1, 1))
            }
        });
    }
}

impl Drop for VelloChartElement {
    fn drop(&mut self) {
        if let Some(BackendState::Wgpu { custom_id, .. }) = &self.state {
            gpui::unregister_custom_draw(*custom_id);
        }
    }
}

impl IntoElement for VelloChartElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for VelloChartElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = if self.absolute {
            Style {
                position: Position::Absolute,
                inset: Edges {
                    top: px(0.0).into(),
                    right: px(0.0).into(),
                    bottom: px(0.0).into(),
                    left: px(0.0).into(),
                },
                size: Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                ..Default::default()
            }
        } else {
            Style {
                size: Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                ..Default::default()
            }
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        if width < 1.0 || height < 1.0 {
            return;
        }

        // (Re)build the scene when the builder exists and the size changed.
        if let Some(builder) = self.builder.clone()
            && self.scene_size != Some((width, height))
        {
            self.scene = builder(width, height);
            self.scene_size = Some((width, height));
            if let Some(BackendState::Wgpu { shared, .. }) = &self.state {
                *shared.borrow_mut() = self.scene.clone();
            }
        }
        if self.scene.is_empty() {
            return;
        }
        self.resolve();

        match self.state.as_mut() {
            Some(BackendState::Wgpu { custom_id, .. }) => {
                window.paint_custom(*custom_id, bounds);
            }
            Some(BackendState::Cpu(rasterizer)) => {
                let (w, h) = (width as u16, height as u16);
                let pixels = rasterizer.rasterize(&self.scene, w, h);
                if pixels.iter().all(|&b| b == 0) {
                    return;
                }
                if let Some(rgba) = RgbaImage::from_raw(w as u32, h as u32, pixels) {
                    let image = RenderImage::new(vec![Frame::new(rgba)]);
                    let _ = window.paint_image(bounds, Corners::default(), Arc::new(image), 0, false);
                }
            }
            None => {}
        }
    }
}
```

Notes for the implementer:
- `WgpuVelloDraw::new(Rc<RefCell<ChartScene>>)` / `.into_custom_draw()` are defined in Task 7; if landing this task first, create Task 7's Step-3 skeleton (struct + constructor + trait impls with an empty `draw_wgpu`) so this compiles, then fill in the GPU body in Task 7.
- The Rust 2024 `let ... && let ...` chains used above (`if let Some(builder) = ... && self.scene_size != ...`) match existing d3rs style (see `render_scatter` at scatter.rs:437-438); if the compiler objects, nest the `if`s.
- CPU rasterization at logical size mirrors `gpu2d/element.rs` (same retina-crispness limitation); do not "improve" it here.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p gpui-d3rs --no-default-features --features vello-gpui --test vello2d_element_tests
cargo test -p gpui-d3rs --no-default-features --tests   # repo-rule regression check
cargo clippy -p gpui-d3rs --features vello-gpui
```

Expected: 3 element tests PASS; no-default-features suite stays green; no new clippy warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/gpui-d3rs/src/vello2d/element.rs crates/gpui-d3rs/tests/vello2d_element_tests.rs
git commit -m "feat(d3rs): VelloChartElement with vello_cpu paint fallback"
```

---

### Task 7: `WgpuVelloDraw` — zero-copy GPU backend

**Files:**
- Create: `crates/gpui-d3rs/src/vello2d/wgpu_draw.rs`

**Interfaces:**
- Consumes: `ChartScene` + `to_vello_scene` (Tasks 2-3), `WgpuCustomDraw`/`WgpuCustomDrawAdapter` with `target_format` (Task 5), `WgpuContext { device: Arc<Device>, queue: Arc<Queue>, .. }` (`gpui_wgpu/src/wgpu_context.rs:9-17`).
- Produces: `pub struct WgpuVelloDraw` with `pub fn new(scene: Rc<RefCell<ChartScene>>) -> Self` and `pub fn into_custom_draw(self) -> Rc<dyn gpui::CustomDraw>` — consumed by element.rs (Task 6); `pub fn physical_size(width: f32, height: f32, scale_factor: f32) -> [u32; 2]`.

- [ ] **Step 1: Write failing unit test for the pure size helper**

Append to `crates/gpui-d3rs/tests/vello2d_element_tests.rs`:

```rust
use d3rs::vello2d::wgpu_draw_physical_size;

#[test]
fn physical_size_scales_and_clamps() {
    assert_eq!(wgpu_draw_physical_size(100.0, 50.0, 2.0), [200, 100]);
    assert_eq!(wgpu_draw_physical_size(0.0, -3.0, 1.0), [1, 1]);
}
```

Run: `cargo test -p gpui-d3rs --no-default-features --features vello-gpui --test vello2d_element_tests physical_size`
Expected: FAIL — function not found.

- [ ] **Step 2: Implement wgpu_draw.rs**

vello 0.10 API (verified against the docs.rs source): `Renderer::new(&Device, RendererOptions) -> vello::Result<Renderer>`; `render_to_texture(&mut self, &Device, &Queue, &Scene, &TextureView, &RenderParams) -> vello::Result<()>` — synchronous, submits to the queue internally; target texture must be `Rgba8Unorm` + `STORAGE_BINDING` (we add `TEXTURE_BINDING` for compositing); `RenderParams { base_color, width, height, antialiasing_method }`; vello output is premultiplied RGBA. Queue ordering: `render_to_texture` submits its own work immediately; GPUI submits the frame encoder (with our composite pass) afterwards, so the offscreen texture is complete before compositing executes.

```rust
//! Zero-copy vello backend: render the scene to an offscreen texture with
//! the shared wgpu device, then alpha-composite into the GPUI frame.

use crate::vello2d::{ChartScene, to_vello_scene};
use gpui::{Bounds, CustomDraw, Pixels};
use gpui_wgpu::{WgpuContext, WgpuCustomDraw, WgpuCustomDrawAdapter};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use vello::peniko::Color;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions};

/// Shared scene handle + lazily-initialized GPU state.
pub struct WgpuVelloDraw {
    scene: Rc<RefCell<ChartScene>>,
    gpu: RefCell<Option<GpuState>>,
    /// Set when Renderer::new fails: never retry inside the paint loop.
    poisoned: Cell<bool>,
}

struct GpuState {
    renderer: Renderer,
    offscreen_view: Option<wgpu::TextureView>,
    size: [u32; 2],
    composite: Option<CompositePipeline>,
}

/// Bounds size (GPUI px) → physical texture size, clamped to >= 1.
pub fn physical_size(width: f32, height: f32, scale_factor: f32) -> [u32; 2] {
    [
        (width * scale_factor).max(1.0) as u32,
        (height * scale_factor).max(1.0) as u32,
    ]
}

impl WgpuVelloDraw {
    pub fn new(scene: Rc<RefCell<ChartScene>>) -> Self {
        Self {
            scene,
            gpu: RefCell::new(None),
            poisoned: Cell::new(false),
        }
    }

    pub fn into_custom_draw(self) -> Rc<dyn CustomDraw> {
        Rc::new(WgpuCustomDrawAdapter(Rc::new(self)))
    }
}

impl CustomDraw for WgpuVelloDraw {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WgpuCustomDraw for WgpuVelloDraw {
    fn draw_wgpu(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: [u32; 2],
        bounds: Bounds<Pixels>,
        scale_factor: f32,
    ) {
        if self.poisoned.get() {
            return;
        }
        let vello_scene = {
            let scene = self.scene.borrow();
            if scene.is_empty() {
                return;
            }
            to_vello_scene(&scene)
        }; // RefCell borrow released before GPU work

        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let size = physical_size(width, height, scale_factor);

        let mut gpu_slot = self.gpu.borrow_mut();
        if gpu_slot.is_none() {
            match Renderer::new(
                &ctx.device,
                RendererOptions {
                    antialiasing_support: AaSupport::area_only(),
                    ..Default::default()
                },
            ) {
                Ok(renderer) => {
                    *gpu_slot = Some(GpuState {
                        renderer,
                        offscreen_view: None,
                        size: [0, 0],
                        composite: None,
                    });
                }
                Err(err) => {
                    log::error!("vello2d: vello::Renderer::new failed: {err}");
                    self.poisoned.set(true);
                    return;
                }
            }
        }
        let Some(gpu) = gpu_slot.as_mut() else { return };

        if gpu.size != size {
            let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vello2d_offscreen"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            gpu.offscreen_view = Some(texture.create_view(&Default::default()));
            gpu.size = size;
        }
        if gpu.composite.is_none() {
            gpu.composite = Some(CompositePipeline::new(ctx, target_format));
        }
        if gpu.offscreen_view.is_none() || gpu.composite.is_none() {
            return;
        }

        // Disjoint field borrows: &mut gpu.renderer + &gpu.offscreen_view.
        if let Err(err) = gpu.renderer.render_to_texture(
            &ctx.device,
            &ctx.queue,
            &vello_scene,
            gpu.offscreen_view.as_ref().unwrap(),
            &RenderParams {
                base_color: Color::TRANSPARENT,
                width: size[0],
                height: size[1],
                antialiasing_method: AaConfig::Area,
            },
        ) {
            // Transient: log and leave the previous frame's content.
            log::error!("vello2d: render_to_texture failed: {err}");
            return;
        }

        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        gpu.composite.as_ref().unwrap().composite(
            ctx,
            encoder,
            target,
            gpu.offscreen_view.as_ref().unwrap(),
            [origin_x * scale_factor, origin_y * scale_factor],
            [size[0] as f32, size[1] as f32],
            [target_size[0] as f32, target_size[1] as f32],
        );
    }
}

// ---------------------------------------------------------------------------
// Composite: draw the premultiplied-RGBA offscreen texture over the frame.

struct CompositePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

const COMPOSITE_WGSL: &str = r#"
struct Uniforms {
    dst_origin: vec2<f32>,
    dst_size: vec2<f32>,
    target_size: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var positions = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let p = positions[i];
    let device_px = u.dst_origin + p * u.dst_size;
    let ndc = vec2<f32>(
        device_px.x / u.target_size.x * 2.0 - 1.0,
        1.0 - device_px.y / u.target_size.y * 2.0,
    );
    return VsOut(vec4<f32>(ndc, 0.0, 1.0), p);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
"#;

impl CompositePipeline {
    fn new(ctx: &WgpuContext, target_format: wgpu::TextureFormat) -> Self {
        let device = &ctx.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vello2d_composite"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_WGSL.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vello2d_composite_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vello2d_composite_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vello2d_composite_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    // vello output is premultiplied.
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vello2d_composite_uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vello2d_composite_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            pipeline,
            bind_group_layout,
            uniform,
            sampler,
        }
    }

    fn composite(
        &self,
        ctx: &WgpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        src: &wgpu::TextureView,
        dst_origin: [f32; 2],
        dst_size: [f32; 2],
        target_size: [f32; 2],
    ) {
        let uniforms: [f32; 8] = [
            dst_origin[0],
            dst_origin[1],
            dst_size[0],
            dst_size[1],
            target_size[0],
            target_size[1],
            0.0,
            0.0,
        ];
        ctx.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&uniforms));
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vello2d_composite_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vello2d_composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
```

Note: wgpu 29 API details (`entry_point: Some("vs")`, `cache: None`, `depth_slice`, `compilation_options`) follow `gpui-d3rs/src/mesh/gpu/wgpu_backend.rs` — if any field differs, that file is the in-repo reference for exact wgpu 29 idioms.

- [ ] **Step 3: Verify**

```bash
cargo test -p gpui-d3rs --no-default-features --features vello-gpui --test vello2d_element_tests
cargo check -p gpui-d3rs --features vello-gpui
cargo clippy -p gpui-d3rs --features vello-gpui
just wasm-check   # vello must also compile for the browser target
```

Expected: all pass. Visual GPU verification happens in Task 9 (needs a real window).

- [ ] **Step 4: Commit**

```bash
git add crates/gpui-d3rs/src/vello2d/wgpu_draw.rs crates/gpui-d3rs/src/vello2d/mod.rs crates/gpui-d3rs/tests/vello2d_element_tests.rs
git commit -m "feat(d3rs): zero-copy vello GPU backend via WgpuCustomDraw"
```

---

### Task 8: Port scatter chart to vello (d3rs + px toggle)

**Files:**
- Modify: `crates/gpui-d3rs/src/shape/scatter.rs` (existing `render_scatter` at :379-449, `compute_scatter_points` above it — read :1-120 first)
- Modify: `crates/gpui-px/src/scatter/scatter_chart.rs` (`ScatterChart` struct + builder + `build_plot_area!` at :810-847)
- Test: `crates/gpui-d3rs/tests/vello2d_cpu_tests.rs` (append golden tests)

**Interfaces:**
- Consumes: `ChartScene` (Task 2), `VelloChartElement`/`RasterBackend` (Task 6), existing `compute_scatter_points(x_scale, y_scale, data) -> Vec<DrawPoint>` with normalized `x_rel`/`y_rel` fields (verify exact names in scatter.rs).
- Produces:
  - `pub fn scatter_chart_scene<XS, YS>(x_scale: &XS, y_scale: &YS, data: &[ScatterPoint], config: &ScatterConfig, width: f32, height: f32) -> ChartScene` (feature `vello`)
  - `pub fn render_scatter_vello<XS, YS>(x_scale: &XS, y_scale: &YS, data: &[ScatterPoint], config: &ScatterConfig, backend: RasterBackend) -> impl IntoElement + use<XS, YS>` (feature `vello-gpui`)
  - px: `ScatterChart::raster_backend(mut self, backend: RasterBackend) -> Self` (feature `vello`); field `raster_backend: Option<RasterBackend>` defaulting to `None` = legacy GPUI path.

- [ ] **Step 1: Write the failing golden tests (append to tests/vello2d_cpu_tests.rs)**

```rust
use d3rs::prelude::*;
use d3rs::shape::{ScatterConfig, ScatterPoint, scatter_chart_scene};
use d3rs::vello2d::CpuRasterizer;

#[test]
fn scatter_scene_golden_pixels() {
    // Fixed 100x80 linear scales, 3 points — the QA oracle for the port.
    let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 100.0);
    let y_scale = LinearScale::new().domain(0.0, 80.0).range(80.0, 0.0);
    let data = vec![
        ScatterPoint::new(50.0, 40.0),
        ScatterPoint::new(10.0, 8.0),
        ScatterPoint::new(90.0, 72.0),
    ];
    let config = ScatterConfig::new()
        .fill_color(D3Color::from_hex(0xff0000))
        .point_radius(3.0);
    let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 100.0, 80.0);
    assert_eq!(scene.len(), 3, "one fill command per point");

    let mut rast = CpuRasterizer::new(100, 80);
    let buf = rast.rasterize(&scene, 100, 80);
    // (50,40) is the center point: opaque red within the radius.
    let i = (40 * 100 + 50) * 4;
    assert!(buf[i] > 200 && buf[i + 3] > 200, "center pixel: {:?}", &buf[i..i + 4]);
    // Far from all points: transparent.
    assert_eq!(buf[3], 0);
}

#[test]
fn scatter_scene_respects_opacity() {
    let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 20.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(20.0, 0.0);
    let data = vec![ScatterPoint::new(5.0, 5.0)];
    let config = ScatterConfig::new()
        .fill_color(D3Color::from_hex(0x00ff00))
        .point_radius(4.0)
        .opacity(0.5);
    let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 20.0, 20.0);
    let mut rast = CpuRasterizer::new(20, 20);
    let buf = rast.rasterize(&scene, 20, 20);
    let alpha = buf[(10 * 20 + 10) * 4 + 3];
    assert!((100..=160).contains(&alpha), "premultiplied alpha ~128, got {alpha}");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_cpu_tests scatter`
Expected: FAIL — `scatter_chart_scene` not found.

- [ ] **Step 3: Implement `scatter_chart_scene` + `render_scatter_vello` in shape/scatter.rs**

First check the cfg gates on `compute_scatter_points` and its point-struct field names (read scatter.rs :1-120). If `compute_scatter_points` is gated `#[cfg(all(feature = "gpui", not(test)))]`, widen its gate to `#[cfg(any(feature = "gpui", feature = "vello"))]` (keeping `not(test)` where present) so the vello path can use it in unit tests. Do NOT change `render_scatter`'s behavior.

Append to `crates/gpui-d3rs/src/shape/scatter.rs`:

```rust
/// Build a backend-neutral vello scene for a scatter series, in
/// element-local coordinates (0..width, 0..height).
#[cfg(feature = "vello")]
pub fn scatter_chart_scene<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
    width: f32,
    height: f32,
) -> crate::vello2d::ChartScene
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let points = compute_scatter_points(x_scale, y_scale, data);
    let mut rgba = config.fill_color.to_rgba();
    rgba.a *= config.opacity;
    let brush = crate::vello2d::peniko::Brush::Solid(crate::vello2d::peniko::Color::new([
        rgba.r, rgba.g, rgba.b, rgba.a,
    ]));
    let radius = config.point_radius as f64;
    let mut scene = crate::vello2d::ChartScene::new();
    for p in &points {
        scene.fill_circle(
            (p.x_rel * width) as f64,
            (p.y_rel * height) as f64,
            radius,
            brush.clone(),
        );
    }
    scene
}

/// Render a scatter series through the vello backend (GPU zero-copy where
/// the wgpu renderer is active, vello_cpu otherwise). The scene is rebuilt
/// from the actual paint bounds on resize.
#[cfg(feature = "vello-gpui")]
pub fn render_scatter_vello<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
    backend: crate::vello2d::RasterBackend,
) -> impl IntoElement + use<XS, YS>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let points = compute_scatter_points(x_scale, y_scale, data);
    let config = config.clone();
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        let mut rgba = config.fill_color.to_rgba();
        rgba.a *= config.opacity;
        let brush = crate::vello2d::peniko::Brush::Solid(crate::vello2d::peniko::Color::new([
            rgba.r, rgba.g, rgba.b, rgba.a,
        ]));
        let radius = config.point_radius as f64;
        let mut scene = crate::vello2d::ChartScene::new();
        for p in &points {
            scene.fill_circle(
                (p.x_rel * width) as f64,
                (p.y_rel * height) as f64,
                radius,
                brush.clone(),
            );
        }
        scene
    })
    .backend(backend)
    .absolute()
}
```

Two alignment notes:
- `render_scatter` (legacy) paints a stroke ring when `config.stroke_color` is set (scatter.rs:412-427): a stroked circle of radius `r + stroke_width/2` with width `stroke_width` per point, painted BEFORE the fill. Mirror that in both functions above: if `let Some(stroke_color) = config.stroke_color`, first emit `scene.stroke_path(Circle::new((cx, cy), r + w/2).to_path(0.1), kurbo::Stroke::new(w), stroke_brush)` per point. Use the real field/constructor names from the scatter config and `D3Color::to_rgba()` (read the color module; `to_rgba` returns an RGBA struct — mirror how render_scatter consumes it at :390-394).
- If `ScatterConfig` isn't `Clone`, capture the needed scalars (`fill_color`, `opacity`, `point_radius`, `stroke_color`, `stroke_width`) into the closure instead of cloning the config.

- [ ] **Step 4: Run golden tests**

Run: `cargo test -p gpui-d3rs --no-default-features --features vello --test vello2d_cpu_tests`
Expected: all PASS (2 new + 4 from Task 4).

- [ ] **Step 5: Add the px toggle**

In `crates/gpui-px/src/scatter/scatter_chart.rs`:

```rust
// struct field (with the other config fields, near `opacity` at :417):
#[cfg(feature = "vello")]
raster_backend: Option<d3rs::vello2d::RasterBackend>,

// wherever the other fields are initialized (ScatterChart constructor /
// Default — read the `pub fn scatter(...)` constructor at :1116 and the
// struct's `new`), initialize:
#[cfg(feature = "vello")]
raster_backend: None,

// builder method, next to `point_radius` (:411):
/// Rasterize markers through vello instead of GPUI paths.
#[cfg(feature = "vello")]
pub fn raster_backend(mut self, backend: d3rs::vello2d::RasterBackend) -> Self {
    self.raster_backend = Some(backend);
    self
}
```

In the `build_plot_area!` macro (:810-847), dispatch each series render. For both the additional-series loop (:828-835) and the primary series (:838-843), replace `plot_area.child(render_scatter(&$x_scale, &$y_scale, data, config))` with:

```rust
#[cfg(feature = "vello")]
let child: gpui::AnyElement = match self.raster_backend {
    Some(backend) => d3rs::shape::render_scatter_vello(&$x_scale, &$y_scale, data, config, backend)
        .into_any_element(),
    None => render_scatter(&$x_scale, &$y_scale, data, config).into_any_element(),
};
#[cfg(not(feature = "vello"))]
let child: gpui::AnyElement =
    render_scatter(&$x_scale, &$y_scale, data, config).into_any_element();
plot_area = plot_area.child(child);
```

(`render_scatter` returns `impl IntoElement`; both branches must materialize to `AnyElement` for the two `cfg` arms to type-check. Adjust names to the actual loop variables in the macro.)

- [ ] **Step 6: Verify px compiles both ways + existing tests stay green**

```bash
cargo check -p gpui-px
cargo check -p gpui-px --features vello
cargo test -p gpui-px
cargo clippy -p gpui-px --features vello
```

Expected: all pass; px unit tests (e.g. `test_scatter_builder_chain` at :1235) unchanged and green.

- [ ] **Step 7: Commit**

```bash
git add crates/gpui-d3rs/src/shape/scatter.rs crates/gpui-d3rs/tests/vello2d_cpu_tests.rs crates/gpui-px/src/scatter/scatter_chart.rs crates/gpui-px/Cargo.toml
git commit -m "feat(d3rs,px): scatter chart vello raster backend"
```

---

### Task 9: Showcase demo + visual QA (native + wasm)

**Files:**
- Modify: px showcase scatter section — find it: `grep -rn "scatter" crates/gpui-px/bin/ crates/gpui-px/src/lib/ | head -20`
- Create: `qa/visual/wasm/baselines/` entry produced by the record run below

**Interfaces:**
- Consumes: `ScatterChart::raster_backend` (Task 8).
- Produces: a "Scatter (vello)" showcase section; a recorded wasm baseline image.

- [ ] **Step 1: Add the showcase section**

Duplicate the existing scatter showcase section with:
- title suffix " (vello)"
- `.raster_backend(d3rs::vello2d::RasterBackend::Auto)` on the chart builder
- at least one series with ≥ 100k points, deterministic data (`x = i as f64 * 0.001`, `y = (i as f64 * 0.013).sin()` — no RNG)
- registered in the showcase nav/mod list the same way the original scatter section is (read the showcase's mod.rs).

Feature-gate the section so the showcase binary still builds without `vello` — check how existing optional sections (gpu2d heatmap/contour) are gated in the px showcase and mirror that pattern.

- [ ] **Step 2: Native visual verification**

```bash
cargo run -p gpui-px --bin px-showcase --features vello
```

Navigate to the vello scatter section. Expected: points render identically in shape/color/position to the legacy scatter section; resizing the window rescales the chart crisply (the `with_builder` path rebuilds the scene); no `vello2d:` errors in the log. This machine runs a wgpu renderer, so this exercises `WgpuVelloDraw` end to end.

- [ ] **Step 3: wasm verification + baseline record**

```bash
just wasm-serve-px   # serves on 127.0.0.1:8082; stop it after QA
just wasm-visual px-vello-scatter 8082 gpui-px record
```

(Recipe signature per root AGENTS.md: `just wasm-visual <name> <port> <pkg> [record] [click_x click_y]`. If the section needs a click to navigate, mirror the coordinates from the documented `just wasm-visual px-scatter 8082 gpui-px '' 80 137` example.)

Then confirm the diff passes:

```bash
just wasm-visual px-vello-scatter 8082 gpui-px
```

Expected: baseline recorded under `qa/visual/wasm/baselines/`; diff passes. If the vello section renders blank on wasm, that's the `cfg(not(wasm))` gap risk from the spec — capture the browser console output from the harness and report it; switch the showcase section to `RasterBackend::Cpu` to validate the rest of the pipeline, and treat wgpu-on-wasm dispatch as a follow-up fix.

- [ ] **Step 4: Commit**

```bash
git add crates/gpui-px qa/visual/wasm/baselines
git commit -m "feat(px): vello scatter showcase section + wasm visual baseline"
```

---

### Task 10: Perf benchmark + documentation

**Files:**
- Modify: `crates/gpui-d3rs/benches/vello2d_bench.rs`
- Create: `qa/perf/2026-08-16-vello2d-bench.md`
- Modify: `crates/gpui-d3rs/AGENTS.md` (features list :16-21, testing :34-44)
- Modify: `crates/gpui-px/AGENTS.md` (features :11-14)
- Modify: root `AGENTS.md` (gpui-d3rs row of the crate table)

**Interfaces:**
- Consumes: `scatter_chart_scene`, `CpuRasterizer` (Tasks 4, 8).
- Produces: committed benchmark results; docs current with the new features.

- [ ] **Step 1: Write the benchmark**

`crates/gpui-d3rs/benches/vello2d_bench.rs`:

```rust
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use d3rs::prelude::*;
use d3rs::shape::{ScatterConfig, ScatterPoint, scatter_chart_scene};
use d3rs::vello2d::CpuRasterizer;

fn scatter_data(n: usize) -> Vec<ScatterPoint> {
    (0..n)
        .map(|i| ScatterPoint::new(i as f64 * 0.01, (i as f64 * 0.017).sin() * 50.0 + 50.0))
        .collect()
}

fn bench_vello2d_scatter(c: &mut Criterion) {
    let mut group = c.benchmark_group("vello2d_scatter");
    for n in [100_000usize, 1_000_000] {
        let data = scatter_data(n);
        let x_scale = LinearScale::new()
            .domain(0.0, n as f64 * 0.01)
            .range(0.0, 800.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(600.0, 0.0);
        let config = ScatterConfig::new()
            .fill_color(D3Color::from_hex(0x1f77b4))
            .point_radius(2.0);

        group.bench_with_input(BenchmarkId::new("scene_build", n), &data, |b, data| {
            b.iter(|| scatter_chart_scene(&x_scale, &y_scale, data, &config, 800.0, 600.0))
        });

        let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 800.0, 600.0);
        let mut rast = CpuRasterizer::new(800, 600);
        group.bench_with_input(BenchmarkId::new("cpu_raster", n), &scene, |b, scene| {
            b.iter(|| rast.rasterize(scene, 800, 600))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_vello2d_scatter);
criterion_main!(benches);
```

Run: `cargo bench -p gpui-d3rs --no-default-features --features vello --bench vello2d_bench`
Expected: completes; record the printed times.

- [ ] **Step 2: Commit benchmark + results doc**

Write `qa/perf/2026-08-16-vello2d-bench.md` with the criterion output table (scene_build + cpu_raster at 100k and 1M points, this Mac), one paragraph of context: what the numbers measure, that GPU frame time is validated via the showcase (Task 9), and that the legacy `paint_path` comparison lands with the follow-up chart-port plan.

```bash
git add crates/gpui-d3rs/benches/vello2d_bench.rs qa/perf/2026-08-16-vello2d-bench.md
git commit -m "bench(d3rs): vello2d scatter scene-build and CPU raster benchmark"
```

- [ ] **Step 3: Update AGENTS.md files**

- `crates/gpui-d3rs/AGENTS.md`: add `vello` / `vello-gpui` to the Features list with one-line descriptions; add `cargo test -p gpui-d3rs --no-default-features --features vello --tests` to Testing.
- `crates/gpui-px/AGENTS.md`: add `vello` to Features.
- Root `AGENTS.md`: extend the gpui-d3rs row — mention vello-backed 2D rasterization (`vello2d` module, zero-copy via `WgpuCustomDraw`, vello_cpu fallback).

```bash
git add crates/gpui-d3rs/AGENTS.md crates/gpui-px/AGENTS.md AGENTS.md
git commit -m "docs: vello2d backend in crate agent guides"
```

- [ ] **Step 4: Final full verification**

```bash
cargo test -p gpui-d3rs --no-default-features --tests
cargo test -p gpui-d3rs --no-default-features --features vello --tests
cargo test -p gpui-d3rs --no-default-features --features vello-gpui --tests
cargo check -p gpui-px --features vello
just wasm-check
```

Expected: everything green.

---

## Self-Review Notes (already applied)

- Spec coverage: deps/unification (T1), scene layer (T2-3), CPU backend + oracle (T4), vendored patches (T5), element (T6), GPU backend (T7), scatter port + per-chart toggle (T8), wasm/visual QA (T9), perf + docs (T10). Spec error-handling rules are embedded in T6/T7 code (poison flag, log-and-skip). Remaining chart ports explicitly deferred to a follow-up plan (scope discipline).
- No placeholders: every code step carries full code. Documented compile-error-driven fallbacks exist only where an upstream API name couldn't be verified offline (`Encoding` field names in T3, `vello_cpu::peniko` re-export in T4, `D3Color::to_rgba` field names in T8).
- Type consistency: `ChartScene` / `ChartCmd` / `to_vello_scene(&ChartScene) -> vello::Scene` / `CpuRasterizer::rasterize(&ChartScene, u16, u16) -> Vec<u8>` / `VelloChartElement::{new, with_builder, backend, absolute}` / `WgpuVelloDraw::new(Rc<RefCell<ChartScene>>).into_custom_draw()` are used identically across tasks.
- Resolved during review: scene is built at paint-time bounds via `with_builder` (resize correctness); element keeps ownership of the current scene so the empty-check works for both backends; disjoint field borrows in `draw_wgpu` (renderer vs offscreen_view); `Frame`/`RenderImage` imported from `gpui`, not `image`; integration tests reach kurbo/peniko through `d3rs::vello2d` re-exports.
