//! Tabs component for tabbed navigation
//!
//! Provides a horizontal tab bar with content panels and theming support.

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, AppContext, Context, ElementId, Entity, FocusHandle, KeyDownEvent, MouseDownEvent, Render,
    SharedString, WeakEntity, Window, div,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod tab_item;
mod tab_render;
mod types;

pub use tab_item::TabItem;
use tab_render::{
    TabColors, TabRenderState, render_card_tab, render_regular_tab, render_underline_tab,
    style_tab_container,
};
pub use types::{IconFactory, TabVariant, TabsTheme};

thread_local! {
    /// Cached render entities so repeated renders reuse the same GPUI entity.
    // Stored as weak references so GPUI can drop the entity when the view is
    // destroyed; otherwise tests report leaked entity handles.
    static TABS_ENTITIES: RefCell<HashMap<ElementId, WeakEntity<TabsEntity>>> =
        RefCell::new(HashMap::new());
}

const MAX_TABS_ENTITIES: usize = 1024;

/// A tabs component with theming support
pub struct Tabs {
    id: ElementId,
    tabs: Vec<TabItem>,
    selected_index: usize,
    variant: TabVariant,
    theme: Option<TabsTheme>,
    on_change: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_close: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Tabs {
    /// Create a new tabs component with an ID
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            selected_index: 0,
            variant: TabVariant::default(),
            theme: None,
            on_change: None,
            on_close: None,
            focus_handle: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the focus handle for keyboard navigation
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the tab items
    pub fn tabs(mut self, tabs: Vec<TabItem>) -> Self {
        self.tabs = tabs;
        self
    }

    /// Set the selected tab index
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Set the visual variant
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the theme
    pub fn theme(mut self, theme: TabsTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set the tab change handler
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Tablist)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Set the tab close handler
    pub fn on_close(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new("tabs")
    }
}

/// Internal entity that renders a [`Tabs`] component. The entity is cached by
/// element id so that stable event handlers and hover state persist across
/// renders.
pub struct TabsEntity {
    props: Tabs,
    focus_handle: FocusHandle,
    tab_count: usize,
    hovered_tab: Option<usize>,
    hovered_close: Option<usize>,
    /// Stable tab IDs copied from the last props update so event handlers can
    /// resolve the close target without cloning IDs on every render.
    tab_ids: Vec<SharedString>,
}

impl TabsEntity {
    fn handle_tab_click(
        &mut self,
        index: usize,
        _event: &MouseDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if let Some(ref handler) = self.props.on_change {
            handler(index, window, _cx);
        }
    }

    fn handle_close_click(
        &mut self,
        index: usize,
        _event: &MouseDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if let Some(tab_id) = self.tab_ids.get(index)
            && let Some(ref handler) = self.props.on_close
        {
            handler(tab_id, window, _cx);
        }
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.focus_handle.is_focused(window) {
            return;
        }

        let key = event.keystroke.key.as_str();
        let selected = self.props.selected_index;
        let tab_count = self.tab_count;

        let new_index = match key {
            "left" => {
                if selected > 0 {
                    Some(selected - 1)
                } else {
                    None
                }
            }
            "right" => {
                if selected + 1 < tab_count {
                    Some(selected + 1)
                } else {
                    None
                }
            }
            "home" => Some(0),
            "end" => {
                if tab_count > 0 {
                    Some(tab_count - 1)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new_idx) = new_index {
            cx.stop_propagation();
            if let Some(ref handler) = self.props.on_change {
                handler(new_idx, window, cx);
            }
        }
    }

    fn set_hovered(&mut self, index: usize, hovered: bool, cx: &mut Context<Self>) {
        let new = if hovered { Some(index) } else { None };
        if self.hovered_tab != new {
            // Only clear if the currently hovered tab matches this one, to
            // avoid race conditions when moving directly between tabs.
            if !hovered && self.hovered_tab != Some(index) {
                return;
            }
            self.hovered_tab = new;
            cx.notify();
        }
    }

    fn set_close_hovered(&mut self, index: usize, hovered: bool, cx: &mut Context<Self>) {
        let new = if hovered { Some(index) } else { None };
        if self.hovered_close != new {
            if !hovered && self.hovered_close != Some(index) {
                return;
            }
            self.hovered_close = new;
            cx.notify();
        }
    }
}

impl Render for TabsEntity {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let id = self.props.id.clone();
        let aria_label = self.props.aria_label.clone();
        let aria_role = self.props.aria_role;
        cx.register_accessible(AccessibilityNode {
            element_id: id.clone(),
            label: aria_label.unwrap_or_default(),
            props: AriaProps::with_role(aria_role.unwrap_or(AriaRole::Tablist)),
        });

        let global_theme = cx.theme();
        let tabs_theme = TabsTheme::from(global_theme.as_ref());
        let theme = self.props.theme.clone().unwrap_or(tabs_theme);

        // Capture theme colors as local Copy values.
        let colors = TabColors::from_theme(&theme);

        let mut container = div()
            .id(id.clone())
            .font_family(global_theme.font_family.clone())
            .track_focus_element(&self.focus_handle)
            .flex()
            .items_center()
            .focusable();

        // Apply variant-specific container styling
        container = style_tab_container(container, self.props.variant, &colors);

        // Consume the tab list for this render. The props are refreshed before
        // each render by the RenderOnce impl, so the vector will be repopulated.
        let tabs = std::mem::take(&mut self.props.tabs);
        let variant = self.props.variant;

        for (index, tab) in tabs.into_iter().enumerate() {
            let state = TabRenderState {
                index,
                is_selected: index == self.props.selected_index,
                hovered: self.hovered_tab == Some(index),
                close_hovered: self.hovered_close == Some(index),
            };
            let tab_element = match variant {
                TabVariant::Underline => render_underline_tab(tab, &state, &colors, cx),
                TabVariant::VerticalCard => render_card_tab(tab, &state, &colors, cx),
                TabVariant::Enclosed | TabVariant::Pills => {
                    render_regular_tab(tab, variant, &state, &colors, cx)
                }
            };

            container = container.child(tab_element);
        }

        // Add keyboard navigation
        container = container.on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            this.handle_key_down(event, window, cx);
        }));

        container
    }
}

impl RenderOnce for Tabs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let focus_handle = self
            .focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle());
        let tab_count = self.tabs.len();

        let entity: Entity<TabsEntity> = TABS_ENTITIES.with(|map| {
            let mut map = map.borrow_mut();
            map.retain(|_, weak| weak.upgrade().is_some());
            if !map.contains_key(&id) && map.len() >= MAX_TABS_ENTITIES {
                map.clear();
            }
            if let Some(weak) = map.get(&id)
                && let Some(entity) = weak.upgrade()
            {
                return entity;
            }
            let entity = cx.new(|_cx| TabsEntity {
                props: Tabs::new(id.clone()),
                focus_handle: focus_handle.clone(),
                tab_count,
                hovered_tab: None,
                hovered_close: None,
                tab_ids: Vec::new(),
            });
            map.insert(id.clone(), entity.downgrade());
            entity
        });
        entity.update(cx, |model, _cx| {
            model.tab_count = tab_count;
            model.tab_ids = self.tabs.iter().map(|tab| tab.id.clone()).collect();
            model.props = self;
        });
        entity
    }
}

impl IntoElement for Tabs {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
