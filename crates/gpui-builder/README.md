# gpui-builder

Generic constraint-based layout solver for GPUI applications.

Platform-agnostic — the core solver has zero framework dependencies. An optional `showcase` feature enables a live GPUI demo binary.

## Features

- **Hard constraints** (`Sizing::Fixed`) — headers, footers, toolbars
- **Soft constraints** (`Sizing::Fractional`, `Sizing::Flex`) — resizable panels
- **Priority-based collapse** — lowest-priority panels collapse first when space is tight
- **Auto-axis switching** — flips horizontal/vertical based on aspect ratio
- **Display tiers** — panels report their active display mode (e.g. Full/Mini) based on resolved size
- **User preferences** — ratio overrides and manual collapse state
- **Draggable dividers** — configurable divider size per container
- **Text-measured sizing** — slots sized by text content via `TextMeasure` trait (from `gpui-pretext`)
- **Layout diagnostics** — stable solved-tree reports with sizing metadata and warnings

## Quick Start

```rust
use gpui_builder::{solve, LayoutNode, ContainerNode, SlotNode, Sizing, Axis, LayoutPreferences};

let children = [
    SlotNode::new("sidebar", Sizing::fractional(0.2, 120.0))
        .collapsible(0.5, "Sidebar")
        .into_node(),
    LayoutNode::slot("content", Sizing::flex(300.0)),
];

let root = ContainerNode::new("root", Axis::Horizontal, Sizing::flex(0.0), &children)
    .auto_axis(1.0) // switch to vertical in portrait
    .divider_size(6.0)
    .into_node();

let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());

let sidebar = solved.find("sidebar").unwrap();
println!("sidebar: {}x{} visible={}", sidebar.width, sidebar.height, sidebar.visible);
```

For resize or frame-rate layout, reuse the flat output storage so steady-state
solves do not allocate:

```rust
use gpui_builder::{SolvedTree, solve_tree_into};

let mut solved = SolvedTree::with_capacity(root.node_count());
solve_tree_into(&root, 1200.0, 800.0, &LayoutPreferences::default(), &mut solved);
solve_tree_into(&root, 900.0, 800.0, &LayoutPreferences::default(), &mut solved);
assert_eq!(solved.root().width(), 900.0);
```

The dedicated allocation contract warms solver scratch and then requires
1,000 nested resize solves plus indexed lookup to perform zero allocations.

## Layout Diagnostics

Use `SolvedNode::debug_report()` while iterating on complex layouts. If you
still have the declaration tree, `debug_report_with_source()` adds sizing mode,
collapse priority, and collapsibility metadata to each line.

```rust
let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
let report = solved.debug_report_with_source(&root);

println!("{report}");
assert!(report.is_clean());

let summary = report.summary();
println!("layout warnings: {}", summary.total);
println!("{}", report.warnings_markdown_table());
```

Reports flag suspicious output such as invalid sizes, hidden nodes without a
collapse label, and visible children that overflow a parent axis.

The structured warning API is intended for app debug panes and CI logs:

- `LayoutDebugReport::summary()` groups warnings by stable category.
- `LayoutDebugWarning::code()` returns stable ids such as
  `main-axis-overflow` and `invalid-size`.
- `LayoutDebugWarning::remediation()` gives a short author-facing fix hint.
- `LayoutDebugReport::warnings_markdown_table()` emits a ready-to-attach
  Markdown table with code, node id, diagnostic, and remediation columns.

## Layout Inspection

Export declared and solved trees as owned, stable records for debug overlays,
showcase tooling, and snapshot tests:

```rust
use gpui_builder::{inspect_layout, inspect_solved};

let declared = inspect_layout(&root);
println!("{declared}");

let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
let solved_export = inspect_solved(&solved);
println!("{solved_export}");
```

The inspection output includes stable node paths, sizing summaries,
container metadata, slot collapse metadata, display tiers, resolved sizes,
visibility, active tiers, and resolved axes.

## Layout Stories

Define Storybook-style layout catalogs for docs, examples, tests, and future
visual tooling:

