use super::chart_size::ChartSize;
pub use super::error::ChartError;
use super::misc::extent_log_padded_iter;
use super::misc::extent_padded;
use super::misc::extent_padded_iter;
use super::validate::validate_data_array;
use super::validate::validate_data_length;
use super::validate::validate_dimensions;
use super::validate::validate_grid_dimensions;
use super::validate::validate_monotonic;
use super::validate::validate_positive;
use super::validate::validate_range;
use super::validate::validate_range_log;

#[test]
fn test_chart_size_layout_dimensions_honor_aspect_ratio() {
    assert_eq!(
        ChartSize::fill()
            .min_size(300.0, 200.0)
            .aspect_ratio(2.0)
            .layout_dimensions(),
        (400.0, 200.0)
    );
    assert_eq!(
        ChartSize::fill()
            .min_size(300.0, 200.0)
            .aspect_ratio(1.0)
            .layout_dimensions(),
        (300.0, 300.0)
    );
    assert_eq!(
        ChartSize::fixed(320.0, 180.0).layout_dimensions(),
        (320.0, 180.0)
    );
}

#[cfg(feature = "gpui")]
#[test]
fn test_accessibility_summary_covers_xy_and_categorical_charts() {
    let scatter_summary = crate::scatter(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0])
        .title("Scatter QA")
        .label("Primary")
        .add_series(
            &[4.0, 5.0],
            &[40.0, 50.0],
            Some("Secondary"),
            0xff7f0e,
            4.0,
            0.8,
        )
        .accessibility_summary();
    assert_eq!(scatter_summary.chart_type, "scatter");
    assert_eq!(scatter_summary.series_count, 2);
    assert_eq!(scatter_summary.datum_count, 5);
    assert_eq!(scatter_summary.x_range, Some([1.0, 5.0]));
    assert_eq!(scatter_summary.y_range, Some([10.0, 50.0]));
    assert_eq!(scatter_summary.series_labels, ["Primary", "Secondary"]);
    assert!(scatter_summary.description.contains("Scatter QA"));

    let line_summary = crate::line(&[1.0, 2.0, 3.0], &[70.0, 72.0, 74.0])
        .label("SPL")
        .add_series_y2(&[4.0, 5.0, 6.0], Some("DI"), 0xff7f0e, 2.0, 1.0)
        .accessibility_summary();
    assert_eq!(line_summary.chart_type, "line");
    assert_eq!(line_summary.series_count, 2);
    assert_eq!(line_summary.datum_count, 6);
    assert!(line_summary.description.contains("secondary Y axis"));

    let bar_summary = crate::bar(&["Q1", "Q2"], &[12.0, 24.0])
        .label("2025")
        .add_series(&[18.0, 30.0], Some("2026"), 0x2ca02c, 1.0)
        .accessibility_summary();
    assert_eq!(bar_summary.chart_type, "bar");
    assert_eq!(bar_summary.series_count, 2);
    assert_eq!(bar_summary.datum_count, 4);
    assert_eq!(bar_summary.value_range, Some([12.0, 30.0]));
    assert_eq!(bar_summary.series_labels, ["2025", "2026"]);
}

