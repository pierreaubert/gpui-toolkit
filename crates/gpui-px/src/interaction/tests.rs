use super::chart_interaction::ChartInteraction;
use super::chart_interaction::apply_wheel_zoom;
use super::mouse_state::MouseState;
use super::types::InteractionMode;
use super::wheel_config::WheelConfig;
use d3rs::brush::BrushConfig;

#[test]
fn test_chart_interaction_creation() {
    let interaction = ChartInteraction::new(0.0, 100.0, -10.0, 10.0);
    assert_eq!(interaction.x_domain(), (0.0, 100.0));
    assert_eq!(interaction.y_domain(), (-10.0, 10.0));
    assert!(!interaction.is_zoomed());
}

#[test]
fn test_brush_lifecycle() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);

    assert!(!interaction.is_brushing());

    interaction.start_brush(100.0, 100.0);
    assert!(interaction.is_brushing());

    interaction.update_brush(300.0, 300.0);
    let selection = interaction.current_brush_selection().unwrap();
    assert_eq!(selection.width(), 200.0);
    assert_eq!(selection.height(), 200.0);

    let domain = interaction.end_brush(false).unwrap();
    assert!(!interaction.is_brushing());
    // Domain values depend on scale conversion
    assert!(domain.x1 > domain.x0);
    assert!(domain.y1 > domain.y0);
}

#[test]
fn test_brush_with_zoom() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);

    // Brush the center 50% of the chart
    interaction.start_brush(125.0, 125.0);
    interaction.update_brush(375.0, 375.0);
    interaction.end_brush(true);

    // Should now be zoomed
    assert!(interaction.is_zoomed());
    let (x_min, x_max) = interaction.x_domain();
    assert!(x_min > 0.0 && x_max < 100.0);
}

#[test]
fn test_zoom_reset() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);

    interaction.zoom_to(25.0, 75.0, 25.0, 75.0);
    assert!(interaction.is_zoomed());

    interaction.reset_zoom();
    assert!(!interaction.is_zoomed());
    assert_eq!(interaction.x_domain(), (0.0, 100.0));
}

#[test]
fn test_zoom_back() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);

    interaction.zoom_to(25.0, 75.0, 25.0, 75.0);
    interaction.zoom_to(35.0, 65.0, 35.0, 65.0);
    assert_eq!(interaction.zoom_level(), 2);

    interaction.zoom_back();
    assert_eq!(interaction.zoom_level(), 1);
    assert_eq!(interaction.x_domain(), (25.0, 75.0));

    interaction.zoom_back();
    assert!(!interaction.is_zoomed());
}

#[test]
fn test_log_scale_interaction() {
    let interaction = ChartInteraction::new(20.0, 20000.0, -40.0, 10.0)
        .with_log_x(true)
        .with_size(500.0, 200.0);

    // Get domain point at center of chart
    let (x, _y) = interaction.point_to_domain(250.0, 100.0);

    // For log scale, center should be geometric mean: sqrt(20 * 20000) ≈ 632
    assert!((x - 632.0).abs() < 50.0);
}

#[test]
fn test_double_click_detection() {
    let mut state = MouseState::default();

    // First click - not a double click
    assert!(!state.is_double_click(100.0, 100.0));
    state.record_click(100.0, 100.0);

    // Immediate second click - should be double click
    assert!(state.is_double_click(101.0, 101.0));

    // Far away click - not a double click
    assert!(!state.is_double_click(200.0, 200.0));
}

#[test]
fn test_wheel_zoom() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);
    let config = WheelConfig::default();

    let original_x = interaction.x_domain();

    // Zoom in (negative delta)
    apply_wheel_zoom(&mut interaction, -1.0, 250.0, 250.0, &config);

    // Should be zoomed in (smaller domain range)
    let new_x = interaction.x_domain();
    assert!(new_x.1 - new_x.0 < original_x.1 - original_x.0);
}

#[test]
fn test_interaction_mode() {
    let interaction = ChartInteraction::default();
    assert_eq!(interaction.mode, InteractionMode::None);

    let interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);
    assert_eq!(interaction.mode, InteractionMode::Brush);

    let interaction = interaction.with_mode(InteractionMode::Zoom);
    assert_eq!(interaction.mode, InteractionMode::Zoom);
}

#[test]
fn test_brush_config() {
    let config = BrushConfig {
        fill_color: (255, 0, 0, 128),
        stroke_color: (255, 0, 0),
        stroke_width: 2.0,
        min_size: 10.0,
    };

    let interaction = ChartInteraction::default().with_brush_config(config.clone());
    assert_eq!(interaction.brush_config.min_size, 10.0);
}

