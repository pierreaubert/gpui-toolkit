//! Focused keyboard, pointer capture, and ordered grid trajectory handling.

// Rust guideline compliant 2026-02-21

use super::types::{Scene2DGridCell, Scene2DInputConfig, Scene2DScene, ScenePoint};
use std::collections::HashMap;

/// Pointer phase delivered to a Scene2D surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DPointerPhase {
    /// A contact or mouse button was pressed.
    Down,
    /// An active pointer moved.
    Move,
    /// A contact or mouse button was released.
    Up,
    /// An active pointer was cancelled.
    Cancel,
}

/// Pointer device associated with an input event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DPointerDevice {
    /// A desktop or trackpad cursor.
    #[default]
    Mouse,
    /// A direct finger contact.
    Touch,
    /// A stylus or pen.
    Pen,
}

/// Keyboard event phase delivered to a focused surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DKeyPhase {
    /// A key became pressed.
    Down,
    /// A key was released.
    Up,
}

/// Modifier key state attached to pointer and keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DModifier {
    /// Shift key.
    Shift,
    /// Control key.
    Control,
    /// Alt or Option key.
    Alt,
    /// Meta, Command, or Windows key.
    Meta,
    /// Function key.
    Fn,
}

/// Mouse button state attached to pointer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
    /// Back mouse button.
    Back,
    /// Forward mouse button.
    Forward,
}

/// Reason a surface clears captured or held input state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DLifecycleReason {
    /// The surface lost keyboard focus.
    FocusLost,
    /// Its containing window or application was suspended.
    Suspended,
    /// The surface was removed from its view.
    Removed,
    /// The active showcase or application section changed.
    SectionChanged,
    /// The connection supplying input or scene state ended.
    Disconnected,
    /// The containing window or application became active again.
    Resumed,
}

/// Normalized event emitted from a surface to its controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DInput {
    /// A pointer event with resolved object and cell identity.
    Pointer {
        /// Pointer lifecycle phase.
        phase: Scene2DPointerPhase,
        /// Source device.
        device: Scene2DPointerDevice,
        /// Stable pointer or contact identity.
        contact_id: u64,
        /// Monotonic timestamp since this surface state was created, in nanoseconds.
        timestamp_ns: u64,
        /// Scene logical coordinate.
        position: ScenePoint,
        /// Buttons currently held.
        #[serde(default)]
        buttons: Vec<Scene2DButton>,
        /// Keyboard modifiers currently held.
        #[serde(default)]
        modifiers: Vec<Scene2DModifier>,
        /// Topmost explicit object hit identifier.
        #[serde(default)]
        hit_id: Option<String>,
        /// Grid cell under the pointer, when applicable.
        #[serde(default)]
        cell: Option<Scene2DGridCell>,
    },
    /// A focused key event.
    Key {
        /// Keyboard lifecycle phase.
        phase: Scene2DKeyPhase,
        /// Normalized key name.
        key: String,
        /// True for an operating-system key repeat.
        repeat: bool,
        /// Keyboard modifiers currently held.
        #[serde(default)]
        modifiers: Vec<Scene2DModifier>,
        /// Monotonic timestamp since this surface state was created, in nanoseconds.
        timestamp_ns: u64,
    },
    /// A focus, visibility, or lifecycle boundary.
    Lifecycle {
        /// Why the controller should clear held state.
        reason: Scene2DLifecycleReason,
        /// Monotonic timestamp since this surface state was created, in nanoseconds.
        timestamp_ns: u64,
    },
    /// Completion signal for a native node transition.
    TransitionComplete {
        /// Stable node identifier.
        id: String,
        /// Controller-provided completion identifier.
        completion_id: String,
        /// Monotonic timestamp since this surface state was created, in nanoseconds.
        timestamp_ns: u64,
    },
    /// Accessibility activation of a semantic drawing object.
    Activate {
        /// Stable semantic object identifier.
        id: String,
        /// Optional semantic hit target identifier.
        #[serde(default)]
        hit_id: Option<String>,
        /// Grid cell represented by the activated object, when applicable.
        #[serde(default)]
        cell: Option<Scene2DGridCell>,
        /// Monotonic timestamp since this surface state was created, in nanoseconds.
        timestamp_ns: u64,
    },
}

