# MeshPlot color-vision-deficiency QA walkthrough

Status: automated regression implemented; human rendered review remains
required before release. The automated color-scale simulation is a regression
screen, not a perceptual or clinical claim.

## Automated gate

Run:

```text
cargo test -p gpui-px --lib color_scale::tests::named_scales_remain_distinguishable_under_cvd_simulations
```

The test samples Viridis, Plasma, Inferno, Magma, Heat, Coolwarm, and Greys
at five normalized scalar positions and applies deterministic protan,
deutan, and tritan simulation matrices. Adjacent scalar samples must retain a
non-zero RGB distance above the regression threshold.

## Manual rendered review

On a compatible reference desktop, run the native GPUI MeshPlot host with the
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

Required state coverage:

| State | Required inspection |
| --- | --- |
| Smooth scalar fill | Low/mid/high values remain ordered and distinguishable |
| Flat cell fill | Adjacent cells remain separable without relying on hue alone |
| Filled contours | Band boundaries and labels remain legible |
| Isolines | Lines remain visible over every scale and masked region |
| Selected cell | Orange selection annotation remains distinct from the field |
| NaN mask | Missing regions are obvious and not mistaken for a scalar value |

Record the OS/version, GPU/adapter, display scale, host revision, layout/theme,
selected scale, deficiency simulation or assistive filter, reviewer, and
pass/fail result for every item. Attach the reviewed PNG/SVG stimuli or a
reference-host capture-manifest case ID. Do not close the manual CVD gate from
the automated test alone.
