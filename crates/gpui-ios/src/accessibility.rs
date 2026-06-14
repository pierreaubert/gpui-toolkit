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

static ACCESSIBILITY_SNAPSHOT: OnceLock<Mutex<Option<Arc<IosAccessibilitySnapshot>>>> = OnceLock::new();
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
}