/// Raw pointer sample supplied by GPUI or a direct-contact adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene2DPointerEvent {
    /// Pointer lifecycle phase.
    pub phase: Scene2DPointerPhase,
    /// Source device.
    pub device: Scene2DPointerDevice,
    /// Stable pointer or contact identity.
    pub contact_id: u64,
    /// Monotonic timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Scene logical coordinate, including coordinates outside the view box.
    pub position: ScenePoint,
    /// Buttons currently held.
    pub buttons: Vec<Scene2DButton>,
    /// Keyboard modifiers currently held.
    pub modifiers: Vec<Scene2DModifier>,
}

impl Scene2DPointerEvent {
    /// Creates a pointer sample with no buttons or modifiers.
    pub fn new(
        phase: Scene2DPointerPhase,
        device: Scene2DPointerDevice,
        contact_id: u64,
        timestamp_ns: u64,
        position: ScenePoint,
    ) -> Self {
        Self {
            phase,
            device,
            contact_id,
            timestamp_ns,
            position,
            buttons: Vec::new(),
            modifiers: Vec::new(),
        }
    }
}

/// Stateful input normalizer for one retained Scene2D surface.
#[derive(Debug, Default)]
pub struct Scene2DInputRouter {
    contacts: HashMap<u64, ActiveContact>,
    contact_order: Vec<u64>,
    held_keys: HashMap<String, Vec<Scene2DModifier>>,
    key_order: Vec<String>,
}

#[derive(Debug, Clone)]
struct ActiveContact {
    device: Scene2DPointerDevice,
    position: ScenePoint,
    buttons: Vec<Scene2DButton>,
    modifiers: Vec<Scene2DModifier>,
    last_cell: Option<Scene2DGridCell>,
}

impl Scene2DInputRouter {
    /// Creates an input router with no captured contacts or held keys.
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes a pointer event and adds hit identities and crossed cells.
    ///
    /// A captured contact remains active until up or cancel. Continuous grid
    /// moves emit one event for each newly crossed cell in trajectory order.
    pub fn route_pointer(
        &mut self,
        scene: &Scene2DScene,
        event: Scene2DPointerEvent,
    ) -> Vec<Scene2DInput> {
        if !scene.input.pointer || !event.position.is_finite() {
            return Vec::new();
        }
        match event.phase {
            Scene2DPointerPhase::Down => self.pointer_down(scene, event),
            Scene2DPointerPhase::Move => self.pointer_move(scene, event),
            Scene2DPointerPhase::Up => self.pointer_end(scene, event, Scene2DPointerPhase::Up),
            Scene2DPointerPhase::Cancel => {
                self.pointer_end(scene, event, Scene2DPointerPhase::Cancel)
            }
        }
    }

    /// Processes a focused key event when its normalized name is enabled.
    pub fn route_key(
        &mut self,
        config: &Scene2DInputConfig,
        phase: Scene2DKeyPhase,
        key: &str,
        repeat: bool,
        modifiers: Vec<Scene2DModifier>,
        timestamp_ns: u64,
    ) -> Option<Scene2DInput> {
        let key = normalize_key(key);
        if !config.keyboard {
            return None;
        }
        let is_system_shortcut = modifiers.iter().any(|modifier| {
            matches!(
                modifier,
                Scene2DModifier::Control | Scene2DModifier::Alt | Scene2DModifier::Meta
            )
        });
        // GPUI reports printable key names in lower case, and the normalizer
        // preserves that case for letters. Keep Tab out of the scene input
        // stream so the window can perform its normal focus traversal.
        let is_tab = key.eq_ignore_ascii_case("tab");
        if (is_tab || is_system_shortcut)
            && (phase == Scene2DKeyPhase::Down || !self.held_keys.contains_key(&key))
        {
            return None;
        }
        match phase {
            Scene2DKeyPhase::Down => {
                if !self.held_keys.contains_key(&key) {
                    self.key_order.push(key.clone());
                }
                self.held_keys.insert(key.clone(), modifiers.clone());
            }
            Scene2DKeyPhase::Up => {
                if self.held_keys.remove(&key).is_none() {
                    return None;
                }
                self.key_order.retain(|held| held != &key);
            }
        }
        Some(Scene2DInput::Key {
            phase,
            key,
            repeat,
            modifiers,
            timestamp_ns,
        })
    }

