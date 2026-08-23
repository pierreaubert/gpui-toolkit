use super::DisplayTier;
use super::axis::Axis;
use super::container_node::ContainerNode;
use super::layout_node::LayoutNode;
use super::layout_preferences::LayoutPreferences;
use super::sizing::Sizing;
use super::slot_node::SlotNode;

use crate::solve;

#[test]
fn layout_preferences_uses_hash_map_for_o1_lookups() {
    let ratios = [
        ("a", Axis::Horizontal, 0.1),
        ("b", Axis::Horizontal, 0.2),
        ("c", Axis::Horizontal, 0.3),
        ("d", Axis::Horizontal, 0.4),
        ("e", Axis::Horizontal, 0.5),
    ];
    let collapsed = [("x", true), ("y", false)];
    let prefs = LayoutPreferences::new(&ratios, &collapsed);

    assert_eq!(prefs.ratio_for("a", Axis::Horizontal), Some(0.1));
    assert_eq!(prefs.ratio_for("e", Axis::Horizontal), Some(0.5));
    assert_eq!(prefs.ratio_for("missing", Axis::Horizontal), None);
    assert_eq!(prefs.ratio_for("a", Axis::Vertical), None);

    assert!(prefs.is_collapsed("x"));
    assert!(!prefs.is_collapsed("y"));
    assert!(!prefs.is_collapsed("missing"));
}

#[test]
fn layout_preferences_last_ratio_wins_for_duplicate_keys() {
    let ratios = [
        ("panel", Axis::Horizontal, 0.25),
        ("panel", Axis::Horizontal, 0.75),
    ];
    let prefs = LayoutPreferences::new(&ratios, &[]);
    assert_eq!(prefs.ratio_for("panel", Axis::Horizontal), Some(0.75));
}

#[test]
fn layout_preferences_collapsed_true_wins_for_duplicates() {
    let collapsed = [("panel", false), ("panel", true)];
    let prefs = LayoutPreferences::new(&[], &collapsed);
    assert!(prefs.is_collapsed("panel"));
}

#[test]
fn slot_constructor_uses_non_collapsible_defaults() {
    let slot = SlotNode::new("main", Sizing::flex(100.0));

    assert_eq!(slot.id, "main");
    assert_eq!(slot.sizing, Sizing::flex(100.0));
    assert_eq!(slot.priority, 1.0);
    assert!(!slot.collapsible);
    assert!(slot.display_tiers.is_empty());
    assert_eq!(slot.collapse_label, None);
}

#[test]
fn fluent_slot_options_set_collapse_and_tiers() {
    static TIERS: &[DisplayTier<'_>] = &[
        DisplayTier {
            name: "Full",
            min_size: 200.0,
        },
        DisplayTier {
            name: "Mini",
            min_size: 100.0,
        },
    ];

    let slot = SlotNode::new("rack", Sizing::fractional(0.3, 80.0))
        .display_tiers(TIERS)
        .collapsible(0.4, "Rack");

    assert_eq!(slot.priority, 0.4);
    assert!(slot.collapsible);
    assert_eq!(slot.display_tiers, TIERS);
    assert_eq!(slot.collapse_label, Some("Rack"));
}

#[test]
fn container_constructors_use_default_options() {
    let children = [LayoutNode::slot("main", Sizing::flex(0.0))];
    let container = ContainerNode::new("root", Axis::Vertical, Sizing::flex(0.0), &children);

    assert_eq!(container.id, "root");
    assert_eq!(container.axis, Axis::Vertical);
    assert_eq!(container.auto_axis, None);
    assert_eq!(container.sizing, Sizing::flex(0.0));
    assert_eq!(container.children.len(), 1);
    assert_eq!(container.divider_size, 0.0);

    let node = LayoutNode::container("root", Axis::Vertical, Sizing::flex(0.0), &children);
    assert!(matches!(node, LayoutNode::Container(_)));
}

