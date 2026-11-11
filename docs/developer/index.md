# Developer documentation

GPUI Toolkit builds native, GPU-rendered interfaces with
[GPUI](https://www.gpui.rs/) — div-based rendering, no HTML. The same UI can
be written from two languages; pick by what you ship:

## Python — fastest start, no Rust toolchain

`pip install gpui-toolkit` and declare your app in pure Python. A native
host bundled in the wheel renders it in a real OS window (Metal, DirectX 12,
or Vulkan). Charts, 2D drawing surfaces, 3D scenes, and themes included.

**[Start with Python →](python/)**

## Rust — full control, native all the way down

Scaffold an app against a source checkout, write views with `gpui-ui-kit`
components, and compile one native binary per platform — desktop and
mobile from the same `app.rs`.

**[Start with Rust →](rust/)**

## Which one?

| | Python | Rust |
|---|---|---|
| Install | one `pip install` | git clone + Rust toolchain |
| Iteration | edit script, rerun; headless IR dumps for CI | `cargo check` / `cargo run` |
| Best for | data tools, dashboards, chart-heavy apps, prototypes that ship | shipping native binaries, mobile, custom rendering |
| Rendering | bundled Rust host renders your declarations | your own binary, GPUI directly |

Both paths are pre-1.0 and versioned together; the package and workspace
share one version number.

## Reference and source

- [Full tutorial (GitHub)](https://github.com/pierreaubert/gpui-toolkit/blob/main/TUTORIAL.md) — the long-form walkthrough behind these pages: themes, design system, keybindings, responsive layout, charts, mobile, the toolkit CLI.
- [PyPI package](https://pypi.org/project/gpui-toolkit/) — the Python wheel.
- [Repository](https://github.com/pierreaubert/gpui-toolkit) — source, issues, changelog.
- [Live demos](/#demos) — every showcase on this site is a real toolkit app; its source is linked from each demo card.
