//! UI Kit Showcase
//!
//! A comprehensive demonstration of all gpui-ui-kit components with theme and i18n support.
//! Use View > Theme menu or Cmd+T to toggle between light/dark themes.
//! Use Language menu to switch between languages.

use gpui::*;
use gpui_ui_kit::accordion::AccordionOrientation;
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::menu::{Menu, MenuItem};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::wizard::StepStatus;
use gpui_ui_kit::workflow::{WorkflowCanvas, WorkflowGraph};
use gpui_ui_kit::*;

/// Section identifiers for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowcaseSection {
    #[default]
    Buttons,
    Text,
    Badges,
    Avatars,
    FormControls,
    Progress,
    Alerts,
    Tabs,
    Cards,
    Breadcrumbs,
    Spinners,
    Layout,
    IconButtons,
    Toasts,
    Dialog,
    Menu,
    Tooltips,
    Potentiometer,
    Accordion,
    Wizard,
    Workflow,
}

impl ShowcaseSection {
    fn all() -> &'static [ShowcaseSection] {
        &[
            ShowcaseSection::Buttons,
            ShowcaseSection::Text,
            ShowcaseSection::Badges,
            ShowcaseSection::Avatars,
            ShowcaseSection::FormControls,
            ShowcaseSection::Progress,
            ShowcaseSection::Alerts,
            ShowcaseSection::Tabs,
            ShowcaseSection::Cards,
            ShowcaseSection::Breadcrumbs,
            ShowcaseSection::Spinners,
            ShowcaseSection::Layout,
            ShowcaseSection::IconButtons,
            ShowcaseSection::Toasts,
            ShowcaseSection::Dialog,
            ShowcaseSection::Menu,
            ShowcaseSection::Tooltips,
            ShowcaseSection::Potentiometer,
            ShowcaseSection::Accordion,
            ShowcaseSection::Wizard,
            ShowcaseSection::Workflow,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            ShowcaseSection::Buttons => "Buttons",
            ShowcaseSection::Text => "Text",
            ShowcaseSection::Badges => "Badges",
            ShowcaseSection::Avatars => "Avatars",
            ShowcaseSection::FormControls => "Form Controls",
            ShowcaseSection::Progress => "Progress",
            ShowcaseSection::Alerts => "Alerts",
            ShowcaseSection::Tabs => "Tabs",
            ShowcaseSection::Cards => "Cards",
            ShowcaseSection::Breadcrumbs => "Breadcrumbs",
            ShowcaseSection::Spinners => "Spinners",
            ShowcaseSection::Layout => "Layout",
            ShowcaseSection::IconButtons => "Icon Buttons",
            ShowcaseSection::Toasts => "Toasts",
            ShowcaseSection::Dialog => "Dialog",
            ShowcaseSection::Menu => "Menu",
            ShowcaseSection::Tooltips => "Tooltips",
            ShowcaseSection::Potentiometer => "Potentiometer",
            ShowcaseSection::Accordion => "Accordion",
            ShowcaseSection::Wizard => "Wizard",
            ShowcaseSection::Workflow => "Workflow",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ShowcaseSection::Buttons => "🔘",
            ShowcaseSection::Text => "📝",
            ShowcaseSection::Badges => "🏷️",
            ShowcaseSection::Avatars => "👤",
            ShowcaseSection::FormControls => "📋",
            ShowcaseSection::Progress => "📊",
            ShowcaseSection::Alerts => "⚠️",
            ShowcaseSection::Tabs => "📑",
            ShowcaseSection::Cards => "🃏",
            ShowcaseSection::Breadcrumbs => "🔗",
            ShowcaseSection::Spinners => "⏳",
            ShowcaseSection::Layout => "📐",
            ShowcaseSection::IconButtons => "🔲",
            ShowcaseSection::Toasts => "🍞",
            ShowcaseSection::Dialog => "💬",
            ShowcaseSection::Menu => "📜",
            ShowcaseSection::Tooltips => "💡",
            ShowcaseSection::Potentiometer => "🎛️",
            ShowcaseSection::Accordion => "🪗",
            ShowcaseSection::Wizard => "🧙",
            ShowcaseSection::Workflow => "🕸️",
        }
    }
}

