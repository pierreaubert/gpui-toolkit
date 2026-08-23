# Perf review: gpui-python-runtime

Date: 2026-08-22

## Role and hot paths

`gpui-python-runtime` is the host side of the Python↔GPUI bridge. There is **no
pyo3**: Python runs as a child process and talks newline-delimited JSON over
stdin/stdout (`src/session.rs`), with raw binary frames for bulk audio/mesh
arrays (`src/audio_stream.rs`, `src/mesh_frames.rs`). The crate owns:

- Session protocol: revisioned patches, mesh generations, job registry
  (`src/session.rs`, message pump in `bin/showcase/python.rs:289-400`).
- UI IR + patch application (`src/ui_ir.rs` — `PythonAppIr::apply_patch_ops`).
- Retained caches: spec parse cache (`src/spec_cache.rs`), dirty-domain
  fingerprinting (`src/cache.rs`), GPUI element cache (`src/gpui_adapter.rs`).
- Native MeshPlot translation (`src/native_mesh_plot.rs`, `src/meshplot.rs`).
- The showcase host binary (`bin/showcase/python_ir_showcase.rs`, 13k lines)
  whose `Render::render` (`python_ir_showcase.rs:13053`) drains the session and
  rebuilds the entire element tree on every `cx.notify()`.

Hot paths are therefore: per-patch IR mutation, per-render IR→element rebuild,
per-render mesh/scene decode + fingerprinting, and per-message JSON
marshalling. No GPU code lives in this crate itself (verified: no `map_async`,
`device.poll`, `pollster`, `read_texture`, or `paint_image` anywhere under
`src/` or `bin/showcase/`); roundtrip exposure is inherited from the d3rs/px
renderers it instantiates.

## Findings

1. **[Alloc] Full-IR serialize→mutate→deserialize round trip per patch.**
   `PythonAppIr::apply_patch_ops` (`src/ui_ir.rs:161-172`) does
   `serde_json::to_value(&*self)`, applies ops to the JSON tree, then
   `serde_json::from_value` + full `validate()` — O(entire app IR) for even a
   one-property `Set`. The host then makes it worse: `apply_patch_message`
   (`bin/showcase/python_ir_showcase.rs:12745-12776`) clones the whole
   `PythonAppIr` (:12758) and serializes the result to JSON **two more times**
   (:12766, :12776) for mesh-resource validation. Per accepted patch: 1 deep
   clone + 3 full-tree serializations + 1 full deserialize + 1 full validate.
   Impact: high for chatty UIs or IRs embedding inline mesh geometry; every
   patch also allocates fresh `HashMap`/`String`s in
   `SessionState::apply_patch_revision` (`src/session.rs:504,577`).

2. **[Alloc] MeshPlot fully decoded and rebuilt on every render.**
   `render_meshplot` (`python_ir_showcase.rs:10612`) runs per render:
   `MeshPlotSpec::from_value(node.spec.clone())` deep-clones + deserializes the
   spec (:10619); `decode_mesh_geometry`/`decode_mesh_field` re-convert the
   retained binary resource into `Vec<f64>`/`Vec<[f64;3]>` every frame
   (:10646-10656; `src/native_mesh_plot.rs:468-502,575-658` — element-wise
   `chunks_exact` + `collect`, plus an f32→f64 widening that doubles memory vs
   the wire format); `build_native_mesh_plot` then re-projects **all**
   positions for bounds even when a retained state is reused
   (`src/native_mesh_plot.rs:881-902`), and rebuilds the `TriangleMesh` +
   `gpui_px::mesh_plot` builder (:817-976). Change detection itself is a deep
   `serde_json::Value` compare of the geometry (`python_ir_showcase.rs:10681`).
   Impact: high — O(mesh size) alloc + decode per frame for any visible plot.

3. **[Alloc] Spec-cache fingerprint allocates one `String` per JSON number, per
   render.** `TypedSpecCache` re-fingerprints the node payload on every
   `parse_*` call to detect changes (`src/spec_cache.rs:149-161`), and
   `hash_value` does `Value::Number(value) => value.to_string().hash(hasher)`
   (`src/spec_cache.rs:271`), plus a `Vec` collect + sort per object
   (:280-281). Called per render from `render_surface_spec`/`render_lines_spec`/
   `render_mesh_summary`/`render_scene_summary` (`python_ir_showcase.rs:10875,
   10919, 10950, 10995`). A 200×200 surface grid = 40k `String` allocs per
   frame. Impact: high for Scene3D-heavy apps. Fix is trivial (hash
   `to_bits`).

4. **[Alloc] MeshPlot dirty-domain fingerprint materializes geometry as JSON
   text.** `mesh_plot_fingerprints` builds `serde_json::json!` trees containing
   the cloned geometry/field values, then `json_fingerprint` stringifies the
   whole thing via `value.to_string()` before hashing
   (`src/cache.rs:222-265`). Runs on every `GpuiMeshPlotCache::upsert`
   (`src/gpui_adapter.rs:60`) — i.e. per render after finding 2. For inline
   geometry this allocates a full JSON string of the mesh per frame. Impact:
   medium-high (overlaps finding 2; resource-backed plots keep specs small).

