# Bug Review: gpui-d3rs — 2026-08-25

Scope: full scan of `crates/gpui-d3rs/src` (~358 Rust files, ~88k lines), prioritizing the GPU paths (`gpu2d/`, `gpu3d/`, `mesh/gpu/`, `vello2d/`, `sphere_gallery/`, `surface/`), the threading primitives (`timer/`), and the core algorithms (`quadtree/`, `force/`, `lod.rs`, `contour/`, `delaunay/`, `scale/`). `Cargo.toml` feature wiring was cross-checked against `cfg` gates, and `cargo check -p gpui-d3rs --lib` was run to confirm the tree compiles (it does, warning-free in 14 s). Binaries under `bin/` were skimmed only where they inform library behavior. Line numbers are from this checkout.

## Findings

### High

1. **Blocking GPU→CPU readback per rendered frame on the UI thread — `Surface3DRenderer`**
   `src/gpu3d/renderer/surface3_drenderer.rs:647-656`: every `render_with_clear` call ends with `map_async` + `device.poll(PollType::Wait)` + `recv_timeout(5s)` — a full pipeline flush and stall on the calling (UI) thread — followed by a row-by-row CPU copy into a fresh `Vec<u8>` (`:663-668`). The caller, `Surface3DElement::paint_cached_surface` (`src/gpu3d/element/surface3_delement.rs:618-629`), wraps those pixels in a `RenderImage` and hands them to `window.paint_image`, which re-uploads them to the GPU. The cache key includes the full camera (`surface3_delement.rs:167-180`), so every camera-drag frame repeats the whole GPU→CPU→GPU cycle.
   Fix: render into a retained texture and composite it in-frame through the `WgpuCustomDraw` zero-copy path the crate already uses for `vello2d` (`src/vello2d/wgpu_draw.rs`) and `mesh/gpu/wgpu_backend.rs`; keep the readback only behind the QA/headless flag.

2. **Same readback cycle in `SphereGalleryRenderer`, plus per-render staging allocation**
   `src/sphere_gallery/renderer/sphere_gallery_renderer.rs:480-487`: unlike `Surface3DRenderer` (which retains its readback buffer), the gallery creates a fresh `MAP_READ` staging buffer on *every* `render()` call, then blocks on `poll(Wait)` + `recv_timeout(5s)` (`:514-525`). The paint cache key includes `hovered` (`src/sphere_gallery/element.rs:57-58`), so a simple mouse-move across cells invalidates the cache and triggers a full render + blocking readback + new `RenderImage` upload per pointer event.
   Fix: keep the rendered scene on the GPU (custom-draw composite), or at minimum retain the staging buffer and exclude `hovered` from the full-scene key by drawing the hover highlight as a separate GPUI overlay quad.

3. **`expect()` on GPU adapter/device creation — reachable panic without a GPU**
   `src/gpu3d/renderer/surface3_drenderer.rs:119` and `:131`, `src/sphere_gallery/renderer/sphere_gallery_renderer.rs:257` and `:269`: when the shared `Gpu2DContext` is unavailable (`Gpu2DContext::try_global()` returns `Err`, e.g. headless CI, VMs, RDP sessions), both renderers fall back to `pollster::block_on(create_device())` which panics on adapter or device failure. The 2D context itself went out of its way to be non-panicking (`src/gpu2d/device.rs:28-47`); the 3D renderers bypass that contract.
   Fix: make renderer construction fallible (`Option`/`Result`) and have the elements skip GPU painting (or fall back to the CPU `surface/` painter) when no device exists.

### High dispositions

1. **Interactive Surface3D/SphereGallery GPU→CPU→GPU paths (2026-08-26): Fixed.** Both live GPUI elements now register `WgpuCustomDraw` instances and record their retained offscreen 3D render plus premultiplied GPU composite into GPUI's frame encoder on the same device. `Surface3DRenderer::with_device` and `SphereGalleryRenderer::with_device` prevent cross-device resources; texture sampling replaces `RenderImage` construction, staging buffers, `map_async`, and `PollType::Wait` in interactive paint. Pixel-returning render/capture paths are gated to `headless-qa`; Surface3D preserves its render-size clamp. Verified `cargo check -p gpui-d3rs --features gpu-3d`, `cargo check -p gpui-d3rs --features gpu-3d,headless-qa`, and `cargo test -p gpui-d3rs --features gpu-3d --test surface3d_cache_tests` (6 passed).

