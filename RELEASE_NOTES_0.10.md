# gpui-toolkit 0.10 — Release Notes (DRAFT)

0.10 is the first minor line cut after the workspace-wide QA, gallery, and
design-system push: one CLI for humans and agents, a supported WASM/browser
target with a live demo gallery, a compile-time layout macro, 30 new UI
components, and a design selector that now drives every design-aware
component. All crates move from 0.9.x to 0.10.x; the neutral design
reproduces previous rendering exactly.

- Docs: `README.md`, `TUTORIAL.md`, `WHATSNEW.md`, per-crate `CHANGELOG.md`
- Live demos and snapshot gallery: <https://gpui-toolkit.spinorama.org/>
- Release policy and lanes: `RELEASE.md`

## Unified `gpui-toolkit` CLI

One binary (`gpui-toolkit-cli`) covers the component catalog
(`component --props`, batch reads), page/block/theme templates, theme
token export, `doctor` health checks, `upgrade` migration notes with
deprecated-pattern detection, and layout-expression tools. Every command
honors `--json` with typed envelopes and stable `ERR_*` codes, and
`manifest --json` exposes a self-describing surface for coding agents.
Project overrides live in `toolkit.toml`.

```bash
cargo install --path crates/gpui-toolkit-cli
gpui-toolkit doctor            # CI gate, exits 1 on failure
gpui-toolkit component Button --props
```

## Layout expressions and the `layout!` macro

`gpui-layout-expr` adds a compact layout-expression language with a
parser, component registry, project aliases, and CLI
`check`/`expand` — plus the compile-time `layout!` proc macro in
`gpui-ui-kit-macros`, so widget trees can be sketched as one-liners
(`V > (Tx"Hi" + B.primary"Go")`) or as structured macro blocks with
unknown names failing the build at the offending span. The README
comparison shows the same UI in builder Rust, `layout!`, and Python.

## 30 new UI-kit components

AppShell/TopNav/MobileNav shell family, Calendar, DateRange/Time/DateTime
inputs, Chat, Markdown/Blockquote, FileInput, Pagination, Skeleton,
Field/FieldStatus, Tokenizer, Lightbox, Carousel, HoverCard,
Grid/Center/AspectRatio layout primitives, Layer, VisuallyHidden,
SelectableCard, MetadataList, Citation, Timestamp, Resizable, and List —
each with theme coverage, behavior-matrix rows, i18n labels, lab
stories, showcase sections, and Python bindings.

## Design system: the selector drives everything

`Alert`, `Toast` (+container), `ButtonSet`, `Badge`, `Progress`
(+circular), `Tabs`, and `Card` take `.design()` and follow
`DesignSystem` tokens for padding, gaps, radii, and type; the audio
`VolumeKnob`, `LevelMeter`, and `Spectrum` family takes
`.design()`/`.design_tokens()` for knob, meter, and spectrum geometry
(mirrored to the Python `AudioDesignTokens` dataclass). The
component-lab design selector gained macOS, Adwaita, Breeze, and Carbon
presets, syncs the app-global design, and threads explicit per-cell
designs through every story renderer, so matrix mode renders each cell
in its own design. The Vello meter/spectrum paint paths now honor
corner radius, matching CPU output.

## WASM/browser target and demo gallery

WASM/browser is a supported target with renderer queries (`auto`, `cpu`,
`legacy`), headless-Chrome smoke tests, and screenshot diffs against
baselines (`just wasm-setup`, `just wasm-serve-*`, `just wasm-test`,
`just wasm-visual`). Charts and audio visuals default to Vello with CPU
fallback. The generated gallery at gpui-toolkit.spinorama.org renders
one featured example across every theme and design style, with each
card linking to a seeded `?style=` demo variant.

## Python runtime

Retained IR nodes plus Python builders and showcase sections for the
new components, `DatasetFrame` Arrow IPC ingest, per-chart-kind chart
validation, and line coverage above the 90% release floor. Tutorial
floor: `gpui-toolkit` 0.10.33+.

## Platforms and quality gates

macOS, Linux, Windows, iOS, tvOS, Android, and WASM, behind the `just
qa` suite: clippy/rustdoc/contract gates, 200-case Metal PR capture
profile with versioned baselines, mesh/visual/CVD/perf/memory gates,
and locked publish dry-runs for the crates.io wave-1 set
(`gpui-design`, `gpui-profiler`, `gpui-ui-kit-macros`).

## Upgrading from 0.9

- Bump workspace/internal requirements from `"0.9"` to `"0.10"`.
- Pre-1.0 minor: intentional breaking changes are possible; see the
  per-crate changelogs and `upgrade` notes. One rendering note:
  `text_xs` labels now resolve through the `small` type token (11px
  under neutral instead of 12px).
- MSRV remains Rust 1.89.
