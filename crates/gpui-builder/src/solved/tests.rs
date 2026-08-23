use super::layout_debug_warning::LayoutDebugWarning;
use super::solved_node::SolvedNode;
use super::solved_tree::{CollapsedSlot, SolvedTree};
use super::types::LayoutDebugWarningKind;
use crate::solver::solve_tree;
use crate::types::{
    Axis, ContainerNode, DisplayTier, LayoutNode, LayoutPreferences, Sizing, SlotNode,
};

fn solved_slot(id: &'static str, width: f32, height: f32) -> SolvedNode<'static> {
    SolvedNode {
        id,
        width,
        height,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: None,
        children: Vec::new(),
    }
}

#[test]
fn debug_report_includes_source_metadata_and_collapsed_labels() {
    let source_children = [
        LayoutNode::Slot(SlotNode {
            id: "header",
            sizing: Sizing::Fixed(40.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
        LayoutNode::Slot(SlotNode {
            id: "inspector",
            sizing: Sizing::fractional(0.25, 80.0),
            priority: 0.4,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Inspector"),
        }),
    ];
    let source = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Vertical,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: &source_children,
        divider_size: 0.0,
    });
    let solved = SolvedNode {
        id: "root",
        width: 320.0,
        height: 240.0,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(Axis::Vertical),
        children: vec![
            solved_slot("header", 320.0, 40.0),
            SolvedNode {
                id: "inspector",
                width: 0.0,
                height: 0.0,
                visible: false,
                active_tier: None,
                collapse_label: Some("Inspector"),
                resolved_axis: None,
                children: Vec::new(),
            },
        ],
    };

    let report = solved.debug_report_with_source(&source);

    assert!(report.is_clean());
    assert_eq!(
        report.tree(),
        concat!(
            "root size=320x240 visible axis=vertical sizing=Flex(min=0,weight=1)\n",
            "  header size=320x40 visible sizing=Fixed(40)\n",
            "  inspector size=0x0 collapsed label=\"Inspector\" ",
            "sizing=Fractional(initial=0.25,min=80,max=unbounded) collapsible priority=0.4",
        )
    );
}

#[test]
fn debug_report_warns_for_invalid_hidden_and_overflowing_nodes() {
    let solved = SolvedNode {
        id: "root",
        width: 100.0,
        height: 40.0,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(Axis::Horizontal),
        children: vec![
            solved_slot("wide", 75.0, 45.0),
            solved_slot("wider", 50.0, 20.0),
            SolvedNode {
                id: "ghost",
                width: f32::NAN,
                height: 0.0,
                visible: false,
                active_tier: None,
                collapse_label: None,
                resolved_axis: None,
                children: Vec::new(),
            },
        ],
    };

    let report = solved.debug_report();

    assert!(report.has_warnings());
    assert_eq!(report.warnings().len(), 4);
    assert_eq!(
        report.warnings()[0],
        LayoutDebugWarning {
            node_id: "root",
            kind: LayoutDebugWarningKind::MainAxisOverflow {
                axis: Axis::Horizontal,
                used: 125.0,
                available: 100.0,
            },
        }
    );
    assert_eq!(
        report.warnings()[1],
        LayoutDebugWarning {
            node_id: "root",
            kind: LayoutDebugWarningKind::CrossAxisOverflow {
                axis: Axis::Vertical,
                child_id: "wide",
                used: 45.0,
                available: 40.0,
            },
        }
    );
    match &report.warnings()[2].kind {
        LayoutDebugWarningKind::InvalidSize { width, height } => {
            assert_eq!(report.warnings()[2].node_id, "ghost");
            assert!(width.is_nan());
            assert_eq!(*height, 0.0);
        }
        other => panic!("expected invalid-size warning, got {other:?}"),
    }
    assert_eq!(
        report.warnings()[3],
        LayoutDebugWarning {
            node_id: "ghost",
            kind: LayoutDebugWarningKind::InvisibleWithoutCollapseLabel,
        }
    );
    assert!(
        report
            .to_string()
            .contains("warnings:\n- root children use 125px")
    );
}

#[test]
fn debug_report_summarizes_warning_counts_and_remediation() {
    let solved = SolvedNode {
        id: "root",
        width: 100.0,
        height: 40.0,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(Axis::Horizontal),
        children: vec![
            solved_slot("wide", 75.0, 45.0),
            solved_slot("wider", 50.0, 20.0),
            SolvedNode {
                id: "ghost",
                width: f32::NAN,
                height: 0.0,
                visible: false,
                active_tier: None,
                collapse_label: None,
                resolved_axis: None,
                children: Vec::new(),
            },
        ],
    };

    let report = solved.debug_report();
    let summary = report.summary();

    assert_eq!(summary.total, 4);
    assert_eq!(summary.invalid_size, 1);
    assert_eq!(summary.invisible_without_collapse_label, 1);
    assert_eq!(summary.main_axis_overflow, 1);
    assert_eq!(summary.cross_axis_overflow, 1);
    assert!(!summary.is_clean());

    let first = &report.warnings()[0];
    assert_eq!(first.code(), "main-axis-overflow");
    assert!(
        first.remediation().contains("Reduce fixed/minimum sizes"),
        "{}",
        first.remediation()
    );

    let table = report.warnings_markdown_table();
    assert!(table.contains("| code | node | diagnostic | remediation |"));
    assert!(table.contains("`main-axis-overflow`"));
    assert!(table.contains("`cross-axis-overflow`"));
    assert!(table.contains("`invalid-size`"));
    assert!(table.contains("`invisible-without-collapse-label`"));
}

#[test]
fn clean_debug_report_has_empty_summary_and_plain_markdown_message() {
    let solved = solved_slot("root", 100.0, 40.0);
    let report = solved.debug_report();

    assert!(report.summary().is_clean());
    assert_eq!(report.warnings_markdown_table(), "No layout warnings.");
}

#[test]
fn as_map_builds_flat_id_index() {
    let solved = SolvedNode {
        id: "root",
        width: 100.0,
        height: 100.0,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(Axis::Horizontal),
        children: vec![
            solved_slot("a", 50.0, 100.0),
            SolvedNode {
                id: "b",
                width: 50.0,
                height: 100.0,
                visible: true,
                active_tier: None,
                collapse_label: None,
                resolved_axis: None,
                children: vec![solved_slot("b1", 50.0, 50.0)],
            },
        ],
    };

    let map = solved.as_map();
    assert_eq!(map.len(), 4);
    assert_eq!(map.get("root").unwrap().width, 100.0);
    assert_eq!(map.get("a").unwrap().width, 50.0);
    assert_eq!(map.get("b").unwrap().width, 50.0);
    assert_eq!(map.get("b1").unwrap().width, 50.0);
    assert!(!map.contains_key("missing"));
}

#[test]
fn debug_report_warning_ids_borrow_from_solved_tree() {
    let solved = SolvedNode {
        id: "root",
        width: 100.0,
        height: 40.0,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(Axis::Horizontal),
        children: vec![SolvedNode {
            id: "wide",
            width: 75.0,
            height: 45.0,
            visible: true,
            active_tier: None,
            collapse_label: None,
            resolved_axis: None,
            children: Vec::new(),
        }],
    };

    let report = solved.debug_report();
    let warning = report
        .warnings()
        .iter()
        .find(|w| matches!(w.kind, LayoutDebugWarningKind::CrossAxisOverflow { .. }))
        .expect("expected a cross-axis overflow warning");

    assert_eq!(warning.node_id, "root");
    assert!(std::ptr::eq(warning.node_id.as_ptr(), solved.id.as_ptr()));
    match &warning.kind {
        LayoutDebugWarningKind::CrossAxisOverflow { child_id, .. } => {
            assert!(std::ptr::eq(
                child_id.as_ptr(),
                solved.children[0].id.as_ptr()
            ));
        }
        _ => unreachable!(),
    }
}

// ===== Flat SolvedTree parity tests =====

fn sample_layout_tree() -> LayoutNode<'static> {
    let inner_children: &'static [LayoutNode<'static>] = Box::leak(
        vec![
            LayoutNode::Slot(SlotNode {
                id: "a",
                sizing: Sizing::flex(0.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "b",
                sizing: Sizing::flex(0.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
        ]
        .into_boxed_slice(),
    );
    let children: &'static [LayoutNode<'static>] = Box::leak(
        vec![
            LayoutNode::Slot(SlotNode {
                id: "header",
                sizing: Sizing::Fixed(50.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Container(ContainerNode {
                id: "content",
                axis: Axis::Horizontal,
                auto_axis: None,
                sizing: Sizing::flex(0.0),
                children: inner_children,
                divider_size: 0.0,
            }),
            LayoutNode::Slot(SlotNode {
                id: "footer",
                sizing: Sizing::Fixed(80.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
        ]
        .into_boxed_slice(),
    );
    LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Vertical,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children,
        divider_size: 0.0,
    })
}

fn collect_ids_recursive<'a>(node: &'a SolvedNode<'a>) -> Vec<&'a str> {
    let mut ids = vec![node.id];
    for child in &node.children {
        ids.extend(collect_ids_recursive(child));
    }
    ids
}

#[test]
fn flat_tree_finds_every_id_recursive_find_finds() {
    let root = sample_layout_tree();
    let recursive = crate::solver::solve(&root, 1000.0, 800.0, &LayoutPreferences::default());
    let flat = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    let expected_ids = collect_ids_recursive(&recursive);
    assert_eq!(flat.len(), expected_ids.len());

    for id in &expected_ids {
        let found = flat.find(id).expect("flat tree should find {id}");
        assert_eq!(found.id(), *id);
    }
}

#[test]
fn flat_tree_iteration_matches_dfs_order() {
    let root = sample_layout_tree();
    let recursive = crate::solver::solve(&root, 1000.0, 800.0, &LayoutPreferences::default());
    let flat = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    let expected_order = collect_ids_recursive(&recursive);
    let actual_order: Vec<&str> = flat.iter().map(|node| node.id()).collect();
    assert_eq!(actual_order, expected_order);
}

#[test]
fn flat_tree_into_tree_matches_recursive_solve() {
    let root = sample_layout_tree();
    let recursive = crate::solver::solve(&root, 1000.0, 800.0, &LayoutPreferences::default());
    let via_into_tree: SolvedTree = recursive.clone().into_tree();
    let direct = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    let expected_order = collect_ids_recursive(&recursive);
    let into_order: Vec<&str> = via_into_tree.iter().map(|node| node.id()).collect();
    let direct_order: Vec<&str> = direct.iter().map(|node| node.id()).collect();
    assert_eq!(into_order, expected_order);
    assert_eq!(direct_order, expected_order);
}

#[test]
fn flat_tree_debug_report_matches_recursive_debug_report() {
    let root = sample_layout_tree();
    let recursive = crate::solver::solve(&root, 1000.0, 800.0, &LayoutPreferences::default());
    let flat = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    let recursive_report = recursive.debug_report();
    let flat_report = flat.debug_report();

    assert_eq!(flat_report.tree(), recursive_report.tree());
    assert_eq!(flat_report.warnings(), recursive_report.warnings());
}

#[test]
fn flat_tree_collapsed_tabs_match_recursive_collapsed_tabs() {
    let children: &'static [LayoutNode<'static>] = Box::leak(
        vec![
            LayoutNode::Slot(SlotNode {
                id: "config",
                sizing: Sizing::fractional(0.2, 100.0),
                priority: 0.5,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Config"),
            }),
            LayoutNode::Slot(SlotNode {
                id: "main",
                sizing: Sizing::flex(300.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "output",
                sizing: Sizing::fractional(0.2, 120.0),
                priority: 0.6,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Output"),
            }),
        ]
        .into_boxed_slice(),
    );
    let root = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Horizontal,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children,
        divider_size: 0.0,
    });

    let recursive = crate::solver::solve(&root, 250.0, 600.0, &LayoutPreferences::default());
    let flat = solve_tree(&root, 250.0, 600.0, &LayoutPreferences::default());

    let mut recursive_tabs = recursive.collapsed_tabs();
    let mut flat_tabs = flat.collapsed_tabs();
    recursive_tabs.sort_by_key(|(id, _)| *id);
    flat_tabs.sort_by_key(|(id, _)| *id);
    assert_eq!(flat_tabs, recursive_tabs);
}

#[test]
fn collapsed_slots_preserve_stable_ids_and_declaration_order() {
    let children = [
        LayoutNode::Slot(SlotNode::new("first", Sizing::Fixed(100.0)).collapsible(0.5, "First")),
        LayoutNode::slot("primary", Sizing::Fixed(200.0)),
        LayoutNode::Slot(SlotNode::new("last", Sizing::Fixed(100.0)).collapsible(0.5, "Last")),
    ];
    let root =
        ContainerNode::new("root", Axis::Horizontal, Sizing::flex(0.0), &children).into_node();
    let solved = solve_tree(&root, 200.0, 400.0, &LayoutPreferences::default());

    let collapsed: Vec<_> = solved.collapsed_slots().collect();
    assert_eq!(
        collapsed,
        vec![
            CollapsedSlot {
                id: "first",
                label: "First",
            },
            CollapsedSlot {
                id: "last",
                label: "Last",
            },
        ]
    );
    assert_eq!(
        solved.collapsed_tabs(),
        collapsed
            .iter()
            .map(|slot| (slot.id, slot.label))
            .collect::<Vec<_>>()
    );
}

#[test]
fn flat_tree_as_map_get_matches_find() {
    let root = sample_layout_tree();
    let flat = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    let map = flat.as_map();
    for node in flat.iter() {
        let from_map = map.get(node.id()).expect("id in map");
        assert_eq!(from_map.id, node.id());
        assert_eq!(from_map.width, node.width());
        assert_eq!(from_map.height, node.height());
    }
}

#[test]
fn as_map_reuses_the_retained_lookup_index() {
    let root = sample_layout_tree();
    let flat = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    assert_eq!(
        flat.cached_index_len(),
        flat.len(),
        "the retained lookup index should be populated by the solver"
    );

    let first = flat.as_map();
    let cached_len = flat.cached_index_len();
    assert!(cached_len > 0, "as_map should populate the cached index");
    assert_eq!(cached_len, first.len());

    let second = flat.as_map();
    assert_eq!(
        flat.cached_index_len(),
        cached_len,
        "second as_map should reuse the cached index"
    );
    assert_eq!(second.len(), first.len());
    for (id, node) in &first {
        assert_eq!(second.get(id).unwrap().width, node.width);
    }
}

#[test]
fn iter_by_id_matches_find_without_materializing_a_map() {
    let root = sample_layout_tree();
    let flat = solve_tree(&root, 1000.0, 800.0, &LayoutPreferences::default());

    for (id, node) in flat.iter_by_id() {
        let found = flat.find(id).expect("id in retained index");
        assert_eq!(found.id(), node.id());
        assert_eq!(found.width(), node.width());
    }
}

#[test]
fn solved_tree_ref_accessors_and_root() {
    static TIERS: &[DisplayTier<'_>] = &[DisplayTier {
        name: "Full",
        min_size: 0.0,
    }];

    let children = [
        LayoutNode::Slot(SlotNode {
            id: "header",
            sizing: Sizing::Fixed(40.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
        LayoutNode::Slot(SlotNode {
            id: "rack",
            sizing: Sizing::flex(0.0),
            priority: 0.5,
            collapsible: true,
            display_tiers: TIERS,
            collapse_label: Some("Rack"),
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
    let prefs = LayoutPreferences::new(&[], &[("rack", true)]);
    let tree = solve_tree(&root, 200.0, 100.0, &prefs);

    let root_ref = tree.root();
    assert_eq!(root_ref.id(), "root");
    assert_eq!(root_ref.width(), 200.0);
    assert_eq!(root_ref.height(), 100.0);
    assert!(root_ref.visible());
    assert_eq!(root_ref.resolved_axis(), Some(Axis::Horizontal));

    let kids: Vec<_> = root_ref.children().collect();
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].id(), "header");
    assert_eq!(kids[0].size_along(Axis::Horizontal), kids[0].width());
    assert_eq!(kids[0].size_along(Axis::Vertical), kids[0].height());
    assert_eq!(kids[1].id(), "rack");
    assert!(!kids[1].visible());
    assert_eq!(kids[1].collapse_label(), Some("Rack"));
    assert_eq!(kids[1].active_tier(), None);
}

#[test]
fn debug_report_with_source_includes_tier_label_and_sizing() {
    struct FixedMeasure;
    impl gpui_pretext::TextMeasure for FixedMeasure {
        fn measure_width(&self, text: &str) -> f64 {
            text.chars().count() as f64 * 10.0
        }
    }
    static MEASURE: FixedMeasure = FixedMeasure;
    static TIERS: &[DisplayTier<'_>] = &[DisplayTier {
        name: "Full",
        min_size: 0.0,
    }];

    let children = [
        LayoutNode::Slot(SlotNode {
            id: "text",
            sizing: Sizing::Text {
                text: "hi",
                measure: &MEASURE,
                line_height: 20.0,
                min: 0.0,
            },
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }),
        LayoutNode::Slot(SlotNode {
            id: "rack",
            sizing: Sizing::Fractional {
                initial: 0.25,
                min: 10.0,
                max: 100.0,
            },
            priority: 0.4,
            collapsible: true,
            display_tiers: TIERS,
            collapse_label: Some("Rack"),
        }),
        LayoutNode::Slot(SlotNode {
            id: "side",
            sizing: Sizing::Fixed(30.0),
            priority: 0.2,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Side"),
        }),
    ];
    let source = LayoutNode::Container(ContainerNode {
        id: "root",
        axis: Axis::Horizontal,
        auto_axis: None,
        sizing: Sizing::flex(0.0),
        children: &children,
        divider_size: 0.0,
    });

    let tree = solve_tree(
        &source,
        200.0,
        50.0,
        &LayoutPreferences::new(&[], &[("side", true)]),
    );
    let report = tree.debug_report_with_source(&source);
    let text = report.tree();

    assert!(text.contains("tier=Full"), "{text}");
    assert!(text.contains("label=\"Side\""), "{text}");
    assert!(
        text.contains("sizing=Text(chars=2,line_height=20,min=0)"),
        "{text}"
    );
    assert!(
        text.contains("sizing=Fractional(initial=0.25,min=10,max=100)"),
        "{text}"
    );
    assert!(text.contains("collapsible priority=0.4"), "{text}");
}

#[test]
fn debug_report_handles_infinite_cross_axis() {
    use super::solved_tree::{NodeIndex, SolvedNodeData};
    use std::collections::HashMap;

    let tree = SolvedTree::from_parts(
        vec![
            SolvedNodeData {
                id: "root",
                width: 100.0,
                height: f32::INFINITY,
                visible: true,
                active_tier: None,
                collapse_label: None,
                resolved_axis: Some(Axis::Horizontal),
                children: vec![NodeIndex(1)],
            },
            SolvedNodeData {
                id: "child",
                width: 50.0,
                height: 20.0,
                visible: true,
                active_tier: None,
                collapse_label: None,
                resolved_axis: None,
                children: vec![],
            },
        ],
        HashMap::from([("root", NodeIndex(0)), ("child", NodeIndex(1))]),
    );

    let report = tree.debug_report();

    assert!(report.has_warnings());
    assert!(
        report
            .warnings()
            .iter()
            .any(|w| matches!(w.kind, LayoutDebugWarningKind::InvalidSize { .. })),
        "expected invalid-size warning for infinite height"
    );
    assert!(
        !report
            .warnings()
            .iter()
            .any(|w| matches!(w.kind, LayoutDebugWarningKind::CrossAxisOverflow { .. })),
        "infinite cross axis should not produce overflow"
    );
}