3. **GPU adapter/device initialization panic (2026-08-26): Fixed.** Surface3D and SphereGallery renderer creation is now fallible: unavailable adapters or device requests return `None`, and their element paint paths skip GPU work instead of unwinding. The full `cargo test -p gpui-d3rs --lib` suite passes (1048 tests), along with `cargo check -p gpui-d3rs --lib`.

### Medium dispositions

4. **Mesh compute batching, upload reuse, and UI contention (2026-08-26): Fixed.** Contour segments and filled bands now batch all requested levels into one encoder submission and one staging-buffer map/readback, with separate output slices retaining level identity. GPU-resident values, projected positions, edge topology, and triangle topology are retained by content fingerprint, so level-only changes do not re-upload mesh inputs. Large live MeshPlot contours run through GPUI’s background executor; its render-adjacent caller uses `try_lock` and deterministically falls back to CPU contours under contention. The shared service also falls back to the CPU reference backend if adapter initialization is unavailable. Verified by `cargo check -p gpui-d3rs --features gpu-compute`, `cargo check -p gpui-px --features gpu-3d`, `cargo test -p gpui-d3rs --features gpu-compute --test mesh_compute_diff_tests` (10 passed), and the focused gpui-px CPU/GPU geometry differential test (1 passed).

### Medium

4. **Mesh compute: per-level blocking readbacks, full mesh re-upload per call, global mutex held across waits**
   `src/mesh/gpu/compute.rs`:
   - `marching_segments` (`:605-691`) and `band_triangles` (`:878-962`) loop over contour levels and, **per level**, submit, `poll(Wait)` up to 5 s, and `recv_timeout` up to 5 s. An N-level contour costs N full pipeline stalls.
   - Every call re-creates and re-uploads all geometry buffers (values/positions/edges, `:513-568`) even when only the level set changed.
   - `shared_mesh_compute()` (`:1004-1010`) guards the service with a process-wide `Mutex`, so any caller holds the lock across those 5-second-capable blocking waits, serializing (and potentially freezing) all other mesh users. Its `expect("mesh compute service is available")` also misrepresents `MeshCompute::try_new`, which is effectively infallible.
   Fix: batch all levels into one submission with per-level output offsets and a single readback; retain input buffers keyed by geometry revision (the crate already has `GeometryRevision` machinery); move the blocking map off the caller's thread or expose an async API.

5. **Dead legacy gpu2d renderers that reference a deleted module**
   `src/gpu2d/shapes/render.rs:1-1093` and `src/gpu2d/shapes/lod_scatter.rs:10`: the entire `#[cfg(not(feature = "vello-gpui"))]` half of the file imports `crate::gpu2d::element::Chart2DElement`, a module that no longer exists (`src/gpu2d/mod.rs:9-13` documents its removal). Because `gpu-2d` implies `vello-gpui` (`Cargo.toml:26`), this cfg branch is unreachable in every legal feature combination, so the ~1100 lines never compile-check. If anyone ever decouples the features, the build breaks immediately.
   Fix: delete the dead variants (the vello replacements already exist at `render.rs:1094-1330`) or restore a compilable legacy element.

