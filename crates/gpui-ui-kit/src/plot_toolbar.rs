//! Compact, keyboard-reachable controls for plot views.
//!
//! The toolbar deliberately reports actions to its owner instead of owning
//! plot state. This keeps menus and camera state in the chart/application
//! model while still giving every visible control a native button, an
//! accessible name, and keyboard activation.

use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::toolbar::{Toolbar, ToolbarItem};
use gpui::{App, SharedString, Window};
use std::rc::Rc;

/// Actions emitted by [`PlotToolbar`]. Menu contents remain owned by the
/// caller, so the caller can choose the appropriate mode/view options for a
/// particular plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotToolbarAction {
    /// Fit the current plot to its data bounds.
    Fit,
    /// Restore the initial camera/view state.
    Reset,
    /// Open the plot-render-mode menu.
    OpenModeMenu,
    /// Toggle the mesh wireframe overlay.
    ToggleWireframe,
    /// Restore automatic color-range selection.
    ResetColorRange,
    /// Open the plot-view menu.
    OpenViewMenu,
    /// Export the current plot.
    Export,
}

type ActionHandler = Rc<dyn Fn(PlotToolbarAction, &mut Window, &mut App) + 'static>;

/// A dense plot toolbar with fit/reset, mode, wireframe, color range, view,
/// and export controls.
pub struct PlotToolbar {
    id: String,
    mode: SharedString,
    view: SharedString,
    wireframe: bool,
    aria_label: SharedString,
    disabled: Vec<PlotToolbarAction>,
    hidden: Vec<PlotToolbarAction>,
    on_action: Option<ActionHandler>,
}

