use super::child_info::allocate_main_axis;
use super::misc::{TextMeasureCache, TextSizeInput, compute_text_size, default_text_cache};
use super::resolve::resolve_axis;
use super::resolve::resolve_display_tier;
use super::types::ChildInfo;
use crate::solved::{NodeIndex, SolvedNode, SolvedNodeData, SolvedTree};
use crate::types::{Axis, ContainerNode, LayoutNode, LayoutPreferences, Sizing, SlotNode};
use gpui_pretext::{EngineProfile, PrepareOptions};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Solve the layout tree into concrete pixel sizes.
///
/// `root` is the declaration tree. `width` and `height` are the available
/// space (typically the window size). `prefs` provides user overrides for
/// ratios and collapsed states.
///
/// This entry point uses a thread-local default text-measurement cache. To
/// share a cache explicitly across calls (or across threads with an
/// `Arc<Mutex<_>>`), use [`solve_with_cache`].
///
/// # Performance note
///
/// This function is allocation-heavy: it builds a fresh recursive
/// [`SolvedNode`] tree on every call, and every container allocates a new
/// `Vec<SolvedNode>` for its children. For frame-rate layout work, prefer
/// [`solve_tree_into`] / [`solve_tree_into_with_cache`], which retain the flat
/// arena, id index, child-index buffers, and text cache across calls.
pub fn solve<'a>(
    root: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
) -> SolvedNode<'a> {
    solve_with_cache(root, width, height, prefs, default_text_cache())
}

/// Solve the layout tree with an explicit text-measurement cache.
///
/// The cache is keyed by `(text, measure, cross_size, line_height, axis)` and
/// stores both the measured size and the intermediate [`gpui_pretext::PreparedText`]
/// so repeated layouts of the same text avoid re-running text analysis.
///
/// # Performance note
///
/// Like [`solve`], this function builds a fresh recursive [`SolvedNode`] tree
/// with a per-container child vector on every call. For frame-rate layout
/// work, prefer [`solve_tree_into_with_cache`], which reuses solver output and
/// avoids those allocations after warm-up.
pub fn solve_with_cache<'a>(
    root: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    cache: Rc<RefCell<TextMeasureCache>>,
) -> SolvedNode<'a> {
    solve_node(root, width, height, prefs, &cache)
}

/// Solve the layout tree directly into an arena/flat [`SolvedTree`].
///
/// This is the same solver logic as [`solve`], but nodes are appended to a
/// single `Vec` as they are resolved and parent/child relationships are stored
/// as indices. The resulting tree supports O(1) id lookup and cache-friendly
/// traversal.
///
/// This entry point uses a thread-local default text-measurement cache. To
/// share a cache explicitly, use [`solve_tree_with_cache`].
pub fn solve_tree<'a>(
    root: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
) -> SolvedTree<'a> {
    solve_tree_with_cache(root, width, height, prefs, default_text_cache())
}

/// Solve the layout tree into a [`SolvedTree`] with an explicit cache.
pub fn solve_tree_with_cache<'a>(
    root: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    cache: Rc<RefCell<TextMeasureCache>>,
) -> SolvedTree<'a> {
    let estimated = root.node_count();
    let mut tree = SolvedTree::with_capacity(estimated);
    solve_tree_into_with_cache(root, width, height, prefs, cache, &mut tree);
    tree
}

/// Re-solve into an existing flat tree while retaining arena, index, and
/// container child-buffer capacity.
///
/// Warm this path once before a frame/event allocation measurement. The
/// source tree and all borrowed ids must use the same lifetime as `target`.
pub fn solve_tree_into<'a>(
    root: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    target: &mut SolvedTree<'a>,
) {
    solve_tree_into_with_cache(root, width, height, prefs, default_text_cache(), target);
}

/// Re-solve into reusable storage with an explicit text-measurement cache.
pub fn solve_tree_into_with_cache<'a>(
    root: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    cache: Rc<RefCell<TextMeasureCache>>,
    target: &mut SolvedTree<'a>,
) {
    let estimated = root.node_count();
    target.prepare_for_reuse(estimated);
    let (nodes, index, child_index_pool) = target.reusable_parts();
    let mut storage = TreeSolveStorage {
        nodes,
        index,
        child_index_pool,
        cache: &cache,
    };
    solve_tree_node(root, width, height, prefs, &mut storage);
}

struct TreeSolveStorage<'storage, 'a> {
    nodes: &'storage mut Vec<SolvedNodeData<'a>>,
    index: &'storage mut HashMap<&'a str, NodeIndex>,
    child_index_pool: &'storage mut Vec<Vec<NodeIndex>>,
    cache: &'storage Rc<RefCell<TextMeasureCache>>,
}

