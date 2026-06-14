//! Example: Audio plugin layout using the compatibility bridge.
//!
//! Demonstrates:
//! - Converting ColumnConstraint arrays to LayoutNode trees
//! - Plugin-specific adaptations (knob size, slider height, etc.)
//! - Priority-based column collapse at various widths
//!
//! Run: cargo run -p gpui-builder --example plugin_layout

use gpui_builder::{
    PluginColumnConstraint, PluginLayoutThresholds, PluginLayoutTree, plugin_adaptations, solve,
    types::LayoutPreferences,
};

fn main() {
    // --- Compressor: Config | Main | Output ---
    println!("=== Compressor Plugin (3 columns) ===");
    let compressor = [
        PluginColumnConstraint::config(100.0, 0.5),
        PluginColumnConstraint::main(300.0),
        PluginColumnConstraint::output(120.0, 0.6),
    ];
    let tree = PluginLayoutTree::from_constraints(&compressor);
    let root = tree.as_layout_node();
    let thresholds = PluginLayoutThresholds::default();

    for width in [1200.0, 800.0, 600.0, 450.0, 300.0] {
        let solved = solve(&root, width, 400.0, &LayoutPreferences::default());
        let adapt = plugin_adaptations(&solved, &thresholds);

        let visible: Vec<&str> = solved
            .children
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id)
            .collect();
        let collapsed: Vec<&str> = solved
            .children
            .iter()
            .filter(|c| !c.visible)
            .map(|c| c.id)
            .collect();

        println!(
            "  {width:>6.0}px  visible={visible:<30}  collapsed={collapsed:<20}  \
             groups={dir:<6}  knob={knob:<2}  slider={sh:.0}px  viz={viz}",
            visible = format!("{visible:?}"),
            collapsed = format!("{collapsed:?}"),
            dir = format!("{:?}", adapt.group_direction),
            knob = format!("{:?}", adapt.knob_size),
            sh = adapt.slider_height,
            viz = if adapt.show_visualizations {
                "yes"
            } else {
                "no "
            },
        );
    }
    println!();

    // --- EQ: Config | Main | Diagnostic | Output ---
    println!("=== EQ Plugin (4 columns) ===");
    let eq = [
        PluginColumnConstraint::config(100.0, 0.5),
        PluginColumnConstraint::main(300.0),
        PluginColumnConstraint::diagnostic(150.0, 0.3),
        PluginColumnConstraint::output(120.0, 0.6),
    ];
    let tree = PluginLayoutTree::from_constraints(&eq);
    let root = tree.as_layout_node();

    for width in [1200.0, 800.0, 600.0, 450.0] {
        let solved = solve(&root, width, 400.0, &LayoutPreferences::default());

        let visible: Vec<&str> = solved
            .children
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id)
            .collect();
        let collapsed: Vec<&str> = solved
            .children
            .iter()
            .filter(|c| !c.visible)
            .map(|c| c.id)
            .collect();

        println!(
            "  {width:>6.0}px  visible={visible:<45}  collapsed={collapsed:?}",
            visible = format!("{visible:?}"),
        );
    }
    println!();

    // --- Simple: Main only (gain plugin) ---
    println!("=== Gain Plugin (main only) ===");
    let gain = [PluginColumnConstraint::main(200.0)];
    let tree = PluginLayoutTree::from_constraints(&gain);
    let root = tree.as_layout_node();

    for width in [800.0, 400.0, 200.0] {
        let solved = solve(&root, width, 400.0, &LayoutPreferences::default());
        let adapt = plugin_adaptations(&solved, &thresholds);
        println!(
            "  {width:>6.0}px  main={main:.0}px  knob={knob:?}",
            main = solved.find("main").unwrap().width,
            knob = adapt.knob_size,
        );
    }
}
