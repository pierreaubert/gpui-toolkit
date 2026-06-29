//! Coverage-focused integration tests for gpui-ui-kit public APIs.
//!
//! These tests exercise builder setters, pure helpers, state machines, and math
//! helpers without requiring a GPUI application context.

use gpui::{IntoElement, SharedString, div, px, rgb, rgba};
use gpui_ui_kit::*;

// ===========================================================================
// Animation
// ===========================================================================

#[test]
fn all_easing_variants_at_boundaries() {
    let variants = [
        Easing::Linear,
        Easing::EaseInQuad,
        Easing::EaseOutQuad,
        Easing::EaseInOutQuad,
        Easing::EaseInCubic,
        Easing::EaseOutCubic,
        Easing::EaseInOutCubic,
        Easing::EaseInQuart,
        Easing::EaseOutQuart,
        Easing::EaseInOutQuart,
        Easing::EaseInQuint,
        Easing::EaseOutQuint,
        Easing::EaseInOutQuint,
        Easing::EaseInSine,
        Easing::EaseOutSine,
        Easing::EaseInOutSine,
        Easing::EaseInExpo,
        Easing::EaseOutExpo,
        Easing::EaseInOutExpo,
        Easing::EaseInCirc,
        Easing::EaseOutCirc,
        Easing::EaseInOutCirc,
        Easing::EaseInBack,
        Easing::EaseOutBack,
        Easing::EaseInOutBack,
        Easing::EaseInElastic,
        Easing::EaseOutElastic,
        Easing::EaseInOutElastic,
        Easing::EaseInBounce,
        Easing::EaseOutBounce,
        Easing::EaseInOutBounce,
    ];

    for easing in variants {
        let at_start = ease(easing, 0.0);
        let at_end = ease(easing, 1.0);
        assert!(
            at_start.abs() < 0.001 || (at_start - 1.0).abs() < 0.001,
            "{:?} should start at 0 or 1, got {}",
            easing,
            at_start
        );
        assert!(
            (at_end - 1.0).abs() < 0.001 || at_end.abs() < 0.001,
            "{:?} should end at 0 or 1, got {}",
            easing,
            at_end
        );
        // Smoke-test an interior point; should not panic or return NaN.
        let mid = ease(easing, 0.5);
        assert!(
            mid.is_finite(),
            "{:?} at 0.5 should be finite, got {}",
            easing,
            mid
        );
    }
}

#[test]
fn interpolate_and_color_helpers() {
    assert_eq!(interpolate(0.0, 10.0, Easing::Linear, 0.0), 0.0);
    assert_eq!(interpolate(0.0, 10.0, Easing::Linear, 0.5), 5.0);
    assert_eq!(interpolate(0.0, 10.0, Easing::Linear, 1.0), 10.0);

    let from = rgba(0x000000ff);
    let to = rgba(0xffffffff);
    let mid = interpolate_color(from, to, Easing::Linear, 0.5);
    assert!((mid.r - 0.5).abs() < 0.01);
    assert!((mid.a - 1.0).abs() < 0.01);
}

#[test]
fn animation_builder_presets() {
    use std::time::Duration;

    let _ = Animation::new()
        .duration_ms(300)
        .easing(Easing::EaseOutCubic)
        .delay_ms(50)
        .repeat(2)
        .alternate(true);

    let quick = Animation::quick();
    assert_eq!(quick.duration, Duration::from_millis(150));
    assert_eq!(quick.easing, Easing::EaseOutQuad);

    let standard = Animation::standard();
    assert_eq!(standard.duration, Duration::from_millis(250));

    let slow = Animation::slow();
    assert_eq!(slow.duration, Duration::from_millis(400));

    let emphasis = Animation::emphasis();
    assert_eq!(emphasis.duration, Duration::from_millis(300));
    assert_eq!(emphasis.easing, Easing::EaseOutBack);

    let bouncy = Animation::bouncy();
    assert_eq!(bouncy.duration, Duration::from_millis(500));

    let anim = Animation::new().duration_ms(100).delay_ms(50);
    assert_eq!(anim.total_duration(), Duration::from_millis(150));
    assert!(!anim.is_complete(Duration::from_millis(100)));
    assert!(anim.is_complete(Duration::from_millis(200)));
    assert_eq!(anim.progress(Duration::from_millis(50)), 0.0);
    assert_eq!(anim.progress(Duration::from_millis(150)), 1.0);
}

