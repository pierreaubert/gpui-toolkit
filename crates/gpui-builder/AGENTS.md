# gpui-builder

Generic constraint-based layout solver for GPUI applications.

## Architecture

Platform-agnostic — the core solver has zero GPUI dependencies. Optional `showcase` feature enables a live GPUI demo.

- `solver.rs` — Core `solve()` function: takes a `LayoutNode` tree + available width/height + user preferences, returns a `SolvedNode` tree with concrete pixel sizes
- `types.rs` — `LayoutNode` (enum: Slot/Container), `SlotNode`, `ContainerNode`, `Sizing` (Fixed/Fractional/Flex/Text), `Axis`, `LayoutPreferences`, `DisplayTier`
- `solved.rs` — `SolvedNode` with `find(id)` lookup, width/height/visible/collapsed fields
- `compat.rs` — Bridge from plugin layout definitions (`ColumnConstraint`) into the generic solver: `PluginLayoutTree`, `PluginAdaptations`, `plugin_adaptations()`

## Key Public API

- `solve(root, width, height, prefs) -> SolvedNode` — main solver entry point (`solver.rs`)
- `LayoutNode::Slot(SlotNode)` / `LayoutNode::Container(ContainerNode)` — tree declaration (`types.rs`)
- `Sizing::Fixed(px)` / `Sizing::fractional(ratio, min)` / `Sizing::flex(min)` / `Sizing::text(measure, text, opts)` — sizing modes (`types.rs`)
- `SolvedNode::find(id) -> Option<&SolvedNode>` — lookup by id in solved tree (`solved.rs`)
- `plugin_adaptations(solved, thresholds) -> PluginAdaptations` — compat layer (`compat.rs`)
- Re-exports `TextMeasure`, `PrepareOptions`, `EngineProfile` from `gpui-pretext`

## Features

- `showcase` — enables GPUI deps + `layout-showcase` binary for interactive visual testing

## Testing

```bash
cargo test -p gpui-builder --lib
cargo check -p gpui-builder --features showcase
```

## Important Notes

- Priority-based collapse: when space is insufficient, lowest-priority slots collapse first
- Auto-axis: containers with `auto_axis: Some(threshold)` flip between horizontal/vertical based on aspect ratio
- Display tiers: slots report their active display mode based on resolved size (useful for responsive layouts)
- The solver is deterministic and pure — no side effects, no global state
