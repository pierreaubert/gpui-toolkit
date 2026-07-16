use gpui_builder::{
    Axis, ContainerNode, LayoutNode, LayoutPreferences, Sizing, SlotNode, solve, solve_tree,
};
use proptest::prelude::*;

fn sizing(kind: u8, primary: u16, secondary: u8) -> Sizing<'static> {
    match kind % 3 {
        0 => Sizing::Fixed(f32::from(primary)),
        1 => Sizing::fractional(f32::from(primary % 101) / 100.0, f32::from(secondary)),
        _ => Sizing::Flex {
            min: f32::from(primary),
            weight: f32::from(secondary.max(1)),
        },
    }
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.001
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn proptest_recursive_and_flat_solvers_match(
        width in 1_u16..=2_000,
        height in 1_u16..=1_500,
        divider_size in 0_u8..=20,
        specs in prop::collection::vec(
            (0_u8..=2, 0_u16..=400, 1_u8..=16, any::<bool>(), 0_u8..=100),
            1..=48,
        ),
    ) {
        let ids: Vec<String> = (0..specs.len()).map(|index| format!("child-{index}")).collect();
        let children: Vec<LayoutNode<'_>> = specs
            .iter()
            .zip(&ids)
            .map(|(&(kind, primary, secondary, collapsible, priority), id)| {
                let slot = SlotNode::new(id, sizing(kind, primary, secondary))
                    .priority(f32::from(priority) / 100.0);
                if collapsible {
                    slot.collapsible(f32::from(priority) / 100.0, id).into_node()
                } else {
                    slot.into_node()
                }
            })
            .collect();
        let root = ContainerNode::new(
            "root",
            Axis::Horizontal,
            Sizing::flex(0.0),
            &children,
        )
        .divider_size(f32::from(divider_size))
        .into_node();
        let preferences = LayoutPreferences::default();

        let recursive = solve(
            &root,
            f32::from(width),
            f32::from(height),
            &preferences,
        );
        let flat = solve_tree(
            &root,
            f32::from(width),
            f32::from(height),
            &preferences,
        );

        prop_assert!(close(recursive.width, flat.root().width()));
        prop_assert!(close(recursive.height, flat.root().height()));
        prop_assert_eq!(recursive.visible, flat.root().visible());

        for id in &ids {
            let recursive_node = recursive.find(id).expect("recursive node");
            let flat_node = flat.find(id).expect("flat node");
            prop_assert!(recursive_node.width.is_finite());
            prop_assert!(recursive_node.height.is_finite());
            prop_assert!(recursive_node.width >= 0.0);
            prop_assert!(recursive_node.height >= 0.0);
            prop_assert!(close(recursive_node.width, flat_node.width()));
            prop_assert!(close(recursive_node.height, flat_node.height()));
            prop_assert_eq!(recursive_node.visible, flat_node.visible());
            prop_assert_eq!(recursive_node.active_tier, flat_node.active_tier());
        }
    }

    #[test]
    fn proptest_zero_minimum_flexible_layout_never_overallocates(
        width in 1_u16..=2_000,
        divider_size in 0_u8..=20,
        specs in prop::collection::vec((any::<bool>(), 0_u8..=100, 1_u8..=16), 1..=64),
    ) {
        prop_assume!(
            f32::from(width)
                >= f32::from(divider_size) * specs.len().saturating_sub(1) as f32
        );
        let ids: Vec<String> = (0..specs.len()).map(|index| format!("flex-{index}")).collect();
        let children: Vec<LayoutNode<'_>> = specs
            .iter()
            .zip(&ids)
            .map(|(&(fractional, ratio, weight), id)| {
                let sizing = if fractional {
                    Sizing::fractional(f32::from(ratio) / 100.0, 0.0)
                } else {
                    Sizing::Flex {
                        min: 0.0,
                        weight: f32::from(weight),
                    }
                };
                LayoutNode::slot(id, sizing)
            })
            .collect();
        let divider = f32::from(divider_size);
        let root = ContainerNode::new(
            "root",
            Axis::Horizontal,
            Sizing::flex(0.0),
            &children,
        )
        .divider_size(divider)
        .into_node();

        let solved = solve(
            &root,
            f32::from(width),
            100.0,
            &LayoutPreferences::default(),
        );
        let visible: Vec<_> = solved.children.iter().filter(|node| node.visible).collect();
        let child_width: f32 = visible.iter().map(|node| node.width).sum();
        let divider_width = divider * visible.len().saturating_sub(1) as f32;

        prop_assert!(child_width + divider_width <= f32::from(width) + 0.01);
    }
}
