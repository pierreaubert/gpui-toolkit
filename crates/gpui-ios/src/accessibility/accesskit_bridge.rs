//! Adapter from GPUI's AccessKit tree updates to the existing UIKit snapshot API.

use super::{
    IosAccessibilityAction, IosAccessibilityFrame, IosAccessibilityNode, IosAccessibilityRole,
    IosAccessibilitySnapshot,
};
use accesskit::{Action, ActionRequest, Node, NodeId, Role, TreeId, TreeUpdate};
use gpui::A11yCallbacks;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Default)]
struct AccessibilityTree {
    nodes: HashMap<NodeId, Node>,
    root: Option<NodeId>,
}

impl AccessibilityTree {
    fn apply(&mut self, update: TreeUpdate) {
        if update.tree_id != TreeId::ROOT {
            return;
        }
        if let Some(tree) = update.tree {
            self.root = Some(tree.root);
        }
        self.nodes.extend(update.nodes);
        self.retain_reachable_nodes();
    }

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

    fn snapshot(&self, scale_factor: f32) -> Option<IosAccessibilitySnapshot> {
        let root = self.root?;
        let mut visited = HashSet::new();
        let root = build_node(
            root,
            &self.nodes,
            &mut visited,
            normalized_scale_factor(scale_factor),
        )?;
        Some(IosAccessibilitySnapshot::new(root))
    }

    fn action_request(&self, id: &str, action: IosAccessibilityAction) -> Option<ActionRequest> {
        let node_id = NodeId(id.parse().ok()?);
        let action = match action {
            IosAccessibilityAction::Activate => Action::Click,
            IosAccessibilityAction::Increment => Action::Increment,
            IosAccessibilityAction::Decrement => Action::Decrement,
            IosAccessibilityAction::Escape => Action::Collapse,
            IosAccessibilityAction::MagicTap => return None,
        };
        if !self
            .nodes
            .get(&node_id)
            .is_some_and(|node| node.supports_action(action))
        {
            return None;
        }
        Some(ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: node_id,
            data: None,
        })
    }
}

#[derive(Default)]
struct AccessibilityState {
    tree: AccessibilityTree,
    callbacks: Option<A11yCallbacks>,
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    scale_factor: f32,
}

static STATE: OnceLock<Mutex<AccessibilityState>> = OnceLock::new();
static CALLBACK_GENERATION: AtomicU64 = AtomicU64::new(0);

fn state() -> &'static Mutex<AccessibilityState> {
    STATE.get_or_init(|| Mutex::new(AccessibilityState::default()))
}

fn lock_state() -> std::sync::MutexGuard<'static, AccessibilityState> {
    state()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