#[cfg(feature = "gpui")]
#[test]
fn test_accessibility_summary_covers_grid_and_slice_charts() {
    let z = [1.0, 2.0, 3.0, 4.0];

    let heatmap_summary = crate::heatmap(&z, 2, 2)
        .x(&[10.0, 20.0])
        .y(&[100.0, 200.0])
        .accessibility_summary();
    assert_eq!(heatmap_summary.chart_type, "heatmap");
    assert_eq!(heatmap_summary.datum_count, 4);
    assert_eq!(heatmap_summary.x_range, Some([10.0, 20.0]));
    assert_eq!(heatmap_summary.y_range, Some([100.0, 200.0]));
    assert_eq!(heatmap_summary.value_range, Some([1.0, 4.0]));

    let contour_summary = crate::contour(&z, 2, 2)
        .thresholds(vec![1.0, 2.0, 3.0])
        .accessibility_summary();
    assert_eq!(contour_summary.chart_type, "contour");
    assert_eq!(contour_summary.value_range, Some([1.0, 4.0]));
    assert!(contour_summary.description.contains("3 thresholds"));

    let isoline_summary = crate::isoline(&z, 2, 2)
        .levels(vec![1.5, 2.5])
        .accessibility_summary();
    assert_eq!(isoline_summary.chart_type, "isoline");
    assert_eq!(isoline_summary.value_range, Some([1.0, 4.0]));
    assert!(isoline_summary.description.contains("2 contour levels"));

    let pie_summary = crate::pie(&[10.0, 20.0, 30.0])
        .labels(&["A", "B", "C"])
        .accessibility_summary();
    assert_eq!(pie_summary.chart_type, "pie");
    assert_eq!(pie_summary.datum_count, 3);
    assert_eq!(pie_summary.value_range, Some([10.0, 30.0]));
    assert_eq!(pie_summary.series_labels, ["A", "B", "C"]);

    let donut_summary = crate::donut(&[10.0, 20.0]).accessibility_summary();
    assert_eq!(donut_summary.chart_type, "donut");
}

#[cfg(feature = "gpui")]
#[test]
fn test_accessibility_summary_covers_area_boxplot_and_treemap() {
    let area_summary = crate::area(&[1.0, 2.0, 3.0], &[2.0, 4.0, 8.0])
        .y0(&[1.0, 1.5, 2.0])
        .accessibility_summary();
    assert_eq!(area_summary.chart_type, "area");
    assert_eq!(area_summary.datum_count, 3);
    assert_eq!(area_summary.x_range, Some([1.0, 3.0]));
    assert_eq!(area_summary.y_range, Some([1.0, 8.0]));
    assert!(area_summary.description.contains("explicit baseline"));

    let boxplot_summary = crate::boxplot(&[1.0, 2.0, 3.0], &[8.0, 13.0, 21.0])
        .bins(3)
        .accessibility_summary();
    assert_eq!(boxplot_summary.chart_type, "boxplot");
    assert_eq!(boxplot_summary.datum_count, 3);
    assert_eq!(boxplot_summary.value_range, Some([8.0, 21.0]));
    assert!(boxplot_summary.description.contains("3 bins"));

    let root = crate::TreemapNode::new("Sales", 0.0)
        .add_child(crate::TreemapNode::new("East", 45.0))
        .add_child(crate::TreemapNode::new("West", 55.0));
    let treemap_summary = crate::treemap(&root).accessibility_summary();
    assert_eq!(treemap_summary.chart_type, "treemap");
    assert_eq!(treemap_summary.datum_count, 3);
    assert_eq!(treemap_summary.value_range, Some([45.0, 55.0]));
    assert_eq!(treemap_summary.series_labels, ["East", "West"]);
    assert!(treemap_summary.description.contains("2 leaves"));
}

#[cfg(feature = "gpui")]
#[test]
fn test_accessibility_summary_exports_bridge_snapshot() {
    let summary = crate::line(&[1.0, 2.0, 3.0], &[70.0, 72.0, 74.0])
        .title("SPL trend")
        .label("Left")
        .add_series(&[68.0, 71.0, 73.0], Some("Right"), 0xff7f0e, 2.0, 1.0)
        .accessibility_summary();

    let snapshot = summary.to_bridge_snapshot(gpui::ElementId::Name("spl-trend".into()));

    assert_eq!(
        snapshot.report_type,
        gpui_ui_kit::ACCESSIBILITY_BRIDGE_REPORT_TYPE
    );
    assert_eq!(snapshot.nodes.len(), 1);
    assert!(snapshot.all_nodes_named());

    let node = &snapshot.nodes[0];
    assert_eq!(node.role, gpui_ui_kit::AriaRole::Img);
    assert_eq!(node.role_name, "img");
    assert_eq!(node.label.as_ref(), "SPL trend");
    assert!(
        node.description
            .as_ref()
            .expect("chart bridge node should include description")
            .contains("line chart")
    );
    let value_text = node
        .value
        .text
        .as_ref()
        .expect("chart bridge node should include value text");
    assert!(value_text.contains("2 series"));
    assert!(value_text.contains("6 data points"));
    assert!(value_text.contains("y range 68.000 to 74.000"));
    assert!(value_text.contains("series Left, Right"));
}