pub struct Showcase {
    // Toggle states
    toggle_on: bool,
    toggle_lg: bool,
    checkbox_checked: bool,
    // Slider value
    slider_value: f32,
    // Vertical slider value
    vertical_slider_value: f64,
    // Number input values
    number_value: f64,
    number_freq: f64,
    number_db: f64,
    // Number input editing state
    editing_number: Option<&'static str>, // Which input is being edited ("basic", "freq", "db")
    edit_text: String,
    text_selected: bool, // True when text is "selected" - first keystroke replaces all
    // Text Input component states (new editable version)
    input_value: String,
    input_editing: bool,
    input_edit_text: String,
    input_selected: bool,
    // Select states
    select_value: Option<SharedString>,
    select_open: bool,
    select_highlighted: Option<usize>,
    // ButtonSet states
    buttonset_view_mode: SharedString,
    buttonset_alignment: SharedString,
    // Tabs state
    selected_tab: usize,
    // Potentiometer values
    pot_0: f64,
    pot_25: f64,
    pot_50: f64,
    pot_75: f64,
    pot_100: f64,
    pot_selected: f64,
    pot_lg: f64,
    pot_freq_log: f64, // Logarithmic frequency potentiometer (20-20000 Hz)
    // Volume knob values
    volume_value: f32,
    volume_muted: bool,
    // Accordion states
    accordion_vertical_single: Vec<SharedString>,
    accordion_vertical_multiple: Vec<SharedString>,
    accordion_horizontal_single: Vec<SharedString>,
    accordion_side_single: Vec<SharedString>,
    // Wizard state
    wizard_step: usize,
    wizard_statuses: Vec<StepStatus>,
    // Workflow state
    workflow_canvas: Entity<WorkflowCanvas>,
    workflow_node_counter: usize,
    // Pane divider states
    pane_left_collapsed: bool,
    pane_left_width: f32,
    pane_dragging_left: bool,
    pane_drag_start_x: f32,
    pane_drag_start_width: f32,
    // Current section for navigation
    current_section: ShowcaseSection,
    // Entity for updating self
    entity: Entity<Self>,
    // Focus handle for keyboard input
    focus_handle: FocusHandle,
}

impl Showcase {
    fn new(cx: &mut Context<Self>) -> Self {
        // Initialize Workflow canvas
        let graph = WorkflowGraph::new();
        let workflow_canvas = cx.new(|cx| WorkflowCanvas::with_graph(graph, cx));

        Self {
            toggle_on: true,
            toggle_lg: false,
            checkbox_checked: true,
            slider_value: 0.5,
            vertical_slider_value: 0.75,
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
            selected_tab: 0,
            pot_0: 0.0,
            pot_25: 0.25,
            pot_50: 0.5,
            pot_75: 0.75,
            pot_100: 1.0,
            pot_selected: 0.5,
            pot_lg: 0.7,
            pot_freq_log: 1000.0, // Default to 1kHz
            volume_value: 0.75,
            volume_muted: false,
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
            workflow_canvas,
            workflow_node_counter: 0,
            pane_left_collapsed: false,
            pane_left_width: 200.0,
            pane_dragging_left: false,
            pane_drag_start_x: 0.0,
            pane_drag_start_width: 0.0,
            current_section: ShowcaseSection::default(),
            entity: cx.entity().clone(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for Showcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let toggle_on = self.toggle_on;
        let checkbox_checked = self.checkbox_checked;
        let slider_value = self.slider_value;
        let current_section = self.current_section;

        // Get theme colors
        let theme = cx.theme();
        let bg_color = theme.background;
        let text_color = theme.text_secondary;
        let surface_color = theme.surface;
        let border_color = theme.border;
        let accent_color = theme.accent;

        // Get translations
        let title = cx.t(TranslationKey::AppTitle);
        let subtitle = cx.t(TranslationKey::AppSubtitle);

        // Build navigation sidebar
        let mut nav = div()
            .flex()
            .flex_col()
            .w(px(200.0))
            .min_w(px(200.0))
            .h_full()
            .bg(surface_color)
            .border_r_1()
            .border_color(border_color)
            .py_4()
            .overflow_hidden();

        for section in ShowcaseSection::all() {
            let section = *section;
            let is_active = section == current_section;
            let entity_clone = entity.clone();

            let mut item = div()
                .id(SharedString::from(format!("nav-{:?}", section)))
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
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
                let hover_bg = theme.surface_hover;
                item = item.text_color(text_color).hover(move |s| s.bg(hover_bg));
            }

            item = item
                .child(div().child(section.icon()))
                .child(div().child(section.label()))
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    entity_clone.update(cx, |this, cx| {
                        this.current_section = section;
                        cx.notify();
                    });
                });

            nav = nav.child(item);
        }