#[test]
fn spring_constructors_and_stepping() {
    let _ = Spring::default();
    let _ = Spring::gentle();
    let _ = Spring::wobbly();
    let _ = Spring::stiff();
    let _ = Spring::slow();
    let _ = Spring::new(100.0, 10.0, 1.0);

    let spring = Spring::default();
    let (pos, vel) = spring.step(0.0, 10.0, 0.0, 1.0);
    assert!(pos.is_finite() && vel.is_finite());
    assert!(spring.is_settled(pos, pos, 0.0, 1.0));
    assert!(!spring.is_settled(0.0, 10.0, 0.0, 0.1));
}

// ===========================================================================
// Color and color tokens
// ===========================================================================

#[test]
fn color_all_constructors_and_conversions() {
    let c1 = Color::new(255, 128, 64, 255);
    let c2 = Color::rgb(255, 128, 64);
    assert_eq!(c1, c2);

    let hex = Color::from_hex(0xff8040);
    assert_eq!(hex.r, 255);
    assert_eq!(hex.g, 128);
    assert_eq!(hex.b, 64);

    let hex_alpha = Color::from_hex_alpha(0xff804080);
    assert_eq!(hex_alpha.a, 128);

    assert_eq!(c2.to_hex_string(), "#ff8040");
    assert!(hex_alpha.to_hex_string().len() > 7);

    assert_eq!(
        Color::from_hex_string("#f80").unwrap(),
        Color::rgb(255, 136, 0)
    );
    assert_eq!(
        Color::from_hex_string("#ff8040").unwrap(),
        Color::rgb(255, 128, 64)
    );
    assert!(Color::from_hex_string("not-a-color").is_none());

    let rgba = c2.to_rgba();
    let back = Color::from_rgba(rgba);
    assert_eq!(back.r, c2.r);

    let with_alpha = c2.with_alpha(0.5);
    assert_eq!(with_alpha.a, 128);

    let (h, s, l) = c2.to_hsl();
    let rebuilt = Color::from_hsl(h, s, l);
    assert!((c2.r as i16 - rebuilt.r as i16).abs() <= 1);
}

#[test]
fn color_token_variants_and_helpers() {
    let token = ColorToken::from_hex(0x007acc);
    assert_ne!(token.base, token.hover);
    assert_ne!(token.base, token.active);
    assert!(token.muted.a < 0.3);

    let from_rgba = ColorToken::from(rgba(0x007accff));
    assert_eq!(from_rgba.base, token.base);

    let from_u32 = ColorToken::from(0x007accu32);
    assert_eq!(from_u32.base, token.base);

    let alpha_token = ColorToken::from_base_with_alpha(rgb(0x007acc), 0.5);
    assert!((alpha_token.base.a - 0.5).abs() < 0.01);

    let lighter = token.lighter(0.1);
    let darker = token.darker(0.1);
    assert_ne!(lighter.base, darker.base);

    let alpha2 = token.with_alpha(0.25);
    assert!((alpha2.base.a - 0.25).abs() < 0.01);
}

#[test]
fn color_token_palette_structs() {
    let _ = SemanticColors::default();
    let _ = SemanticColors::dark();
    let _ = SemanticColors::light();

    let _ = TextColors::default();
    let _ = TextColors::dark();
    let _ = TextColors::light();

    let _ = BackgroundColors::default();
    let _ = BackgroundColors::dark();
    let _ = BackgroundColors::light();

    let _ = BorderColors::default();
    let _ = BorderColors::dark();
    let _ = BorderColors::light();

    let dark = ColorPalette::dark();
    let light = ColorPalette::light();
    assert_ne!(dark.backgrounds.page.base, light.backgrounds.page.base);

    let base = rgb(0x808080);
    let alpha = with_alpha(base, 0.5);
    assert!((alpha.a - 0.5).abs() < 0.01);

    let lightened = lighten(base, 0.1);
    let darkened = darken(base, 0.1);
    assert_ne!(lightened, darkened);

    let saturated = saturate(base, 0.1);
    let desaturated = desaturate(base, 0.1);
    assert_ne!(saturated, desaturated);
}