#[cfg(feature = "gpui")]
#[test]
fn test_legend_summary_covers_native_legend_families() {
    let line_legend = crate::line(&[1.0, 2.0, 3.0], &[70.0, 72.0, 74.0])
        .label("SPL")
        .add_series_y2(&[4.0, 5.0, 6.0], Some("DI"), 0xff7f0e, 2.0, 1.0)
        .hidden_series(&[1])
        .legend_position(crate::LegendPosition::Bottom)
        .legend_summary();
    assert_eq!(line_legend.chart_type, "line");
    assert!(line_legend.visible);
    assert!(line_legend.position_explicit);
    assert_eq!(line_legend.position, crate::LegendPosition::Bottom);
    assert_eq!(line_legend.item_count(), 2);
    assert_eq!(line_legend.items[0].label, "SPL");
    assert_eq!(line_legend.items[0].marker, crate::ChartLegendMarker::Line);
    assert!(!line_legend.items[0].hidden);
    assert_eq!(line_legend.items[1].label, "DI");
    assert!(line_legend.items[1].hidden);
    assert!(line_legend.items[1].uses_secondary_axis);
    assert!(line_legend.description.contains("secondary Y axis"));

    let scatter_legend = crate::scatter(&[1.0, 2.0], &[3.0, 4.0])
        .label("Primary")
        .add_series(
            &[2.0, 3.0],
            &[4.0, 5.0],
            Some("Secondary"),
            0xff7f0e,
            5.0,
            0.8,
        )
        .legend_summary();
    assert_eq!(scatter_legend.item_count(), 2);
    assert_eq!(
        scatter_legend.items[0].marker,
        crate::ChartLegendMarker::Circle
    );
    assert!(scatter_legend.visible);

    let bar_legend = crate::bar(&["Q1", "Q2"], &[10.0, 20.0])
        .label("2025")
        .add_series(&[12.0, 24.0], Some("2026"), 0x2ca02c, 0.8)
        .legend_summary();
    assert_eq!(bar_legend.item_count(), 2);
    assert_eq!(bar_legend.items[0].marker, crate::ChartLegendMarker::Square);
    assert!(bar_legend.visible);
}

#[cfg(feature = "gpui")]
#[test]
fn test_legend_summary_respects_hidden_position_and_unlabeled_charts() {
    let hidden_legend = crate::line(&[1.0, 2.0], &[1.0, 2.0])
        .label("Hidden")
        .legend_position(crate::LegendPosition::Hidden)
        .legend_summary();
    assert!(!hidden_legend.visible);
    assert_eq!(hidden_legend.item_count(), 1);

    let unlabeled = crate::scatter(&[1.0, 2.0], &[1.0, 2.0]).legend_summary();
    assert!(!unlabeled.visible);
    assert_eq!(unlabeled.item_count(), 0);
}

#[cfg(feature = "gpui")]
#[test]
fn test_annotation_summary_covers_native_annotation_families() {
    let line_annotations = crate::line(&[1.0, 2.0], &[3.0, 4.0])
        .annotation(crate::ChartAnnotation::point("peak", "Peak", 2.0, 4.0).color(0xff0000))
        .annotation(crate::ChartAnnotation::x_value("release", "Release", 1.5))
        .annotation_summary();
    assert_eq!(line_annotations.chart_type, "line");
    assert_eq!(line_annotations.annotation_count(), 2);
    assert_eq!(line_annotations.annotations[0].target.kind(), "point");
    assert!(line_annotations.description.contains("2 annotations"));

    let scatter_annotations = crate::scatter(&[1.0, 2.0], &[3.0, 4.0])
        .annotations(vec![
            crate::ChartAnnotation::point("outlier", "Outlier", 2.0, 4.0).series_index(0),
            crate::ChartAnnotation::y_value("threshold", "Threshold", 3.5),
        ])
        .annotation_summary();
    assert_eq!(scatter_annotations.chart_type, "scatter");
    assert_eq!(scatter_annotations.annotation_count(), 2);
    assert_eq!(scatter_annotations.annotations[0].series_index, Some(0));
    assert!(scatter_annotations.description.contains("point"));

    let bar_annotations = crate::bar(&["Q1", "Q2"], &[10.0, 20.0])
        .annotation(crate::ChartAnnotation::category(
            "best",
            "Best quarter",
            "Q2",
        ))
        .annotation_summary();
    assert_eq!(bar_annotations.chart_type, "bar");
    assert_eq!(bar_annotations.annotation_count(), 1);
    assert_eq!(bar_annotations.annotations[0].target.kind(), "category");
}

