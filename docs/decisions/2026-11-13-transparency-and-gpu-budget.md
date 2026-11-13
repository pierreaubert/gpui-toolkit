# Decision: transparency ordering and GPU budget scope

Date: 2026-11-13
Step: audit-20261113 §9 step 10 (closes the C4 transparency bullet and
the C5 total-budget bullet as documented approximations)

## Transparency: sorted blending retained; WBOIT not adopted

Evaluation summary:

- Current model (both backends): opaque pass is z-buffered; transparent
  actors render afterwards back-to-front (bounds-centre distance on the
  GPU, plus per-triangle depth sort within an actor on the CPU) with
  source-over blending and no depth writes. This is exact for
  non-intersecting transparent actors, which is the dominant scene
  shape for this toolkit (one translucent volume/isosurface over opaque
  geometry).
- Weighted-blended OIT would remove sorting artifacts for
  *intersecting* translucent surfaces, at the cost of: an extra
  accumulation + revealage target pair and a resolve pass on the GPU
  path, a second blending model to keep CPU/GPU coherent, color
  weighting that is only approximately order-independent (halos at
  depth discontinuities), and loss of the exact source-over result for
  the common non-intersecting case.
- The CPU rasterizer is the visual-regression reference; adding WBOIT
  only on GPU would fork backend semantics further (the audit's C4
  goal is parity), and adding it to both doubles the surface.

Decision: **do not adopt WBOIT.** Sorted source-over remains the model
on both backends, and the approximation is now stated identically in
both backends' documentation: intersecting translucent surfaces may
blend in the wrong order where they cross. Scenes that need exact
results there should split intersecting geometry at the intersection
or render one side opaque. Revisit only with a concrete product scene
that intersects translucent surfaces and a paired CPU/GPU prototype.

Evidence: `intersecting_translucent_quads_blend_in_sorted_order` in
`crates/gpui-scene3d/src/raster.rs` pins the documented behavior at the
crossing point.

## GPU budget: requested-resources scope retained

`GpuUploadLimits.max_resource_bytes` bounds the resources this crate
requests (geometry, primitives, material uniforms/LUTs), checked
before allocation including the still-live previous scene during
replacement. It explicitly does not bound pipelines, render targets,
CPU prepared data, or driver overhead, and is not a physical VRAM
limit (see `crates/gpui-scene3d/src/limits.rs`).

Decision: **keep this scope.** A true total-device budget cannot be
honored from inside one crate — the host owns targets, pipelines, and
other subsystems' allocations — and wgpu exposes no portable
device-memory accounting to enforce against. Hosts that need a total
budget compose it themselves: `GpuScene::resource_bytes()` reports
this crate's requested share, render targets report their dimensions,
and `PreparedScene::byte_len()` reports retained CPU geometry.
