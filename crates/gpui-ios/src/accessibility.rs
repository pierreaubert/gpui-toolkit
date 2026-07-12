//! iOS accessibility snapshot model.
//!
//! GPUI does not expose UIKit accessibility objects directly, so app and
//! component code publish a compact snapshot here. The iOS window bridge mirrors
//! that snapshot into `UIAccessibilityElement`s attached to the Metal view.

use std::sync::{Arc, Mutex, OnceLock};

use std::collections::HashMap;

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

#[derive(Debug)]
pub struct IosAccessibilitySnapshot {
    pub root: IosAccessibilityNode,
    pub announcements: Vec<String>,
    flattened_cache: OnceLock<Arc<Vec<IosAccessibilityNode>>>,
    id_index_cache: OnceLock<HashMap<String, usize>>,
}

impl Clone for IosAccessibilitySnapshot {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            announcements: self.announcements.clone(),
            flattened_cache: OnceLock::new(),
            id_index_cache: OnceLock::new(),
        }
    }
}

impl PartialEq for IosAccessibilitySnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.announcements == other.announcements
    }
}

impl IosAccessibilitySnapshot {
    pub fn new(root: IosAccessibilityNode) -> Self {
        Self {
            root,
            announcements: Vec::new(),
            flattened_cache: OnceLock::new(),
            id_index_cache: OnceLock::new(),
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

    fn flatten_root(&self) -> Vec<IosAccessibilityNode> {
        fn visit(node: &IosAccessibilityNode, out: &mut Vec<IosAccessibilityNode>) {
            if node.is_accessible_element() {
                out.push(node.clone());
            }
            for child in &node.children {
                visit(child, out);
            }
        }

        let mut nodes = Vec::new();
        visit(&self.root, &mut nodes);
        nodes
    }

    /// Returns the cached flattened accessible nodes as a borrowed slice.
    ///
    /// This avoids the per-call `Vec<&IosAccessibilityNode>` allocation that
    /// [`Self::flattened_nodes`] performs and is the preferred path for hot
    /// internal callers.
    pub fn flattened_node_slice(&self) -> &[IosAccessibilityNode] {
        let cached = self
            .flattened_cache
            .get_or_init(|| Arc::new(self.flatten_root()));
        cached.as_slice()
    }

    pub fn flattened_nodes(&self) -> Vec<&IosAccessibilityNode> {
        self.flattened_node_slice().iter().collect()
    }

    pub(crate) fn id_index_map(&self) -> &HashMap<String, usize> {
        self.id_index_cache.get_or_init(|| {
            let mut map = HashMap::new();
            for (idx, node) in self.flattened_node_slice().iter().enumerate() {
                map.insert(node.id.clone(), idx);
            }
            map
        })
    }

    #[cfg(test)]
    fn is_flattened_cached(&self) -> bool {
        self.flattened_cache.get().is_some()
    }

    #[cfg(test)]
    fn is_id_index_cached(&self) -> bool {
        self.id_index_cache.get().is_some()
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

/// Reusable, lifetime-independent storage for accessibility snapshot diffs.
///
/// Entries are node indices rather than references, so this buffer can live on
/// a window and be reused safely as snapshots change between frames.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccessibilityDiffScratch {
    unchanged: Vec<usize>,
    changed: Vec<(usize, NodeChanges)>,
    added: Vec<usize>,
    removed: Vec<usize>,
    pub order_changed: bool,
}

impl AccessibilityDiffScratch {
    /// Clear entries while retaining all vector capacity for the next frame.
    pub fn clear(&mut self) {
        self.unchanged.clear();
        self.changed.clear();
        self.added.clear();
        self.removed.clear();
        self.order_changed = false;
    }

    pub fn unchanged_indices(&self) -> &[usize] {
        &self.unchanged
    }

    pub fn changed_indices(&self) -> &[(usize, NodeChanges)] {
        &self.changed
    }

    pub fn added_indices(&self) -> &[usize] {
        &self.added
    }

    pub fn removed_indices(&self) -> &[usize] {
        &self.removed
    }
}

fn accessibility_value_inputs(node: &IosAccessibilityNode) -> (Option<&str>, Option<bool>) {
    (
        node.value.as_deref().filter(|value| !value.is_empty()),
        node.expanded,
    )
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
    let mut scratch = AccessibilityDiffScratch::default();
    compute_accessibility_diff_into(prev, next, &mut scratch);
    let next_nodes = next.flattened_node_slice();
    let prev_nodes = prev.map(IosAccessibilitySnapshot::flattened_node_slice);

    AccessibilityDiff {
        unchanged: scratch
            .unchanged
            .iter()
            .map(|&idx| &next_nodes[idx])
            .collect(),
        changed: scratch
            .changed
            .iter()
            .map(|&(idx, changes)| (&next_nodes[idx], changes))
            .collect(),
        added: scratch.added.iter().map(|&idx| &next_nodes[idx]).collect(),
        removed: scratch
            .removed
            .iter()
            .map(|&idx| {
                prev_nodes.expect("removed nodes require a previous snapshot")[idx]
                    .id
                    .as_str()
            })
            .collect(),
        order_changed: scratch.order_changed,
    }
}

/// Compare snapshots into reusable index buffers.
///
/// After the snapshots' flatten/id caches and this scratch object are warmed,
/// unchanged and bounded-churn diffs perform no heap allocation.
pub fn compute_accessibility_diff_into(
    prev: Option<&IosAccessibilitySnapshot>,
    next: &IosAccessibilitySnapshot,
    scratch: &mut AccessibilityDiffScratch,
) {
    let next_nodes = next.flattened_node_slice();
    let next_id_map = next.id_index_map();

    let prev_nodes = prev.map(IosAccessibilitySnapshot::flattened_node_slice);
    let prev_id_map = prev.map(IosAccessibilitySnapshot::id_index_map);

    scratch.clear();
    scratch.unchanged.reserve(next_nodes.len());
    scratch.changed.reserve(next_nodes.len());
    scratch.added.reserve(next_nodes.len());
    scratch
        .removed
        .reserve(prev_nodes.map_or(0, <[IosAccessibilityNode]>::len));

    for (next_idx, next_node) in next_nodes.iter().enumerate() {
        let maybe_prev = prev_id_map
            .as_ref()
            .and_then(|map| map.get(&next_node.id))
            .map(|&idx| prev_nodes.unwrap().get(idx).unwrap());

        if let Some(prev_node) = maybe_prev {
            let mut changes = NodeChanges::default();
            if prev_node.label != next_node.label {
                changes.label_changed = true;
            }
            if prev_node.hint != next_node.hint {
                changes.hint_changed = true;
            }
            if accessibility_value_inputs(prev_node) != accessibility_value_inputs(next_node) {
                changes.value_changed = true;
            }
            if prev_node.frame != next_node.frame {
                changes.frame_changed = true;
            }
            if traits_inputs_changed(prev_node, next_node) {
                changes.traits_changed = true;
            }

            if changes.any() {
                scratch.changed.push((next_idx, changes));
            } else {
                scratch.unchanged.push(next_idx);
            }
        } else {
            scratch.added.push(next_idx);
        }
    }

    if let Some(prev_list) = prev_nodes {
        scratch.removed.extend(
            prev_list
                .iter()
                .enumerate()
                .filter(|(_, node)| !next_id_map.contains_key(node.id.as_str()))
                .map(|(idx, _)| idx),
        );
    }

    scratch.order_changed = match prev_nodes {
        None => true,
        Some(prev_list) => {
            next_nodes.len() != prev_list.len()
                || next_nodes
                    .iter()
                    .zip(prev_list.iter())
                    .any(|(next, prev)| next.id != prev.id)
        }
    };
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
#[path = "accessibility/tests.rs"]
mod tests;
