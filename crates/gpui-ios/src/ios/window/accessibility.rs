use super::misc::{
    UIAccessibilityTraitAdjustable, UIAccessibilityTraitButton, UIAccessibilityTraitHeader,
    UIAccessibilityTraitImage, UIAccessibilityTraitLink, UIAccessibilityTraitNotEnabled,
    UIAccessibilityTraitSearchField, UIAccessibilityTraitSelected, UIAccessibilityTraitStaticText,
    UIAccessibilityTraitUpdatesFrequently,
};

use super::ios_window::IosWindow;
use super::misc::{
    UIAccessibilityAnnouncementNotification, UIAccessibilityLayoutChangedNotification,
    UIAccessibilityPostNotification,
};
use super::register::register_accessibility_element_class;
use objc::{class, msg_send, runtime::Object, sel, sel_impl};

pub(super) fn accessibility_traits_for_node(
    node: &crate::accessibility::IosAccessibilityNode,
) -> u64 {
    use crate::accessibility::{IosAccessibilityAction, IosAccessibilityRole};

    let mut traits = 0_u64;
    unsafe {
        // SAFETY: These UIKit constants are process-lifetime globals exported
        // by the UIKit framework linked into iOS/tvOS binaries.
        traits |= match node.role {
            IosAccessibilityRole::Button
            | IosAccessibilityRole::Checkbox
            | IosAccessibilityRole::Switch
            | IosAccessibilityRole::Tab => UIAccessibilityTraitButton,
            IosAccessibilityRole::Link => UIAccessibilityTraitLink,
            IosAccessibilityRole::Header => UIAccessibilityTraitHeader,
            IosAccessibilityRole::Image => UIAccessibilityTraitImage,
            IosAccessibilityRole::SearchField => UIAccessibilityTraitSearchField,
            IosAccessibilityRole::Slider | IosAccessibilityRole::Adjustable => {
                UIAccessibilityTraitAdjustable
            }
            IosAccessibilityRole::StaticText => UIAccessibilityTraitStaticText,
            IosAccessibilityRole::TextField
            | IosAccessibilityRole::Container
            | IosAccessibilityRole::None => 0,
        };

        if node.selected {
            traits |= UIAccessibilityTraitSelected;
        }
        if !node.enabled {
            traits |= UIAccessibilityTraitNotEnabled;
        }
        if node.actions.iter().any(|action| {
            matches!(
                action,
                IosAccessibilityAction::Increment | IosAccessibilityAction::Decrement
            )
        }) {
            traits |= UIAccessibilityTraitAdjustable;
        }
        if node.value.as_ref().is_some_and(|value| value.len() > 16)
            && matches!(node.role, IosAccessibilityRole::StaticText)
        {
            traits |= UIAccessibilityTraitUpdatesFrequently;
        }
    }

    traits
}

pub(super) fn accessibility_value_for_node(
    node: &crate::accessibility::IosAccessibilityNode,
) -> Option<String> {
    match (node.value.as_deref(), node.expanded) {
        (Some(value), Some(true)) if !value.is_empty() => Some(format!("{value}, expanded")),
        (Some(value), Some(false)) if !value.is_empty() => Some(format!("{value}, collapsed")),
        (Some(value), _) if !value.is_empty() => Some(value.to_string()),
        (_, Some(true)) => Some("expanded".to_string()),
        (_, Some(false)) => Some("collapsed".to_string()),
        _ => None,
    }
}

