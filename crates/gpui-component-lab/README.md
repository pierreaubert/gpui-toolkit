# gpui-component-lab

Prop-driven story registry and component lab scaffolding for `gpui-toolkit`.

## What It Owns

- `ComponentStory`, `StoryRegistry`, `StoryMetadataItem`, `StoryRenderer`,
  `StoryRendererRegistry`, `StoryProp`, `StoryPropValue`
- `ViewportPreset`, `ThemePreset`, `MotionPreset`, `ResponsivePreviewMatrix`
- Built-in story metadata items for `gpui-ui-kit`, `gpui-px`, and
  `gpui-audio-kit`, shown in the lab inspector and persisted in story JSON
- Built-in renderer metadata for all first-party stories, including renderer
  kind, interactivity, and responsive matrix behavior
- Preview handler coverage for all built-in renderer-backed stories, so a
  first-party story cannot stop at metadata-only registration
- Separate bespoke, prop-driven renderer-backed stories for exported
  renderable `gpui-ui-kit` component types, including workflow/focus
  renderables such as `FocusGroup`, `Port`, `WorkflowNode`, and
  `WorkflowCanvas`, plus public module renderables such as `AnimatedQrCode`
  and `Showcase`
- Renderer-backed built-in stories:
  `ui-kit.button`, `ui-kit.form`, `ui-kit.status`, `ui-kit.navigation`,
  `ui-kit.feedback`, `ui-kit.card`, every public `gpui-ui-kit` showcase
  section, `px.line`, `px.bar`, `px.scatter`, `px.area`, `px.heatmap`,
  `px.contour`, `px.isoline`, `px.pie`, `px.donut`, `px.boxplot`,
  `px.treemap`, `px.surface3d`, `audio-kit.potentiometer`,
  `audio-kit.vertical-slider`, `audio-kit.volume-knob`, `audio-kit.meter`,
  `audio-kit.horizontal-meter`, `audio-kit.spectrum`, and
  `audio-kit.spectrum-axis`
- Designer state persistence as `*.story.json`, including selected motion and
  builder layout constraints, alignment, overflow, surface, gap, and border
  settings
- Conformance checks that first-party toolkit stories have matching lab
  renderers, responsive rendered bounds, touch target metadata, focus labels,
  valid builder layout state, design presets, motion presets, and token reports
- Stricter touch/focus/overflow conformance metadata including observed
  rendered max bounds, touch target counts and area checks, and exact,
  non-duplicate focus labels for focusable previews
- PX-specific conformance checks that every chart story in the exported chart
  inventory has renderer metadata, fill/fixed responsive sizing props, and
  mobile-safe rendered bounds

## Commands

```bash
cargo run -p gpui-component-lab --bin gpui-component-lab -- --json
cargo run -p gpui-component-lab --bin gpui-component-lab -- --conformance
cargo run -p gpui-component-lab --bin gpui-component-lab -- \
  --conformance \
  --report-json target/gpui-conformance/component-lab.json \
  --report-markdown target/gpui-conformance/component-lab.md
cargo run -p gpui-component-lab --bin gpui-component-lab -- \
  --visual-manifest \
  --visual-output-root target/gpui-component-lab/visual \
  --visual-manifest-json target/gpui-component-lab/visual/manifest.json \
  --visual-manifest-markdown target/gpui-component-lab/visual/manifest.md
```

The visual manifest is display-server independent: it expands renderer-backed
stories across their responsive preview matrices and writes deterministic
baseline, actual, and diff image paths. CI screenshot runners can consume the
JSON, capture each `actual_path`, compare it with `baseline_path`, and write the
diff image to `diff_path`.

Safe live preview launches the lab and uses in-process polling for story/token
JSON:

```bash
cargo run -p gpui-component-lab --bin gpui-component-lab -- --watch --token tokens.json
```

Rust source changes can be supervised with child-process relaunches. Dynamic
library reload is intentionally unsupported.

```bash
cargo run -p gpui-component-lab --bin gpui-component-lab -- \
  --supervise-rust \
  --child-command "cargo run -p gpui-component-lab --bin gpui-component-lab -- --watch"
```
