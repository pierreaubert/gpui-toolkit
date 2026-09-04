# Unreleased

## Added

- Added a `gpui_diff_design_tokens` binary plus compact token export,
  borrowed (allocation-free) validation, DTCG format support, and a
  `--durable` output mode.

## Refactored

- Normalized formatting in the token validation and diff paths; no behavior
  change.

# 0.7.3

## Features

- Added toolkit-owned design token export, import, validation, and
  conformance CLI tooling.
- Added `gpui-export-design-tokens`, `gpui-import-design-tokens`, and
  `gpui-validate-design-tokens` binaries backed by
  `gpui_design::DesignSystem` token exports.
- `gpui-validate-design-tokens` can emit both JSON and Markdown reports for CI
  through `--report-json` and `--report-markdown`.
