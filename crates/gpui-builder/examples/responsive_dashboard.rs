//! Example: Responsive dashboard with nested containers.
//!
//! Demonstrates:
//! - Deeply nested layout (3 levels)
//! - Multiple flex children with different weights
//! - Display tiers for adaptive content
//! - Solved layout diagnostics for copy/pasteable debug output
//! - Simulating a window resize sequence
//! - Snapshot matrix output across a resize sequence
//!
//! Run: cargo run -p gpui-builder --example responsive_dashboard

use gpui_builder::{
    Axis, ContainerNode, DisplayTier, LayoutNode, LayoutPreferences, LayoutViewport, Sizing,
    SlotNode, solve, solve_snapshot_matrix,
};

static CHART_TIERS: &[DisplayTier<'_>] = &[
    DisplayTier {
        name: "full",
        min_size: 300.0,
    },
    DisplayTier {
        name: "compact",
        min_size: 150.0,
    },
    DisplayTier {
        name: "sparkline",
        min_size: 50.0,
    },
];

fn main() {
    // Dashboard layout:
    //
    // ┌──────────────────────────────────┐
    // │ toolbar (fixed 48px)             │
    // ├──────────┬───────────────────────┤
    // │ sidebar  │ main area             │
    // │ (20%)    │ ┌───────────────────┐ │
    // │          │ │ chart (flex 2)    │ │
    // │          │ ├───────────────────┤ │
    // │          │ │ table (flex 1)    │ │
    // │          │ └───────────────────┘ │
    // ├──────────┴───────────────────────┤
    // │ status bar (fixed 24px)          │
    // └──────────────────────────────────┘

    let main_area_children = [
        LayoutNode::Slot(SlotNode {
            id: "chart",
            sizing: Sizing::Flex {
                min: 100.0,
                weight: 2.0,
            },
            priority: 1.0,
            collapsible: false,
            display_tiers: CHART_TIERS,
            collapse_label: None,
        }),
        LayoutNode::Slot(SlotNode {
            id: "table",
            sizing: Sizing::Flex {
                min: 80.0,
                weight: 1.0,
            },
            priority: 0.8,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Table"),
        }),
    ];

    let content_children = [
        LayoutNode::Slot(SlotNode {
            id: "sidebar",
            sizing: Sizing::fractional(0.20, 120.0),
            priority: 0.4,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Nav"),
        }),
        LayoutNode::Container(ContainerNode {
            id: "main_area",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(200.0),
            children: &main_area_children,
            divider_size: 2.0,
        }),
    ];

    let root_children = [
        LayoutNode::Slot(SlotNode {
            id: "toolbar",
            sizing: Sizing::Fixed(48.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
        LayoutNode::Container(ContainerNode {
            id: "content",
            axis: Axis::Horizontal,
            auto_axis: Some(1.0),
            sizing: Sizing::flex(0.0),
            children: &content_children,
            divider_size: 4.0,
        }),
        LayoutNode::Slot(SlotNode {
            id: "status_bar",
            sizing: Sizing::Fixed(24.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
    ];

    let root = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Vertical,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: &root_children,
        divider_size: 0.0,
    });

    // Simulate a window resize sequence
    let viewports = [
        LayoutViewport::new("Full HD desktop", 1920.0, 1080.0),
        LayoutViewport::new("Laptop", 1280.0, 720.0),
        LayoutViewport::new("Small window", 800.0, 600.0),
        LayoutViewport::new("Tall/narrow (portrait)", 500.0, 800.0),
        LayoutViewport::new("Very small", 400.0, 300.0),
    ];

    for viewport in viewports {
        println!(
            "=== {} ({:.0}x{:.0}) ===",
            viewport.label, viewport.width, viewport.height
        );
        let solved = solve(
            &root,
            viewport.width,
            viewport.height,
            &LayoutPreferences::default(),
        );
        print!("{}", solved.debug_report_with_source(&root));

        let tabs = solved.collapsed_tabs();
        if !tabs.is_empty() {
            let labels: Vec<&str> = tabs.iter().map(|(_, l)| *l).collect();
            println!("  Collapsed → tabs: {labels:?}");
        }
        println!();
    }

    let matrix = solve_snapshot_matrix(&root, &viewports, &LayoutPreferences::default());
    println!("{}", matrix.to_text());
    println!("{}", matrix.to_markdown_table());

    // Show flex weight effect: chart gets 2x the space of table
    println!("=== Flex weight demo (1200x800) ===");
    let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
    let chart = solved.find("chart").unwrap();
    let table = solved.find("table").unwrap();
    println!(
        "  chart: {:.0}px (weight=2)  table: {:.0}px (weight=1)  ratio: {:.2}",
        chart.height,
        table.height,
        chart.height / table.height,
    );
}