#[test]
fn test_trivial_brush_rejected() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);

    // Very small brush (less than min_size of 5.0)
    interaction.start_brush(100.0, 100.0);
    interaction.update_brush(102.0, 102.0);
    let result = interaction.end_brush(false);

    // Should return None because selection is too small
    assert!(result.is_none());
}

#[test]
fn test_cancel_brush() {
    let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);

    interaction.start_brush(100.0, 100.0);
    assert!(interaction.is_brushing());

    interaction.cancel_brush();
    assert!(!interaction.is_brushing());
    assert!(interaction.current_brush_selection().is_none());
}

#[test]
fn test_wheel_zoom_clamps_log_scale() {
    let mut interaction = ChartInteraction::new(1.0, 100.0, 1.0, 100.0)
        .with_log_x(true)
        .with_log_y(true)
        .with_size(500.0, 500.0);
    let config = WheelConfig::default();

    // Zoom in aggressively at the lower-left corner
    apply_wheel_zoom(&mut interaction, -10.0, 10.0, 10.0, &config);

    let (x_min, x_max) = interaction.x_domain();
    let (y_min, y_max) = interaction.y_domain();
    assert!(
        x_min > 0.0,
        "log x_min should be clamped above 0, got {}",
        x_min
    );
    assert!(
        y_min > 0.0,
        "log y_min should be clamped above 0, got {}",
        y_min
    );
    assert!(x_min < x_max);
    assert!(y_min < y_max);
}

#[test]
fn test_pan_clamps_log_scale() {
    #[cfg(feature = "gpui")]
    {
        use super::interactive_chart::*;
        let state = InteractiveChartState::new(1.0, 100.0, 1.0, 100.0)
            .with_log_x(true)
            .with_log_y(true)
            .with_size(500.0, 500.0);

        // Pan far to the left/down
        state.apply_pan(-5000.0, 5000.0);

        let (x_min, x_max) = state.x_domain();
        let (y_min, y_max) = state.y_domain();
        assert!(
            x_min > 0.0,
            "log x_min should be clamped above 0, got {}",
            x_min
        );
        assert!(
            y_min > 0.0,
            "log y_min should be clamped above 0, got {}",
            y_min
        );
        assert!(x_min < x_max);
        assert!(y_min < y_max);
    }
    #[cfg(not(feature = "gpui"))]
    {
        // Skip when gpui feature is not available
    }
}

#[cfg(feature = "gpui")]
mod interactive_chart_state_tests {
    use super::super::interactive_chart::*;

    #[test]
    fn test_interactive_chart_state_creation() {
        let state = InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0);
        assert_eq!(state.x_domain(), (20.0, 20000.0));
        assert_eq!(state.y_domain(), (-40.0, 10.0));
        assert!(!state.is_zoomed());
    }

    #[test]
    fn test_interactive_chart_state_with_log_x() {
        let state = InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0).with_log_x(true);
        assert!(state.interaction.borrow().x_is_log);
    }

    #[test]
    fn test_interactive_chart_state_with_size() {
        let state = InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0).with_size(800.0, 400.0);
        assert_eq!(state.interaction.borrow().plot_size, (800.0, 400.0));
    }

    #[test]
    fn test_interactive_chart_state_reset_zoom() {
        let state = InteractiveChartState::new(0.0, 100.0, 0.0, 100.0);

        // Zoom in
        state
            .interaction
            .borrow_mut()
            .zoom_to(25.0, 75.0, 25.0, 75.0);
        assert!(state.is_zoomed());

        // Reset
        state.reset_zoom();
        assert!(!state.is_zoomed());
        assert_eq!(state.x_domain(), (0.0, 100.0));
    }

    #[test]
    fn test_interactive_chart_config() {
        let config = InteractiveChartConfig::new()
            .with_left_margin(60.0)
            .with_top_margin(40.0)
            .with_pan(true)
            .with_wheel_zoom(true)
            .with_double_click_reset(true);

        assert_eq!(config.left_margin, 60.0);
        assert_eq!(config.top_margin, 40.0);
        assert!(config.enable_pan);
        assert!(config.enable_wheel_zoom);
        assert!(config.enable_double_click_reset);
    }

    #[test]
    fn test_interactive_chart_state_with_config() {
        let config = InteractiveChartConfig::new()
            .with_left_margin(80.0)
            .with_pan(false);

        let state = InteractiveChartState::new(0.0, 100.0, 0.0, 100.0).with_config(config.clone());

        assert_eq!(state.config.left_margin, 80.0);
        assert!(!state.config.enable_pan);
    }
}
