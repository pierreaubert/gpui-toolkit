# MeshPlot Metal reference evidence

- source revision: `15ce711`
- source tree: dirty during development capture; promote again from a clean release commit
- renderer: macOS Metal, 2× pixel scale
- local capture manifest: `target/qa/visual/component-lab-capture.json`
- local actual captures: 99 requested, 99 captured, 0 failed
- baseline archive: `qa/visual/baselines/component-lab-metal-pr-v1.tar.zst`
- diff report: `target/qa/visual/component-lab-diff.json`
- versioned baseline captures: 99
- cases: 99 compared, 0 failed
- visual threshold: 0 changed pixels

The 99 local actual captures are the complete MeshPlot component-lab story ×
viewport × scheme matrix: 11 MeshPlot stories across dashboard-wide,
panel-compact, and mobile-card layouts in dark, light, and high-contrast
themes. The versioned archive contains the same 99 reviewed `px-mesh-plot`
captures; the release-evidence gate checks both counts and requires the diff
report IDs to match the archive.

The baseline members were promoted from the corresponding successful Metal
captures and then compared by component-lab. No synthetic pixels or inferred
visual results are included.

## WGPU adapter-backed contract lane

The worktree provides `scripts/qa_mesh_wgpu_visual.sh`, backed by the
`gpui-d3rs` `mesh_wgpu_visual_capture` example. It captures five real retained
scenes—mesh-only, smooth scalar fill, wireframe, isoline, and axisymmetric
revolve—and records PNG dimensions, opaque-pixel counts, and RGBA checksums in
a manifest. `scripts/mesh_wgpu_manifest.py` is the shared validator used by
the developer lane and `scripts/qa_release_evidence.py`; it checks the exact
five case IDs, dimensions, safe repository-local paths, checksum format, and
baseline equality. A release lane must promote
`qa/visual/baselines/mesh-plot-wgpu-v1/manifest.json` from a clean revision and
run with `QA_WGPU_REQUIRED=1`; ordinary developer runs skip explicitly when no
headless adapter is available. In that case the lane writes an explicit
skipped manifest so ordinary `qa` evidence remains honest; the
`qa_release_evidence.py --require-clean` gate rejects skipped or missing WGPU
evidence. The current sandbox has no usable Metal/WGPU adapter, so it produced
that documented skip rather than a false baseline.