// Reusable scratch buffer pool for per-container child-info vectors.
//
// Layout solving is recursive, so a single flat buffer would be clobbered by
// nested containers. The pool holds one vector per recursion level; vectors are
// cleared and returned to the pool after use.
thread_local! {
    static CHILD_INFO_POOL: RefCell<Vec<Vec<ChildInfo>>> = const { RefCell::new(Vec::new()) };
}

// Maximum number of child-info vectors to keep in the pool. Layout trees are
// rarely deeper than a handful of levels, so this cap avoids unbounded growth
// while still eliminating allocations for typical cases.
const CHILD_INFO_POOL_CAP: usize = 16;

/// Take a cleared child-info vector from the thread-local pool, allocating a
/// new one only when the pool is empty.
fn take_child_info_scratch() -> Vec<ChildInfo> {
    CHILD_INFO_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        let mut vec = pool.pop().unwrap_or_default();
        vec.clear();
        vec
    })
}

/// Return a child-info vector to the thread-local pool for reuse.
fn return_child_info_scratch(mut vec: Vec<ChildInfo>) {
    vec.clear();
    CHILD_INFO_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        if pool.len() < CHILD_INFO_POOL_CAP {
            pool.push(vec);
        }
    })
}

fn solve_tree_node<'a>(
    node: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    storage: &mut TreeSolveStorage<'_, 'a>,
) -> NodeIndex {
    match node {
        LayoutNode::Slot(slot) => {
            solve_tree_slot(slot, width, height, prefs, storage.nodes, storage.index)
        }
        LayoutNode::Container(container) => {
            solve_tree_container(container, width, height, prefs, storage)
        }
    }
}

fn solve_tree_slot<'a>(
    slot: &SlotNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    nodes: &mut Vec<SolvedNodeData<'a>>,
    index: &mut HashMap<&'a str, NodeIndex>,
) -> NodeIndex {
    let user_collapsed = slot.collapsible && prefs.is_collapsed(slot.id);

    if user_collapsed {
        return push_solved_node(
            SolvedNodeData {
                id: slot.id,
                width: 0.0,
                height: 0.0,
                visible: false,
                active_tier: None,
                collapse_label: slot.collapse_label,
                resolved_axis: None,
                children: Vec::new(),
            },
            nodes,
            index,
        );
    }

    let active_tier = resolve_display_tier(slot, width);

    push_solved_node(
        SolvedNodeData {
            id: slot.id,
            width,
            height,
            visible: true,
            active_tier,
            collapse_label: slot.collapse_label,
            resolved_axis: None,
            children: Vec::new(),
        },
        nodes,
        index,
    )
}

fn solve_tree_container<'a>(
    container: &ContainerNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    storage: &mut TreeSolveStorage<'_, 'a>,
) -> NodeIndex {
    let axis = resolve_axis(container, width, height);

    let main_size = match axis {
        Axis::Horizontal => width,
        Axis::Vertical => height,
    };
    let cross_size = match axis {
        Axis::Horizontal => height,
        Axis::Vertical => width,
    };

    let profile = EngineProfile::default();
    let options = PrepareOptions::default();

    let mut child_infos: Vec<ChildInfo> = take_child_info_scratch();
    child_infos.extend(
        container
            .children
            .iter()
            .enumerate()
            .map(|(node_index, child)| {
                let user_collapsed = child.collapsible() && prefs.is_collapsed(child.id());
                let computed_text_size = if !user_collapsed {
                    if let Sizing::Text {
                        text,
                        measure,
                        line_height,
                        min,
                    } = child.sizing()
                    {
                        Some(compute_text_size(
                            TextSizeInput {
                                text,
                                measure,
                                line_height,
                                min,
                                axis,
                                cross_size,
                                profile: &profile,
                                options: &options,
                            },
                            storage.cache,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };
                ChildInfo {
                    node_index,
                    user_collapsed,
                    solver_collapsed: false,
                    allocated_size: 0.0,
                    computed_text_size,
                }
            }),
    );

    allocate_main_axis(
        &mut child_infos,
        container.children,
        main_size,
        container.divider_size,
        axis,
        prefs,
    );

    // Reserve the container node first so children are stored after it in
    // pre-order DFS order.
    let container_idx = NodeIndex(storage.nodes.len());
    storage.index.insert(container.id, container_idx);
    storage.nodes.push(SolvedNodeData {
        id: container.id,
        width,
        height,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(axis),
        // Filled from a recycled child-index buffer after descendants solve.
        children: Vec::new(),
    });

    let mut child_indices = storage.child_index_pool.pop().unwrap_or_default();
    child_indices.clear();
    child_indices.reserve(child_infos.len());
    for info in &child_infos {
        let node = &container.children[info.node_index];
        let visible = !info.user_collapsed && !info.solver_collapsed;
        if !visible {
            let collapse_label = match node {
                LayoutNode::Slot(s) => s.collapse_label,
                LayoutNode::Container(_) => None,
            };
            let child_idx = push_solved_node(
                SolvedNodeData {
                    id: node.id(),
                    width: 0.0,
                    height: 0.0,
                    visible: false,
                    active_tier: None,
                    collapse_label,
                    resolved_axis: None,
                    children: Vec::new(),
                },
                storage.nodes,
                storage.index,
            );
            child_indices.push(child_idx);
            continue;
        }

        let (child_w, child_h) = match axis {
            Axis::Horizontal => (info.allocated_size, cross_size),
            Axis::Vertical => (cross_size, info.allocated_size),
        };

        match node {
            LayoutNode::Slot(slot) => {
                let active_tier = resolve_display_tier(slot, info.allocated_size);
                let child_idx = push_solved_node(
                    SolvedNodeData {
                        id: slot.id,
                        width: child_w,
                        height: child_h,
                        visible: true,
                        active_tier,
                        collapse_label: slot.collapse_label,
                        resolved_axis: None,
                        children: Vec::new(),
                    },
                    storage.nodes,
                    storage.index,
                );
                child_indices.push(child_idx);
            }
            LayoutNode::Container(_) => {
                let child_idx = solve_tree_node(node, child_w, child_h, prefs, storage);
                child_indices.push(child_idx);
            }
        }
    }

    return_child_info_scratch(child_infos);

    storage.nodes[container_idx.0].children = child_indices;
    container_idx
}

fn push_solved_node<'a>(
    data: SolvedNodeData<'a>,
    nodes: &mut Vec<SolvedNodeData<'a>>,
    index: &mut HashMap<&'a str, NodeIndex>,
) -> NodeIndex {
    let idx = NodeIndex(nodes.len());
    index.insert(data.id, idx);
    nodes.push(data);
    idx
}

fn solve_node<'a>(
    node: &LayoutNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    cache: &Rc<RefCell<TextMeasureCache>>,
) -> SolvedNode<'a> {
    match node {
        LayoutNode::Slot(slot) => solve_slot(slot, width, height, prefs),
        LayoutNode::Container(container) => solve_container(container, width, height, prefs, cache),
    }
}