// ===========================================================================
// Size
// ===========================================================================

#[test]
fn component_size_math() {
    assert_eq!(ComponentSize::Xs.multiplier(), 0.5);
    assert_eq!(ComponentSize::Sm.multiplier(), 0.75);
    assert_eq!(ComponentSize::Md.multiplier(), 1.0);
    assert_eq!(ComponentSize::Lg.multiplier(), 1.5);
    assert_eq!(ComponentSize::Xl.multiplier(), 2.0);

    assert_eq!(ComponentSize::Lg.to_px(24.0), 36.0);
}

// ===========================================================================
// Mobile primitives
// ===========================================================================

#[test]
fn mobile_primitives() {
    let insets = EdgeInsets::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(insets.horizontal(), 6.0);
    assert_eq!(insets.vertical(), 4.0);

    assert!(matches!(
        PullToRefreshState::from_drag(-1.0, 80.0),
        PullToRefreshState::Idle
    ));
    assert!(matches!(
        PullToRefreshState::from_drag(40.0, 80.0),
        PullToRefreshState::Pulling { .. }
    ));
    assert_eq!(
        PullToRefreshState::from_drag(120.0, 80.0),
        PullToRefreshState::Armed
    );
    assert_eq!(PullToRefreshState::Armed.progress(), 1.0);
    assert_eq!(PullToRefreshState::Refreshing.progress(), 1.0);

    let action = SwipeAction::new("delete", "Delete", SwipeDirection::Trailing).destructive();
    assert!(action.destructive);

    let ok = ContextPreview {
        title: "Title".into(),
        subtitle: Some("sub".into()),
        preferred_width: 100.0,
        preferred_height: 100.0,
    };
    assert!(ok.validate().is_ok());

    let bad_title = ContextPreview {
        title: "   ".into(),
        subtitle: None,
        preferred_width: 100.0,
        preferred_height: 100.0,
    };
    assert!(bad_title.validate().is_err());

    let bad_size = ContextPreview {
        title: "Title".into(),
        subtitle: None,
        preferred_width: 0.0,
        preferred_height: 100.0,
    };
    assert!(bad_size.validate().is_err());

    let policy = DynamicTypePolicy {
        scale_factor: 1.5,
        min_size: 10.0,
        max_size: 20.0,
    };
    assert_eq!(policy.resolve(8.0), 12.0);
    assert_eq!(policy.resolve(100.0), 20.0);

    let mut scrubber = WaveformScrubber {
        duration_seconds: 0.0,
        position_seconds: 5.0,
        samples: vec![0.0],
    };
    assert_eq!(scrubber.normalized_position(), 0.0);
    scrubber.duration_seconds = 100.0;
    scrubber.seek_to_fraction(0.25);
    assert_eq!(scrubber.normalized_position(), 0.25);
    scrubber.seek_to_fraction(2.0);
    assert_eq!(scrubber.normalized_position(), 1.0);
}

// ===========================================================================
// Theme
// ===========================================================================

#[test]
fn theme_variants_and_accessors() {
    for variant in ThemeVariant::all() {
        let theme = Theme::for_variant(*variant);
        assert_eq!(theme.variant, *variant);
        assert!(!theme.variant.name().is_empty());

        let _ = theme.accent_token();
        let _ = theme.success_token();
        let _ = theme.warning_token();
        let _ = theme.error_token();
        let _ = theme.info_token();
        let _ = theme.surface_token();
        let _ = theme.text_primary_token();
        let _ = theme.border_token();
        let _ = theme.to_palette();
    }

    let mut state = ThemeState::new();
    state.set_variant(ThemeVariant::Light);
    state.toggle();
    assert_ne!(state.theme.variant, ThemeVariant::Dark);
}

// ===========================================================================
// Stack layout
// ===========================================================================