// extent_padded tests
#[test]
fn test_extent_padded_normal_values() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let (min, max) = extent_padded(&values, 0.05);
    // Min should be 1.0 - 0.05 * 4.0 = 0.8
    // Max should be 5.0 + 0.05 * 4.0 = 5.2
    assert!((min - 0.8).abs() < 1e-10);
    assert!((max - 5.2).abs() < 1e-10);
}

#[test]
fn test_extent_padded_iter_matches_slice_version() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let (min_slice, max_slice) = extent_padded(&values, 0.05);
    let (min_iter, max_iter) = extent_padded_iter(values.into_iter(), 0.05);
    assert_eq!(min_slice, min_iter);
    assert_eq!(max_slice, max_iter);
}

#[test]
fn test_extent_log_padded_iter_uses_positive_multiplicative_padding() {
    let (min, max) = extent_log_padded_iter([0.001, 1.0].into_iter(), 0.05);

    assert!(min > 0.0);
    assert!((min - (0.001 / 1.05)).abs() < f64::EPSILON);
    assert!((max - 1.05).abs() < f64::EPSILON);
}

#[test]
fn test_extent_padded_constant_values() {
    let values = vec![5.0, 5.0, 5.0, 5.0];
    let (min, max) = extent_padded(&values, 0.05);
    // Range is 0, so padding should be 1.0
    assert!((min - 4.0).abs() < 1e-10);
    assert!((max - 6.0).abs() < 1e-10);
}

#[test]
fn test_extent_padded_single_value() {
    let values = vec![3.0];
    let (min, max) = extent_padded(&values, 0.1);
    // Range is 0, so padding should be 1.0
    assert!((min - 2.0).abs() < 1e-10);
    assert!((max - 4.0).abs() < 1e-10);
}

// validate_data_array tests
#[test]
fn test_validate_data_array_valid() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert!(validate_data_array(&values, "test").is_ok());
}

#[test]
fn test_validate_data_array_empty() {
    let values: Vec<f64> = vec![];
    let result = validate_data_array(&values, "test");
    assert!(matches!(
        result,
        Err(ChartError::EmptyData { field: "test" })
    ));
}

#[test]
fn test_validate_data_array_nan() {
    let values = vec![1.0, 2.0, f64::NAN, 4.0];
    let result = validate_data_array(&values, "test");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "test",
            reason: "contains NaN or Infinity"
        })
    ));
}

#[test]
fn test_validate_data_array_infinity() {
    let values = vec![1.0, f64::INFINITY, 3.0];
    let result = validate_data_array(&values, "test");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "test",
            reason: "contains NaN or Infinity"
        })
    ));
}

#[test]
fn test_validate_data_array_neg_infinity() {
    let values = vec![1.0, 2.0, f64::NEG_INFINITY];
    let result = validate_data_array(&values, "test");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "test",
            reason: "contains NaN or Infinity"
        })
    ));
}

// validate_data_length tests
#[test]
fn test_validate_data_length_matching() {
    assert!(validate_data_length(5, 5, "x", "y").is_ok());
}

#[test]
fn test_validate_data_length_mismatched() {
    let result = validate_data_length(3, 5, "x", "y");
    assert!(matches!(
        result,
        Err(ChartError::DataLengthMismatch {
            x_field: "x",
            y_field: "y",
            x_len: 3,
            y_len: 5,
        })
    ));
}

#[test]
fn test_validate_data_length_zero() {
    assert!(validate_data_length(0, 0, "x", "y").is_ok());
}

// validate_dimensions tests
#[test]
fn test_validate_dimensions_valid() {
    assert!(validate_dimensions(600.0, 400.0).is_ok());
}

