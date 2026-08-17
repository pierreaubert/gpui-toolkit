# MeshPlot color-vision-deficiency QA

Status: automated regression implemented; human rendered review remains
required before release.

## Automated gate

Run:

```text
cargo test -p gpui-px --lib color_scale::tests::named_scales_remain_distinguishable_under_cvd_simulations
```

The test samples Viridis, Plasma, Inferno, Magma, Heat, Coolwarm, and Greys
at five normalized scalar positions and applies deterministic protan,
deutan, and tritan simulation matrices. Adjacent scalar samples must retain a
non-zero RGB distance above the regression threshold. This is a screening
test for accidental palette collapse, not a perceptual or clinical claim.

## Manual rendered review

On the compatible reference host, inspect the real MeshPlot captures for each
named scale in light, dark, and high-contrast themes. Review these states:

| State | Required inspection |
| --- | --- |
| Smooth scalar fill | Low/mid/high values remain ordered and distinguishable |
| Flat cell fill | Adjacent cells remain separable without relying on hue alone |
| Filled contours | Band boundaries and labels remain legible |
| Isolines | Lines remain visible over every scale and masked region |
| Selected cell | Orange selection annotation remains distinct from the field |
| NaN mask | Missing regions are obvious and not mistaken for a scalar value |

Record the OS, display scale, source revision, scale/state, deficiency
simulation or assistive filter used, reviewer, and pass/fail result. Attach
the reviewed PNGs and notes to the clean release evidence set. Do not close
the manual CVD gate from the automated test alone.