#[test]
fn hstack_builder() {
    use gpui_ui_kit::stack::{StackAlign, StackJustify, StackOverflow, StackSize, StackSpacing};

    let design = gpui_ui_kit::design::neutral_design();
    let _ = HStack::new()
        .child(div())
        .children(vec![div(), div()])
        .spacing(StackSpacing::Lg)
        .align(StackAlign::End)
        .justify(StackJustify::SpaceBetween)
        .wrap(true)
        .width(StackSize::Full)
        .height(StackSize::Fixed(px(100.0)))
        .grow(2.0)
        .flex_1()
        .shrink(0.5)
        .basis(px(10.0))
        .overflow_x(StackOverflow::Hidden)
        .overflow_y(StackOverflow::Scroll)
        .overflow(StackOverflow::Auto)
        .min_w(px(10.0))
        .min_h(px(10.0))
        .max_w(px(500.0))
        .max_h(px(500.0))
        .design(design.clone())
        .build_with_design(&design);
}

#[test]
fn vstack_builder() {
    use gpui_ui_kit::stack::{StackAlign, StackJustify, StackOverflow, StackSize, StackSpacing};

    let design = gpui_ui_kit::design::neutral_design();
    let _ = VStack::new()
        .child(div())
        .children(vec![div()])
        .spacing(StackSpacing::Custom(px(12.0)))
        .align(StackAlign::Start)
        .justify(StackJustify::Center)
        .width(StackSize::Fraction(0.5))
        .height(StackSize::Auto)
        .full()
        .grow(1.0)
        .flex_1()
        .shrink(1.0)
        .basis(px(0.0))
        .overflow(StackOverflow::Hidden)
        .min_w(px(0.0))
        .min_h(px(0.0))
        .max_w(px(1000.0))
        .max_h(px(1000.0))
        .design(design.clone())
        .build();
}

#[test]
fn divider_builder() {
    let theme = Theme::dark();
    let _ = Divider::new()
        .id("divider")
        .color(rgb(0xffffff))
        .hover_color(rgb(0x000000))
        .thickness(px(2.0))
        .interactive()
        .build();
    let _ = Divider::vertical().build_with_theme(&theme);
    let _ = Divider::new().build_simple();
    let _ = Divider::new().resolve_color(&theme);
}

#[test]
fn spacer_and_stack_spacing() {
    let _ = Spacer::new().build();
    let _ = Spacer.build();
}

// ===========================================================================
// Focus
// ===========================================================================

#[test]
fn focus_group_builder() {
    let _ = FocusGroup::new("group")
        .direction(FocusDirection::Horizontal)
        .wraparound(true)
        .focus_ring(false)
        .gap(px(8.0))
        .child(div())
        .children(vec![div(), div()]);

    let _ = FocusGroup::new("grid").direction(FocusDirection::Grid { columns: 3 });
}

// ===========================================================================
// Menu
// ===========================================================================

#[test]
fn menu_bar_builder() {
    let items = vec![
        MenuBarItem::new("file", "File"),
        MenuBarItem::new("edit", "Edit"),
    ];
    let bar = MenuBar::new(items)
        .active_menu(Some("file".into()))
        .on_select(|_id, _window, _cx| {})
        .on_menu_toggle(|_id, _window, _cx| {});

    assert_eq!(bar.items().len(), 2);
    assert_eq!(bar.get_active_menu(), Some(&SharedString::from("file")));

    let theme = MenuTheme::default();
    let _ = bar.build_with_theme(&theme);

    let _ = menu_bar_button("help", "Help", false, &theme);
}

// ===========================================================================
// Form builders
// ===========================================================================

#[test]
fn input_builder() {
    let _ = Input::new("input")
        .value("hello")
        .placeholder("type here")
        .label("Name")
        .size(InputSize::default())
        .variant(InputVariant::default())
        .disabled(true)
        .readonly(true)
        .error("bad")
        .icon_left("left")
        .icon_right("right")
        .bg_color(rgb(0x000000))
        .text_color(rgb(0xffffff))
        .border_color(rgb(0x333333))
        .placeholder_color(rgb(0x666666))
        .on_change(|_v, _window, _cx| {})
        .on_edit_start(|_window, _cx| {})
        .on_edit_end(|_v, _window, _cx| {})
        .on_text_change(|_v, _window, _cx| {})
        .aria_label("name")
        .aria_role(AriaRole::Textbox);
}

