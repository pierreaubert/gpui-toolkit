# Changelog

## 0.8.5 - 2026-07-09

### Release QA

- Added aggregate release-readiness metadata in `gpui-toolkit`, including
  stability, publish-plan, release-note, release-packaging, dependency-hygiene,
  release-QA, and vendored-patch reports.
- Added a structured dependency-hygiene policy with `deny.toml`, RustSec
  advisory triage, quick-xml risk acceptance for the current internal snapshot,
  and explicit remaining `cargo-deny` release gates.
- Documented crate-by-crate QA status, missing SOTA features, platform gates,
  and internal/public release posture in `docs/qa-20260707.md`.

### Platform and UX

- Added or tightened platform QA artifacts for AU, iOS, Android, tvOS,
  Windows, showcase, scaffolding, visual-regression manifests, and accessibility
  bridge readiness.
- Expanded UI-kit keyboard, accessibility, virtualization, visual-regression,
  and security-surface metadata for release review.

### Visualization and Tooling

- Expanded `gpui-d3rs` checked APIs, D3 parity reports, benchmark coverage, and
  renderer-independent layout helpers.
- Expanded `gpui-px` chart capability, accessibility, interaction,
  visual-regression, annotation, legend, and static SVG export surfaces.
- Added release artifacts for design docs, design tooling handoff, component
  lab visual diffs, Python packaging/schema metadata, and audio-control
  accessibility/automation/visual reports.

### Vendored Dependencies

- Documented active vendored patches and retained changes for Zed platform
  backends, `objc`, `block`, and `zed-font-kit`.
- Patched `block` locally to resolve the current future-incompatibility report
  and cleaned modern Rust warning debt in active Objective-C/font vendored
  dependencies.
