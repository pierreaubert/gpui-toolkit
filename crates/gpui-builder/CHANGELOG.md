# Unreleased

## Demos

- Layout showcase now starts directly on the platform GPUI backends and installs
  the theme and design-system globals without depending on `gpui-miniapp`.
- Removed the showcase dependency on `gpui-ui-kit`, breaking the builder/UI-kit
  crate cycle so `gpui-ui-kit` can depend on `gpui-builder`.

## Visual regression

- Visual regression manifests are now serializable for screenshot runners.
- Added required color-scheme coverage validation and deterministic output
  paths for capture artifacts.

# 0.6.3

## Accessibility tree model

- New `accessibility` module with a pure accessibility representation for solved layouts:
  `AccessibilityTree`, `AccessibilityNode`, `AccessibilityRole`, and `AccessibilityMetadata`.
- Added `accessibility_tree_from_solved(&SolvedNode, metadata)` to derive a hierarchical
  accessibility view from solver output while preserving visibility/collapse semantics.
- Added default role mapping for container/slot/collapsed-tab cases, with optional
  per-node overrides for role, label, and description.
- Re-exported the accessibility API from `lib.rs`.

# 0.6.2

## Plugin chassis layout primitives

- New `plugin_chassis` module: owned, platform-agnostic descriptors for
  audio plugin chassis layouts — `ChassisLayout`, `HeaderSpec`,
  `SectionSpec`, `RowSpec` (`KnobRow` / `BandToggle` / `ReadoutTile` /
  `ToggleGroup`), `KnobSlot`, optional `FooterSpec`. Stays free of GPUI
  dependencies per the crate's contract; renderers consume the
  descriptors directly.
- `ChassisLayout::solve(available_width)` returns `SolvedChassis` —
  per-section width + visibility — using a small purpose-built priority
  collapse algorithm (lowest priority drops first; ties drop the later
  index first; sections with `priority >= 1.0` are never dropped, even
  when the available width is below their `min_width`). Avoids the
  arena gymnastics of building a nested `LayoutNode` tree for what is a
  one-axis flex with collapse.
- 11 new tests covering wide / narrow / tie-break / never-collapse /
  empty-chassis / total-width-fits cases.
- Re-exported `ChassisLayout`, `HeaderSpec`, `FooterSpec`, `SectionSpec`,
  `RowSpec`, `KnobSlot`, `SolvedChassis`, `SolvedSection` from `lib.rs`.

# 0.6.1

## New

- Started to migrate to new design/builder pattern
- Added gpui-builder in app-gpui
- Started the migration to the new gpui-builder
