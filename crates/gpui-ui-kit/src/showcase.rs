//! UI Kit Showcase (library module)
//!
//! A comprehensive demonstration of all gpui-ui-kit components with theme and i18n support.
//! This module exposes the Showcase component for embedding in other applications.

use crate::i18n::{I18nExt, TranslationKey};
use crate::theme::ThemeExt;
use crate::wizard::StepStatus;
use crate::workflow::{WorkflowCanvas, WorkflowGraph};
use crate::{
    AnimatedQrCode, Divider, Heading, PaginationState, Sidebar, SidebarSide, SortDirection,
    SortState, Text,
};
use gpui::{
    AppContext, Context, Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, WeakEntity, Window, div, px, rgba,
};
use std::collections::HashSet;

mod showcase_group;
mod showcase_section;
mod types;

pub mod sections;

pub use showcase_group::ShowcaseGroup;
pub use showcase_section::ShowcaseSection;
pub use types::User;

pub struct Showcase {
    // Toggle states
    pub toggle_on: bool,
    pub toggle_lg: bool,
    pub checkbox_checked: bool,
    // Slider value
    pub slider_value: f32,
    // Vertical slider value
    // Number input values
    pub number_value: f64,
    pub number_freq: f64,
    pub number_db: f64,
    // Number input editing state
    pub editing_number: Option<&'static str>,
    pub edit_text: String,
    pub text_selected: bool,
    // Text Input component states
    pub input_value: String,
    pub input_editing: bool,
    pub input_edit_text: String,
    pub input_selected: bool,
    // Select states
    pub select_value: Option<SharedString>,
    pub select_open: bool,
    pub select_highlighted: Option<usize>,
    // ButtonSet states
    pub buttonset_view_mode: SharedString,
    pub buttonset_alignment: SharedString,
    pub buttonset_toggle_lg: SharedString,
    pub buttonset_disabled_demo: SharedString,
    // Tabs state
    pub selected_tab: usize,
    // Accordion states
    pub accordion_vertical_single: Vec<SharedString>,
    pub accordion_vertical_multiple: Vec<SharedString>,
    pub accordion_horizontal_single: Vec<SharedString>,
    pub accordion_side_single: Vec<SharedString>,
    // Wizard state
    pub wizard_step: usize,
    pub wizard_statuses: Vec<StepStatus>,
    // Persistent workflow canvas entity; graph state lives inside it.
    pub workflow_canvas: Entity<WorkflowCanvas>,
    pub workflow_node_counter: usize,
    // Table states
    pub users: Vec<User>,
    pub selected_users: HashSet<usize>,
    pub sort_state: Option<SortState>,
    pub pagination: PaginationState,
    // Pane divider states
    pub pane_left_collapsed: bool,
    pub pane_left_width: f32,
    pub pane_dragging_left: bool,
    pub pane_drag_start_x: f32,
    pub pane_drag_start_width: f32,
    // Search bar state
    pub search_bar_value: SharedString,
    // Drag list state
    pub drag_vertical_items: Vec<SharedString>,
    pub drag_horizontal_items: Vec<SharedString>,
    // Settings/accessibility demo state
    pub settings_mute: bool,
    pub accessibility_terms: bool,
    pub accessibility_dark: bool,
    pub accessibility_volume: f32,
    // Tooltip hover state
    pub tooltip_hovered: Option<&'static str>,
    // Popover open state
    pub popover_open: Option<&'static str>,
    // Animated QR codes
    pub animated_qr_tiny: Entity<AnimatedQrCode>,
    pub animated_qr_small: Entity<AnimatedQrCode>,
    // Current section for navigation
    pub current_section: ShowcaseSection,
    // Render only the selected section when embedded in another tool.
    pub embedded: bool,
    // Entity for updating self
    pub entity: Entity<Self>,
    // Focus handle for keyboard input
    pub focus_handle: FocusHandle,
    // Persistent child render entities to avoid rebuilding stable UI every frame.
    sidebar_entity: Entity<ShowcaseSidebar>,
    header_entity: Entity<ShowcaseHeader>,
    content_entity: Entity<ShowcaseContent>,
}

impl Showcase {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let workflow_canvas = cx.new(|cx| WorkflowCanvas::with_graph(WorkflowGraph::new(), cx));
        let animated_qr_tiny =
            cx.new(|cx| AnimatedQrCode::new("https://example.com/animated-qr-demo", px(50.0), cx));
        let animated_qr_small =
            cx.new(|cx| AnimatedQrCode::new("https://example.com/animated-qr-demo", px(80.0), cx));
        let entity = cx.entity().clone();
        let parent = entity.downgrade();