pub(crate) fn init_accesskit(callbacks: A11yCallbacks, scale_factor: f32) {
    let initial_update = (callbacks.activation)();
    CALLBACK_GENERATION.fetch_add(1, Ordering::AcqRel);
    let snapshot = {
        let mut state = lock_state();
        state.callbacks = Some(callbacks);
        state.scale_factor = normalized_scale_factor(scale_factor);
        if let Some(update) = initial_update {
            state.tree.apply(update);
        }
        state.tree.snapshot(state.scale_factor)
    };
    publish(snapshot);
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
pub(crate) fn update_accesskit_tree(update: TreeUpdate, scale_factor: f32) {
    let snapshot = {
        let mut state = lock_state();
        state.scale_factor = normalized_scale_factor(scale_factor);
        state.tree.apply(update);
        state.tree.snapshot(state.scale_factor)
    };
    publish(snapshot);
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
pub(crate) fn update_accesskit_scale_factor(scale_factor: f32) -> bool {
    let snapshot = {
        let mut state = lock_state();
        let scale_factor = normalized_scale_factor(scale_factor);
        if state.scale_factor.to_bits() == scale_factor.to_bits() {
            return false;
        }
        state.scale_factor = scale_factor;
        state.tree.snapshot(scale_factor)
    };
    publish(snapshot);
    true
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
fn publish(snapshot: Option<IosAccessibilitySnapshot>) {
    if let Some(snapshot) = snapshot {
        if let Err(error) = super::set_accessibility_snapshot(snapshot) {
            log::warn!("could not publish the GPUI iOS accessibility tree: {error}");
        }
    }
}

pub(crate) fn dispatch_accesskit_action(id: &str, action: IosAccessibilityAction) -> bool {
    let (generation, callbacks, request) = {
        let mut state = lock_state();
        let Some(request) = state.tree.action_request(id, action) else {
            return false;
        };
        let Some(callbacks) = state.callbacks.take() else {
            return false;
        };
        (
            CALLBACK_GENERATION.load(Ordering::Acquire),
            callbacks,
            request,
        )
    };

    let callbacks = invoke_action(callbacks, request);
    let mut state = lock_state();
    if CALLBACK_GENERATION.load(Ordering::Acquire) == generation && state.callbacks.is_none() {
        state.callbacks = Some(callbacks);
    }
    true
}

fn invoke_action(callbacks: A11yCallbacks, request: ActionRequest) -> A11yCallbacks {
    let A11yCallbacks {
        activation,
        action,
        deactivation,
    } = callbacks;
    action(request);
    A11yCallbacks {
        activation,
        action,
        deactivation,
    }
}

fn build_node(
    id: NodeId,
    nodes: &HashMap<NodeId, Node>,
    visited: &mut HashSet<NodeId>,
    scale_factor: f32,
) -> Option<IosAccessibilityNode> {
    if !visited.insert(id) {
        return None;
    }
    let node = nodes.get(&id)?;
    let role = ios_role(node.role());
    let mut result = IosAccessibilityNode::new(id.0.to_string(), role);
    if node.role() == Role::Label {
        result.label = node.value().map(str::to_owned);
    } else {
        result.label = node.label().map(str::to_owned);
        result.value = node.value().map(str::to_owned);
    }
    result.hint = node.description().map(str::to_owned);
    result.enabled = !node.is_disabled();
    result.selected = node.is_selected().unwrap_or(false);
    result.expanded = node.is_expanded();
    if let Some(bounds) = node.bounds() {
        result.frame = IosAccessibilityFrame {
            x: bounds.x0 as f32 / scale_factor,
            y: bounds.y0 as f32 / scale_factor,
            width: (bounds.x1 - bounds.x0) as f32 / scale_factor,
            height: (bounds.y1 - bounds.y0) as f32 / scale_factor,
        };
    }
    if node.supports_action(Action::Click) {
        result.actions.push(IosAccessibilityAction::Activate);
    }
    if node.supports_action(Action::Increment) {
        result.actions.push(IosAccessibilityAction::Increment);
    }
    if node.supports_action(Action::Decrement) {
        result.actions.push(IosAccessibilityAction::Decrement);
    }
    if node.supports_action(Action::Collapse) {
        result.actions.push(IosAccessibilityAction::Escape);
    }
    result.children = node
        .children()
        .iter()
        .filter_map(|child| build_node(*child, nodes, visited, scale_factor))
        .collect();
    Some(result)
}

fn normalized_scale_factor(scale_factor: f32) -> f32 {
    if scale_factor.is_finite() && scale_factor > f32::EPSILON {
        scale_factor
    } else {
        // The bridge can initialize before UIKit publishes display metrics.
        1.0
    }
}

fn ios_role(role: Role) -> IosAccessibilityRole {
    match role {
        Role::Unknown | Role::GenericContainer | Role::IframePresentational => {
            IosAccessibilityRole::None
        }
        Role::Button
        | Role::DefaultButton
        | Role::MenuItem
        | Role::MenuItemCheckBox
        | Role::MenuItemRadio
        | Role::ListBoxOption
        | Role::MenuListOption => IosAccessibilityRole::Button,
        Role::CheckBox | Role::RadioButton => IosAccessibilityRole::Checkbox,
        Role::Switch => IosAccessibilityRole::Switch,
        Role::Image => IosAccessibilityRole::Image,
        Role::Link => IosAccessibilityRole::Link,
        Role::Search | Role::SearchInput => IosAccessibilityRole::SearchField,
        Role::Slider | Role::SpinButton | Role::Meter => IosAccessibilityRole::Adjustable,
        Role::Tab => IosAccessibilityRole::Tab,
        Role::Header | Role::Heading | Role::RowHeader | Role::ColumnHeader => {
            IosAccessibilityRole::Header
        }
        Role::TextInput
        | Role::MultilineTextInput
        | Role::DateInput
        | Role::DateTimeInput
        | Role::WeekInput
        | Role::MonthInput
        | Role::TimeInput
        | Role::EmailInput
        | Role::NumberInput
        | Role::PasswordInput
        | Role::PhoneNumberInput
        | Role::UrlInput => IosAccessibilityRole::TextField,
        Role::TextRun
        | Role::Label
        | Role::Paragraph
        | Role::LineBreak
        | Role::Abbr
        | Role::Caption
        | Role::Code
        | Role::Emphasis
        | Role::Strong
        | Role::Term
        | Role::Time
        | Role::Status
        | Role::ProgressIndicator => IosAccessibilityRole::StaticText,
        _ => IosAccessibilityRole::Container,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{Rect, Tree};
    use std::sync::{Arc, Mutex};

    fn sample_tree() -> AccessibilityTree {
        let root_id = NodeId(1);
        let button_id = NodeId(2);
        let mut root = Node::new(Role::Window);
        root.set_children(vec![button_id]);
        let mut button = Node::new(Role::Button);
        button.set_label("Start game");
        button.set_value("Ready");
        button.set_bounds(Rect::new(10.0, 20.0, 110.0, 70.0));
        button.set_selected(true);
        button.add_action(Action::Click);
        let mut tree = AccessibilityTree::default();
        tree.apply(TreeUpdate {
            nodes: vec![(root_id, root), (button_id, button)],
            tree: Some(Tree::new(root_id)),
            tree_id: TreeId::ROOT,
            focus: button_id,
        });
        tree
    }

    #[test]
    fn converts_tree_roles_properties_bounds_and_children() {
        let snapshot = sample_tree().snapshot(1.0).expect("tree snapshot");
        snapshot.validate().expect("valid snapshot");
        assert_eq!(snapshot.root.id, "1");
        assert_eq!(snapshot.root.children.len(), 1);
        let button = &snapshot.root.children[0];
        assert_eq!(button.id, "2");
        assert_eq!(button.role, IosAccessibilityRole::Button);
        assert_eq!(button.label.as_deref(), Some("Start game"));
        assert_eq!(button.value.as_deref(), Some("Ready"));
        assert!(button.selected);
        assert_eq!(button.frame.x, 10.0);
        assert_eq!(button.frame.y, 20.0);
        assert_eq!(button.frame.width, 100.0);
        assert_eq!(button.frame.height, 50.0);
        assert_eq!(button.actions, vec![IosAccessibilityAction::Activate]);
    }

    #[test]
    fn incremental_update_prunes_removed_accessibility_nodes() {
        let mut tree = sample_tree();
        let root_id = NodeId(1);
        let replacement_id = NodeId(3);
        let mut root = Node::new(Role::Window);
        root.set_children(vec![replacement_id]);
        let mut replacement = Node::new(Role::Button);
        replacement.set_label("Continue");
        replacement.add_action(Action::Click);
        tree.apply(TreeUpdate {
            nodes: vec![(root_id, root), (replacement_id, replacement)],
            tree: None,
            tree_id: TreeId::ROOT,
            focus: replacement_id,
        });
        let snapshot = tree.snapshot(1.0).expect("updated tree snapshot");
        assert_eq!(snapshot.root.children.len(), 1);
        assert_eq!(snapshot.root.children[0].id, "3");
    }

    #[test]
    fn accesskit_physical_bounds_become_uikit_points_at_display_scale() {
        for scale_factor in [2.0, 3.0] {
            let snapshot = sample_tree().snapshot(scale_factor).expect("tree snapshot");
            let button = &snapshot.root.children[0];
            assert_eq!(button.frame.x, 10.0 / scale_factor);
            assert_eq!(button.frame.y, 20.0 / scale_factor);
            assert_eq!(button.frame.width, 100.0 / scale_factor);
            assert_eq!(button.frame.height, 50.0 / scale_factor);
        }
    }

    #[test]
    fn ios_actions_forward_supported_accesskit_requests() {
        let requested = Arc::new(Mutex::new(Vec::<ActionRequest>::new()));
        let requested_from_callback = requested.clone();
        let callbacks = A11yCallbacks {
            activation: Box::new(|| None),
            action: Box::new(move |request| {
                requested_from_callback.lock().unwrap().push(request);
            }),
            deactivation: Box::new(|| {}),
        };
        let callbacks = invoke_action(
            callbacks,
            sample_tree()
                .action_request("2", IosAccessibilityAction::Activate)
                .expect("supported activate action"),
        );
        drop(callbacks);
        let requests = requested.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].action, Action::Click);
        assert_eq!(requests[0].target_tree, TreeId::ROOT);
        assert_eq!(requests[0].target_node, NodeId(2));
    }
}