#[test]
fn number_input_builder() {
    let _ = NumberInput::new("number")
        .value(5.0)
        .min(0.0)
        .max(10.0)
        .range(0.0, 10.0)
        .step(0.5)
        .decimals(2)
        .unit("px")
        .label("Width")
        .size(NumberInputSize::default())
        .width(100.0)
        .disabled(true)
        .theme(NumberInputTheme::default())
        .on_change(|_v, _window, _cx| {})
        .aria_label("width")
        .aria_role(AriaRole::Spinbutton);
}

#[test]
fn slider_builder() {
    let _ = Slider::new("slider")
        .value(50.0)
        .min(0.0)
        .max(100.0)
        .range(0.0, 100.0)
        .step(5.0)
        .size(SliderSize::default())
        .disabled(true)
        .show_value(true)
        .label("Volume")
        .width(200.0)
        .on_change(|_v, _window, _cx| {})
        .on_drag_start(|_v, _x, _window, _cx| {})
        .on_reset(|_window, _cx| {})
        .track_color(rgb(0x000000))
        .fill_color(rgb(0xffffff))
        .thumb_color(rgb(0x888888))
        .theme(SliderTheme::default())
        .aria_label("volume")
        .aria_role(AriaRole::Slider);
}

#[test]
fn table_column_builder() {
    let _ = Column::<i32>::new("id", "ID")
        .width(px(100.0))
        .min_width(px(50.0))
        .sortable(true)
        .filterable(true)
        .resizable(false)
        .cell_render(|_item, _idx, _window, _cx| div())
        .header_render(|_window, _cx| div())
        .footer_render(|_window, _cx| div());
}

// ===========================================================================
// Workflow canvas
// ===========================================================================

#[test]
fn workflow_graph_operations() {
    let mut graph = WorkflowGraph::new();
    assert!(graph.is_empty());

    let n1 = WorkflowNodeData::new("A", Position::new(0.0, 0.0));
    let n2 = WorkflowNodeData::new("B", Position::new(100.0, 0.0));
    let id1 = graph.add_node(n1);
    let id2 = graph.add_node(n2);
    assert!(!graph.is_empty());

    let conn_id = graph.add_connection(id1, 0, id2, 0).unwrap();
    assert_eq!(graph.connections_from(id1).len(), 1);
    assert_eq!(graph.connections_to(id2).len(), 1);

    // Duplicate connection
    assert!(graph.add_connection(id1, 0, id2, 0).is_err());
    // Self-loop
    assert!(graph.add_connection(id1, 0, id1, 0).is_err());
    // Missing node
    let missing = NodeId::new_v4();
    assert!(graph.add_connection(missing, 0, id2, 0).is_err());
    assert!(graph.add_connection(id1, 0, missing, 0).is_err());
    // Port out of bounds
    assert!(graph.add_connection(id1, 99, id2, 0).is_err());

    // Cycle detection
    assert!(graph.would_create_cycle(id2, id1));
    assert!(!graph.would_create_cycle(id1, id2));

    graph.remove_connection(conn_id);
    assert!(graph.connections.is_empty());

    graph.remove_node(id1);
    assert!(graph.connections_from(id1).is_empty());
}

#[test]
fn workflow_node_data_helpers() {
    let mut node = WorkflowNodeData::new("Node", Position::new(10.0, 20.0))
        .with_ports(2, 3)
        .with_size(200.0, 120.0)
        .with_max_ports(Some(4), Some(5))
        .with_user_data(serde_json::json!({"key": "value"}));

    assert_eq!(node.input_count, 2);
    assert_eq!(node.output_count, 3);
    assert_eq!(node.user_data["key"], "value");

    let center = node.center();
    assert_eq!(center.x, 110.0);
    assert_eq!(center.y, 80.0);

    node.grow_inputs_to(10);
    assert_eq!(node.input_count, 4);

    let in_pos = node.input_port_position(0);
    let out_pos = node.output_port_position(0);
    assert_eq!(in_pos.x, node.position.x);
    assert_eq!(out_pos.x, node.position.x + node.width);

    // Zero-port fallback
    let zero = WorkflowNodeData::new("Zero", Position::new(0.0, 0.0)).with_ports(0, 0);
    let in_pos = zero.input_port_position(0);
    assert_eq!(in_pos.y, zero.position.y + zero.height / 2.0);
}

