# MeshPlot color-vision-deficiency QA walkthrough

Status: manual-required. The automated color-scale simulation is a regression
screen; this checklist is the acceptance record for rendered MeshPlot stimuli.

Run the native GPUI MeshPlot host on a compatible reference desktop with the
same source revision as the visual evidence. Review each item in light, dark,
high-contrast, compact-panel, and mobile-card layouts:

1. Review named scalar scales and confirm adjacent low/high values remain
   distinguishable under protanopia, deuteranopia, and tritanopia. Do not rely
   on red-versus-green contrast alone.
2. Review masked and non-finite regions. Confirm the mask is visible through
   the documented missing-value treatment and is not confused with a valid
   scalar value or a background color.
3. Review isolines and filled contour bands. Confirm isolines remain visible
   against every band and that adjacent bands remain separable without hue
   being the only cue.
4. Review flat cell fill, smooth vertex fill, wireframe edges, and the
   selected-cell annotation. Confirm cell boundaries and selection remain
   legible when the underlying scale is simulated for each deficiency.
5. Review the colorbar, range labels, axis labels, orientation triad, and
   selection/tooltip text. Confirm textual and geometric cues still identify
   the same values when color discrimination is reduced.

Record the OS/version, GPU/adapter, host revision, layout/theme, selected
scale, deficiency simulation, and pass/fail result for every item. Attach the
reviewed PNG/SVG stimuli or a reference-host capture-manifest case ID. The
automated regression is expected to remain necessary but is not sufficient to
close this manual gate.
