#[cfg(test)]
mod tests {
    use d3rs::force::{ForceManyBody, Simulation, SimulationNode};
    use d3rs::hierarchy::{HierarchyNode, TreeLayout};

    #[test]
    fn test_hierarchy_golden() {
        // Create a known hierarchy
        // Root -> A, B
        let root = HierarchyNode::new("Root".to_string());
        let a = HierarchyNode::new("A".to_string());
        let b = HierarchyNode::new("B".to_string());

        {
            let mut r = root.borrow_mut();
            r.set_children(&root, vec![a.clone(), b.clone()]);
        }

        // Count leaves (required for cluster layout spacing in current impl)
        HierarchyNode::count(root.clone());

        // Layout with fixed size
        let layout = TreeLayout::new().size((100.0, 100.0));
        layout.layout(root.clone());

        // Verify coordinates against expected values
        let r = root.borrow();
        let a_node = a.borrow();
        let b_node = b.borrow();

        println!("Root: ({}, {})", r.x, r.y);
        println!("A: ({}, {})", a_node.x, a_node.y);
        println!("B: ({}, {})", b_node.x, b_node.y);

        // Root should be centered vertically if children are spread
        // In current cluster layout:
        // Leaves (A, B) get x=0 and x=1 (mapped to Y in implementation due to horizontal swap)
        // Root gets average x=0.5
        // Then scaling applies.

        // This is a "golden" test in the sense that we pin the behavior.
        // If layout algorithm changes, these assertions need update.
        // Current implementation is a simplified cluster layout.

        // X is depth (0 or 1) * scale
        // Y is accumulated index * scale

        // Depth 0 -> X = 0
        // Depth 1 -> X = 100

        assert_eq!(r.depth, 0);
        assert_eq!(a_node.depth, 1);

        // Check structural constraints rather than exact floats to be robust against minor float math changes
        assert!(r.x < a_node.x); // Root is to the left of children
        assert!(r.x < b_node.x);

        assert!(a_node.y != b_node.y); // Children are separated vertically
        assert!(r.y >= a_node.y.min(b_node.y) && r.y <= a_node.y.max(b_node.y));
        // Root y is between children
    }

    #[test]
    fn test_force_simulation_golden() {
        // Two nodes, connected by many-body force (repulsion)
        // They should move apart.

        let n1 = SimulationNode::new(0, 48.0, 50.0);
        let n2 = SimulationNode::new(1, 52.0, 50.0);

        let mut sim =
            Simulation::new(vec![n1.clone(), n2.clone()]).force(Box::new(ForceManyBody::new())); // Default strength -30 (repulsion)

        // Tick multiple times
        for _ in 0..10 {
            sim.tick();
        }

        let n1_rf = n1.borrow();
        let n2_rf = n2.borrow();

        println!("N1 x: {}, N2 x: {}", n1_rf.x, n2_rf.x);

        // They started at 48 and 52 (center 50).
        // Repulsion should push N1 < 48 and N2 > 52.
        assert!(n1_rf.x < 48.0);
        assert!(n2_rf.x > 52.0);
    }
}