        let sidebar_entity = cx.new(|_cx| ShowcaseSidebar::new(parent.clone()));
        let header_entity = cx.new(|_cx| ShowcaseHeader::new());
        let content_entity = cx.new(|_cx| ShowcaseContent::new(parent.clone()));

        Self {
            toggle_on: true,
            toggle_lg: false,
            checkbox_checked: true,
            slider_value: 0.5,
            number_value: 42.0,
            number_freq: 1000.0,
            number_db: -3.0,
            editing_number: None,
            edit_text: String::new(),
            text_selected: false,
            input_value: String::from("Hello World!"),
            input_editing: false,
            input_edit_text: String::new(),
            input_selected: false,
            select_value: Some("apple".into()),
            select_open: false,
            select_highlighted: None,
            buttonset_view_mode: "grid".into(),
            buttonset_alignment: "center".into(),
            buttonset_toggle_lg: "on".into(),
            buttonset_disabled_demo: "a".into(),
            selected_tab: 0,
            accordion_vertical_single: vec!["v-single-1".into()],
            accordion_vertical_multiple: vec!["v-multi-1".into(), "v-multi-2".into()],
            accordion_horizontal_single: vec!["h-single-1".into()],
            accordion_side_single: vec!["side-single-1".into(), "side-single-2".into()],
            wizard_step: 0,
            wizard_statuses: vec![
                StepStatus::Active,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
            ],
            users: vec![
                User {
                    id: 1,
                    name: "Alice Smith".into(),
                    email: "alice@example.com".into(),
                    role: "Admin".into(),
                },
                User {
                    id: 2,
                    name: "Bob Jones".into(),
                    email: "bob@example.com".into(),
                    role: "User".into(),
                },
                User {
                    id: 3,
                    name: "Charlie Brown".into(),
                    email: "charlie@example.com".into(),
                    role: "Editor".into(),
                },
                User {
                    id: 4,
                    name: "David Wilson".into(),
                    email: "david@example.com".into(),
                    role: "User".into(),
                },
                User {
                    id: 5,
                    name: "Eve Adams".into(),
                    email: "eve@example.com".into(),
                    role: "Admin".into(),
                },
            ],
            selected_users: HashSet::new(),
            sort_state: Some(SortState {
                column_id: "name".into(),
                direction: SortDirection::Ascending,
            }),
            pagination: PaginationState {
                current_page: 0,
                page_size: 5,
                total_items: 5,
            },
            workflow_canvas,
            workflow_node_counter: 0,
            pane_left_collapsed: false,
            pane_left_width: 200.0,
            pane_dragging_left: false,
            pane_drag_start_x: 0.0,
            pane_drag_start_width: 0.0,
            search_bar_value: "Beethoven".into(),
            drag_vertical_items: vec![
                "eq".into(),
                "comp".into(),
                "limiter".into(),
                "upmixer".into(),
            ],
            drag_horizontal_items: vec!["Track 1".into(), "Track 2".into(), "Track 3".into()],
            settings_mute: false,
            accessibility_terms: true,
            accessibility_dark: true,
            accessibility_volume: 75.0,
            tooltip_hovered: None,
            popover_open: None,
            animated_qr_tiny,
            animated_qr_small,
            current_section: ShowcaseSection::default(),
            embedded: false,
            entity,
            focus_handle: cx.focus_handle(),
            sidebar_entity,
            header_entity,
            content_entity,
        }
    }

    pub fn embedded_section(section: ShowcaseSection, cx: &mut Context<Self>) -> Self {
        let mut showcase = Self::new(cx);
        showcase.current_section = section;
        showcase.embedded = true;
        showcase
    }
}

