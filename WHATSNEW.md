# What is new in 0.9

## 0.9.33

### Feature: unified `gpui-toolkit` CLI for humans and agents

One binary now covers the component catalog, page/block/theme templates,
theme token export, `doctor` health checks, `upgrade` migration notes with
deprecated-pattern detection, and layout expression tools. Every command
honors `--json` with typed envelopes and stable `ERR_*` codes, and
`manifest --json` is a self-describing surface for coding agents.

```bash
cargo install --path crates/gpui-toolkit-cli
gpui-toolkit manifest --json | head -c 300
gpui-toolkit doctor            # CI gate, exits 1 on failure
gpui-toolkit component Button --props
```

Project overrides live in `toolkit.toml` (custom layout components,
tracker URL, extra upgrade rules).

### Feature: layout expressions and the `layout!` macro

Sketch widget trees as one-liners (`V > (Tx"Hi" + B.primary"Go")`),
validate/expand them with `layout check` / `layout expand`, or build them
at compile time with `layout!` from `gpui-ui-kit-macros` — unknown names
fail the build with the offending span. See TUTORIAL §15.

### Feature: 30 new UI-kit components

AppShell/TopNav/MobileNav, Calendar, DateRange/Time/DateTime inputs, Chat,
Markdown/Blockquote, FileInput, Pagination, Skeleton, Field/FieldStatus,
Tokenizer, Lightbox, Carousel, HoverCard, Grid/Center/AspectRatio, Layer,
VisuallyHidden, SelectableCard, MetadataList, Citation, Timestamp,
Resizable, and List — each with gallery stories, Python bindings, and
showcase sections. (Banner, BottomSheet, AvatarGroup, StatusDot, and
CodeBlock needs were already covered by Alert, SwipePanel, Avatar,
BadgeDot, and Code::block, so no new modules were added for those.)

## 0.9.10

### Feature: integrated with Vello with 2d plots ; benefit is that all
operations execute on the GPU with no ping pong with the CPU

Ordinary d3rs/px charts and audio visuals now default to Vello. `Auto` uses WGPU custom drawing when available and falls back to the CPU rasterizer; Legacy remains an explicit feature-off/compatibility path. The WASM gallery exposes `auto`, `cpu`, and `legacy` renderer queries for deterministic QA.

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