#[test]
fn workflow_viewport_and_selection() {
    let mut viewport = ViewportState::default();
    let canvas = viewport.screen_to_canvas(100.0, 100.0);
    assert_eq!(canvas, Position::new(100.0, 100.0));

    viewport.pan(10.0, -10.0);
    assert_eq!(viewport.offset.x, 10.0);
    assert_eq!(viewport.offset.y, -10.0);

    viewport.zoom_at(1.0, 100.0, 100.0);
    assert!(viewport.zoom >= 1.0);

    let screen = viewport.canvas_to_screen(&Position::new(0.0, 0.0));
    assert!(screen.x.is_finite() && screen.y.is_finite());

    let mut selection = SelectionState::default();
    assert!(selection.is_empty());
    let id = NodeId::new_v4();
    selection.select_node(id, false);
    assert!(selection.is_node_selected(id));
    selection.toggle_node(id);
    assert!(!selection.is_node_selected(id));
    selection.toggle_node(id);

    let conn = ConnectionId::new_v4();
    selection.select_connection(conn, true);
    assert!(selection.is_connection_selected(conn));
    selection.clear();
    assert!(selection.is_empty());
}

#[test]
fn workflow_connection_variants() {
    let a = NodeId::new_v4();
    let b = NodeId::new_v4();
    let fat = Connection::new(a, 0, b, 0);
    let thin = Connection::new_thin(a, 0, b, 0);
    assert_eq!(fat.link_type, LinkType::Fat);
    assert_eq!(thin.link_type, LinkType::Thin);

    let swapped = thin.with_link_type(LinkType::Fat);
    assert_eq!(swapped.link_type, LinkType::Fat);
}

#[test]
fn workflow_hit_testing() {
    let mut graph = WorkflowGraph::new();
    let n1 = WorkflowNodeData::new("A", Position::new(0.0, 0.0))
        .with_ports(1, 1)
        .with_size(180.0, 100.0);
    let n2 = WorkflowNodeData::new("B", Position::new(300.0, 0.0))
        .with_ports(1, 1)
        .with_size(180.0, 100.0);
    let id1 = graph.add_node(n1);
    let id2 = graph.add_node(n2);
    graph.add_connection(id1, 0, id2, 0).unwrap();

    let tester = HitTester::new()
        .with_port_radius(10.0)
        .with_connection_tolerance(5.0);

    // Hit an output port of node A (right content edge)
    let result = tester.hit_test(Position::new(178.0, 64.0), &graph);
    assert!(
        matches!(result, HitTestResult::OutputPort(_, 0)),
        "expected output port, got {:?}",
        result
    );

    // Hit an input port of node B (left content edge)
    let result = tester.hit_test(Position::new(302.0, 64.0), &graph);
    assert!(
        matches!(result, HitTestResult::InputPort(_, 0)),
        "expected input port, got {:?}",
        result
    );

    // Hit node body
    let result = tester.hit_test(Position::new(50.0, 50.0), &graph);
    assert!(matches!(result, HitTestResult::Node(_)));

    // Hit connection roughly midpoint
    let result = tester.hit_test(Position::new(240.0, 64.0), &graph);
    assert!(
        matches!(result, HitTestResult::Connection(_)),
        "got {:?}",
        result
    );

    // Hit canvas
    let result = tester.hit_test(Position::new(1000.0, 1000.0), &graph);
    assert_eq!(result, HitTestResult::Canvas);

    // Nodes in rect
    let nodes = tester.nodes_in_rect(-10.0, -10.0, 500.0, 200.0, &graph);
    assert_eq!(nodes.len(), 2);
}