#[test]
fn test_validate_dimensions_zero_width() {
    let result = validate_dimensions(0.0, 400.0);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "width",
            value: 0.0
        })
    ));
}

#[test]
fn test_validate_dimensions_negative_width() {
    let result = validate_dimensions(-100.0, 400.0);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "width",
            value: -100.0
        })
    ));
}

#[test]
fn test_validate_dimensions_zero_height() {
    let result = validate_dimensions(600.0, 0.0);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "height",
            value: 0.0
        })
    ));
}

#[test]
fn test_validate_dimensions_negative_height() {
    let result = validate_dimensions(600.0, -50.0);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "height",
            value: -50.0
        })
    ));
}

#[test]
fn test_validate_dimensions_nan_width() {
    let result = validate_dimensions(f32::NAN, 400.0);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "width",
            value: _,
        })
    ));
}

#[test]
fn test_validate_dimensions_nan_height() {
    let result = validate_dimensions(600.0, f32::NAN);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "height",
            value: _,
        })
    ));
}

#[test]
fn test_validate_dimensions_infinite_width() {
    let result = validate_dimensions(f32::INFINITY, 400.0);
    assert!(matches!(
        result,
        Err(ChartError::InvalidDimension {
            field: "width",
            value: _,
        })
    ));
}

// validate_grid_dimensions tests
#[test]
fn test_validate_grid_dimensions_valid() {
    let z = vec![1.0; 12]; // 3x4 grid
    assert!(validate_grid_dimensions(&z, 3, 4).is_ok());
}

#[test]
fn test_validate_grid_dimensions_mismatch() {
    let z = vec![1.0; 10];
    let result = validate_grid_dimensions(&z, 3, 4);
    assert!(matches!(
        result,
        Err(ChartError::GridDimensionMismatch {
            z_len: 10,
            width: 3,
            height: 4,
            expected: 12,
        })
    ));
}

// validate_monotonic tests
#[test]
fn test_validate_monotonic_valid() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert!(validate_monotonic(&values, "x").is_ok());
}

#[test]
fn test_validate_monotonic_not_increasing() {
    let values = vec![1.0, 2.0, 2.0, 4.0]; // 2.0 == 2.0
    let result = validate_monotonic(&values, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "must be strictly monotonically increasing"
        })
    ));
}

#[test]
fn test_validate_monotonic_decreasing() {
    let values = vec![1.0, 3.0, 2.0, 4.0];
    let result = validate_monotonic(&values, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "must be strictly monotonically increasing"
        })
    ));
}

// validate_positive tests
#[test]
fn test_validate_positive_valid() {
    let values = vec![0.1, 1.0, 10.0, 100.0];
    assert!(validate_positive(&values, "x").is_ok());
}

#[test]
fn test_validate_positive_with_zero() {
    let values = vec![0.0, 1.0, 2.0];
    let result = validate_positive(&values, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "contains non-positive values for log scale"
        })
    ));
}

#[test]
fn test_validate_positive_with_negative() {
    let values = vec![-1.0, 1.0, 2.0];
    let result = validate_positive(&values, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "contains non-positive values for log scale"
        })
    ));
}

#[test]
fn test_validate_range_valid() {
    assert!(validate_range(1.0, 10.0, "x").is_ok());
}

#[test]
fn test_validate_range_reversed() {
    let result = validate_range(10.0, 1.0, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "range min must be less than max"
        })
    ));
}

#[test]
fn test_validate_range_equal() {
    let result = validate_range(5.0, 5.0, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "range min must be less than max"
        })
    ));
}

#[test]
fn test_validate_range_log_valid() {
    assert!(validate_range_log(1.0, 10.0, "x").is_ok());
}

#[test]
fn test_validate_range_log_negative_min() {
    let result = validate_range_log(-1.0, 10.0, "x");
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "log scale range must be strictly positive"
        })
    ));
}

#[test]
fn test_validate_range_log_zero_max() {
    let result = validate_range_log(1.0, 0.0, "x");
    // Range reversal is caught first
    assert!(matches!(
        result,
        Err(ChartError::InvalidData {
            field: "x",
            reason: "range min must be less than max"
        })
    ));
}
