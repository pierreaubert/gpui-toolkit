//! iOS accessibility snapshot model.
//!
//! GPUI does not expose UIKit accessibility objects directly, so app and
//! component code publish a compact snapshot here. The iOS window bridge mirrors
//! that snapshot into `UIAccessibilityElement`s attached to the Metal view.

use std::sync::{Arc, Mutex, OnceLock};

type AccessibilityActionCallback =
    Box<dyn FnMut(&str, IosAccessibilityAction) -> bool + Send + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IosAccessibilityRole {
    None,
    Button,
    Checkbox,
    Header,
    Image,
    Link,
    SearchField,
    Slider,
    StaticText,
    Switch,
    Tab,
    TextField,
    Adjustable,
    Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IosAccessibilityAction {
    Activate,
    Increment,
    Decrement,
    Escape,
    MagicTap,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IosAccessibilityFrame {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl IosAccessibilityFrame {
    pub fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width >= 0.0
            && self.height >= 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IosAccessibilityNode {
    pub id: String,
    pub role: IosAccessibilityRole,
    pub label: Option<String>,
    pub hint: Option<String>,
    pub value: Option<String>,
    pub frame: IosAccessibilityFrame,
    pub enabled: bool,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub actions: Vec<IosAccessibilityAction>,
    pub children: Vec<IosAccessibilityNode>,
}

impl IosAccessibilityNode {
    pub fn new(id: impl Into<String>, role: IosAccessibilityRole) -> Self {
        Self {
            id: id.into(),
            role,
            label: None,
            hint: None,
            value: None,
            frame: IosAccessibilityFrame::default(),
            enabled: true,
            selected: false,
            expanded: None,
            actions: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn frame(mut self, frame: IosAccessibilityFrame) -> Self {
        self.frame = frame;
        self
    }

    pub fn action(mut self, action: IosAccessibilityAction) -> Self {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
        self
    }

    pub fn child(mut self, child: IosAccessibilityNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn is_accessible_element(&self) -> bool {
        self.role != IosAccessibilityRole::None
            && (self.label.as_ref().is_some_and(|label| !label.is_empty())
                || self.value.as_ref().is_some_and(|value| !value.is_empty())
                || !self.actions.is_empty())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("accessibility node id must not be empty".to_string());
        }
        if !self.frame.is_valid() {
            return Err(format!(
                "accessibility node {:?} has invalid frame",
                self.id
            ));
        }
        for child in &self.children {
            child.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IosAccessibilitySnapshot {
    pub root: IosAccessibilityNode,
    pub announcements: Vec<String>,
}

impl IosAccessibilitySnapshot {
    pub fn new(root: IosAccessibilityNode) -> Self {
        Self {
            root,
            announcements: Vec::new(),
        }
    }

    pub fn announce(mut self, message: impl Into<String>) -> Self {
        let message = message.into();
        if !message.trim().is_empty() {
            self.announcements.push(message);
        }
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        self.root.validate()
    }

    pub fn flattened_nodes(&self) -> Vec<&IosAccessibilityNode> {
        fn visit<'a>(node: &'a IosAccessibilityNode, out: &mut Vec<&'a IosAccessibilityNode>) {
            if node.is_accessible_element() {
                out.push(node);
            }
            for child in &node.children {
                visit(child, out);
            }
        }

        let mut nodes = Vec::new();
        visit(&self.root, &mut nodes);
        nodes
    }
}

/// Which accessibility properties changed for a single node.
///
/// This mirrors the UIKit setters that `IosWindow::refresh_accessibility`
/// applies, so only the flagged properties need to be pushed to the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeChanges {
    pub label_changed: bool,
    pub hint_changed: bool,
    pub value_changed: bool,
    pub frame_changed: bool,
    pub traits_changed: bool,
}

impl NodeChanges {
    pub fn any(&self) -> bool {
        self.label_changed
            || self.hint_changed
            || self.value_changed
            || self.frame_changed
            || self.traits_changed
    }
}

/// The result of diffing two accessibility snapshots.
///
/// All ids are borrowed from the `next` snapshot, so the structure is usable on
/// the host for benchmarking and unit testing without UIKit.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityDiff<'a> {
    pub unchanged: Vec<&'a IosAccessibilityNode>,
    pub changed: Vec<(&'a IosAccessibilityNode, NodeChanges)>,
    pub added: Vec<&'a IosAccessibilityNode>,
    pub removed: Vec<&'a str>,
    pub order_changed: bool,
}

/// Host-runnable computation of the accessibility value string that UIKit would
/// receive via `accessibilityValue`.
fn accessibility_value_for_node(node: &IosAccessibilityNode) -> Option<String> {
    match (node.value.as_deref(), node.expanded) {
        (Some(value), Some(true)) if !value.is_empty() => Some(format!("{value}, expanded")),
        (Some(value), Some(false)) if !value.is_empty() => Some(format!("{value}, collapsed")),
        (Some(value), _) if !value.is_empty() => Some(value.to_string()),
        (_, Some(true)) => Some("expanded".to_string()),
        (_, Some(false)) => Some("collapsed".to_string()),
        _ => None,
    }
}

/// Returns `true` if any input that contributes to `UIAccessibilityTraits` has
/// changed between `prev` and `next`.
fn traits_inputs_changed(prev: &IosAccessibilityNode, next: &IosAccessibilityNode) -> bool {
    if prev.role != next.role
        || prev.enabled != next.enabled
        || prev.selected != next.selected
        || prev.actions != next.actions
    {
        return true;
    }
    // The `UpdatesFrequently` trait depends on value length for static text.
    if prev.role == IosAccessibilityRole::StaticText
        && next.role == IosAccessibilityRole::StaticText
    {
        let prev_len = prev.value.as_ref().map_or(0, String::len);
        let next_len = next.value.as_ref().map_or(0, String::len);
        if (prev_len > 16) != (next_len > 16) {
            return true;
        }
    }
    false
}

/// Compare two flattened accessibility snapshots and report what actually
/// changed. The returned ids borrow from `next` (and `removed` borrows from
/// `prev`).
///
/// This is the policy that `IosWindow::refresh_accessibility` uses to decide
/// which UIKit setters to call and whether the `accessibilityElements` array
/// must be rebuilt.
pub fn compute_accessibility_diff<'a>(
    prev: Option<&'a IosAccessibilitySnapshot>,
    next: &'a IosAccessibilitySnapshot,
) -> AccessibilityDiff<'a> {
    use std::collections::{HashMap, HashSet};

    let next_nodes = next.flattened_nodes();
    let prev_nodes = prev.map(IosAccessibilitySnapshot::flattened_nodes);

    let prev_by_id: HashMap<&str, &IosAccessibilityNode> =
        prev_nodes.as_ref().map_or_else(HashMap::new, |nodes| {
            nodes.iter().map(|node| (node.id.as_str(), *node)).collect()
        });

    let mut unchanged = Vec::new();
    let mut changed = Vec::new();
    let mut added = Vec::new();

    for next_node in &next_nodes {
        if let Some(&prev_node) = prev_by_id.get(next_node.id.as_str()) {
            let mut changes = NodeChanges::default();
            if prev_node.label != next_node.label {
                changes.label_changed = true;
            }
            if prev_node.hint != next_node.hint {
                changes.hint_changed = true;
            }
            if accessibility_value_for_node(prev_node) != accessibility_value_for_node(next_node) {
                changes.value_changed = true;
            }
            if prev_node.frame != next_node.frame {
                changes.frame_changed = true;
            }
            if traits_inputs_changed(prev_node, next_node) {
                changes.traits_changed = true;
            }

            if changes.any() {
                changed.push((*next_node, changes));
            } else {
                unchanged.push(*next_node);
            }
        } else {
            added.push(*next_node);
        }
    }

    let next_ids: HashSet<&str> = next_nodes.iter().map(|node| node.id.as_str()).collect();
    let removed: Vec<&str> = prev_by_id
        .keys()
        .filter(|id| !next_ids.contains(**id))
        .copied()
        .collect();

    let order_changed = prev_nodes.as_deref().is_none_or(|prev_list| {
        next_nodes.len() != prev_list.len()
            || next_nodes
                .iter()
                .zip(prev_list.iter())
                .any(|(next, prev)| next.id != prev.id)
    });

    AccessibilityDiff {
        unchanged,
        changed,
        added,
        removed,
        order_changed,
    }
}

static ACCESSIBILITY_SNAPSHOT: OnceLock<Mutex<Option<Arc<IosAccessibilitySnapshot>>>> =
    OnceLock::new();
static ACCESSIBILITY_ACTION_CALLBACK: OnceLock<Mutex<Option<AccessibilityActionCallback>>> =
    OnceLock::new();

fn snapshot_slot() -> &'static Mutex<Option<Arc<IosAccessibilitySnapshot>>> {
    ACCESSIBILITY_SNAPSHOT.get_or_init(|| Mutex::new(None))
}

fn action_callback_slot() -> &'static Mutex<Option<AccessibilityActionCallback>> {
    ACCESSIBILITY_ACTION_CALLBACK.get_or_init(|| Mutex::new(None))
}

pub fn set_accessibility_snapshot(snapshot: IosAccessibilitySnapshot) -> Result<(), String> {
    snapshot.validate()?;
    *snapshot_slot().lock().unwrap() = Some(Arc::new(snapshot));

    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    crate::ios::ffi::gpui_ios_refresh_accessibility();

    Ok(())
}

pub fn set_accessibility_action_callback(callback: Option<AccessibilityActionCallback>) {
    *action_callback_slot().lock().unwrap() = callback;
}

pub fn dispatch_accessibility_action(id: &str, action: IosAccessibilityAction) -> bool {
    action_callback_slot()
        .lock()
        .unwrap()
        .as_mut()
        .is_some_and(|callback| callback(id, action))
}

pub fn accessibility_snapshot() -> Option<Arc<IosAccessibilitySnapshot>> {
    snapshot_slot().lock().unwrap().as_ref().map(Arc::clone)
}

pub fn clear_accessibility_snapshot() {
    *snapshot_slot().lock().unwrap() = None;
}

#[cfg(test)]
mod tests {
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
}