impl PlotToolbar {
    /// Create a toolbar with neutral labels and wireframe disabled.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            mode: "Mesh".into(),
            view: "Planar".into(),
            wireframe: false,
            aria_label: "Plot controls".into(),
            disabled: Vec::new(),
            hidden: Vec::new(),
            on_action: None,
        }
    }

    /// Set the label shown by the mode menu button.
    pub fn mode(mut self, mode: impl Into<SharedString>) -> Self {
        self.mode = mode.into();
        self
    }

    /// Set the label shown by the view menu button.
    pub fn view(mut self, view: impl Into<SharedString>) -> Self {
        self.view = view.into();
        self
    }

    /// Set the current wireframe state.
    pub fn wireframe(mut self, wireframe: bool) -> Self {
        self.wireframe = wireframe;
        self
    }

    /// Set the accessible name of the toolbar region.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Disable or re-enable an individual action.
    pub fn disabled(mut self, action: PlotToolbarAction, disabled: bool) -> Self {
        if disabled {
            if !self.disabled.contains(&action) {
                self.disabled.push(action);
            }
        } else {
            self.disabled.retain(|candidate| *candidate != action);
        }
        self
    }

    /// Hide or show an individual action. Hidden actions are omitted from
    /// layout and keyboard navigation rather than merely disabled.
    pub fn hidden(mut self, action: PlotToolbarAction, hidden: bool) -> Self {
        if hidden {
            if !self.hidden.contains(&action) {
                self.hidden.push(action);
            }
        } else {
            self.hidden.retain(|candidate| *candidate != action);
        }
        self
    }

    /// Register the callback used by all toolbar controls.
    pub fn on_action(
        mut self,
        handler: impl Fn(PlotToolbarAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(handler));
        self
    }

    /// Alias emphasizing that menu buttons and toggles emit selections.
    pub fn on_selection(
        self,
        handler: impl Fn(PlotToolbarAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action(handler)
    }

    /// Build the toolbar from the existing accessible button and toolbar
    /// primitives.
    pub fn build(self) -> Toolbar {
        let handler = self.on_action.clone();
        let disabled = self.disabled.clone();
        let id = self.id.clone();
        let button = |suffix: &str,
                      label: SharedString,
                      accessible_label: SharedString,
                      action: PlotToolbarAction,
                      selected: bool|
         -> Button {
            let mut button = Button::new(format!("{id}-{suffix}"), label)
                .size(ButtonSize::Xs)
                .variant(ButtonVariant::Ghost)
                .selected(selected)
                .disabled(disabled.contains(&action))
                .aria_label(accessible_label);
            if let Some(handler) = handler.clone() {
                button = button.on_click(move |window, cx| handler(action, window, cx));
            }
            button
        };

        let hidden = self.hidden.clone();
        let mut toolbar = Toolbar::new(self.id).aria_label(self.aria_label);
        macro_rules! item {
            ($action:expr, $item:expr) => {
                if !hidden.contains(&$action) {
                    toolbar = toolbar.item($item);
                }
            };
        }
        item!(
            PlotToolbarAction::Fit,
            ToolbarItem::custom(button(
                "fit",
                "Fit".into(),
                "Fit plot to data".into(),
                PlotToolbarAction::Fit,
                false
            ))
        );
        item!(
            PlotToolbarAction::Reset,
            ToolbarItem::custom(button(
                "reset",
                "Reset".into(),
                "Reset plot view".into(),
                PlotToolbarAction::Reset,
                false
            ))
        );
        if !hidden.contains(&PlotToolbarAction::Fit) || !hidden.contains(&PlotToolbarAction::Reset)
        {
            toolbar = toolbar.separator();
        }
        item!(
            PlotToolbarAction::OpenModeMenu,
            ToolbarItem::custom(button(
                "mode",
                self.mode.clone(),
                format!("Plot mode: {}. Open mode menu", self.mode).into(),
                PlotToolbarAction::OpenModeMenu,
                false
            ))
        );
        item!(
            PlotToolbarAction::ToggleWireframe,
            ToolbarItem::custom(button(
                "wireframe",
                "Wireframe".into(),
                if self.wireframe {
                    "Wireframe on. Toggle wireframe off"
                } else {
                    "Wireframe off. Toggle wireframe on"
                }
                .into(),
                PlotToolbarAction::ToggleWireframe,
                self.wireframe
            ))
        );
        item!(
            PlotToolbarAction::ResetColorRange,
            ToolbarItem::custom(button(
                "color-range",
                "Auto range".into(),
                "Reset color range to automatic".into(),
                PlotToolbarAction::ResetColorRange,
                false
            ))
        );
        item!(
            PlotToolbarAction::OpenViewMenu,
            ToolbarItem::custom(button(
                "view",
                self.view.clone(),
                format!("Plot view: {}. Open view menu", self.view).into(),
                PlotToolbarAction::OpenViewMenu,
                false
            ))
        );
        if !hidden.contains(&PlotToolbarAction::Export) {
            toolbar = toolbar.separator().item(ToolbarItem::custom(button(
                "export",
                "Export".into(),
                "Export plot".into(),
                PlotToolbarAction::Export,
                false,
            )));
        }
        toolbar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toolbar_has_neutral_state() {
        let toolbar = PlotToolbar::new("plot-toolbar");
        assert_eq!(toolbar.mode, "Mesh");
        assert_eq!(toolbar.view, "Planar");
        assert!(!toolbar.wireframe);
        assert!(toolbar.disabled.is_empty());
    }

    #[test]
    fn toolbar_state_and_disabled_actions_are_builder_configurable() {
        let toolbar = PlotToolbar::new("plot-toolbar")
            .mode("Scalar fill")
            .view("Surface 3D")
            .wireframe(true)
            .disabled(PlotToolbarAction::Export, true)
            .disabled(PlotToolbarAction::Export, false);

        assert_eq!(toolbar.mode, "Scalar fill");
        assert_eq!(toolbar.view, "Surface 3D");
        assert!(toolbar.wireframe);
        assert!(!toolbar.disabled.contains(&PlotToolbarAction::Export));
    }

    #[test]
    fn individual_actions_can_be_hidden_and_reenabled() {
        let toolbar = PlotToolbar::new("plot-toolbar")
            .hidden(PlotToolbarAction::Export, true)
            .hidden(PlotToolbarAction::Export, false)
            .hidden(PlotToolbarAction::OpenViewMenu, true);
        assert!(!toolbar.hidden.contains(&PlotToolbarAction::Export));
        assert!(toolbar.hidden.contains(&PlotToolbarAction::OpenViewMenu));
    }

    #[test]
    fn selection_callback_is_available_without_rendering() {
        let toolbar = PlotToolbar::new("plot-toolbar").on_selection(|_, _, _| {});
        assert!(toolbar.on_action.is_some());
    }
}
