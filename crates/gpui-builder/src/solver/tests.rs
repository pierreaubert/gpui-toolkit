use std::cell::Cell;

use super::misc::clear_text_cache;
use super::{solve, solve_tree};
use crate::types::{Axis, ContainerNode, LayoutNode, LayoutPreferences, Sizing, SlotNode};

struct CountingMeasure {
    char_width: f64,
    calls: Cell<usize>,
}

impl gpui_pretext::TextMeasure for CountingMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        self.calls.set(self.calls.get() + 1);
        text.chars().count() as f64 * self.char_width
    }
}

fn simple_text_slot<'a>(
    id: &'a str,
    text: &'a str,
    measure: &'a dyn gpui_pretext::TextMeasure,
) -> LayoutNode<'a> {
    LayoutNode::Slot(SlotNode {
        id,
        sizing: Sizing::Text {
            text,
            measure,
            line_height: 20.0,
            min: 0.0,
        },
        priority: 1.0,
        collapsible: false,
        display_tiers: &[],
        collapse_label: None,
    })
}

#[test]
fn solve_reuses_text_size_cache_across_calls() {
    clear_text_cache();

    let measure = CountingMeasure {
        char_width: 10.0,
        calls: Cell::new(0),
    };

    // Use text that no other test uses to avoid cache collisions from
    // recycled trait-object addresses on the same thread.
    let children = [
        simple_text_slot("a", "persistent-cache-alpha", &measure),
        simple_text_slot("b", "persistent-cache-alpha", &measure),
    ];
    let root = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Horizontal,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: &children,
        divider_size: 0.0,
    });
    let prefs = LayoutPreferences::default();

    let first = solve(&root, 500.0, 100.0, &prefs);
    let first_calls = measure.calls.get();
    assert!(first_calls > 0, "first solve should measure text");
    assert_eq!(first.find("a").unwrap().width, 220.0);
    assert_eq!(first.find("b").unwrap().width, 220.0);

    measure.calls.set(0);
    let second = solve(&root, 500.0, 100.0, &prefs);
    let second_calls = measure.calls.get();
    assert_eq!(
        second_calls, 0,
        "second solve should reuse cached PreparedText and not measure again"
    );
    assert_eq!(second.find("a").unwrap().width, 220.0);
    assert_eq!(second.find("b").unwrap().width, 220.0);
}

#[test]
fn solve_tree_flat_path_caches_results() {
    clear_text_cache();

    let measure = CountingMeasure {
        char_width: 10.0,
        calls: Cell::new(0),
    };

    let children = [
        simple_text_slot("a", "persistent-cache-beta", &measure),
        simple_text_slot("b", "persistent-cache-gamma", &measure),
    ];
    let root = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Horizontal,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: &children,
        divider_size: 0.0,
    });
    let prefs = LayoutPreferences::default();

    let first = solve_tree(&root, 500.0, 100.0, &prefs);
    let first_calls = measure.calls.get();
    assert!(first_calls > 0, "first solve_tree should measure text");
    assert_eq!(first.find("a").unwrap().width(), 210.0);
    assert_eq!(first.find("b").unwrap().width(), 220.0);

    measure.calls.set(0);
    let second = solve_tree(&root, 500.0, 100.0, &prefs);
    let second_calls = measure.calls.get();
    assert_eq!(
        second_calls, 0,
        "second solve_tree should reuse cached PreparedText and not measure again"
    );
    assert_eq!(second.find("a").unwrap().width(), 210.0);
    assert_eq!(second.find("b").unwrap().width(), 220.0);
}
