# MeshPlot screen-reader QA walkthrough

Status: manual-required. This checklist is a runbook, not evidence that an
OS screen reader has executed.

Run the native MeshPlot host on each supported desktop lane with a scalar
field active and record the screen-reader output plus the source revision:

1. Enable VoiceOver (macOS), Narrator (Windows), or Orca/AT-SPI (Linux).
2. Navigate to the plot with keyboard focus only. Confirm one image/graphic
   node is exposed with the stable ID `mesh-plot-<mesh-id>`.
3. Confirm the accessible name contains the plot title and the description
   identifies the view, mesh vertex/triangle counts, field label/unit,
   association, finite displayed range, and available controls.
4. Confirm the value text reports the active scalar range and selection state;
   select a known cell or vertex and verify its stable external ID and value
   are announced without changing the camera or clearing the field.
5. Exercise keyboard pan, zoom, fit, reset, view selection, and export. Verify
   focus remains on the plot/toolbar and the summary updates after selection.
6. Repeat in dark, light, high-contrast, compact-panel, and mobile-card
   layouts. Record any clipping, missing labels, or focus-order changes.

Required evidence for closing the gate: OS name/version, host revision,
screen-reader version, a captured accessibility-tree/bridge snapshot, the
spoken-output transcript or equivalent event log, and a pass/fail result for
each step above. Until those artifacts are attached, capability status must
remain `Partial`.