impl Render for Showcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_section = self.current_section;
        let embedded = self.embedded;

        // Sync child entity state only when it changes so stable subtrees are
        // not marked dirty on every frame.
        {
            let sidebar = self.sidebar_entity.read(cx);
            if sidebar.current_section != current_section || sidebar.embedded != embedded {
                self.sidebar_entity.update(cx, |sidebar, _cx| {
                    sidebar.current_section = current_section;
                    sidebar.embedded = embedded;
                });
            }
        }

        let title: SharedString = cx.t(TranslationKey::AppTitle).into();
        let subtitle: SharedString = cx.t(TranslationKey::AppSubtitle).into();
        {
            let header = self.header_entity.read(cx);
            if header.title != title || header.subtitle != subtitle {
                self.header_entity.update(cx, |header, _cx| {
                    header.title = title;
                    header.subtitle = subtitle;
                });
            }
        }

        {
            let content = self.content_entity.read(cx);
            if content.current_section != current_section || content.embedded != embedded {
                self.content_entity.update(cx, |content, _cx| {
                    content.current_section = current_section;
                    content.embedded = embedded;
                });
            }
        }

        // Get theme colors. Scope the borrow so the mutable `cx` calls below
        // do not conflict with the shared theme reference.
        let (bg_color, text_color) = {
            let theme = cx.theme();
            (theme.background, theme.text_secondary)
        };

        if embedded {
            return div()
                .id("showcase-embedded-root")
                .size_full()
                .bg(bg_color)
                .text_color(text_color)
                .overflow_y_scroll()
                .p_3()
                .child(self.content_entity.clone());
        }

        div()
            .id("showcase-root")
            .track_focus(&self.focus_handle)
            .w_full()
            .h_full()
            .bg(bg_color)
            .text_color(text_color)
            .flex()
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(self.sidebar_entity.clone())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(self.header_entity.clone())
                    .child(self.content_entity.clone()),
            )
    }
}

impl Showcase {
    /// Build the element for the currently selected section.
    fn render_section_content(
        &mut self,
        section: ShowcaseSection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entity = self.entity.clone();
        let toggle_on = self.toggle_on;
        let checkbox_checked = self.checkbox_checked;
        let slider_value = self.slider_value;

        match section {
            ShowcaseSection::Buttons => self.render_buttons_section(cx).into_any_element(),
            ShowcaseSection::Text => self.render_text_section(cx).into_any_element(),
            ShowcaseSection::Badges => self.render_badges_section(cx).into_any_element(),
            ShowcaseSection::Avatars => self.render_avatars_section(cx).into_any_element(),
            ShowcaseSection::FormControls => self
                .render_form_controls_section(
                    toggle_on,
                    self.toggle_lg,
                    checkbox_checked,
                    slider_value,
                    self.number_value,
                    self.number_freq,
                    self.number_db,
                    self.editing_number,
                    self.edit_text.clone(),
                    self.text_selected,
                    self.input_value.clone(),
                    self.input_editing,
                    self.input_edit_text.clone(),
                    self.input_selected,
                    self.buttonset_view_mode.clone(),
                    self.buttonset_alignment.clone(),
                    entity.clone(),
                    cx,
                )
                .into_any_element(),
            ShowcaseSection::Progress => self.render_progress_section(cx).into_any_element(),
            ShowcaseSection::Alerts => self.render_alerts_section(cx).into_any_element(),
            ShowcaseSection::Tabs => self.render_tabs_section(cx).into_any_element(),
            ShowcaseSection::Cards => self.render_card_section(cx).into_any_element(),
            ShowcaseSection::Breadcrumbs => self.render_breadcrumbs_section(cx).into_any_element(),
            ShowcaseSection::Spinners => self.render_spinners_section(cx).into_any_element(),
            ShowcaseSection::Layout => self.render_layout_section(cx).into_any_element(),
            ShowcaseSection::IconButtons => self.render_icon_buttons_section(cx).into_any_element(),
            ShowcaseSection::Toasts => self.render_toasts_section(cx).into_any_element(),
            ShowcaseSection::Dialog => self.render_dialog_section(cx).into_any_element(),
            ShowcaseSection::Menu => self.render_menu_section(cx).into_any_element(),
            ShowcaseSection::Table => self.render_table_section(cx).into_any_element(),
            ShowcaseSection::Tooltips => self.render_tooltip_section(cx).into_any_element(),
            ShowcaseSection::Accordion => self.render_accordion_section(cx).into_any_element(),
            ShowcaseSection::Wizard => self.render_wizard_section(cx).into_any_element(),
            ShowcaseSection::Workflow => self.render_workflow_section(cx).into_any_element(),
            ShowcaseSection::QrCode => self.render_qr_section(cx).into_any_element(),
            ShowcaseSection::ContextMenu => self.render_context_menu_section(cx).into_any_element(),
            ShowcaseSection::Popover => self.render_popover_section(cx).into_any_element(),
            ShowcaseSection::Sidebar => self.render_sidebar_section(cx).into_any_element(),
            ShowcaseSection::StatusBar => self.render_status_bar_section(cx).into_any_element(),
            ShowcaseSection::SearchBar => self.render_search_bar_section(cx).into_any_element(),
            ShowcaseSection::KeyboardShortcut => {
                self.render_keyboard_shortcut_section(cx).into_any_element()
            }
            ShowcaseSection::EmptyState => self.render_empty_state_section(cx).into_any_element(),
            ShowcaseSection::ConfirmDialog => {
                self.render_confirm_dialog_section(cx).into_any_element()
            }
            ShowcaseSection::SplitPane => self.render_split_pane_section(cx).into_any_element(),
            ShowcaseSection::ImageView => self.render_image_view_section(cx).into_any_element(),
            ShowcaseSection::SettingsForm => {
                self.render_settings_form_section(cx).into_any_element()
            }
            ShowcaseSection::StepIndicator => {
                self.render_step_indicator_section(cx).into_any_element()
            }
            ShowcaseSection::LoadingOverlay => {
                self.render_loading_overlay_section(cx).into_any_element()
            }
            ShowcaseSection::Tag => self.render_tag_section(cx).into_any_element(),
            ShowcaseSection::Toolbar => self.render_toolbar_section(cx).into_any_element(),
            ShowcaseSection::Notification => {
                self.render_notification_section(cx).into_any_element()
            }
            ShowcaseSection::TreeView => self.render_tree_view_section(cx).into_any_element(),
            ShowcaseSection::DragList => self.render_drag_list_section(cx).into_any_element(),
            ShowcaseSection::CommandPalette => {
                self.render_command_palette_section(cx).into_any_element()
            }
            ShowcaseSection::Accessibility => {
                self.render_accessibility_section(cx).into_any_element()
            }
        }
    }
}

