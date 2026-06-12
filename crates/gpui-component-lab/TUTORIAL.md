# gpui-component-lab Tutorial

`gpui-component-lab` is a prop-driven component lab and responsive preview
matrix for the toolkit.

## 1. Run the lab

```bash
cargo run -p gpui-component-lab --bin gpui-component-lab
```

The built-in registry shows toolkit components across theme, viewport, sizing,
and motion presets.

## 2. Run conformance

```bash
mkdir -p target/gpui-conformance
cargo run -p gpui-component-lab --bin gpui-component-lab -- \
  --conformance \
  --report-json target/gpui-conformance/component-lab.json \
  --report-markdown target/gpui-conformance/component-lab.md
```

## 3. Add a component story

1. Add or update the component in `gpui-ui-kit`, `gpui-audio-kit`, or another
   UI crate.
2. Add stateful examples under that crate's `examples/` directory.
3. Add story metadata and prop variants in `gpui-component-lab`.
4. Register the story in the story registry.
5. Run conformance and review the markdown report.

## 4. Use reports in review

The JSON report is useful for automation. The markdown report is better for
humans because it groups findings by story, viewport, and theme.

## 5. Verify

```bash
cargo test -p gpui-component-lab
just qa-gpui-conformance
```