#[test]
fn workflow_history_undo_redo() {
    let mut graph = WorkflowGraph::new();
    let mut history = HistoryManager::new();

    let node = WorkflowNodeData::new("N", Position::new(0.0, 0.0));
    let id = node.id;
    history.execute(Box::new(AddNodeCommand { node }), &mut graph);
    assert!(graph.nodes.contains_key(&id));

    let conn = Connection::new(id, 0, id, 0);
    let conn_id = conn.id;
    history.execute(
        Box::new(AddConnectionCommand { connection: conn }),
        &mut graph,
    );
    // Note: self-loop is invalid in graph but command pushes directly; verify presence.
    assert!(graph.connections.iter().any(|c| c.id == conn_id));

    assert!(history.can_undo());
    assert_eq!(history.undo_description(), Some("Add connection"));
    history.undo(&mut graph);
    assert!(!graph.connections.iter().any(|c| c.id == conn_id));

    assert!(history.can_redo());
    assert_eq!(history.redo_description(), Some("Add connection"));
    history.redo(&mut graph);
    assert!(graph.connections.iter().any(|c| c.id == conn_id));

    // Remove connection command
    let to_remove = graph
        .connections
        .iter()
        .find(|c| c.id == conn_id)
        .unwrap()
        .clone();
    history.execute(
        Box::new(RemoveConnectionCommand {
            connection: to_remove,
        }),
        &mut graph,
    );
    assert!(!graph.connections.iter().any(|c| c.id == conn_id));

    // Remove node command
    let node = graph.nodes.remove(&id).unwrap();
    let conns = graph.connections.clone();
    history.record(Box::new(RemoveNodeCommand {
        node,
        connections: conns,
    }));
    assert!(!graph.nodes.contains_key(&id));

    // Change port counts
    let node = WorkflowNodeData::new("M", Position::new(0.0, 0.0));
    let mid = node.id;
    graph.add_node(node.clone());
    history.execute(
        Box::new(ChangePortCountsCommand {
            node_id: mid,
            old_input_count: 1,
            new_input_count: 3,
            old_output_count: 1,
            new_output_count: 4,
            old_height: 100.0,
            new_height: 140.0,
        }),
        &mut graph,
    );
    assert_eq!(graph.nodes[&mid].input_count, 3);
    assert_eq!(graph.nodes[&mid].output_count, 4);

    // Max history trim
    let mut big_history = HistoryManager::with_max_history(2);
    big_history.execute(
        Box::new(AddNodeCommand {
            node: WorkflowNodeData::new("1", Position::new(0.0, 0.0)),
        }),
        &mut graph,
    );
    big_history.execute(
        Box::new(AddNodeCommand {
            node: WorkflowNodeData::new("2", Position::new(0.0, 0.0)),
        }),
        &mut graph,
    );
    big_history.execute(
        Box::new(AddNodeCommand {
            node: WorkflowNodeData::new("3", Position::new(0.0, 0.0)),
        }),
        &mut graph,
    );
    // History trimmed to max 2; undo should still work for the most recent commands.
    assert!(big_history.can_undo());

    history.clear();
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn theme_variant_helpers() {
    let mut v = ThemeVariant::Dark;
    for _ in 0..ThemeVariant::all().len() {
        v = v.toggle();
    }
    assert_eq!(v, ThemeVariant::Dark);
    assert_eq!(ThemeVariant::Light.name(), "Light");
}

#[test]
fn select_builder() {
    let _ = Select::new("select")
        .options(vec![
            SelectOption::new("a", "Alpha"),
            SelectOption::new("b", "Beta"),
        ])
        .selected("a")
        .placeholder("Choose")
        .label("Letter")
        .size(SelectSize::Md)
        .disabled(true)
        .is_open(true)
        .highlighted_index(Some(0))
        .theme(SelectTheme::default())
        .on_change(|_v, _window, _cx| {})
        .on_toggle(|_open, _window, _cx| {})
        .on_highlight(|_idx, _window, _cx| {})
        .aria_label("letter")
        .aria_role(AriaRole::Combobox);
}

#[test]
fn tabs_and_tab_item_builder() {
    let item = TabItem::new("home", "Home")
        .icon("🏠")
        .custom_icon(div())
        .icon_with_color(|_color| div().into_any_element())
        .badge("3")
        .disabled(true)
        .closeable(true);
    assert_eq!(item.id(), &SharedString::from("home"));

    let _ = Tabs::new("tabs")
        .tabs(vec![item])
        .selected_index(0)
        .variant(TabVariant::default())
        .theme(TabsTheme::default())
        .on_change(|_idx, _window, _cx| {})
        .on_close(|_id, _window, _cx| {})
        .aria_label("navigation")
        .aria_role(AriaRole::Tablist);
}

#[test]
fn table_builder() {
    use gpui_ui_kit::table::{PaginationState, SelectionMode, SortDirection, SortState};

    let col = Column::<i32>::new("id", "ID").width(px(50.0));
    let _ = Table::new("table", vec![1, 2, 3])
        .column(col)
        .columns(vec![])
        .sort(SortState {
            column_id: "id".into(),
            direction: SortDirection::Ascending,
        })
        .on_sort(|_state, _window, _cx| {})
        .selection_mode(SelectionMode::Multiple)
        .selected_indices([0].into())
        .on_selection_change(|_set, _window, _cx| {})
        .pagination(PaginationState {
            current_page: 0,
            page_size: 10,
            total_items: 30,
        })
        .on_page_change(|_page, _window, _cx| {})
        .on_resize(|_id, _px, _window, _cx| {})
        .alternating_rows(false)
        .show_footer(true)
        .theme(TableTheme::default())
        .design(gpui_ui_kit::design::neutral_design())
        .aria_label("data")
        .aria_role(AriaRole::Table);

    assert_eq!(SortDirection::Ascending.toggle(), SortDirection::Descending);
    assert_eq!(SortDirection::Descending.toggle(), SortDirection::Ascending);
}

#[test]
fn toggle_builder() {
    let theme = ToggleTheme::default();
    let _ = Toggle::new("toggle")
        .checked(true)
        .label("Enable")
        .size(ToggleSize::Md)
        .style(ToggleStyle::Sliding)
        .disabled(true)
        .selected(true)
        .theme(ToggleTheme::default())
        .on_change(|_v, _window, _cx| {})
        .aria_label("enable")
        .aria_role(AriaRole::Switch)
        .build_with_theme(&theme);
}

#[test]
fn checkbox_builder() {
    let _ = Checkbox::new("checkbox")
        .checked(true)
        .indeterminate(true)
        .label("Accept")
        .size(CheckboxSize::Lg)
        .disabled(true)
        .design(gpui_ui_kit::design::neutral_design())
        .on_change(|_v, _window, _cx| {})
        .aria_label("accept")
        .aria_role(AriaRole::Checkbox);
}

#[test]
fn button_and_icon_button_builders() {
    let _ = Button::new("save", "Save")
        .variant(ButtonVariant::Primary)
        .size(ButtonSize::default())
        .disabled(true)
        .selected(true)
        .full_width(true)
        .icon_left("★")
        .icon_right("→")
        .theme(ButtonTheme::default())
        .design(gpui_ui_kit::design::neutral_design())
        .on_click(|_window, _cx| {})
        .aria_label("save")
        .aria_role(AriaRole::Button);

    let _ = IconButton::new("close", "✕")
        .size(IconButtonSize::Md)
        .variant(IconButtonVariant::Ghost)
        .disabled(true)
        .selected(true)
        .theme(IconButtonTheme::default())
        .on_click(|_window, _cx| {})
        .aria_label("close");
}

#[test]
fn badge_and_tag_builders() {
    let theme = Theme::dark();
    let _ = Badge::new("new")
        .variant(BadgeVariant::Primary)
        .size(BadgeSize::Sm)
        .rounded(true)
        .icon("★")
        .build_with_theme(&theme);

    let tag_theme = TagTheme::default();
    let _ = Tag::new("tag", "Label")
        .size(TagSize::Sm)
        .variant(TagVariant::Outlined)
        .icon("★")
        .removable(true)
        .on_click(|_window, _cx| {})
        .on_remove(|_window, _cx| {})
        .build_with_theme(&tag_theme);
}