impl Showcase {
    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Handle keys when editing text input
        if self.input_editing {
            match event.keystroke.key.as_str() {
                "enter" => {
                    self.input_value = self.input_edit_text.clone();
                    self.input_editing = false;
                    self.input_edit_text.clear();
                    self.input_selected = false;
                    cx.notify();
                }
                "escape" => {
                    self.input_editing = false;
                    self.input_edit_text.clear();
                    self.input_selected = false;
                    cx.notify();
                }
                "backspace" => {
                    if self.input_selected {
                        self.input_edit_text.clear();
                        self.input_selected = false;
                    } else {
                        self.input_edit_text.pop();
                    }
                    cx.notify();
                }
                key if key.len() == 1 => {
                    let ch = key.chars().next().unwrap();
                    if self.input_selected {
                        self.input_edit_text.clear();
                        self.input_selected = false;
                    }
                    self.input_edit_text.push(ch);
                    cx.notify();
                }
                _ => {}
            }
        }
        // Handle keys when editing a number input
        else if let Some(editing_id) = self.editing_number {
            match event.keystroke.key.as_str() {
                "enter" => {
                    if let Ok(value) = self.edit_text.parse::<f64>() {
                        match editing_id {
                            "basic" => self.number_value = value.clamp(0.0, 100.0),
                            "freq" => self.number_freq = value.clamp(20.0, 20000.0),
                            "db" => self.number_db = value.clamp(-12.0, 12.0),
                            _ => {}
                        }
                    }
                    self.editing_number = None;
                    self.edit_text.clear();
                    self.text_selected = false;
                    cx.notify();
                }
                "escape" => {
                    self.editing_number = None;
                    self.edit_text.clear();
                    self.text_selected = false;
                    cx.notify();
                }
                "backspace" => {
                    if self.text_selected {
                        self.edit_text.clear();
                        self.text_selected = false;
                    } else {
                        self.edit_text.pop();
                    }
                    cx.notify();
                }
                key if key.len() == 1 => {
                    let ch = key.chars().next().unwrap();
                    if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                        if self.text_selected {
                            self.edit_text.clear();
                            self.text_selected = false;
                        }
                        self.edit_text.push(ch);
                        cx.notify();
                    }
                }
                _ => {}
            }
        }
    }
}

impl Showcase {
    pub fn section_header(&self, title: impl Into<SharedString>) -> impl IntoElement {
        Heading::h2(title)
    }
}

// ---------------------------------------------------------------------------
// Persistent child render entities
// ---------------------------------------------------------------------------