```rust
use gpui_builder::{LayoutScenario, LayoutStory, LayoutStoryCatalog};

let scenarios = [
    LayoutScenario::new("desktop", "Desktop", 1200.0, 800.0),
    LayoutScenario::new("narrow", "Narrow", 500.0, 800.0),
];
let story = LayoutStory::new("shell", "Application shell", root, &scenarios);
let stories = [story];
let catalog = LayoutStoryCatalog::new(&stories);

println!("{catalog}");
let solved = catalog.solve_all();
```

Scenarios can also carry ratio overrides and collapsed-slot preferences, so the
same catalog can drive responsive examples and regression snapshots.

## Visual Regression Manifests

Turn layout story catalogs into deterministic screenshot-runner input. The
manifest serializes capture ids, viewport sizes, color schemes, solved layout
text, and stable output paths.

```rust
use gpui_builder::{VisualColorScheme, VisualRegressionManifest};

let manifest = VisualRegressionManifest::from_catalog(
    &catalog,
    &[VisualColorScheme::Light, VisualColorScheme::Dark],
);
let coverage = manifest.validate_required_schemes(&[
    VisualColorScheme::Light,
    VisualColorScheme::Dark,
]);

assert!(coverage.passed());
println!("{}", manifest.to_markdown_table());
```

## Benchmark Coverage

Use `benchmark_report()` to expose the solver benchmark contract in release
notes or CI artifacts. The report is schema-versioned, names the Criterion
command to run, and lists every case covered by `benches/solved_tree.rs`:
balanced and wide lookup paths, recursive versus flat traversal, and text
measurement cache-hit solving.

```rust
use gpui_builder::benchmark_report;

let report = benchmark_report();
assert_eq!(report.report_type, "gpui-builder-benchmark-coverage");
println!("{}", report.to_markdown_table());
```

The release baseline policy is to run
`cargo bench -p gpui-builder --bench solved_tree` on the release machine and
compare the same benchmark case ids across releases.

## Layout State

Use `LayoutState` for mutable, persistent user-driven overrides and convert it to solver input
right before calling `solve`.

```rust
use gpui_builder::{
    solve,
    Axis,
    ContainerNode,
    LayoutNode,
    LayoutState,
    Sizing,
    SlotNode,
};

let children = [
    LayoutNode::Slot(SlotNode {
        id: "library",
        sizing: Sizing::fractional(0.30, 100.0),
        priority: 0.5,
        collapsible: true,
        display_tiers: &[],
        collapse_label: Some("Library"),
    }),
    LayoutNode::Slot(SlotNode {
        id: "queue",
        sizing: Sizing::flex(200.0),
        priority: 1.0,
        collapsible: false,
        display_tiers: &[],
        collapse_label: None,
    }),
];

let root = LayoutNode::Container(ContainerNode {
    id: "root",
    axis: Axis::Horizontal,
    auto_axis: None,
    sizing: Sizing::flex(0.0),
    children: &children,
    divider_size: 0.0,
});

let mut state = LayoutState::new();
state.set_ratio("library", Axis::Horizontal, 0.45);
state.set_collapsed("queue", true);

let prefs = state.preferences().as_preferences();
let solved = solve(&root, 1200.0, 800.0, &prefs);
assert!(solved.find("library").is_some());
```

## Accessibility Tree

Use `accessibility_tree_from_solved` to build a platform-agnostic accessibility tree from
`SolvedNode` output. You can attach optional per-node metadata for role/label/description.

```rust
use gpui_builder::{
    AccessibilityMetadata,
    AccessibilityRole,
    accessibility_tree_from_solved,
};

let tree = accessibility_tree_from_solved(
    &solved,
    &[
        (
            "sidebar",
            AccessibilityMetadata {
                role: Some(AccessibilityRole::Region),
                label: Some("Navigation Sidebar"),
                description: Some("Primary navigation and filters"),
            },
        ),
    ],
);

let sidebar = tree.find("sidebar").unwrap();
assert_eq!(sidebar.label.as_deref(), Some("Navigation Sidebar"));
```

## Macro DSL

