# Code Review: gpui-python-runtime — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-python-runtime` (~48.5k LOC)

## 1. Purpose / role
Python-declared scene/mesh/UI bridge: `#![deny(unsafe_code)]` (`lib.rs:1`) boundary where Python owns declarations (ids, arrays, cameras, callbacks) and Rust owns validation/caching/GPUI adapters. Big files: `bin/showcase/python_ir_showcase.rs` (16778), `python_extension.rs` (9065), `ui_ir.rs` (5329), `dataset_frames.rs` (3642), `mesh_frames.rs` (1604), `native_mesh_plot.rs` (1785), `meshplot.rs` (1265), `gpui_adapter.rs` (1484), `session.rs` (1423), `spec_cache.rs` (630), `cache.rs` (531).

Public API: `RetainedSceneCache/CacheUpdate/DirtyResources`, `MeshFrame/MeshFrameStore/Outcome/RetainedMeshResource/MAX_*_BYTES`, `MeshPlotSpec/MESHPLOT_SPEC_SCHEMA_VERSION`, `scene3d::{SceneSpec,MeshSpec,SurfaceSpec,CameraSpec,…}`, `SessionState/HostMessage/PythonMessage/SessionError`, `spec_cache::{validate_scene3d_spec_schema_version,…}`, `ui_ir::{PythonAppIr,MeshPlotNode,UiIrError}`; `python-extension` feature gates pyo3 module (`lib.rs:29-30`).

## 2. SOTA gap analysis (vs PyO3/maturin, plotly.js, Streamlit, Jupyter widgets)
1. **No maturin/PyPI story** — abi3 extension private, no `pip install` wheel flow (`pyproject.toml:46`, `Cargo.toml:101`).
2. **No async/GIL-release discipline** — 9k-line `python_extension.rs` suggests GIL-held compute (`force_simulate`, `axis_layout`).
3. **No zero-copy buffers** (numpy `__array_interface__`/DLPack) — frames via JSON (`session.rs:799 from_slice`, `mesh_frames.rs:240 from_str`).
4. **No incremental streaming parity** — `apply_patch_op` exists (`ui_ir.rs:277`) but no backpressure/versioning.
5. **No error-span mapping** Python-line ↔ Rust validation (`UiIrError::UnknownNodeId`, `:340` carries id only).
6. **Unclear async callback execution model** (declared but not bridged).
7. **No benchmark/compat matrix** vs plotly/matplotlib.

## 3. Performance evaluation
Worst validators in review: `ui_ir::validate` 599 lines/cyclo 60/cogn 257/CRAP 3660 (`ui_ir.rs:2548`); `meshplot::validate` 294 lines/cyclo 49 (`meshplot.rs:160`) — both per spec update. `MeshPlotSpec::from_value(self.spec.clone())` clones entire spec JSON per validate (`ui_ir.rs:3508`); tests repeat `value.clone()` (`:3607,3612,3728`). `MeshFrameStore` getters clone full arrays on hit — `mesh/field/positions/triangles/values/mask/ids.clone()` (`mesh_frames.rs:589-739`) instead of `Rc/Arc`. `session.rs` `from_slice` per NDJSON line (`:799`) + `plot_id/resource_id.clone()` per patch (`:644-727`). `apply_patch_op` 198 lines/fan-out 23 (`ui_ir.rs:277`) with `property/value.clone()` inserts (`:295,440`).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `validate` per node-type; `#[derive]`-based schema to collapse branches | L | removes 3.6k CRAP |
| 2 | Validate by `&Value` — remove `spec.clone()` | S | spec-update allocs |
| 3 | Return `Arc<[f32]>`/handles from `MeshFrameStore`, not cloned arrays | M | frame-ingest cost |
| 4 | Bytes/Arrow IPC ingest alongside JSON for numpy parity | M | interop + speed |
| 5 | Criterion benches for validate + frame-ingest with budgets (reuse `gpui-profiler`) | S | regression gate |

## 5. Verdict
Largest bridge crate; correctness and JSON-clone discipline dominate. SOTA = wheels + zero-copy + patch streaming. Perf = validate-by-reference + Arc frames + benches.
