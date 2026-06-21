//! Example: Music player app layout (mirrors SotF app-gpui).
//!
//! Demonstrates:
//! - Hard constraints (header/footer)
//! - Nested container with auto-axis switching
//! - Collapsible panels with display tiers
//! - User preferences (dragged ratios, collapsed state)
//!
//! Run: cargo run -p gpui-builder --example app_layout

use gpui_builder::{
    Axis, ContainerNode, DisplayTier, LayoutNode, LayoutPreferences, LayoutScenario, LayoutState,
    LayoutStory, LayoutStoryCatalog, Sizing, SlotNode, inspect_layout, inspect_solved, solve,
    validate_layout,
};

static RACK_TIERS: &[DisplayTier<'_>] = &[
    DisplayTier {
        name: "Full",
        min_size: 200.0,
    },
    DisplayTier {
        name: "Mini",
        min_size: 100.0,
    },
];

fn app_tree() -> (
    [LayoutNode<'static>; 3], // content children
    [LayoutNode<'static>; 3], // root children need content ref
) {
    let content_children = [
        SlotNode::new("library", Sizing::fractional(0.30, 100.0))
            .collapsible(0.5, "Library")
            .into_node(),
        LayoutNode::slot("queue", Sizing::flex(200.0)),
        SlotNode::new("rack", Sizing::fractional(0.30, 0.0))
            .display_tiers(RACK_TIERS)
            .collapsible(0.3, "Rack")
            .into_node(),
    ];
    // We can't return a reference to a local, so we return the parts
    // and assemble outside.
    let root_children = [
        LayoutNode::slot("header", Sizing::Fixed(40.0)),
        // placeholder — will be replaced
        LayoutNode::slot("_placeholder", Sizing::flex(0.0)),
        LayoutNode::slot("footer", Sizing::Fixed(100.0)),
    ];
    (content_children, root_children)
}

fn print_solved(solved: &gpui_builder::SolvedNode, indent: usize) {
    let pad = " ".repeat(indent);
    let status = if solved.visible {
        "visible"
    } else {
        "collapsed"
    };
    let tier = solved.active_tier.unwrap_or("-");
    let axis = match solved.resolved_axis {
        Some(Axis::Horizontal) => " [H]",
        Some(Axis::Vertical) => " [V]",
        None => "",
    };
    println!(
        "{pad}{id}{axis}  {w:.0}x{h:.0}  {status}  tier={tier}",
        id = solved.id,
        w = solved.width,
        h = solved.height,
    );
    for child in &solved.children {
        print_solved(child, indent + 2);
    }
}

fn main() {
    let (content_children, mut root_children) = app_tree();

    // Build the content container referencing content_children
    let content = ContainerNode::new(
        "content",
        Axis::Horizontal,
        Sizing::flex(0.0),
        &content_children,
    )
    .auto_axis(1.0) // switch to vertical when height > width
    .divider_size(6.0)
    .into_node();
    root_children[1] = content;

    let root = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Vertical,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: &root_children,
        divider_size: 0.0,
    });
    let validation = validate_layout(&root);
    assert!(validation.is_clean(), "{validation}");

    println!("=== Layout inspection ===");
    print!("{}", inspect_layout(&root));
    println!();

    let user_ratios = [("library", Axis::Horizontal, 0.45)];
    let user_collapsed = [("rack", true)];
    let scenarios = [
        LayoutScenario::new("desktop", "Wide desktop", 1200.0, 800.0),
        LayoutScenario::new("narrow", "Narrow window", 500.0, 800.0),
        LayoutScenario::new("portrait", "Tall window", 600.0, 1000.0),
        LayoutScenario::new("custom", "User preferences", 1200.0, 800.0)
            .with_preferences(&user_ratios, &user_collapsed),
    ];
    let story = LayoutStory::new("app-layout", "Music player app layout", root, &scenarios)
        .with_description("SotF shell with library, queue, rack, header, and footer regions");
    let stories = [story];
    let catalog = LayoutStoryCatalog::new(&stories);
    println!("=== Story catalog ===");
    print!("{catalog}");
    println!();

    // --- Scenario 1: Wide desktop window ---
    println!("=== Wide desktop (1200x800) ===");
    let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
    print!("{}", inspect_solved(&solved));
    print_solved(&solved, 0);

    let tabs = solved.collapsed_tabs();
    if !tabs.is_empty() {
        println!("  Collapsed tabs: {:?}", tabs);
    }
    println!();

    // --- Scenario 2: Narrow window (library collapses) ---
    println!("=== Narrow window (500x800) ===");
    let solved = solve(&root, 500.0, 800.0, &LayoutPreferences::default());
    print_solved(&solved, 0);

    let tabs = solved.collapsed_tabs();
    if !tabs.is_empty() {
        println!("  Collapsed tabs: {:?}", tabs);
    }
    println!();

    // --- Scenario 3: Tall window (auto-axis switches to vertical) ---
    println!("=== Tall window (600x1000) ===");
    let solved = solve(&root, 600.0, 1000.0, &LayoutPreferences::default());
    print_solved(&solved, 0);
    println!();

    // --- Scenario 4: User dragged library wider, collapsed rack ---
    println!("=== User prefs: library=45%, rack collapsed (1200x800) ===");
    let mut layout_state = LayoutState::new();
    layout_state.set_ratio("library", Axis::Horizontal, 0.45);
    layout_state.set_collapsed("rack", true);

    let layout_prefs = layout_state.preferences();
    let solved = solve(&root, 1200.0, 800.0, &layout_prefs.as_preferences());
    print_solved(&solved, 0);

    let tabs = solved.collapsed_tabs();
    if !tabs.is_empty() {
        println!("  Collapsed tabs: {:?}", tabs);
    }
}
