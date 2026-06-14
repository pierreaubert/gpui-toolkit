use super::*;

#[test]
fn snapshot_flattens_accessible_nodes() {
    let snapshot = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container).child(
            IosAccessibilityNode::new("play", IosAccessibilityRole::Button)
                .label("Play")
                .action(IosAccessibilityAction::Activate),
        ),
    );

    let nodes = snapshot.flattened_nodes();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, "play");
}

#[test]
fn invalid_frames_are_rejected() {
    let snapshot = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("bad", IosAccessibilityRole::Button)
            .label("Bad")
            .frame(IosAccessibilityFrame {
                x: 0.0,
                y: 0.0,
                width: f32::NAN,
                height: 20.0,
            }),
    );

    assert!(snapshot.validate().is_err());
}

#[test]
fn action_callback_dispatches_node_actions() {
    set_accessibility_action_callback(Some(Box::new(|id, action| {
        id == "volume" && action == IosAccessibilityAction::Increment
    })));

    assert!(dispatch_accessibility_action(
        "volume",
        IosAccessibilityAction::Increment
    ));
    assert!(!dispatch_accessibility_action(
        "volume",
        IosAccessibilityAction::Decrement
    ));

    set_accessibility_action_callback(None);
}

#[test]
fn snapshot_is_shared_via_arc() {
    clear_accessibility_snapshot();
    let snapshot = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container).child(
            IosAccessibilityNode::new("play", IosAccessibilityRole::Button)
                .label("Play")
                .frame(IosAccessibilityFrame {
                    x: 0.0,
                    y: 0.0,
                    width: 44.0,
                    height: 44.0,
                }),
        ),
    );
    set_accessibility_snapshot(snapshot).unwrap();

    let first = accessibility_snapshot().unwrap();
    let second = accessibility_snapshot().unwrap();
    assert!(Arc::ptr_eq(&first, &second));

    clear_accessibility_snapshot();
}

fn button(id: &str, label: &str) -> IosAccessibilityNode {
    IosAccessibilityNode::new(id, IosAccessibilityRole::Button)
        .label(label)
        .frame(IosAccessibilityFrame {
            x: 0.0,
            y: 0.0,
            width: 44.0,
            height: 44.0,
        })
}

#[test]
fn diff_identical_snapshots_is_empty() {
    let prev = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A"))
            .child(button("b", "B")),
    );
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A"))
            .child(button("b", "B")),
    );

    let diff = compute_accessibility_diff(Some(&prev), &next);
    assert_eq!(diff.unchanged.len(), 2);
    assert!(diff.changed.is_empty());
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(!diff.order_changed);
}

#[test]
fn diff_property_change_only() {
    let prev = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A"))
            .child(button("b", "B")),
    );
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A changed"))
            .child(button("b", "B")),
    );

    let diff = compute_accessibility_diff(Some(&prev), &next);
    assert_eq!(diff.unchanged.len(), 1);
    assert_eq!(diff.changed.len(), 1);
    assert_eq!(diff.changed[0].0.id, "a");
    assert!(diff.changed[0].1.label_changed);
    assert!(!diff.changed[0].1.frame_changed);
    assert!(!diff.changed[0].1.traits_changed);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(!diff.order_changed);
}

#[test]
fn diff_adds_removes_and_reorders() {
    let prev = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A"))
            .child(button("b", "B")),
    );
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("c", "C"))
            .child(button("a", "A")),
    );

    let diff = compute_accessibility_diff(Some(&prev), &next);
    assert_eq!(diff.unchanged.len(), 1);
    assert_eq!(diff.unchanged[0].id, "a");
    assert!(diff.changed.is_empty());
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].id, "c");
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0], "b");
    assert!(diff.order_changed);
}

#[test]
fn diff_detects_frame_change() {
    let prev = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A")),
    );
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container).child(
            IosAccessibilityNode::new("a", IosAccessibilityRole::Button)
                .label("A")
                .frame(IosAccessibilityFrame {
                    x: 10.0,
                    y: 0.0,
                    width: 44.0,
                    height: 44.0,
                }),
        ),
    );

    let diff = compute_accessibility_diff(Some(&prev), &next);
    assert_eq!(diff.changed.len(), 1);
    assert!(diff.changed[0].1.frame_changed);
    assert!(!diff.order_changed);
}

#[test]
fn diff_detects_traits_change() {
    let prev = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A")),
    );
    let mut next_node = button("a", "A");
    next_node.enabled = false;
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container).child(next_node),
    );

    let diff = compute_accessibility_diff(Some(&prev), &next);
    assert_eq!(diff.changed.len(), 1);
    assert!(diff.changed[0].1.traits_changed);
    assert!(!diff.changed[0].1.label_changed);
}

#[test]
fn diff_first_snapshot_treats_all_nodes_as_added() {
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A")),
    );

    let diff = compute_accessibility_diff(None, &next);
    assert_eq!(diff.added.len(), 1);
    assert!(diff.unchanged.is_empty());
    assert!(diff.order_changed);
}

#[test]
fn compute_accessibility_diff_reuses_flattened_map() {
    let prev = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A"))
            .child(button("b", "B")),
    );
    let next = IosAccessibilitySnapshot::new(
        IosAccessibilityNode::new("root", IosAccessibilityRole::Container)
            .child(button("a", "A"))
            .child(button("b", "B")),
    );

    assert!(!next.is_flattened_cached());
    assert!(!next.is_id_index_cached());

    let diff1 = compute_accessibility_diff(Some(&prev), &next);
    assert!(next.is_flattened_cached());
    assert!(next.is_id_index_cached());
    assert_eq!(diff1.unchanged.len(), 2);

    let diff2 = compute_accessibility_diff(Some(&prev), &next);
    assert_eq!(diff1, diff2);
}
