# What is new in 0.9

## 0.9.10

### Feature: integrated with Vello with 2d plots ; benefit is that all
operations execute on the GPU with no ping pong with the CPU

## 0.9.9

### Feature: added support for wasm/browser as a target

wasm/browser is a now supported target.

Run it:
```bash
     just wasm-setup        # one-time toolchain bits
     just wasm-serve-px     # http://127.0.0.1:8082 (also: wasm-serve-hello :8080, wasm-serve-showcase :8081)
     just wasm-test         # headless-Chrome smoke test
     just wasm-visual       # screenshot diff against baselines
```

## 0.9.8

Feature: added support for meshed plots in 2d and 3d

## 0.9.7

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


