# MeshPlot Metal reference evidence

- source revision: `15ce711`
- source tree: dirty during development capture; promote again from a clean release commit
- renderer: macOS Metal, 2× pixel scale
- local capture manifest: `target/qa/visual/component-lab-capture.json`
- local actual captures: 99 requested, 99 captured, 0 failed
- baseline archive: `qa/visual/baselines/component-lab-metal-pr-v1.tar.zst`
- diff report: `target/qa/visual/component-lab-diff.json`
- versioned baseline captures: 99
- cases: 99 compared, 72 zero-diff, 27 changed against the stale archive
- visual threshold: 0 changed pixels

The 99 local actual captures are the complete MeshPlot component-lab story ×
viewport × scheme matrix: 11 MeshPlot stories across dashboard-wide,
panel-compact, and mobile-card layouts in dark, light, and high-contrast
themes. The versioned archive contains the same 99 reviewed `px-mesh-plot`
captures; the release-evidence gate checks both counts and requires the diff
report IDs to match the archive. The current archive predates the selected
overlay, revolved-cylinder, filled-surface, and orientation-triad changes:
27 cases therefore differ. The developer evidence lane records this complete
but stale comparison; strict release evidence still requires a regenerated
zero-diff archive from one clean source revision.

The baseline members were promoted from the corresponding successful Metal
captures and then compared by component-lab. No synthetic pixels or inferred
visual results are included.

## WGPU adapter-backed contract lane

The worktree provides `scripts/qa_mesh_wgpu_visual.sh`, backed by the
`gpui-d3rs` `mesh_wgpu_visual_capture` example. It captures six real retained
scenes—mesh-only, smooth scalar fill, flat cell fill, wireframe, isoline, and
axisymmetric revolve—and records PNG dimensions, opaque-pixel counts, and RGBA
checksums in a manifest. `scripts/mesh_wgpu_manifest.py` is the shared
validator used by the developer lane and `scripts/qa_release_evidence.py`; it
checks the exact six case IDs, dimensions, safe repository-local paths,
canonical `comparison_id` values, checksum format, and baseline equality. A
release lane must promote
`qa/visual/baselines/mesh-plot-wgpu-v1/manifest.json` from a clean revision and
run with `QA_WGPU_REQUIRED=1`; ordinary developer runs skip explicitly when no
headless adapter is available. In that case the lane writes an explicit
skipped manifest so ordinary `qa` evidence remains honest; the
`qa_release_evidence.py --require-clean` gate rejects skipped or missing WGPU
evidence. The restricted sandbox still has no usable Metal/WGPU adapter and
produces that documented skip; the compatible reference-host lane has now
captured and validated the six-case WGPU manifest and promoted baseline.

## Cross-adapter comparison contract

`scripts/mesh_plot_visual_compare.py` compares two capture manifests using
canonical `comparison_id` values. It accepts both the WGPU `id`/`path` schema and the
component-lab `capture_id`/`actual_path` schema, decodes repository-produced
8-bit RGB/RGBA PNGs without third-party dependencies, and emits a deterministic
per-case JSON report. Use zero thresholds for exact parity; for a reference
adapter lane that has known antialiasing differences, pass an explicit
`--max-channel-delta` and `--max-changed-fraction`. The comparator and its
malformed-input, exact-match, tolerance-boundary, case-set, schema, and dimension tests
are covered by `scripts/tests/test_mesh_plot_visual_compare.py`.

The reference-host paired Metal/WGPU capture now compares all six canonical
scenes with zero changed pixels and zero failed cases. The broader camera,
displayed-range, axes, selection, and masked-data matrix remains a separate
follow-up beyond this canonical report.

The high-level product capture lane separately persists axes-composition and
selected-annotation cases for both adapters. Its manifest assigns
`px.mesh_plot.product.axes` and `px.mesh_plot.product.selection` comparison IDs
and records finite paired changed-pixel metrics. Those metrics prove that the
same product tree was captured on both adapters; they intentionally do not
claim exact text-atlas pixel parity. Strict release use still requires the
captured product manifest to be clean and source-bound.

The current paired report is written to
`target/qa/visual/mesh-plot-cross-adapter.json`; the release wrapper validates
its six canonical IDs, distinct renderer labels, repository-local image paths,
and zero-failure result.

The manual color-vision review is documented in
[`qa/visual/mesh-plot-cvd-qa.md`](mesh-plot-cvd-qa.md). Automated named-scale
simulation is a regression screen; it does not close the manual rendered-
stimulus review.

When product PNGs are available, `scripts/qa_mesh_cvd.sh` writes
`target/qa/visual/mesh-plot-cvd.json` with protan, deutan, and tritan
transforms of all four paired product cases. It checks that the selected
annotation remains distinguishable from the plain state for both adapters;
strict release evidence additionally requires the report to be captured and
source-bound. A no-adapter developer run writes an explicit skip.

The release wrapper `scripts/qa_mesh_cross_adapter_visual.sh` consumes the two
paired manifests, writes `target/qa/visual/mesh-plot-cross-adapter.json`, and
revalidates the report with the strict release-evidence schema. It skips when
the pair is unavailable during developer QA; set
`QA_CROSS_ADAPTER_REQUIRED=1` for a release lane, with
`QA_MESH_METAL_MANIFEST` and `QA_MESH_WGPU_MANIFEST` pointing at the captured
six-case manifests. It never promotes a skipped or synthetic comparison.