5. **[GPU] Scene3D mesh/surface paths rasterize on the CPU with per-triangle
   allocations instead of the d3rs GPU mesh path.** `Gpui3DCache::mesh_element`
   and `scene_element` convert meshes/surfaces into projected `Polygon3D` lists
   for the 2D line renderer (`src/gpui_adapter.rs:160-202,370-452`): a fresh
   `Vec` per triangle for `vertices` (:177-181, :421-427), another per-triangle
   temp `Vec` just to compute lighting (:431-433), CPU Lambert lighting per
   triangle (`scene_lit_color`, :520-539), and CPU triangulation + colormap for
   surfaces in scenes (`surface_polygons`, :471-518) — while standalone
   surfaces use the GPU `Surface3DElement` (:124) and MeshPlot has a
   `Surface3d` GPU view (`src/native_mesh_plot.rs:858,869`). Impact: high for
   large composed scenes; also rebuild-instead-of-update (element replaced on
   any geometry/material change, :200, :225-227). Moving composed-scene meshes
   to the d3rs `gpu-3d` mesh pipeline removes both the CPU rasterization and
   the per-triangle churn. Effort: L.

6. **[Alloc] Whole-section deep clone + full-tree element rebuild per render.**
   `render_content` clones the active section's entire node tree
   (`python_ir_showcase.rs:6764`) and `render` rebuilds sidebar + content +
   overlays on every `cx.notify()` (:13053-13154). Chart builders are
   re-created from scratch each render — e.g. heatmap re-collects the full `z`
   grid into a new `Vec` (:10092-10100), scatter/line re-clone titles/labels
   (:9943-10010) — and per-node `ElementId::Name(format!(...))` strings are
   allocated per frame (:10577). Impact: medium-high under chatty sessions
   (jobs, patches, meter frames all funnel through `drain_session` → notify).
   Mitigation would need structural sharing in the IR or memoized subtrees —
   effort L; the section-content clone alone is an easy removal (S).

7. **[Roundtrip] Mesh/audio bulk transport is already binary — good — but frame
   headers are parsed twice.** `read_python_messages` parses each line into
   `PythonMessage` (`bin/showcase/python.rs:323`), then for mesh frames parses
   the same line again as `serde_json::Value` just to read `byte_length`
   (:354-358). Also a fresh `Vec` per line (:295) and a zero-fill +
   `read_exact` overwrite for payloads (:341, :371). Impact: low (per mesh
   frame, not per frame of video), but trivially fixable.

8. **[Alloc] gpui-pretext paths are command-scope, not per-frame — low
   priority.** The pretext usage near lines 4449/11541 is the bridge's
   `text.rich` / `text.prepare_layout` / `text.reports` command handlers
   (`python_ir_showcase.rs:4447-4452, 11541-11570`). `FixedTextMeasure::
   measure_width` does `text.chars().count()` (O(n) per call) and report
   handlers rebuild large `serde_json::json!` trees per command, but these run
   per explicit Python command, not per frame. (needs profiling) Only matters
   if an app calls layout commands in a loop.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Cache decoded `TriangleMesh`/field `Arc`s in `MeshFrameStore` or the mesh-plot cache, keyed by (resource_id, generation); skip decode + bounds recompute when unchanged | 2 | M | High |
| 2 | Replace `hash_value`'s per-number `to_string()` with `f64::to_bits`/integer hashing; avoid the key-`Vec` when object order is already stable | 3 | S | High |
| 3 | Apply patches directly on typed IR (or keep the JSON tree as the canonical store) instead of `to_value`→patch→`from_value`; serialize once for resource validation, reuse for refs | 1 | M | High |
| 4 | Fingerprint mesh-plot specs structurally (hash fields/arrays directly) instead of `json!` clone + `to_string` | 4 | S | Medium |
| 5 | Route composed-scene meshes/surfaces through d3rs `gpu-3d` instead of CPU `Polygon3D` projection; pre-size vertex buffers, no per-triangle `Vec` | 5 | L | High |
| 6 | Drop the per-render `section.content.clone()` (borrow or `Rc` the tree); intern per-node `ElementId` strings | 6 | S | Medium |
| 7 | Parse mesh-frame headers once (decode `byte_length` from the typed header struct); reuse line buffer | 7 | S | Low |

## Quick wins

- `src/spec_cache.rs:271`: hash `Number` via bits instead of `to_string()` —
  one-line change, kills the largest per-frame alloc source for Scene3D.
- `src/cache.rs:261-265`: stream-hash the `Value` (as in `spec_cache.rs`) instead
  of `value.to_string()` — removes the full-geometry string per upsert.
- `python_ir_showcase.rs:6764`: remove the per-render deep clone of the section
  content.
- `bin/showcase/python.rs:354`: reuse the already-parsed header for
  `byte_length`; hoist the `Vec` line buffer out of the loop (:295).
- `src/gpui_adapter.rs:421-437`: pre-allocate polygon vertex storage with
  `Vec::with_capacity(3 * triangle_count)`-style flattening instead of a `Vec`
  per triangle (stopgap until finding 5's GPU move).
