# GPUI Toolkit Figma Code Connect Mappings

Schema version: 1
Report type: `gpui-toolkit-figma-code-connect-mappings`
Reviewed on: 2026-07-08

This artifact records the stable Figma-to-GPUI mapping plan used by release QA.
It is intentionally repository-local and does not claim that the external Figma
Code Connect publishing step or a live plugin session has run.

| Figma component set | GPUI crate/API | Token source | Story or QA artifact | Notes |
| --- | --- | --- | --- | --- |
| Button / IconButton | `gpui_ui_kit::{Button, IconButton}` | `gpui_design::DesignTokenExport` color, typography, radius, spacing tokens | `ui_kit_visual_regression_manifest()` group `core` | Map Figma variants for size, tone, disabled, loading, and icon placement to builder props. |
| Text input / Number input | `gpui_ui_kit::{Input, NumberInput}` | `gpui_design::DesignTokenExport` form spacing, typography, border, focus tokens | `ui_kit_visual_regression_manifest()` group `form` | Keep validation/error/help-text states explicit so Code Connect examples match QA stories. |
| Checkbox / Toggle / Slider | `gpui_ui_kit::{Checkbox, Toggle, Slider}` | `gpui_design::DesignTokenExport` control sizing and interaction tokens | `ui_kit_visual_regression_manifest()` group `form` | Binary and numeric controls should expose checked/value/disabled states in the mapping examples. |
| Menu / Popover / Dialog | `gpui_ui_kit::{Menu, ContextMenu, Popover, Dialog, ConfirmDialog}` | `gpui_design::DesignTokenExport` elevation, radius, spacing, typography tokens | `ui_kit_visual_regression_manifest()` group `overlay` | Overlay mappings should name focus restore, Escape dismissal, and modal state as host behaviors. |
| Table / Tree view / Command palette | `gpui_ui_kit::{Table, TreeView, CommandPalette}` | `gpui_design::DesignTokenExport` data-display density and focus tokens | `ui_kit_visual_regression_manifest()` groups `data-display`, `navigation` | Map selection, focused row/node, empty, dense, and virtual-window examples. |
| Swipe panel / Mobile surfaces | `gpui_ui_kit::SwipePanel` | `gpui_design::DesignTokenExport` motion, spacing, and touch target tokens | `ui_kit_visual_regression_manifest()` group `mobile` | Document anchor, expansion, and keyboard/touch behaviors as platform-sensitive examples. |
| Audio controls | `gpui_audio_kit::{Potentiometer, VerticalSlider, VolumeKnob, Meter}` | `gpui_design::DesignTokenExport` plus audio-control visual tokens | `audio_visual_regression_manifest()` | Keep units, value ranges, automation affordances, and accessibility summaries visible. |
| Charts | `gpui_px::{line, scatter, bar, area, pie, donut, heatmap, boxplot, treemap, isoline, contour}` | `gpui_design::DesignTokenExport` chart palette and typography tokens | `chart_visual_regression_manifest()` and `chart_capability_report()` | Code Connect examples should link each chart family to static SVG export and accessibility summaries. |
| Layout patterns | `gpui_builder::{layout, validation, visual_regression}` | `gpui_design::DesignTokenExport` spacing, breakpoint, and density tokens | `gpui_builder::visual_regression_manifest()` | Map responsive container, splitter, dense panel, and collapsed/hidden-slot examples. |

## Release Use

- Regenerate or review this file whenever a public component name, builder API,
  token path, or visual-regression story id changes.
- Attach this artifact with `gpui_design_tools::design_tooling_handoff_report()`
  when release notes claim static Figma handoff coverage.
- Keep live preview, data editing, and external Figma publication evidence as
  separate gates; this file only proves the repository mapping contract.
