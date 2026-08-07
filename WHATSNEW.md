# What is new in 0.9

GPUI Toolkit 0.9 is the first release line with an explicit public quality
contract. It is based on history-free GPUI snapshots from Zed v1.9.0 and ships
native components, responsive layout, design tokens, audio controls, themes,
text layout, D3-style visualization, Plotly Express-style charts, Python scene
integration, and Apple/mobile platform experiments.

The component lab now has real Metal-rendered regression evidence: a compact
200-case release gallery and nightly coverage of all 1,922 registered cases.
Missing, blank, malformed, incorrectly sized, or changed images fail QA.

The first crates.io wave is intentionally small: `gpui-design`,
`gpui-profiler`, and `gpui-ui-kit-macros`. GPUI-dependent crates are available
in the source tag as beta while their unpublished runtime dependency prevents
honest registry packaging. See `RELEASE.md` and `qa.md` for support and
platform limits.
