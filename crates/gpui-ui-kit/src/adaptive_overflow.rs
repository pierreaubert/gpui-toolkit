//! Responsive overflow surface for arbitrary interactive content.
//!
//! Desktop windows use a [`Popover`], while mobile platforms and narrow
//! preview windows use a bottom-anchored [`SwipePanel`]. The caller owns the
//! open state; every dismissal path reports the requested state through the
//! callback.

use crate::mobile::is_mobile;
use crate::{Popover, PopoverPlacement, SwipePanel, SwipePanelState};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{AnyElement, App, ClickEvent, ElementId, KeyDownEvent, Window, div, px};
use std::rc::Rc;

/// Surface selected by [`AdaptiveOverflow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveOverflowPresentation {
    /// Anchored desktop popover.
    Popover,
    /// Mobile bottom sheet.
    BottomSheet,
}

type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Controlled responsive overflow for arbitrary trigger and panel content.
pub struct AdaptiveOverflow {
    id: ElementId,
    open: bool,
    trigger: Option<AnyElement>,
    content: Option<AnyElement>,
    on_open_change: Option<OpenChangeHandler>,
}

impl AdaptiveOverflow {
    /// Create an overflow surface with a stable element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            trigger: None,
            content: None,
            on_open_change: None,
        }
    }

    /// Set the caller-controlled open state.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set arbitrary interactive trigger content.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Set arbitrary interactive overflow content.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Report requested controlled-state changes.
    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    fn presentation_for_mobile(mobile: bool) -> AdaptiveOverflowPresentation {
        if mobile {
            AdaptiveOverflowPresentation::BottomSheet
        } else {
            AdaptiveOverflowPresentation::Popover
        }
    }
}

impl RenderOnce for AdaptiveOverflow {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mobile = is_mobile(window, cx);
        let presentation = Self::presentation_for_mobile(mobile);
        let trigger_focus = cx.focus_handle();
        let surface_focus = cx.focus_handle();
        let open = self.open;
        let on_open_change = self.on_open_change.clone();

        let mut trigger = div()
            .id((self.id.clone(), "trigger"))
            .track_focus(&trigger_focus)
            .focusable()
            .cursor_pointer();
        if mobile {
            trigger = trigger.min_w(px(44.0)).min_h(px(44.0));
        }
        if let Some(content) = self.trigger.take() {
            trigger = trigger.child(content);
        }
        if let Some(handler) = on_open_change.clone() {
            let click_handler = handler.clone();
            trigger = trigger
                .on_click(move |_event: &ClickEvent, window, cx| {
                    click_handler(!open, window, cx);
                })
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                        handler(!open, window, cx);
                        cx.stop_propagation();
                    }
                });
        }

        let mut root = div().relative().child(trigger);
        if !open {
            return root;
        }

        let content = self.content.take().map(|content| {
            div()
                .id((self.id.clone(), "scroll"))
                .w_full()
                .max_h(px(600.0))
                .overflow_y_scroll()
                .child(content)
        });

        match presentation {
            AdaptiveOverflowPresentation::Popover => {
                let mut popover = Popover::new(self.id)
                    .placement(PopoverPlacement::BottomEnd)
                    .focus_handle(surface_focus)
                    .restore_focus_to(trigger_focus);
                if let Some(content) = content {
                    popover = popover.content(content);
                }
                if let Some(handler) = on_open_change {
                    popover = popover.on_close(move |window, cx| handler(false, window, cx));
                }
                root = root.child(popover);
            }
            AdaptiveOverflowPresentation::BottomSheet => {
                let mut panel = SwipePanel::new(self.id)
                    .state(SwipePanelState::Expanded)
                    .focus_handle(surface_focus)
                    .restore_focus_to(trigger_focus);
                if let Some(content) = content {
                    panel = panel.content(content);
                }
                if let Some(handler) = on_open_change {
                    panel = panel.on_state_change(move |state, window, cx| {
                        handler(state != SwipePanelState::Collapsed, window, cx);
                    });
                }
                root = root.child(panel);
            }
        }
        root
    }
}

impl IntoElement for AdaptiveOverflow {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{AdaptiveOverflow, AdaptiveOverflowPresentation};

    #[test]
    fn presentation_uses_popover_on_desktop_and_sheet_on_mobile() {
        assert_eq!(
            AdaptiveOverflow::presentation_for_mobile(false),
            AdaptiveOverflowPresentation::Popover
        );
        assert_eq!(
            AdaptiveOverflow::presentation_for_mobile(true),
            AdaptiveOverflowPresentation::BottomSheet
        );
    }

    #[test]
    fn builder_records_controlled_state_callback() {
        let overflow = AdaptiveOverflow::new("more")
            .open(true)
            .on_open_change(|_, _, _| {});
        assert!(overflow.open);
        assert!(overflow.on_open_change.is_some());
    }
}