fn solve_slot<'a>(
    slot: &SlotNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
) -> SolvedNode<'a> {
    let user_collapsed = slot.collapsible && prefs.is_collapsed(slot.id);

    if user_collapsed {
        return SolvedNode {
            id: slot.id,
            width: 0.0,
            height: 0.0,
            visible: false,
            active_tier: None,
            collapse_label: slot.collapse_label,
            resolved_axis: None,
            children: Vec::new(),
        };
    }

    // A root slot has no parent main axis. Use its resolved width as the
    // inline size; container children are re-tiered from their allocated
    // parent-axis size below.
    let active_tier = resolve_display_tier(slot, width);

    SolvedNode {
        id: slot.id,
        width,
        height,
        visible: true,
        active_tier,
        collapse_label: slot.collapse_label,
        resolved_axis: None,
        children: Vec::new(),
    }
}

fn solve_container<'a>(
    container: &ContainerNode<'a>,
    width: f32,
    height: f32,
    prefs: &LayoutPreferences<'a>,
    cache: &Rc<RefCell<TextMeasureCache>>,
) -> SolvedNode<'a> {
    // Step 1: Resolve axis
    let axis = resolve_axis(container, width, height);

    let main_size = match axis {
        Axis::Horizontal => width,
        Axis::Vertical => height,
    };
    let cross_size = match axis {
        Axis::Horizontal => height,
        Axis::Vertical => width,
    };

    // Step 2: Classify children, apply user collapse, pre-compute Text sizes
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();

    let mut child_infos: Vec<ChildInfo> = take_child_info_scratch();
    child_infos.extend(
        container
            .children
            .iter()
            .enumerate()
            .map(|(node_index, child)| {
                let user_collapsed = child.collapsible() && prefs.is_collapsed(child.id());
                let computed_text_size = if !user_collapsed {
                    if let Sizing::Text {
                        text,
                        measure,
                        line_height,
                        min,
                    } = child.sizing()
                    {
                        Some(compute_text_size(
                            TextSizeInput {
                                text,
                                measure,
                                line_height,
                                min,
                                axis,
                                cross_size,
                                profile: &profile,
                                options: &options,
                            },
                            cache,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };
                ChildInfo {
                    node_index,
                    user_collapsed,
                    solver_collapsed: false,
                    allocated_size: 0.0,
                    computed_text_size,
                }
            }),
    );

    // Step 3: Allocate main-axis space
    allocate_main_axis(
        &mut child_infos,
        container.children,
        main_size,
        container.divider_size,
        axis,
        prefs,
    );

    // Step 4+5: Build solved children (determine tiers, recurse into containers)
    let children: Vec<SolvedNode<'a>> = child_infos
        .iter()
        .map(|info| {
            let node = &container.children[info.node_index];
            let visible = !info.user_collapsed && !info.solver_collapsed;
            if !visible {
                // Collapsed child
                let collapse_label = match node {
                    LayoutNode::Slot(s) => s.collapse_label,
                    LayoutNode::Container(_) => None,
                };
                return SolvedNode {
                    id: node.id(),
                    width: 0.0,
                    height: 0.0,
                    visible: false,
                    active_tier: None,
                    collapse_label,
                    resolved_axis: None,
                    children: Vec::new(),
                };
            }

            let (child_w, child_h) = match axis {
                Axis::Horizontal => (info.allocated_size, cross_size),
                Axis::Vertical => (cross_size, info.allocated_size),
            };

            match node {
                LayoutNode::Slot(slot) => {
                    let active_tier = resolve_display_tier(slot, info.allocated_size);
                    SolvedNode {
                        id: slot.id,
                        width: child_w,
                        height: child_h,
                        visible: true,
                        active_tier,
                        collapse_label: slot.collapse_label,
                        resolved_axis: None,
                        children: Vec::new(),
                    }
                }
                LayoutNode::Container(_) => solve_node(node, child_w, child_h, prefs, cache),
            }
        })
        .collect();

    return_child_info_scratch(child_infos);

    SolvedNode {
        id: container.id,
        width,
        height,
        visible: true,
        active_tier: None,
        collapse_label: None,
        resolved_axis: Some(axis),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutNode, LayoutPreferences, solve, solve_tree};
    use crate::solved::SolvedNode;
    use crate::types::{Axis, ContainerNode, DisplayTier, Sizing, SlotNode};

    fn simple_slot<'a>(id: &'a str, sizing: Sizing<'a>) -> LayoutNode<'a> {
        LayoutNode::Slot(SlotNode {
            id,
            sizing,
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })
    }

    fn collapsible_slot<'a>(
        id: &'a str,
        sizing: Sizing<'a>,
        priority: f32,
        label: &'a str,
    ) -> LayoutNode<'a> {
        LayoutNode::Slot(SlotNode {
            id,
            sizing,
            priority,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some(label),
        })
    }

    // ===== Basic layout tests =====

    #[test]
    fn single_flex_child_gets_all_space() {
        let children = [simple_slot("main", Sizing::flex(100.0))];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 800.0, 600.0, &LayoutPreferences::default());
        let main = solved.find("main").unwrap();
        assert_eq!(main.width, 800.0);
        assert_eq!(main.height, 600.0);
        assert!(main.visible);
    }

    #[test]
    fn fixed_plus_flex() {
        let children = [
            simple_slot("header", Sizing::Fixed(50.0)),
            simple_slot("content", Sizing::flex(100.0)),
            simple_slot("footer", Sizing::Fixed(80.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());

        let header = solved.find("header").unwrap();
        assert_eq!(header.height, 50.0);
        assert_eq!(header.width, 1200.0); // cross-axis = full

        let content = solved.find("content").unwrap();
        assert_eq!(content.height, 670.0); // 800 - 50 - 80
        assert_eq!(content.width, 1200.0);

        let footer = solved.find("footer").unwrap();
        assert_eq!(footer.height, 80.0);
    }

    #[test]
    fn fractional_children_with_flex_center() {
        let children = [
            simple_slot("left", Sizing::fractional(0.3, 100.0)),
            simple_slot("center", Sizing::flex(200.0)),
            simple_slot("right", Sizing::fractional(0.2, 80.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 1000.0, 600.0, &LayoutPreferences::default());

        let left = solved.find("left").unwrap();
        assert_eq!(left.width, 300.0); // 0.3 * 1000

        let right = solved.find("right").unwrap();
        assert_eq!(right.width, 200.0); // 0.2 * 1000

        let center = solved.find("center").unwrap();
        assert_eq!(center.width, 500.0); // 1000 - 300 - 200
    }

    #[test]
    fn divider_space_reserved() {
        let children = [
            simple_slot("a", Sizing::flex(100.0)),
            simple_slot("b", Sizing::flex(100.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 6.0,
        });

        let solved = solve(&root, 1000.0, 600.0, &LayoutPreferences::default());
        let a = solved.find("a").unwrap();
        let b = solved.find("b").unwrap();

        // Total = a + b + divider = 1000
        let total = a.width + b.width + 6.0;
        assert!(
            (total - 1000.0).abs() < 0.01,
            "total={total}, expected 1000.0"
        );
    }

    // ===== Collapse tests =====

    #[test]
    fn user_collapsed_slot_gets_zero_size() {
        let children = [
            collapsible_slot("sidebar", Sizing::fractional(0.3, 100.0), 0.5, "Sidebar"),
            simple_slot("main", Sizing::flex(200.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let prefs = LayoutPreferences::new(&[], &[("sidebar", true)]);

        let solved = solve(&root, 1000.0, 600.0, &prefs);
        let sidebar = solved.find("sidebar").unwrap();
        assert!(!sidebar.visible);
        assert_eq!(sidebar.width, 0.0);

        let main = solved.find("main").unwrap();
        assert_eq!(main.width, 1000.0);
    }

    #[test]
    fn priority_collapse_when_space_tight() {
        let children = [
            collapsible_slot("config", Sizing::fractional(0.2, 100.0), 0.5, "Config"),
            simple_slot("main", Sizing::flex(300.0)),
            collapsible_slot("output", Sizing::fractional(0.2, 120.0), 0.6, "Output"),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        // 450px: Main needs 300 min, Output needs 120 min = 420, Config needs 100 min = 520
        // Config (priority 0.5) collapses first since 300 + 120 + 100 > 450
        let solved = solve(&root, 450.0, 600.0, &LayoutPreferences::default());

        let config = solved.find("config").unwrap();
        assert!(!config.visible, "Config should collapse (lowest priority)");
        assert_eq!(config.collapse_label, Some("Config"));

        let output = solved.find("output").unwrap();
        assert!(output.visible);

        let main = solved.find("main").unwrap();
        assert!(main.visible);
        assert!(main.width >= 300.0);
    }

    #[test]
    fn equal_priority_collapses_later_declarations_first() {
        let children = [
            collapsible_slot("first", Sizing::Fixed(100.0), 0.5, "First"),
            collapsible_slot("second", Sizing::Fixed(100.0), 0.5, "Second"),
            collapsible_slot("third", Sizing::Fixed(100.0), 0.5, "Third"),
        ];
        let root = LayoutNode::Container(ContainerNode::new(
            "root",
            Axis::Horizontal,
            Sizing::flex(0.0),
            &children,
        ));

        let solved = solve(&root, 200.0, 100.0, &LayoutPreferences::default());
        assert!(solved.find("first").unwrap().visible);
        assert!(solved.find("second").unwrap().visible);
        assert!(!solved.find("third").unwrap().visible);
    }

    #[test]
    fn all_collapsible_collapse_when_very_tight() {
        let children = [
            collapsible_slot("config", Sizing::fractional(0.2, 100.0), 0.5, "Config"),
            simple_slot("main", Sizing::flex(300.0)),
            collapsible_slot("output", Sizing::fractional(0.2, 120.0), 0.6, "Output"),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        // 250px: only Main(300 min) can fit — everything else collapses
        let solved = solve(&root, 250.0, 600.0, &LayoutPreferences::default());
        assert!(!solved.find("config").unwrap().visible);
        assert!(!solved.find("output").unwrap().visible);
        assert!(solved.find("main").unwrap().visible);
    }

    #[test]
    fn collapsed_tabs_collected() {
        let children = [
            collapsible_slot("config", Sizing::fractional(0.2, 100.0), 0.5, "Config"),
            simple_slot("main", Sizing::flex(300.0)),
            collapsible_slot("output", Sizing::fractional(0.2, 120.0), 0.6, "Output"),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 250.0, 600.0, &LayoutPreferences::default());
        let tabs = solved.collapsed_tabs();
        assert_eq!(tabs.len(), 2);
        assert!(
            tabs.iter()
                .any(|(id, label)| *id == "config" && *label == "Config")
        );
        assert!(
            tabs.iter()
                .any(|(id, label)| *id == "output" && *label == "Output")
        );
    }

    // ===== Auto-axis tests =====

    #[test]
    fn auto_axis_switches_based_on_aspect_ratio() {
        let children = [
            simple_slot("a", Sizing::flex(0.0)),
            simple_slot("b", Sizing::flex(0.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal, // default
            auto_axis: Some(1.0),   // switch at w/h ratio 1.0
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        // Wide window → Horizontal
        let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
        assert_eq!(solved.resolved_axis, Some(Axis::Horizontal));

        // Tall window → Vertical
        let solved = solve(&root, 600.0, 800.0, &LayoutPreferences::default());
        assert_eq!(solved.resolved_axis, Some(Axis::Vertical));

        // Square → Vertical (ratio = 1.0, not > threshold)
        let solved = solve(&root, 800.0, 800.0, &LayoutPreferences::default());
        assert_eq!(solved.resolved_axis, Some(Axis::Vertical));
    }

    // ===== Display tier tests =====

    #[test]
    fn display_tiers_resolve_correctly() {
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

        let children = [LayoutNode::Slot(SlotNode {
            id: "rack",
            sizing: Sizing::flex(0.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: TIERS,
            collapse_label: None,
        })];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        // Wide → Full tier
        let solved = solve(&root, 300.0, 600.0, &LayoutPreferences::default());
        assert_eq!(solved.find("rack").unwrap().active_tier, Some("Full"));

        // Medium → Mini tier
        let solved = solve(&root, 150.0, 600.0, &LayoutPreferences::default());
        assert_eq!(solved.find("rack").unwrap().active_tier, Some("Mini"));

        // Tiny → no tier
        let solved = solve(&root, 50.0, 600.0, &LayoutPreferences::default());
        assert_eq!(solved.find("rack").unwrap().active_tier, None);
    }

    #[test]
    fn root_slot_display_tier_uses_width_not_short_height() {
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

        let root = LayoutNode::Slot(SlotNode {
            id: "root-slot",
            sizing: Sizing::flex(0.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: TIERS,
            collapse_label: None,
        });

        let solved = solve(&root, 240.0, 48.0, &LayoutPreferences::default());
        assert_eq!(solved.active_tier, Some("Full"));
    }

    // ===== Preference override tests =====

    #[test]
    fn ratio_preference_overrides_initial() {
        let children = [
            simple_slot("left", Sizing::fractional(0.3, 50.0)),
            simple_slot("right", Sizing::flex(100.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let prefs = LayoutPreferences::new(&[("left", Axis::Horizontal, 0.5)], &[]);

        let solved = solve(&root, 1000.0, 600.0, &prefs);
        let left = solved.find("left").unwrap();
        assert_eq!(left.width, 500.0); // 0.5 * 1000
    }

    #[test]
    fn per_axis_ratio_preferences() {
        let children = [
            simple_slot("panel", Sizing::fractional(0.3, 50.0)),
            simple_slot("main", Sizing::flex(100.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: Some(1.0),
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let prefs = LayoutPreferences::new(
            &[
                ("panel", Axis::Horizontal, 0.4),
                ("panel", Axis::Vertical, 0.25),
            ],
            &[],
        );

        // Wide → Horizontal → uses 0.4
        let solved = solve(&root, 1000.0, 600.0, &prefs);
        let panel = solved.find("panel").unwrap();
        assert_eq!(panel.width, 400.0);

        // Tall → Vertical → uses 0.25
        let solved = solve(&root, 600.0, 1000.0, &prefs);
        let panel = solved.find("panel").unwrap();
        assert_eq!(panel.height, 250.0);
    }

    // ===== Nested container tests =====

    #[test]
    fn nested_containers() {
        let inner_children = [
            simple_slot("a", Sizing::flex(0.0)),
            simple_slot("b", Sizing::flex(0.0)),
        ];
        let children = [
            simple_slot("header", Sizing::Fixed(50.0)),
            LayoutNode::Container(ContainerNode {
                id: "content",
                axis: Axis::Horizontal,
                auto_axis: None,
                sizing: Sizing::flex(0.0),
                children: &inner_children,
                divider_size: 0.0,
            }),
            simple_slot("footer", Sizing::Fixed(80.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 1000.0, 800.0, &LayoutPreferences::default());

        let content = solved.find("content").unwrap();
        assert_eq!(content.height, 670.0); // 800 - 50 - 80
        assert_eq!(content.width, 1000.0);

        let a = solved.find("a").unwrap();
        assert_eq!(a.width, 500.0); // 1000/2
        assert_eq!(a.height, 670.0);

        let b = solved.find("b").unwrap();
        assert_eq!(b.width, 500.0);
    }

    // ===== Total width/height invariant =====

    #[test]
    fn total_allocation_never_exceeds_available() {
        let children = [
            collapsible_slot("config", Sizing::fractional(0.2, 100.0), 0.5, "Config"),
            simple_slot("main", Sizing::flex(300.0)),
            collapsible_slot("diag", Sizing::fractional(0.15, 150.0), 0.3, "Diag"),
            collapsible_slot("output", Sizing::fractional(0.15, 120.0), 0.6, "Output"),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 6.0,
        });

        for width in [200.0, 450.0, 600.0, 800.0, 1200.0] {
            let solved = solve(&root, width, 600.0, &LayoutPreferences::default());
            let visible: Vec<&SolvedNode> = solved.children.iter().filter(|c| c.visible).collect();
            let total_children: f32 = visible.iter().map(|c| c.width).sum();
            let dividers = if visible.len() > 1 {
                6.0 * (visible.len() - 1) as f32
            } else {
                0.0
            };
            let total = total_children + dividers;
            assert!(
                total <= width + 0.01,
                "width={width}: total={total} (children={total_children} + dividers={dividers})"
            );
        }
    }

    #[test]
    fn flex_min_sum_exceeds_remaining_is_scaled_down() {
        // Two flex children each with min=50 and weight=1 in a container
        // with only 80 pixels of remaining space. Without scaling, each
        // would get 50 (total 100 > 80). The fix scales them down so the
        // total never exceeds the available space.
        let children = [
            simple_slot(
                "a",
                Sizing::Flex {
                    min: 50.0,
                    weight: 1.0,
                },
            ),
            simple_slot(
                "b",
                Sizing::Flex {
                    min: 50.0,
                    weight: 1.0,
                },
            ),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 80.0, 600.0, &LayoutPreferences::default());
        let a = solved.find("a").unwrap();
        let b = solved.find("b").unwrap();
        assert_eq!(
            a.width + b.width,
            80.0,
            "total flex allocation should not exceed available space"
        );
        assert!(a.width >= 0.0 && b.width >= 0.0);
    }

    // ===== App-like layout test =====

    #[test]
    fn app_layout_scenario() {
        // Models the SotF app: header | (library | queue | rack) | footer
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

        let content_children = [
            LayoutNode::Slot(SlotNode {
                id: "library",
                sizing: Sizing::fractional(0.3, 100.0),
                priority: 0.5,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Library"),
            }),
            simple_slot("queue", Sizing::flex(200.0)),
            LayoutNode::Slot(SlotNode {
                id: "rack",
                sizing: Sizing::fractional(0.3, 0.0),
                priority: 0.3,
                collapsible: true,
                display_tiers: RACK_TIERS,
                collapse_label: Some("Rack"),
            }),
        ];

        let root_children = [
            simple_slot("header", Sizing::Fixed(40.0)),
            LayoutNode::Container(ContainerNode {
                id: "content",
                axis: Axis::Horizontal,
                auto_axis: Some(1.0),
                sizing: Sizing::flex(0.0),
                children: &content_children,
                divider_size: 6.0,
            }),
            simple_slot("footer", Sizing::Fixed(100.0)),
        ];

        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &root_children,
            divider_size: 0.0,
        });

        // Wide window
        let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());

        assert_eq!(solved.find("header").unwrap().height, 40.0);
        assert_eq!(solved.find("footer").unwrap().height, 100.0);

        let content = solved.find("content").unwrap();
        assert_eq!(content.resolved_axis, Some(Axis::Horizontal));
        assert_eq!(content.height, 660.0); // 800 - 40 - 100

        let library = solved.find("library").unwrap();
        assert!(library.visible);

        let rack = solved.find("rack").unwrap();
        assert!(rack.visible);
        assert_eq!(rack.active_tier, Some("Full"));

        // Narrow tall window → vertical
        let solved = solve(&root, 500.0, 900.0, &LayoutPreferences::default());
        let content = solved.find("content").unwrap();
        assert_eq!(content.resolved_axis, Some(Axis::Vertical));
    }

    // ===== Sizing::Text tests =====

    struct FixedWidthMeasure {
        char_width: f64,
    }

    impl gpui_pretext::TextMeasure for FixedWidthMeasure {
        fn measure_width(&self, text: &str) -> f64 {
            text.chars().count() as f64 * self.char_width
        }
    }

    #[test]
    fn text_sizing_vertical_container() {
        // Each char is 10px wide. "hello world" = 110px wide, wraps at 80px.
        // At 80px max_width: "hello " on line 1, "world" on line 2 → height = 2 * 20 = 40.
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let line_height = 20.0_f32;

        let children = [
            simple_slot("header", Sizing::Fixed(30.0)),
            LayoutNode::Slot(SlotNode {
                id: "label",
                sizing: Sizing::Text {
                    text: "hello world",
                    measure: &measure,
                    line_height,
                    min: 0.0,
                },
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            simple_slot("footer", Sizing::Fixed(10.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        // Container is 80px wide → text wraps to 2 lines → label height = 40
        let solved = solve(&root, 80.0, 500.0, &LayoutPreferences::default());

        assert_eq!(solved.find("header").unwrap().height, 30.0);
        let label = solved.find("label").unwrap();
        assert!(label.visible);
        assert_eq!(label.height, 40.0);
        assert_eq!(solved.find("footer").unwrap().height, 10.0);
    }

    #[test]
    fn text_sizing_horizontal_container() {
        // Each char is 10px wide. "hi" = 20px wide → single line, width = 20.
        let measure = FixedWidthMeasure { char_width: 10.0 };

        let children = [
            LayoutNode::Slot(SlotNode {
                id: "tag",
                sizing: Sizing::Text {
                    text: "hi",
                    measure: &measure,
                    line_height: 20.0,
                    min: 0.0,
                },
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            simple_slot("rest", Sizing::flex(0.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 500.0, 100.0, &LayoutPreferences::default());
        let tag = solved.find("tag").unwrap();
        assert!(tag.visible);
        assert_eq!(tag.width, 20.0); // "hi" = 2 chars * 10px
    }

    #[test]
    fn text_sizing_respects_min_floor() {
        // Empty text → height = 0, but min = 50 → height = 50.
        let measure = FixedWidthMeasure { char_width: 10.0 };

        let children = [LayoutNode::Slot(SlotNode {
            id: "label",
            sizing: Sizing::Text {
                text: "",
                measure: &measure,
                line_height: 20.0,
                min: 50.0,
            },
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 200.0, 500.0, &LayoutPreferences::default());
        assert_eq!(solved.find("label").unwrap().height, 50.0);
    }

    #[test]
    fn text_size_cache_shares_measurements_for_duplicate_text() {
        use std::cell::Cell;

        super::super::misc::clear_text_cache();

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

        let measure = CountingMeasure {
            char_width: 10.0,
            calls: Cell::new(0),
        };

        let children = [
            LayoutNode::Slot(SlotNode {
                id: "a",
                sizing: Sizing::Text {
                    text: "hello",
                    measure: &measure,
                    line_height: 20.0,
                    min: 0.0,
                },
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "b",
                sizing: Sizing::Text {
                    text: "hello",
                    measure: &measure,
                    line_height: 20.0,
                    min: 0.0,
                },
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
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

        let solved = solve(&root, 500.0, 100.0, &LayoutPreferences::default());
        assert_eq!(solved.find("a").unwrap().width, 50.0);
        assert_eq!(solved.find("b").unwrap().width, 50.0);
        let calls_for_two_duplicates = measure.calls.get();
        assert!(
            calls_for_two_duplicates > 0,
            "text should be measured at least once"
        );

        // Clear the persistent cache so the single-child solve starts from the
        // same empty-cache state as the duplicate scenario.
        super::super::misc::clear_text_cache();
        measure.calls.set(0);
        let single_child = [LayoutNode::Slot(SlotNode {
            id: "only",
            sizing: Sizing::Text {
                text: "hello",
                measure: &measure,
                line_height: 20.0,
                min: 0.0,
            },
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })];
        let single_root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &single_child,
            divider_size: 0.0,
        });
        solve(&single_root, 500.0, 100.0, &LayoutPreferences::default());
        let calls_for_single = measure.calls.get();

        assert_eq!(
            calls_for_two_duplicates, calls_for_single,
            "duplicate text nodes should reuse the cached measurement"
        );
    }

    #[test]
    fn root_slot_collapsed_gets_zero_size() {
        static TIERS: &[DisplayTier<'_>] = &[DisplayTier {
            name: "Full",
            min_size: 0.0,
        }];

        let root = LayoutNode::Slot(SlotNode {
            id: "root-slot",
            sizing: Sizing::flex(0.0),
            priority: 1.0,
            collapsible: true,
            display_tiers: TIERS,
            collapse_label: Some("Tab"),
        });
        let prefs = LayoutPreferences::new(&[], &[("root-slot", true)]);

        let solved = solve(&root, 200.0, 100.0, &prefs);
        assert!(!solved.visible);
        assert_eq!(solved.width, 0.0);
        assert_eq!(solved.height, 0.0);
        assert_eq!(solved.active_tier, None);

        let tree = solve_tree(&root, 200.0, 100.0, &prefs);
        let root_ref = tree.root();
        assert!(!root_ref.visible());
        assert_eq!(root_ref.width(), 0.0);
        assert_eq!(root_ref.collapse_label(), Some("Tab"));
    }

    #[test]
    fn solve_tree_user_collapsed_slot_child_gets_zero_size() {
        let children = [
            collapsible_slot("sidebar", Sizing::Fixed(50.0), 0.5, "Sidebar"),
            simple_slot("main", Sizing::flex(0.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });
        let prefs = LayoutPreferences::new(&[], &[("sidebar", true)]);

        let tree = solve_tree(&root, 200.0, 100.0, &prefs);
        let sidebar = tree.find("sidebar").unwrap();
        assert!(!sidebar.visible());
        assert_eq!(sidebar.collapse_label(), Some("Sidebar"));
        assert_eq!(tree.find("main").unwrap().width(), 200.0);
    }

    #[test]
    fn fractional_ratios_are_scaled_when_total_exceeds_one() {
        let children = [
            simple_slot("a", Sizing::fractional(0.6, 0.0)),
            simple_slot("b", Sizing::fractional(0.6, 0.0)),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 100.0, 10.0, &LayoutPreferences::default());
        assert_eq!(solved.find("a").unwrap().width, 50.0);
        assert_eq!(solved.find("b").unwrap().width, 50.0);
    }

    #[test]
    fn collapsible_text_child_gets_measured_size() {
        struct FixedMeasure;
        impl gpui_pretext::TextMeasure for FixedMeasure {
            fn measure_width(&self, text: &str) -> f64 {
                text.chars().count() as f64 * 10.0
            }
        }
        static MEASURE: FixedMeasure = FixedMeasure;

        let children = [LayoutNode::Slot(SlotNode {
            id: "label",
            sizing: Sizing::Text {
                text: "abc",
                measure: &MEASURE,
                line_height: 20.0,
                min: 0.0,
            },
            priority: 1.0,
            collapsible: true,
            display_tiers: &[],
            collapse_label: Some("Label"),
        })];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let solved = solve(&root, 200.0, 50.0, &LayoutPreferences::default());
        assert_eq!(solved.find("label").unwrap().width, 30.0);
    }
}