    /// Cancels all active pointers and keys, then emits one lifecycle event.
    pub fn cancel_all(
        &mut self,
        reason: Scene2DLifecycleReason,
        timestamp_ns: u64,
    ) -> Vec<Scene2DInput> {
        let mut events = Vec::with_capacity(self.contacts.len() + self.held_keys.len() + 1);
        for contact_id in std::mem::take(&mut self.contact_order) {
            let Some(contact) = self.contacts.remove(&contact_id) else {
                continue;
            };
            events.push(Scene2DInput::Pointer {
                phase: Scene2DPointerPhase::Cancel,
                device: contact.device,
                contact_id,
                timestamp_ns,
                position: contact.position,
                buttons: contact.buttons,
                modifiers: contact.modifiers,
                hit_id: None,
                cell: None,
            });
        }
        self.contacts.clear();
        for key in std::mem::take(&mut self.key_order) {
            let Some(modifiers) = self.held_keys.remove(&key) else {
                continue;
            };
            events.push(Scene2DInput::Key {
                phase: Scene2DKeyPhase::Up,
                key,
                repeat: false,
                modifiers,
                timestamp_ns,
            });
        }
        self.held_keys.clear();
        events.push(Scene2DInput::Lifecycle {
            reason,
            timestamp_ns,
        });
        events
    }

    /// Returns whether a pointer is currently captured by this surface.
    pub fn is_captured(&self, contact_id: u64) -> bool {
        self.contacts.contains_key(&contact_id)
    }

    /// Returns whether the router has any active pointer or held key.
    pub fn has_active_input(&self) -> bool {
        !self.contacts.is_empty() || !self.held_keys.is_empty()
    }

    fn pointer_down(
        &mut self,
        scene: &Scene2DScene,
        event: Scene2DPointerEvent,
    ) -> Vec<Scene2DInput> {
        let hit = scene.hit_test(event.position);
        self.contact_order
            .retain(|contact_id| *contact_id != event.contact_id);
        self.contact_order.push(event.contact_id);
        self.contacts.insert(
            event.contact_id,
            ActiveContact {
                device: event.device,
                position: event.position,
                buttons: event.buttons.clone(),
                modifiers: event.modifiers.clone(),
                last_cell: hit.cell.clone(),
            },
        );
        vec![pointer_input(event, hit.hit_id, hit.cell)]
    }

    fn pointer_move(
        &mut self,
        scene: &Scene2DScene,
        event: Scene2DPointerEvent,
    ) -> Vec<Scene2DInput> {
        let hit = scene.hit_test(event.position);
        let Some(contact) = self.contacts.get_mut(&event.contact_id) else {
            if !scene.input.continuous {
                return Vec::new();
            }
            return vec![pointer_input(event, hit.hit_id, hit.cell)];
        };
        let mut output = Vec::new();
        if scene.input.continuous {
            let crossed = scene
                .grid
                .as_ref()
                .map(|grid| grid.crossed_cells(contact.position, event.position))
                .unwrap_or_default();
            let mut emitted_new_cell = false;
            for cell in crossed {
                if contact
                    .last_cell
                    .as_ref()
                    .is_some_and(|last| last.index == cell.index)
                {
                    continue;
                }
                let position = scene
                    .grid
                    .as_ref()
                    .map(|grid| {
                        ScenePoint::new(
                            grid.x
                                + cell.column as f32 * (grid.cell_width + grid.gap)
                                + grid.cell_width * 0.5,
                            grid.y
                                + cell.row as f32 * (grid.cell_height + grid.gap)
                                + grid.cell_height * 0.5,
                        )
                    })
                    .unwrap_or(event.position);
                let node_hit = scene.hit_test(position).hit_id;
                output.push(Scene2DInput::Pointer {
                    phase: Scene2DPointerPhase::Move,
                    device: event.device,
                    contact_id: event.contact_id,
                    timestamp_ns: event.timestamp_ns,
                    position,
                    buttons: event.buttons.clone(),
                    modifiers: event.modifiers.clone(),
                    hit_id: node_hit.or_else(|| Some(cell.id.clone())),
                    cell: Some(cell.clone()),
                });
                contact.last_cell = Some(cell);
                emitted_new_cell = true;
            }
            if !emitted_new_cell {
                output.push(pointer_input(event.clone(), hit.hit_id, hit.cell.clone()));
            }
        }
        contact.position = event.position;
        contact.buttons = event.buttons;
        contact.modifiers = event.modifiers;
        if hit.cell.is_some() {
            contact.last_cell = hit.cell;
        }
        output
    }