/// Persistent navigation sidebar for the showcase.
///
/// Only re-renders when the selected section or embedded mode changes.
struct ShowcaseSidebar {
    current_section: ShowcaseSection,
    embedded: bool,
    parent: WeakEntity<Showcase>,
}

impl ShowcaseSidebar {
    fn new(parent: WeakEntity<Showcase>) -> Self {
        Self {
            current_section: ShowcaseSection::default(),
            embedded: false,
            parent,
        }
    }
}

impl Render for ShowcaseSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let current_section = self.current_section;
        let text_color = theme.text_secondary;
        let accent_color = theme.accent;
        let text_muted_color = theme.text_muted;
        let surface_hover_color = theme.surface_hover;
        let border_color = theme.border;

        let mut nav_items = div().flex().flex_col().py_4().gap_1();

        for group in ShowcaseGroup::all() {
            nav_items = nav_items.child(
                div()
                    .px_4()
                    .pt_3()
                    .pb_1()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(text_muted_color)
                    .child(group.label().to_uppercase()),
            );

            for section in group.sections() {
                let section = *section;
                let is_active = section == current_section;
                let parent = self.parent.clone();

                let mut item = div()
                    .id(SharedString::from(format!("nav-{:?}", section)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py(px(5.0))
                    .mx_2()
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm();

                if is_active {
                    item = item
                        .bg(accent_color)
                        .text_color(rgba(0xffffffff))
                        .font_weight(FontWeight::SEMIBOLD);
                } else {
                    item = item
                        .text_color(text_color)
                        .hover(move |s| s.bg(surface_hover_color));
                }

                item = item.child(div().child(section.label())).on_mouse_down(
                    MouseButton::Left,
                    move |_event, _window, cx| {
                        if let Some(parent) = parent.upgrade() {
                            parent.update(cx, |this, cx| {
                                this.current_section = section;
                                cx.notify();
                            });
                        }
                    },
                );

                nav_items = nav_items.child(item);
            }

            nav_items = nav_items.child(div().mx_4().my_1().h(px(1.0)).bg(border_color));
        }

        Sidebar::new("showcase-nav")
            .side(SidebarSide::Left)
            .width(px(220.0))
            .content(nav_items)
    }
}

/// Persistent header for the showcase.
///
/// Only re-renders when the translated title/subtitle changes (essentially
/// never after the first frame).
struct ShowcaseHeader {
    title: SharedString,
    subtitle: SharedString,
}

impl ShowcaseHeader {
    fn new() -> Self {
        Self {
            title: SharedString::default(),
            subtitle: SharedString::default(),
        }
    }
}

impl Render for ShowcaseHeader {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_shrink_0()
            .p_8()
            .pb_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(Heading::h1(self.title.clone()))
            .child(Text::new(self.subtitle.clone()))
            .child(Divider::new().build())
    }
}

/// Persistent main content area for the showcase.
///
/// The section element itself is still built by the parent because the section
/// renderers need access to the full Showcase state and a `Context<Showcase>`.
/// Keeping this area as a stable child entity means the scrollable container
/// and group info are not rebuilt every frame; only the inner section element
/// is reconstructed when the active section changes.
struct ShowcaseContent {
    current_section: ShowcaseSection,
    embedded: bool,
    parent: WeakEntity<Showcase>,
}

impl ShowcaseContent {
    fn new(parent: WeakEntity<Showcase>) -> Self {
        Self {
            current_section: ShowcaseSection::default(),
            embedded: false,
            parent,
        }
    }
}

impl Render for ShowcaseContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let section = self.current_section;
        let embedded = self.embedded;

        match self.parent.update(cx, |parent, cx| {
            let content = parent
                .render_section_content(section, cx)
                .into_any_element();

            if embedded {
                return div()
                    .id("showcase-embedded-root")
                    .size_full()
                    .child(content);
            }

            let current_group = section.group();
            let theme = cx.theme();

            let group_info = div()
                .mb_4()
                .p_4()
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded(px(6.0))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text_muted)
                        .child(current_group.label()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text_muted)
                        .child(current_group.description()),
                );

            div()
                .id("content-scroll")
                .flex_1()
                .overflow_y_scroll()
                .p_8()
                .pt_4()
                .child(group_info)
                .child(content)
        }) {
            Ok(element) => element.into_any_element(),
            Err(_) => div().into_any_element(),
        }
    }
}