Use `solve_layout!` when you want to describe and solve a nested tree in one
expression without manually threading child arrays through every container.
Node identifiers become layout ids via `stringify!`, and the macro returns a
`SolvedNode`.

```rust
use gpui_builder::{Axis, LayoutPreferences, Sizing, solve_layout};

let solved = solve_layout! {
    width: 1200.0,
    height: 800.0,
    prefs: &LayoutPreferences::default(),
    container root(Axis::Horizontal, Sizing::flex(0.0);
        auto_axis = 1.0,
        divider_size = 6.0
    ) {
        slot sidebar(Sizing::fractional(0.2, 120.0);
            priority = 0.5,
            collapsible = true,
            collapse_label = "Sidebar"
        );
        slot content(Sizing::flex(300.0));
    }
};
```

## Responsive Snapshots

Use `solve_snapshot_matrix` to inspect the same layout across named viewport
sizes from tests, examples, or CI logs without running the GPUI showcase.

```rust
use gpui_builder::{LayoutPreferences, LayoutViewport, solve_snapshot_matrix};

let viewports = [
    LayoutViewport::new("desktop", 1200.0, 800.0),
    LayoutViewport::new("portrait", 500.0, 800.0),
];

let matrix = solve_snapshot_matrix(&root, &viewports, &LayoutPreferences::default());
println!("{}", matrix.to_markdown_table());
```

## Layout Validation

Run validation in examples, tests, or debug tooling before solving a layout tree:

```rust
use gpui_builder::validate_layout;

let report = validate_layout(&root);
assert!(report.is_clean(), "{report}");
```

Validation reports hard errors for ids and numeric constraints that can make layout
behavior ambiguous, and warnings for quality issues such as unlabeled collapsible
slots, duplicate display tiers, and empty containers.
## Sizing Modes

| Mode | Constructor | Behavior |
|------|-------------|----------|
| Fixed | `Sizing::Fixed(px)` | Exact pixel size, never flexes |
| Fractional | `Sizing::fractional(ratio, min)` | Fraction of remaining space with minimum |
| Flex | `Sizing::flex(min)` | Equal share of remaining space |
| Text | `Sizing::text(measure, text, opts)` | Sized by measured text content |

## Plugin Compatibility

The `compat` module bridges existing plugin layout definitions (`ColumnConstraint`) into the generic solver:

```rust
use gpui_builder::{PluginLayoutTree, PluginLayoutThresholds, plugin_adaptations};

let tree = PluginLayoutTree::from_constraints(&constraints);
let solved = solve(tree.as_layout_node(), width, height, &prefs);
let adapt = plugin_adaptations(&solved, &PluginLayoutThresholds::default());
// adapt.orientation, adapt.knob_size, adapt.slider_height, ...
```

## Showcase Binary

```bash
cargo run -p gpui-builder --features showcase --bin layout-showcase
```

Interactive demo with draggable dividers, collapsible panels, auto-axis switching, and display tiers.
The right-hand inspector panel renders the solved visual tree. Click a tree row
to select a node and highlight the matching region in the live layout.

CI can exercise the native platform backend without manual interaction:

```bash
cargo run -p gpui-builder --features showcase --bin layout-showcase -- \
  --smoke-test \
  --smoke-artifact target/qa/native-ui/gpui-builder-smoke.json
```

Smoke mode exits after the root view renders and records window/render evidence.
It does not claim pixel capture or visual-diff coverage.

On a macOS QA host, capture the real desktop windows with:

```bash
just qa-native-ui-macos
just qa-native-ui-utm-linux
just qa-native-ui-utm-windows
```

The macOS recipe runs directly on the host. Linux and Windows run in logged-in
UTM desktop guests, capture the exact showcase window inside each guest, reject
blank or near-uniform pixels, and copy the PNG plus schema-v2 smoke report back
to `target/qa/native-ui/<platform>/`. Docker and cross-compilation are not
treated as Windows or macOS renderer evidence.

## Testing

```bash
cargo test -p gpui-builder --lib
cargo check -p gpui-builder --features showcase
```
