#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use accesskit::{Action, ActionRequest, Node, NodeId, TreeId, TreeUpdate};
use gpui::A11yCallbacks;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

#[derive(Default)]
struct AccessibilityTree {
    nodes: HashMap<NodeId, Node>,
    root: Option<NodeId>,
    focus: Option<NodeId>,
}

impl AccessibilityTree {
    fn apply(&mut self, update: TreeUpdate) {
        if update.tree_id != TreeId::ROOT {
            return;
        }
        if let Some(tree) = update.tree {
            self.root = Some(tree.root);
        }
        self.focus = Some(update.focus);
        self.nodes.extend(update.nodes);
        self.retain_reachable_nodes();
    }

    /// Tree updates are incremental, so removed nodes are absent from an
    /// update. Prune nodes no longer reachable from the current root after
    /// applying the update rather than retaining stale accessibility objects
    /// forever.
    fn retain_reachable_nodes(&mut self) {
        let Some(root) = self.root else {
            return;
        };

        let mut reachable = HashSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if !reachable.insert(id) {
                continue;
            }
            if let Some(node) = self.nodes.get(&id) {
                pending.extend(node.children().iter().copied());
            }
        }
        self.nodes.retain(|id, _| reachable.contains(id));
    }

    fn snapshot(&self) -> String {
        let nodes: Vec<Value> = self
            .nodes
            .iter()
            .map(|(id, node)| {
                let bounds = node
                    .bounds()
                    .map(|bounds| json!([bounds.x0, bounds.y0, bounds.x1, bounds.y1]));
                json!({
                    "id": id.0,
                    "role": format!("{:?}", node.role()),
                    "label": node.label(),
                    "value": node.value(),
                    "description": node.description(),
                    "disabled": node.is_disabled(),
                    "bounds": bounds,
                    "children": node.children().iter().map(|child| child.0).collect::<Vec<_>>(),
                    "click": node.supports_action(Action::Click),
                    "focus": node.supports_action(Action::Focus),
                    "increment": node.supports_action(Action::Increment),
                    "decrement": node.supports_action(Action::Decrement),
                })
            })
            .collect();
        json!({
            "root": self.root.map(|id| id.0),
            "focus": self.focus.map(|id| id.0),
            "nodes": nodes,
        })
        .to_string()
    }
}

#[derive(Default)]
struct AccessibilityState {
    tree: AccessibilityTree,
    callbacks: Option<A11yCallbacks>,
}

static STATE: OnceLock<Mutex<AccessibilityState>> = OnceLock::new();

fn state() -> &'static Mutex<AccessibilityState> {
    STATE.get_or_init(|| Mutex::new(AccessibilityState::default()))
}

pub(crate) fn init(callbacks: A11yCallbacks) {
    let initial = (callbacks.activation)();
    let mut state = state().lock();
    state.callbacks = Some(callbacks);
    if let Some(update) = initial {
        state.tree.apply(update);
    }
}

pub(crate) fn update(update: TreeUpdate) {
    state().lock().tree.apply(update);
    #[cfg(target_os = "android")]
    crate::android::jni::notify_accessibility_changed();
}

pub(crate) fn snapshot() -> String {
    state().lock().tree.snapshot()
}

pub(crate) fn perform_action(node_id: u64, action_code: i32) -> bool {
    let action = match action_code {
        1 => Action::Click,
        2 => Action::Focus,
        3 => Action::Increment,
        4 => Action::Decrement,
        _ => return false,
    };
    let callbacks = state().lock().callbacks.take();
    let Some(callbacks) = callbacks else {
        return false;
    };
    (callbacks.action)(ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: NodeId(node_id),
        data: None,
    });
    let mut state = state().lock();
    if state.callbacks.is_none() {
        state.callbacks = Some(callbacks);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{Node, Rect, Role, Tree};

    #[test]
    fn snapshot_contains_gpui_nodes_actions_and_bounds() {
        let root_id = NodeId(1);
        let button_id = NodeId(2);
        let mut root = Node::new(Role::Window);
        root.set_children(vec![button_id]);
        let mut button = Node::new(Role::Button);
        button.set_label("Play");
        button.set_bounds(Rect::new(10.0, 20.0, 110.0, 70.0));
        button.add_action(Action::Click);

        let mut tree = AccessibilityTree::default();
        tree.apply(TreeUpdate {
            nodes: vec![(root_id, root), (button_id, button)],
            tree: Some(Tree::new(root_id)),
            tree_id: TreeId::ROOT,
            focus: button_id,
        });
        let snapshot: Value = serde_json::from_str(&tree.snapshot()).unwrap();

        assert_eq!(snapshot["root"], 1);
        assert_eq!(snapshot["focus"], 2);
        let button = snapshot["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == 2)
            .unwrap();
        assert_eq!(button["label"], "Play");
        assert_eq!(button["bounds"], json!([10.0, 20.0, 110.0, 70.0]));
        assert_eq!(button["click"], true);
    }

    #[test]
    fn snapshot_drops_nodes_disconnected_by_an_incremental_update() {
        let root_id = NodeId(1);
        let removed_id = NodeId(2);
        let retained_id = NodeId(3);

        let mut root = Node::new(Role::Window);
        root.set_children(vec![removed_id]);
        let mut tree = AccessibilityTree::default();
        tree.apply(TreeUpdate {
            nodes: vec![(root_id, root), (removed_id, Node::new(Role::Button))],
            tree: Some(Tree::new(root_id)),
            tree_id: TreeId::ROOT,
            focus: removed_id,
        });

        let mut updated_root = Node::new(Role::Window);
        updated_root.set_children(vec![retained_id]);
        tree.apply(TreeUpdate {
            nodes: vec![
                (root_id, updated_root),
                (retained_id, Node::new(Role::Button)),
            ],
            tree: None,
            tree_id: TreeId::ROOT,
            focus: retained_id,
        });

        let snapshot: Value = serde_json::from_str(&tree.snapshot()).unwrap();
        let node_ids = snapshot["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["id"].as_u64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(node_ids.len(), 2);
        assert!(node_ids.contains(&root_id.0));
        assert!(node_ids.contains(&retained_id.0));
        assert!(!node_ids.contains(&removed_id.0));
    }
}