    fn pointer_end(
        &mut self,
        scene: &Scene2DScene,
        event: Scene2DPointerEvent,
        phase: Scene2DPointerPhase,
    ) -> Vec<Scene2DInput> {
        if self.contacts.remove(&event.contact_id).is_none() {
            return Vec::new();
        }
        self.contact_order
            .retain(|contact_id| *contact_id != event.contact_id);
        let hit = scene.hit_test(event.position);
        vec![pointer_input_with_phase(event, phase, hit.hit_id, hit.cell)]
    }
}

fn pointer_input(
    event: Scene2DPointerEvent,
    hit_id: Option<String>,
    cell: Option<Scene2DGridCell>,
) -> Scene2DInput {
    let phase = event.phase;
    pointer_input_with_phase(event, phase, hit_id, cell)
}

fn pointer_input_with_phase(
    event: Scene2DPointerEvent,
    phase: Scene2DPointerPhase,
    hit_id: Option<String>,
    cell: Option<Scene2DGridCell>,
) -> Scene2DInput {
    Scene2DInput::Pointer {
        phase,
        device: event.device,
        contact_id: event.contact_id,
        timestamp_ns: event.timestamp_ns,
        position: event.position,
        buttons: event.buttons,
        modifiers: event.modifiers,
        hit_id,
        cell,
    }
}

/// Normalizes key names to the cross-platform Scene2D contract.
pub fn normalize_key(key: &str) -> String {
    let normalized = key.to_lowercase();
    match normalized.as_str() {
        "left" | "arrowleft" => "ArrowLeft".to_owned(),
        "right" | "arrowright" => "ArrowRight".to_owned(),
        "up" | "arrowup" => "ArrowUp".to_owned(),
        "down" | "arrowdown" => "ArrowDown".to_owned(),
        "enter" | "return" => "Enter".to_owned(),
        "tab" => "Tab".to_owned(),
        "space" | " " => "Space".to_owned(),
        "escape" | "esc" => "Escape".to_owned(),
        "backspace" => "Backspace".to_owned(),
        "delete" | "del" => "Delete".to_owned(),
        printable => printable.to_owned(),
    }
}