impl IosWindow {
    pub fn refresh_accessibility(&self) {
        use crate::accessibility::compute_accessibility_diff_into;

        let snapshot = crate::accessibility::accessibility_snapshot();
        let element_count = snapshot
            .as_ref()
            .map(|snapshot| snapshot.flattened_node_slice().len())
            .unwrap_or_default();
        crate::instrumentation::emit_signpost(
            crate::instrumentation::IosSignpostCategory::Accessibility,
            format!("accessibility_nodes={element_count}"),
        );

        let prev_snapshot = self.prev_accessibility_snapshot.borrow().clone();
        let mut diff = self.accessibility_diff_scratch.borrow_mut();
        let has_diff = if let Some(next) = snapshot.as_ref() {
            compute_accessibility_diff_into(prev_snapshot.as_deref(), next, &mut diff);
            true
        } else {
            diff.clear();
            false
        };

        // Store the current snapshot for the next diff, even on non-Apple hosts
        // where the UIKit mutations below are a no-op.
        *self.prev_accessibility_snapshot.borrow_mut() = snapshot.clone();

        #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
        let _ = has_diff;

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        unsafe {
            // SAFETY: All UIKit accessibility objects are created and assigned
            // on the main thread while `self.view` is a live UIView owned by
            // this IosWindow. UIKit retains the array assigned through the
            // `accessibilityElements` property.
            if self.view.is_null() {
                return;
            }

            let _: () = msg_send![self.view, setIsAccessibilityElement: false];

            let Some(snapshot) = snapshot.as_ref() else {
                self.clear_accessibility_elements();
                let _: () = msg_send![
                    self.view,
                    setAccessibilityElements: std::ptr::null_mut::<Object>()
                ];
                return;
            };

            if !has_diff {
                return;
            }
            let nodes = snapshot.flattened_node_slice();

            let element_class = register_accessibility_element_class();
            let mut elements_map = self.accessibility_elements.borrow_mut();

            // Create or update elements for the current snapshot.
            for &idx in diff.added_indices() {
                let node = &nodes[idx];
                let element: *mut Object = msg_send![element_class, alloc];
                let element: *mut Object = msg_send![
                    element,
                    initWithAccessibilityContainer: self.view
                ];
                if element.is_null() {
                    continue;
                }
                elements_map.insert(node.id.clone(), element);

                let id = super::super::ns_string_from_str(&node.id);
                let _: () = msg_send![element, setAccessibilityIdentifier: id];

                if let Some(label) = node.label.as_deref() {
                    let label = super::super::ns_string_from_str(label);
                    let _: () = msg_send![element, setAccessibilityLabel: label];
                }
                if let Some(hint) = node.hint.as_deref() {
                    let hint = super::super::ns_string_from_str(hint);
                    let _: () = msg_send![element, setAccessibilityHint: hint];
                }
                if let Some(value) = accessibility_value_for_node(node) {
                    let value = super::super::ns_string_from_str(&value);
                    let _: () = msg_send![element, setAccessibilityValue: value];
                }

                let frame = core_graphics::geometry::CGRect {
                    origin: core_graphics::geometry::CGPoint {
                        x: node.frame.x as core_graphics::base::CGFloat,
                        y: node.frame.y as core_graphics::base::CGFloat,
                    },
                    size: core_graphics::geometry::CGSize {
                        width: node.frame.width as core_graphics::base::CGFloat,
                        height: node.frame.height as core_graphics::base::CGFloat,
                    },
                };
                let _: () = msg_send![element, setAccessibilityFrameInContainerSpace: frame];

                let traits = accessibility_traits_for_node(node);
                let _: () = msg_send![element, setAccessibilityTraits: traits];
            }

            for &(idx, changes) in diff.changed_indices() {
                let node = &nodes[idx];
                let Some(&element) = elements_map.get(&node.id) else {
                    continue;
                };

                if changes.label_changed
                    && let Some(label) = node.label.as_deref()
                {
                    let label = super::super::ns_string_from_str(label);
                    let _: () = msg_send![element, setAccessibilityLabel: label];
                }
                if changes.hint_changed
                    && let Some(hint) = node.hint.as_deref()
                {
                    let hint = super::super::ns_string_from_str(hint);
                    let _: () = msg_send![element, setAccessibilityHint: hint];
                }
                if changes.value_changed
                    && let Some(value) = accessibility_value_for_node(node)
                {
                    let value = super::super::ns_string_from_str(&value);
                    let _: () = msg_send![element, setAccessibilityValue: value];
                }
                if changes.frame_changed {
                    let frame = core_graphics::geometry::CGRect {
                        origin: core_graphics::geometry::CGPoint {
                            x: node.frame.x as core_graphics::base::CGFloat,
                            y: node.frame.y as core_graphics::base::CGFloat,
                        },
                        size: core_graphics::geometry::CGSize {
                            width: node.frame.width as core_graphics::base::CGFloat,
                            height: node.frame.height as core_graphics::base::CGFloat,
                        },
                    };
                    let _: () = msg_send![element, setAccessibilityFrameInContainerSpace: frame];
                }
                if changes.traits_changed {
                    let traits = accessibility_traits_for_node(node);
                    let _: () = msg_send![element, setAccessibilityTraits: traits];
                }
            }

            // Removed indices resolve against the previous cached snapshot,
            // avoiding temporary HashSet/String collections.
            if let Some(prev) = prev_snapshot.as_ref() {
                let prev_nodes = prev.flattened_node_slice();
                for &idx in diff.removed_indices() {
                    if let Some(element) = elements_map.remove(&prev_nodes[idx].id) {
                        let _: () = msg_send![element, release];
                    }
                }
            }

            // Only rebuild the `accessibilityElements` array and post the layout
            // notification when the node set or ordering changed.
            if diff.order_changed {
                let ordered_elements: Vec<*mut Object> = snapshot
                    .flattened_node_slice()
                    .iter()
                    .filter_map(|node| elements_map.get(&node.id).copied())
                    .collect();
                drop(elements_map);

                let elements: *mut Object =
                    msg_send![class!(NSMutableArray), arrayWithCapacity: ordered_elements.len()];
                for element in ordered_elements.iter().copied() {
                    let _: () = msg_send![elements, addObject: element];
                }
                let _: () = msg_send![self.view, setAccessibilityElements: elements];

                if element_count > 0 {
                    let first_element: *mut Object = msg_send![elements, firstObject];
                    UIAccessibilityPostNotification(
                        UIAccessibilityLayoutChangedNotification,
                        first_element,
                    );
                }
            } else {
                drop(elements_map);
            }

            for announcement in &snapshot.announcements {
                let announcement = super::super::ns_string_from_str(announcement);
                UIAccessibilityPostNotification(
                    UIAccessibilityAnnouncementNotification,
                    announcement,
                );
            }
        }
    }

    fn clear_accessibility_elements(&self) {
        unsafe {
            for element in self.accessibility_elements.borrow_mut().drain() {
                if !element.1.is_null() {
                    let _: () = msg_send![element.1, release];
                }
            }
        }
    }
}