#[test]
fn fluent_constructors_match_explicit_struct_layout() {
    let explicit_children = [
        LayoutNode::Slot(SlotNode {
            id: "sidebar",
            sizing: Sizing::fractional(0.25, 100.0),
            priority: 0.5,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Sidebar"),
        }),
        LayoutNode::Slot(SlotNode {
            id: "main",
            sizing: Sizing::flex(200.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
    ];
    let explicit = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Horizontal,
        auto_axis: Some(1.0),
        sizing: Sizing::flex(0.0),
        children: &explicit_children,
        divider_size: 6.0,
    });

    let fluent_children = [
        SlotNode::new("sidebar", Sizing::fractional(0.25, 100.0))
            .collapsible(0.5, "Sidebar")
            .into(),
        LayoutNode::slot("main", Sizing::flex(200.0)),
    ];
    let fluent = ContainerNode::new(
        "root",
        Axis::Horizontal,
        Sizing::flex(0.0),
        &fluent_children,
    )
    .auto_axis(1.0)
    .divider_size(6.0)
    .into_node();

    let explicit_solved = solve(&explicit, 1000.0, 600.0, &LayoutPreferences::default());
    let fluent_solved = solve(&fluent, 1000.0, 600.0, &LayoutPreferences::default());

    for id in ["root", "sidebar", "main"] {
        let explicit = explicit_solved.find(id).unwrap();
        let fluent = fluent_solved.find(id).unwrap();
        assert_eq!(fluent.width, explicit.width, "width mismatch for {id}");
        assert_eq!(fluent.height, explicit.height, "height mismatch for {id}");
        assert_eq!(
            fluent.visible, explicit.visible,
            "visibility mismatch for {id}"
        );
    }
}

struct DummyMeasure;

impl gpui_pretext::TextMeasure for DummyMeasure {
    fn measure_width(&self, _text: &str) -> f64 {
        0.0
    }
}

#[test]
fn sizing_text_debug_equality_and_min_size() {
    static M1: DummyMeasure = DummyMeasure;
    static M2: DummyMeasure = DummyMeasure;

    let t1 = Sizing::Text {
        text: "a",
        measure: &M1,
        line_height: 20.0,
        min: 5.0,
    };
    let t2 = Sizing::Text {
        text: "a",
        measure: &M1,
        line_height: 20.0,
        min: 5.0,
    };
    let t3 = Sizing::Text {
        text: "a",
        measure: &M2,
        line_height: 20.0,
        min: 5.0,
    };

    assert_eq!(t1, t2);
    assert_ne!(t1, t3);
    assert_eq!(
        format!("{:?}", t1),
        r#"Text { text: "a", line_height: 20, min: 5 }"#
    );
    assert_eq!(t1.min_size(), 5.0);

    let fixed = Sizing::Fixed(12.0);
    assert_eq!(format!("{:?}", fixed), "Fixed(12)");
    assert_eq!(fixed.min_size(), 12.0);

    let frac = Sizing::Fractional {
        initial: 0.25,
        min: 10.0,
        max: 100.0,
    };
    assert_eq!(
        format!("{:?}", frac),
        "Fractional { initial: 0.25, min: 10, max: 100 }"
    );
    assert_eq!(frac.min_size(), 10.0);

    let flex = Sizing::Flex {
        min: 20.0,
        weight: 2.0,
    };
    assert_eq!(format!("{:?}", flex), "Flex { min: 20, weight: 2 }");
    assert_eq!(flex.min_size(), 20.0);
}

#[test]
fn slot_node_fluent_setters_and_into_node() {
    let slot = SlotNode::new("main", Sizing::flex(0.0))
        .priority(0.3)
        .collapse_label(Some("Tab"));
    assert_eq!(slot.priority, 0.3);
    assert_eq!(slot.collapse_label, Some("Tab"));
    assert!(!slot.collapsible);

    let node: LayoutNode = slot.into_node();
    assert!(matches!(node, LayoutNode::Slot(s) if s.id == "main"));
}

#[test]
fn layout_preferences_accessors_expose_maps() {
    let prefs = LayoutPreferences::new(&[("a", Axis::Horizontal, 0.1)], &[("b", true)]);
    assert_eq!(prefs.ratios().len(), 1);
    assert_eq!(prefs.collapsed().len(), 1);
    assert_eq!(prefs.ratio_for("a", Axis::Horizontal), Some(0.1));
    assert!(prefs.is_collapsed("b"));
}

#[test]
fn layout_preferences_updates_existing_overrides() {
    let mut prefs =
        LayoutPreferences::new(&[("panel", Axis::Horizontal, 0.2)], &[("panel", false)]);

    prefs.set_ratio("panel", Axis::Horizontal, 0.6);
    prefs.set_collapsed("panel", true);

    assert_eq!(prefs.ratio_for("panel", Axis::Horizontal), Some(0.6));
    assert!(prefs.is_collapsed("panel"));
}

#[test]
fn layout_node_constructors_and_from_impls() {
    let slot = SlotNode::new("s", Sizing::Fixed(1.0));
    let container = ContainerNode::new("c", Axis::Vertical, Sizing::flex(0.0), &[]);

    let _node_from_slot: LayoutNode = slot.into();
    let _node_from_container: LayoutNode = container.into_node();

    assert_eq!(LayoutNode::slot("s", Sizing::Fixed(1.0)).id(), "s");
    assert_eq!(
        LayoutNode::container("c", Axis::Vertical, Sizing::flex(0.0), &[]).id(),
        "c"
    );
    assert_eq!(
        LayoutNode::Container(container).priority(),
        1.0,
        "containers have neutral priority"
    );
}