6. **`ThresholdScale` / `QuantileScale` panic on NaN input data**
   `src/scale/threshold.rs:124` and `src/scale/quantile.rs:171`: `scale(NaN)` panics. D3 returns `undefined` for out-of-domain/NaN inputs, and NaNs are routine in real measurement data (this crate's own mesh code has an explicit NaN-validity-mask concept). A single bad sample takes down the whole app during render.
   Fix: clamp/return a designated default, or add `try_scale` and demote the panicking wrapper to `debug_assert`.

7. **Panicking infallible wrappers across the public API**
   Config-validation panics reachable from user input: `src/interpolate/piecewise.rs:30,70,112,163`, `src/interpolate/number.rs:304`, `src/scale/quantize.rs:127,155`, `src/scale/quantile.rs:167,203`, `src/scale/threshold.rs:120,158`, `src/chord/mod.rs:139`, `src/tile/mod.rs:225`, `src/legend/layout.rs:242`, `src/axis/layout.rs:153`, `src/text_layout.rs:234`, `src/grid/layout.rs:225,344`. Most have `try_*` counterparts, so the wrappers are deliberate, but a plotting library embedded in long-lived apps should prefer recoverable errors at these boundaries (the workspace's own convention elsewhere — `try_compute`, `try_tiles`, `try_from_text` — shows the intent).
   Fix: deprecate the panicking wrappers in favor of the `try_*` forms, or `debug_assert` + graceful fallback.

8. **vello2d wgpu path re-rasterizes the scene every frame**
   `src/vello2d/wgpu_draw.rs:279-297`: the encoded `vello::Scene` is correctly retained by revision (`:252-271`), but `Renderer::render_to_texture` runs unconditionally on every `draw_wgpu` call, re-rasterizing an unchanged scene into the offscreen texture each frame. The composite bind group is also re-created every draw (`:505-522`) although the source view is stable per size.
   Fix: skip `render_to_texture` when neither the encoded scene nor the offscreen size changed, and cache the bind group alongside the offscreen view.

### Medium dispositions

7. **Public panicking convenience wrappers (2026-08-26): Not a separate defect.** The cited entry points deliberately pair clearly named panic-on-invalid-input convenience methods (`compute`, `tiles`, `from_text`, etc.) with documented, tested `try_*`/`Result` APIs that preserve structured validation errors. The public module docs for chord explicitly state this contract, and invalid-input tests exercise the fallible forms. No broad behavior change is warranted without a semver/API decision.

5. **Legacy non-Vello GPU2D branch (2026-08-26): Not a reachable product bug.** `gpu2d` is compiled only with the `gpu-2d` feature, and that feature unconditionally enables `vello-gpui`; therefore every supported GPU2D build selects the Vello path. `cargo check -p gpui-d3rs --no-default-features --features gpu-2d` confirms the minimal supported closure compiles. The stale branch remains a cleanup/maintainability task, not a runtime or compilation regression in a supported configuration.

8. **Vello custom-draw steady-state rerasterization (2026-08-26): Fixed.** `render_to_texture` now runs only when the scene revision, physical texture size, or logical size changes; unchanged draws only composite the retained offscreen texture. The composite bind group is retained across frames and invalidated only when the source texture is reallocated. `cargo check -p gpui-d3rs` passes.

6. **Threshold/quantile NaN panics (2026-08-26): Fixed.** Both scales now route `NaN` to their first range value, the same low/out-of-domain bucket returned for finite values below the first threshold, rather than panicking in an interactive render path. Focused fallback tests pass; threshold (12) and quantile (17) groups pass.

### Low

### Low dispositions

9. **Surface3D per-paint data hashing (2026-08-26): Fixed.** `Surface3DElement` now owns a data revision advanced only by `set_data`, which also clears all dependent caches. The surface cache key hashes that revision instead of every x/y/z value and range, so ordinary paint calls are independent of grid resolution while data replacement remains invalidating. `cargo test -p gpui-d3rs --features gpu-3d --test surface3d_cache_tests` passes (6 tests).

12. **Timer UI-dispatcher `join` deadlock (2026-08-26): Fixed.** `Timer`, `Interval`, and `Timeout` now expose bounded `try_join(Duration)`, which returns completion status and prevents an unbounded UI-queue self-deadlock. The legacy blocking `join` documentation explicitly directs UI-dispatcher callers to the bounded API. `try_join_times_out_without_blocking` passes.

11. **Metal interleaved-value buffer capacity (2026-08-26): Fixed.** The 2D scalar-only update path now refuses an upload whose index count differs from the retained Metal vertex-buffer allocation before it performs any pointer writes. This closes the potential out-of-bounds write if retained scene state is replaced independently of its Metal resource generation. `cargo check -p gpui-d3rs --features gpu-metal` passes.

13. **Offscreen QA readback copy (2026-08-26): Fixed.** The QA-only WGPU readback now copies each padded source row in one slice operation, then performs an in-place BGRA→RGBA swizzle only when needed. It preserves the unsupported-format cleanup path and eliminates thousands of four-byte slice copies per image. `cargo test -p gpui-d3rs --features gpu-compute --test mesh_wgpu_embedded_viewport offscreen_wgpu_readback_contains_retained_scalar_mesh` passes.

14. **GPU contour level re-mapping (2026-08-26): Fixed.** The adapter contour path now tags every decoded segment with the input level index while it is generated. The projected public API restores its caller-supplied `f64` level with that index, removing the previous O(levels × segments) f32/f64 equality scan and retaining exact public level labels. `cargo test -p gpui-d3rs --features gpu-compute --test mesh_compute_diff_tests` passes (10 tests).

15. **`ContourGenerator` clone scratch sharing (2026-08-26): Fixed.** `Clone` now creates fresh `RefCell`-backed scratch buffers rather than sharing the source generator's `Rc`s, so contour extraction on a clone cannot trigger a re-entrant mutable-borrow panic in the original. The focused `clone_owns_independent_scratch_buffers` regression test passes, as does `cargo fmt --check -p gpui-d3rs`.

9. **Per-paint O(data) cache-key hashing in the 3D surface element**
   `src/gpu3d/element/surface3_delement.rs:182-202`: `compute_surface_cache_key` hashes every x/y/z datum, and it runs on *every* paint (cache hit or miss) via `paint_cached_surface` (`:582`) and again inside `compute_geometry_cache_key` (`:234`). For a 361×181 spinorama grid that's ~65k `f64` hashes per frame per element.
   Fix: hash a data-generation counter bumped by `set_data`, plus the cheap config/camera fields.

10. **Text atlas permanently drops glyphs once full**
    `src/gpu2d/text/atlas.rs:198-201`: when the shelf-packed atlas fills, `get_glyph` returns `None` forever — no eviction, no growth, no second atlas. Text silently goes missing on long-running dashboards with many sizes/codepoints. (Note: the row-advance at `:193` also doesn't add the 1 px padding used horizontally at `:246`, harmless but inconsistent.)
    Fix: evict LRU glyphs, grow/repack, or fall back to GPUI text for misses.

11. **`MetalResources::update_values` 2D path writes without a capacity check**
    `src/mesh/gpu/metal_backend.rs:514-540`: the non-3D branch writes `(*contents.add(offset)).value` for every index in the *current* upload; unlike the 3D branch (`:498-500`) it never checks `offset` against the allocated vertex count. If a field write arrives whose upload has more vertices than the buffer was allocated for, this is a heap write past the buffer end. I did not trace whether callers guarantee same-geometry writes — confirming that invariant (or asserting it in the `debug_assert` sense) would close this. Related: the file uses `unsafe` in a crate the workspace QA policy treats as portable; it is `gpu-metal`-gated, but the exemption is worth stating explicitly in `scripts/qa_unsafe_policy.py` terms.

12. **Timer scheduler: slow callbacks stall all timers; `join` can deadlock the UI**
    `src/timer/mod.rs:207-222`: without a UI dispatcher, callbacks execute on the single scheduler thread, so one slow callback delays every timer. With a dispatcher installed, `Timer::join` (`:387-395`) blocks the calling thread on a condvar whose signal requires the callback to run on the UI queue — calling `join` from the UI thread deadlocks.
    Fix: document both constraints, and make `join` panic-free but non-blocking-aware (e.g. `try_join` with timeout).

13. **Minor: per-pixel readback copy in offscreen QA path**
    `src/mesh/gpu/offscreen.rs:427-443`: 4-byte `copy_from_slice` per pixel instead of row-wise copies; QA-only, but trivially improvable with `extend_from_slice` per row plus a BGRA swizzle pass.

14. **Minor: O(levels × segments) float-equality level re-mapping**
    `src/mesh/gpu/compute.rs:1197-1205`: adapter segments are produced in level order, yet each segment's f64 level is recovered by a linear scan with `==` on f32-converted values. Correct today (levels are appended in order), but O(n²) and fragile; track the current level index while iterating instead.

15. **Minor: `ContourGenerator` `Clone` shares `RefCell` scratch buffers**
    `src/contour/marching_squares/contour_generator.rs:58-64`: `visited_cache`, `upsampled_buf`, `band_points`, `band_crossings` are `Rc<RefCell<_>>`, so `#[derive(Clone)]` shares mutable scratch state between clones. Sequential use is fine, but any re-entrant use (a contour computed inside a callback triggered by another contour) panics on the double borrow.
    Fix: implement `Clone` manually with fresh buffers, or document the shared-scratch semantics.

## GPU/CPU data-flow notes

Three independent GPU→CPU→GPU cycles exist:

- **3D surface (finding 1)**: wgpu renders to `Rgba8Unorm` texture → `copy_texture_to_buffer` → blocking `map_async` → CPU row-copy → `RenderImage` → GPUI sprite-atlas re-upload. Everything needed to composite already lives on the GPU; the crate's own `WgpuCustomDraw` composite shader (`src/vello2d/wgpu_draw.rs:333-372`) is the template: keep the surface texture, draw it with a 6-vertex blit, and only read back for QA screenshots (`headless-qa` feature).
- **Sphere gallery (finding 2)**: same cycle, plus a fresh staging buffer per render and hover-invalidate-per-mousemove. Highlight state (`selected_index`/`hovered_index`) is already shader uniforms — only the camera genuinely requires a re-render, and compositing on-GPU removes the readback entirely.
- **Mesh compute contours (finding 4)**: data starts on the CPU and the results (`IsolineSegment`/`ContourBand`) are consumed by retained CPU caches and SVG export, so a readback is inherent to the current design — but it should be *one* batched submission + async map for all levels, not N blocking round trips, and geometry buffers should be retained across level-set changes.

The retained-mesh wgpu backend (`src/mesh/gpu/wgpu_backend.rs`) and the vello2d custom draw are the good in-crate references: revision-gated uploads, no per-frame buffer/texture churn, no readbacks.

## UI/UX consistency

- The 3D elements hardcode typography (`font_size = 8.0/9.0/10.0`, `surface3_delement.rs:871,1251,1287`) and colors via ad-hoc config fields rather than `gpui-design` tokens; sibling GPUI components in `gpui-ui-kit` go through the design system. Acceptable for a low-level viz crate, but charts embedded in themed apps will not follow platform spacing/type ramps.
- `SphereGalleryState::move_selection` (`src/sphere_gallery/element.rs:158-170`) gives keyboard grid navigation, but the elements themselves expose no focus ring, focus handle, or ARIA metadata at paint time; sibling components in `gpui-ui-kit` carry ARIA roles. Worth a follow-up if these elements are used in product surfaces rather than demos.
- The two label conventions in `surface3_delement.rs` (`x_ticks` doubles as frequency ticks in Cartesian mode and elevation ticks in Spherical mode, `:346-359` vs `:1158-1164`) are consistent only because the host resets ticks per plot type; a doc comment on `SurfaceData::x_ticks` stating this dual role would prevent a subtle mislabeling bug.

## Clean bill

- `quadtree` (insert/remove/find with ordered quadrant pruning), `lod.rs` (M4 and the count-conserving `DensityPyramid`), `delaunay`, and the BVH (`mesh/bvh.rs`, iterative fixed-stack ray cast) read as correct and allocation-aware.
- `mesh/gpu/wgpu_backend.rs` / `retained.rs` implement proper revision-gated buffer reuse (`replace_retained_field` reuses capacity); the vello2d CPU rasterizer caches by (revision, size, scale) and releases stale sprite-atlas images.
- `timer/mod.rs`'s single-scheduler-thread design, poison-tolerant locks, and `catch_unwind` around callbacks are sound apart from the notes in finding 12.
- No `unsafe` outside the `gpu-metal`-gated backend; `cargo check -p gpui-d3rs --lib` is warning-free.

## Resolution status

- [x] 10. **Text atlas exhaustion** (2026-08-26): the single atlas now doubles and deterministically repacks cached glyphs up to the adapter's maximum 2D texture size. A failed maximum-size repack restores the previous texture, bind group, and glyph cache rather than dropping existing text. Verified by `cargo test -p gpui-d3rs --test text_atlas_growth` (1 passed).

## Follow-up regression evidence

- Vello retained-scene invalidation is now checked to rerasterize only when the scene or extent changes. The Metal interleaved-field upload path also rejects a changed vertex topology before reusing buffer capacity. Verified with the focused Vello test and `cargo test -p gpui-d3rs --features gpu-metal --lib interleaved_field_upload_rejects_a_changed_vertex_topology`; `cargo check -p gpui-d3rs --features gpu-metal` passed.
