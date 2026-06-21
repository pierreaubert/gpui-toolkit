//! Example: Nested layout solved with the macro DSL.
//!
//! Demonstrates:
//! - Declaring a nested layout tree without manual child arrays
//! - Slot and container options in the macro DSL
//! - Solving directly to a `SolvedNode`
//!
//! Run: cargo run -p gpui-builder --example macro_layout

use gpui_builder::{Axis, DisplayTier, LayoutPreferences, Sizing, solve_layout};

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

fn main() {
    let prefs = LayoutPreferences::new(&[("library", Axis::Horizontal, 0.35)], &[]);

    let solved = solve_layout! {
        width: 1200.0,
        height: 800.0,
        prefs: &prefs,
        container root(Axis::Vertical, Sizing::flex(0.0)) {
            slot header(Sizing::Fixed(40.0));
            container content(
                Axis::Horizontal,
                Sizing::flex(0.0);
                auto_axis = 1.0,
                divider_size = 6.0
            ) {
                slot library(Sizing::fractional(0.30, 100.0);
                    priority = 0.5,
                    collapsible = true,
                    collapse_label = "Library"
                );
                slot queue(Sizing::flex(200.0));
                slot rack(Sizing::fractional(0.30, 0.0);
                    priority = 0.3,
                    collapsible = true,
                    collapse_label = "Rack",
                    display_tiers = RACK_TIERS
                );
            };
            slot footer(Sizing::Fixed(100.0));
        }
    };

    for id in ["header", "library", "queue", "rack", "footer"] {
        let node = solved.find(id).unwrap();
        println!(
            "{id}: {:.0}x{:.0} visible={} tier={}",
            node.width,
            node.height,
            node.visible,
            node.active_tier.unwrap_or("-")
        );
    }
}