        // Main content area
        let content = match current_section {
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
                    self.vertical_slider_value,
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
            ShowcaseSection::Tooltips => self.render_tooltip_section(cx).into_any_element(),
            ShowcaseSection::Potentiometer => {
                self.render_potentiometer_section(cx).into_any_element()
            }
            ShowcaseSection::Accordion => self.render_accordion_section(cx).into_any_element(),
            ShowcaseSection::Wizard => self.render_wizard_section(cx).into_any_element(),
            ShowcaseSection::Workflow => self.render_workflow_section(cx).into_any_element(),
        };

        div()
            .id("showcase-root")
            .track_focus(&self.focus_handle)
            .w_full()
            .h_full()
            .bg(bg_color)
            .text_color(text_color)
            .flex()
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(nav)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        // Header (fixed)
                        div()
                            .flex_shrink_0()
                            .p_8()
                            .pb_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(Heading::h1(title))
                            .child(Text::new(subtitle))
                            .child(Divider::new().build()),
                    )
                    .child(
                        // Scrollable content area
                        div()
                            .id("content-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .p_8()
                            .pt_4()
                            .child(content),
                    ),
            )
    }
}

impl Showcase {
    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Handle keys when editing text input
        if self.input_editing {
            match event.keystroke.key.as_str() {
                "enter" => {
                    // Confirm edit
                    self.input_value = self.input_edit_text.clone();
                    self.input_editing = false;
                    self.input_edit_text.clear();
                    self.input_selected = false;
                    cx.notify();
                }
                "escape" => {
                    // Cancel edit
                    self.input_editing = false;
                    self.input_edit_text.clear();
                    self.input_selected = false;
                    cx.notify();
                }
                "backspace" => {
                    // If text is selected, clear all; otherwise delete last character
                    if self.input_selected {
                        self.input_edit_text.clear();
                        self.input_selected = false;
                    } else {
                        self.input_edit_text.pop();
                    }
                    cx.notify();
                }
                key if key.len() == 1 => {
                    // Single character - if text is selected, replace all; otherwise append
                    let ch = key.chars().next().unwrap();
                    if self.input_selected {
                        // Replace all text with the new character
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
                    // Confirm edit - parse and apply the value
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
                    // Cancel edit
                    self.editing_number = None;
                    self.edit_text.clear();
                    self.text_selected = false;
                    cx.notify();
                }
                "backspace" => {
                    // If text is selected, clear all; otherwise delete last character
                    if self.text_selected {
                        self.edit_text.clear();
                        self.text_selected = false;
                    } else {
                        self.edit_text.pop();
                    }
                    cx.notify();
                }
                key if key.len() == 1 => {
                    // Single character - if text is selected, replace all; otherwise append
                    let ch = key.chars().next().unwrap();
                    if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                        if self.text_selected {
                            // Replace all text with the new character
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
    fn section_header(&self, title: impl Into<SharedString>) -> impl IntoElement {
        Heading::h2(title)
    }
}

include!("includes/render_accordion.inc.rs");
include!("includes/render_alert.inc.rs");
include!("includes/render_avatar.inc.rs");
include!("includes/render_badge.inc.rs");
include!("includes/render_breadcrumbs.inc.rs");
include!("includes/render_button.inc.rs");
include!("includes/render_card.inc.rs");
include!("includes/render_dialog.inc.rs");
include!("includes/render_form.inc.rs");
include!("includes/render_icon.inc.rs");
include!("includes/render_layout.inc.rs");
include!("includes/render_menu.inc.rs");
include!("includes/render_potentiometer.inc.rs");
include!("includes/render_progress.inc.rs");
include!("includes/render_spinners.inc.rs");
include!("includes/render_tabs.inc.rs");
include!("includes/render_text.inc.rs");
include!("includes/render_toast.inc.rs");
include!("includes/render_tooltip.inc.rs");
include!("includes/render_wizard.inc.rs");
include!("includes/render_workflow.inc.rs");

fn main() {
    MiniApp::run(
        MiniAppConfig::new("UI Kit Showcase")
            .size(1200.0, 900.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(Showcase::new),
    );
}