// The serde imports are kept local to input because these event types cross
// the Python host boundary.
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene2d::{
        Scene2DBrush, Scene2DColor, Scene2DGrid, Scene2DNode, Scene2DNodeKind, SceneRect,
    };

    fn scene(continuous: bool) -> Scene2DScene {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 40.0, 20.0));
        scene.input.pointer = true;
        scene.input.continuous = continuous;
        scene.input.capture = true;
        scene.input.keyboard = true;
        scene.grid = Some(Scene2DGrid {
            rows: 2,
            columns: 4,
            x: 0.0,
            y: 0.0,
            cell_width: 10.0,
            cell_height: 10.0,
            gap: 0.0,
            row_labels: Vec::new(),
            column_labels: Vec::new(),
        });
        scene.nodes.push(Scene2DNode {
            id: "cell-0".to_owned(),
            hit_id: Some("first-cell".to_owned()),
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Default::default(),
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(0.0, 0.0, 10.0, 10.0),
                fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.2, 0.3, 0.4))),
                stroke: None,
            },
        });
        scene
    }

    #[test]
    fn captured_grid_moves_emit_every_cell_in_path_order() {
        let scene = scene(true);
        let mut router = Scene2DInputRouter::new();
        let down = router.route_pointer(
            &scene,
            Scene2DPointerEvent::new(
                Scene2DPointerPhase::Down,
                Scene2DPointerDevice::Touch,
                7,
                1,
                ScenePoint::new(5.0, 5.0),
            ),
        );
        assert_eq!(down.len(), 1);
        let moves = router.route_pointer(
            &scene,
            Scene2DPointerEvent::new(
                Scene2DPointerPhase::Move,
                Scene2DPointerDevice::Touch,
                7,
                2,
                ScenePoint::new(35.0, 5.0),
            ),
        );
        let cells = moves
            .iter()
            .map(|event| match event {
                Scene2DInput::Pointer {
                    cell: Some(cell), ..
                } => cell.id.as_str(),
                _ => panic!("grid drag should report each crossed cell"),
            })
            .collect::<Vec<_>>();
        assert_eq!(cells, ["r0c1", "r0c2", "r0c3"]);
        assert!(router.is_captured(7));
    }

    #[test]
    fn pointer_cancel_and_focus_loss_clear_captured_and_held_state() {
        let scene = scene(false);
        let mut router = Scene2DInputRouter::new();
        router.route_pointer(
            &scene,
            Scene2DPointerEvent::new(
                Scene2DPointerPhase::Down,
                Scene2DPointerDevice::Mouse,
                0,
                1,
                ScenePoint::new(5.0, 5.0),
            ),
        );
        router.route_key(
            &scene.input,
            Scene2DKeyPhase::Down,
            "left",
            false,
            Vec::new(),
            2,
        );
        let cancelled = router.cancel_all(Scene2DLifecycleReason::FocusLost, 3);
        assert_eq!(cancelled.len(), 3);
        assert!(!router.has_active_input());
        assert!(matches!(
            cancelled.last(),
            Some(Scene2DInput::Lifecycle {
                reason: Scene2DLifecycleReason::FocusLost,
                ..
            })
        ));
    }

    #[test]
    fn lifecycle_cancellation_preserves_pointer_and_key_press_order() {
        let scene = scene(false);
        let mut router = Scene2DInputRouter::new();
        for (contact_id, x) in [(9, 5.0), (3, 15.0)] {
            router.route_pointer(
                &scene,
                Scene2DPointerEvent::new(
                    Scene2DPointerPhase::Down,
                    Scene2DPointerDevice::Touch,
                    contact_id,
                    contact_id,
                    ScenePoint::new(x, 5.0),
                ),
            );
        }
        for key in ["z", "a"] {
            router.route_key(
                &scene.input,
                Scene2DKeyPhase::Down,
                key,
                false,
                Vec::new(),
                20,
            );
        }
        let events = router.cancel_all(Scene2DLifecycleReason::Suspended, 21);
        assert!(matches!(
            events[0],
            Scene2DInput::Pointer { contact_id: 9, .. }
        ));
        assert!(matches!(
            events[1],
            Scene2DInput::Pointer { contact_id: 3, .. }
        ));
        assert!(matches!(events[2], Scene2DInput::Key { ref key, .. } if key == "z"));
        assert!(matches!(events[3], Scene2DInput::Key { ref key, .. } if key == "a"));
    }

    #[test]
    fn key_names_are_normalized_and_releases_require_a_prior_press() {
        let config = scene(false).input;
        let mut router = Scene2DInputRouter::new();
        assert!(
            router
                .route_key(&config, Scene2DKeyPhase::Up, "left", false, Vec::new(), 1)
                .is_none()
        );
        let down = router
            .route_key(
                &config,
                Scene2DKeyPhase::Down,
                "left",
                false,
                vec![Scene2DModifier::Shift],
                2,
            )
            .unwrap();
        assert!(matches!(
            down,
            Scene2DInput::Key { key, .. } if key == "ArrowLeft"
        ));
    }

    #[test]
    fn input_serializes_with_versioned_event_tag_and_resolved_cell() {
        let scene = scene(true);
        let mut router = Scene2DInputRouter::new();
        let event = router.route_pointer(
            &scene,
            Scene2DPointerEvent::new(
                Scene2DPointerPhase::Down,
                Scene2DPointerDevice::Mouse,
                0,
                5,
                ScenePoint::new(5.0, 5.0),
            ),
        );
        let value = serde_json::to_value(&event[0]).unwrap();
        assert_eq!(value["type"], "pointer");
        assert_eq!(value["phase"], "down");
        assert_eq!(value["cell"]["id"], "r0c0");
        assert_eq!(value["hit_id"], "first-cell");
    }

    #[test]
    fn keyboard_wire_switch_is_boolean_and_normalizes_printable_names() {
        let mut config = Scene2DInputConfig::default();
        assert_eq!(serde_json::to_value(&config).unwrap()["keyboard"], true);
        config.keyboard = true;
        assert_eq!(serde_json::to_value(&config).unwrap()["keyboard"], true);
        assert_eq!(normalize_key("A"), "a");
        assert_eq!(normalize_key("return"), "Enter");
    }

    #[test]
    fn tab_and_system_shortcuts_are_left_to_window_dispatch() {
        let config = Scene2DInputConfig::default();
        let mut router = Scene2DInputRouter::new();
        assert!(
            router
                .route_key(&config, Scene2DKeyPhase::Down, "Tab", false, Vec::new(), 1,)
                .is_none()
        );
        assert!(
            router
                .route_key(
                    &config,
                    Scene2DKeyPhase::Down,
                    "a",
                    false,
                    vec![Scene2DModifier::Meta],
                    2,
                )
                .is_none()
        );
        assert!(
            router
                .route_key(
                    &config,
                    Scene2DKeyPhase::Down,
                    "ArrowLeft",
                    false,
                    Vec::new(),
                    3,
                )
                .is_some()
        );
    }
}
