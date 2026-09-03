//! RadioGroup component
//!
//! A single-select option group with keyboard navigation, mirroring native
//! radio-group semantics: arrow keys move *and* select, `Space`/`Enter`
//! re-affirm the highlighted option, and screen readers see a `radiogroup`
//! containing `radio` options with `checked` state.
//!
//! State is parent-owned (like [`Checkbox`]): pass `selected` plus
//! `on_change` and the group stays fully controlled.

use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use crate::{ComponentTheme, ComponentVariant};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled,
};
use gpui::{
    App, Div, ElementId, MouseButton, Pixels, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for radio group styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct RadioGroupTheme {
    /// Outer circle border when selected
    #[theme(default = 0x007acc, from = accent)]
    pub selected_border: Rgba,
    /// Inner dot when selected
    #[theme(default = 0x007acc, from = accent)]
    pub selected_dot: Rgba,
    /// Outer circle border when unselected
    #[theme(default = 0x555555, from = border)]
    pub unselected_border: Rgba,
    /// Option label color
    #[theme(default = 0xcccccc, from = text_secondary)]
    pub label: Rgba,
    /// Border on hover
    #[theme(default = 0x007acc, from = accent)]
    pub hover_border: Rgba,
    /// Label color for disabled options
    #[theme(default = 0x9c9c9c, from = text_muted)]
    pub disabled_label: Rgba,
}

/// Radio group layout orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ComponentVariant)]
pub enum RadioGroupOrientation {
    /// Options stacked vertically (default)
    #[default]
    Vertical,
    /// Options laid out horizontally
    Horizontal,
}

/// Radio group size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ComponentVariant)]
pub enum RadioGroupSize {
    /// Small (14px circles)
    Sm,
    /// Medium (18px circles, default)
    #[default]
    Md,
    /// Large (22px circles)
    Lg,
}

impl RadioGroupSize {
    fn circle_with_design(&self, design: &DesignSystem) -> Pixels {
        match self {
            RadioGroupSize::Sm => px(design.interaction.min_touch_target * 0.4375),
            RadioGroupSize::Md => px(design.interaction.min_touch_target * 0.5625),
            RadioGroupSize::Lg => px(design.interaction.min_touch_target * 0.6875),
        }
    }
}

impl From<crate::ComponentSize> for RadioGroupSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// A single selectable option inside a [`RadioGroup`].
#[derive(Debug, Clone)]
pub struct RadioOption {
    /// Stable value reported to `on_change`.
    pub value: SharedString,
    /// Visible label next to the circle.
    pub label: SharedString,
    /// Whether this option can be selected.
    pub disabled: bool,
}

impl RadioOption {
    /// Create an option with a value and label.
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Mark the option as non-selectable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A single-select radio group.
///
/// The parent owns `selected`; arrow keys move selection to the nearest
/// enabled option in that direction.
pub struct RadioGroup {
    id: ElementId,
    options: Vec<RadioOption>,
    option_ids: Vec<ElementId>,
    selected: Option<SharedString>,
    orientation: RadioGroupOrientation,
    size: RadioGroupSize,
    disabled: bool,
    design: Option<Arc<DesignSystem>>,
    on_change: Option<Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl RadioGroup {
    /// Create a new empty radio group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            option_ids: Vec::new(),
            selected: None,
            orientation: RadioGroupOrientation::default(),
            size: RadioGroupSize::default(),
            disabled: false,
            design: None,
            on_change: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Refresh the stable element IDs for each option (scoped to the group
    /// id so two groups on one screen never collide). IDs are precomputed
    /// when options change so render performs no per-option allocation.
    fn refresh_option_ids(&mut self) {
        self.option_ids = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, _)| (self.id.clone(), idx.to_string()).into())
            .collect();
    }

    /// Set the full option list (replaces any previous options).
    pub fn options(mut self, options: Vec<RadioOption>) -> Self {
        self.options = options;
        self.refresh_option_ids();
        self
    }

    /// Set the selected option value (`None` selects nothing).
    pub fn selected(mut self, value: Option<SharedString>) -> Self {
        self.selected = value;
        self
    }

    /// Set the layout orientation.
    pub fn orientation(mut self, orientation: RadioGroupOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set the circle size.
    pub fn size(mut self, size: RadioGroupSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the whole group.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Override the design system used for sizing and spacing.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the selection handler (receives the selected option value).
    pub fn on_change(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label for the group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (`Radiogroup`).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Index of the selected option, if it is still present.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected.as_ref().and_then(|selected| {
            self.options
                .iter()
                .position(|option| &option.value == selected)
        })
    }

    /// Build into an element with an explicit theme.
    pub fn build_with_theme(self, theme: &RadioGroupTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build into an element with a theme and design-system sizing tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &RadioGroupTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let group_label = self.aria_label.clone().unwrap_or_default();
        let group_props =
            AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Radiogroup))
                .maybe_state(self.disabled, AriaState::Disabled);
        let circle = self.size.circle_with_design(design);
        let dot = circle - px(8.0);
        let gap = px(design.spacing.control_gap);

        // `selected_index` borrows all of `self`; compute it before the
        // partial move of `on_change` below.
        let selected_index = self.selected_index();
        let handler_rc = self.on_change.map(std::rc::Rc::new);
        let mut container = div()
            .id(self.id.clone())
            // Test instrumentation: registers the container bounds under
            // `Name("<id>")` in `VisualTestContext::debug_bounds`. The closure
            // only runs with gpui's `test-support` feature; release builds
            // keep the noop and pay nothing.
            .debug_selector(|| format!("{:?}", self.id))
            .flex()
            .flex_none()
            .gap(gap);
        container = match self.orientation {
            RadioGroupOrientation::Vertical => container.flex_col(),
            RadioGroupOrientation::Horizontal => container.flex_row().flex_wrap(),
        };
        if self.disabled {
            container = container.opacity(0.5).cursor_not_allowed();
        }

        for (index, option) in self.options.iter().enumerate() {
            let is_selected = selected_index == Some(index);
            let option_disabled = self.disabled || option.disabled;
            let border_color = if is_selected {
                theme.selected_border
            } else {
                theme.unselected_border
            };
            // Precomputed in `options()`; fall back to a group-scoped id when
            // options were mutated without the setter (never in practice).
            let row_id: ElementId = self
                .option_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| (self.id.clone(), index.to_string()).into());

            let mut circle_el = div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .w(circle)
                .h(circle)
                .rounded_full()
                .border_2()
                .border_color(border_color);

            if is_selected {
                circle_el = circle_el.child(
                    div()
                        .w(dot)
                        .h(dot)
                        .rounded_full()
                        .bg(theme.selected_dot),
                );
            }

            if !option_disabled {
                let hover_border = theme.hover_border;
                circle_el = circle_el.hover(move |s| s.border_color(hover_border));
            }

            let mut row = div()
                .id(row_id)
                // Test instrumentation (see container): option bounds land
                // under `Name("<group-id>")-option-<index>`.
                .debug_selector(|| format!("{:?}-option-{index}", self.id))
                .flex()
                .items_center()
                .gap(px(design.spacing.control_gap / 2.0))
                .child(circle_el)
                .child(
                    div()
                        .text_color(if option_disabled {
                            theme.disabled_label
                        } else {
                            theme.label
                        })
                        .child(option.label.clone()),
                );
            row = match self.size {
                RadioGroupSize::Sm => row.text_xs(),
                RadioGroupSize::Md => row.text_sm(),
                RadioGroupSize::Lg => row,
            };

            if option_disabled {
                row = row.cursor_not_allowed();
            } else {
                row = row.cursor_pointer();
                let option_props = AriaProps::with_role(AriaRole::Radio)
                    .state(AriaState::Checked(is_selected))
                    .maybe_state(option.disabled, AriaState::Disabled);
                let option_label = option.label.clone();
                row = apply_native_accessibility(row, option_label, &option_props);

                if let Some(handler) = handler_rc.clone() {
                    let value = option.value.clone();
                    let click_handler = handler.clone();
                    row = row.on_mouse_up(
                        MouseButton::Left,
                        move |_event, window, cx| {
                            click_handler(value.clone(), window, cx);
                        },
                    );
                }
            }

            container = container.child(row);
        }

        // Keyboard: arrows move selection (wrapping, skipping disabled
        // options); Space/Enter re-affirm the current selection. Selection
        // state is parent-owned, so navigation starts from the selected index
        // captured at render time and reports through `on_change`.
        if !self.disabled
            && let Some(handler) = handler_rc.clone()
        {
            let options = self.options.clone();
            container = container.on_key_down(move |event, window, cx| {
                let direction = match event.keystroke.key.as_str() {
                    "up" | "left" => Some(-1),
                    "down" | "right" => Some(1),
                    "space" | " " | "enter" => Some(0),
                    _ => None,
                };
                if let Some(direction) = direction
                    && let Some(next) =
                        step_enabled_index(&options, selected_index, direction)
                    && let Some(option) = options.get(next)
                {
                    handler(option.value.clone(), window, cx);
                }
            });
        }

        apply_native_accessibility(container, group_label, &group_props)
    }
}

/// Nearest enabled option index for arrow navigation.
///
/// `direction` is +1 (forward) or -1 (back) with wraparound, or 0 to stay on
/// the current selection (`Space`/`Enter` re-affirm). Starts from `from`, or
/// from the first/last enabled option when nothing is selected. Returns
/// `None` when every option is disabled.
fn step_enabled_index(
    options: &[RadioOption],
    from: Option<usize>,
    direction: i32,
) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    if direction == 0 {
        return from.filter(|index| {
            options
                .get(*index)
                .is_some_and(|option| !option.disabled)
        });
    }
    let len = options.len();
    let mut index = from.map_or_else(
        || {
            if direction >= 0 {
                0
            } else {
                len - 1
            }
        },
        |current| (current as i32 + direction).rem_euclid(len as i32) as usize,
    );
    for _ in 0..len {
        if options
            .get(index)
            .is_some_and(|option| !option.disabled)
        {
            return Some(index);
        }
        index = (index as i32 + direction).rem_euclid(len as i32) as usize;
    }
    None
}

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let group_label = self.aria_label.clone().unwrap_or_default();
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: group_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Radiogroup))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let radio_theme = RadioGroupTheme::from(global_theme);
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&radio_theme, &design)
    }
}

impl IntoElement for RadioGroup {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_options() -> Vec<RadioOption> {
        vec![
            RadioOption::new("a", "Alpha"),
            RadioOption::new("b", "Beta"),
            RadioOption::new("c", "Gamma").disabled(true),
        ]
    }

    #[test]
    fn step_forward_wraps_and_skips_disabled() {
        let options = test_options();
        // From a: forward lands on b.
        assert_eq!(step_enabled_index(&options, Some(0), 1), Some(1));
        // From b: forward wraps past disabled c to a.
        assert_eq!(step_enabled_index(&options, Some(1), 1), Some(0));
    }

    #[test]
    fn step_back_from_none_starts_at_last_enabled() {
        let options = test_options();
        assert_eq!(step_enabled_index(&options, None, -1), Some(1));
        assert_eq!(step_enabled_index(&options, None, 1), Some(0));
    }

    #[test]
    fn step_zero_reaffirms_only_enabled_selection() {
        let options = test_options();
        assert_eq!(step_enabled_index(&options, Some(1), 0), Some(1));
        // c is disabled: re-affirm refuses.
        assert_eq!(step_enabled_index(&options, Some(2), 0), None);
        assert_eq!(step_enabled_index(&options, None, 0), None);
    }

    #[test]
    fn step_all_disabled_returns_none() {
        let options = vec![
            RadioOption::new("a", "Alpha").disabled(true),
            RadioOption::new("b", "Beta").disabled(true),
        ];
        assert_eq!(step_enabled_index(&options, Some(0), 1), None);
        assert_eq!(step_enabled_index(&options, None, 1), None);
    }
}
