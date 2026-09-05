use d3rs::contour::{Contour, ContourRing};
use d3rs::scale::LinearScale;
use d3rs::shape::{
    BarConfig, BarDatum, ContourConfig, HeatmapData, LineConfig, LinePoint, bar_chart_scene,
    contour_chart_scene, heatmap_chart_scene, line_chart_scene, line_scene_geometry,
};
use d3rs::vello2d::{ChartCmd, CpuRasterizer};

#[test]
fn line_and_bar_scene_builders_have_deterministic_cpu_output() {
    let x_scale = LinearScale::new().domain(0.0, 2.0).range(0.0, 80.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(40.0, 0.0);

    let line_config = LineConfig::new().show_points(true).point_radius(3.0);
    let geometry = line_scene_geometry(
        &x_scale,
        &y_scale,
        &[LinePoint::new(0.0, 2.0), LinePoint::new(2.0, 8.0)],
        &line_config,
    );
    let line = line_chart_scene(&geometry, &line_config, 80.0, 40.0);
    assert_eq!(line.len(), 2, "stroke plus batched point fill");

    let bars = bar_chart_scene(
        &x_scale,
        &y_scale,
        &[BarDatum::new("one", 3.0), BarDatum::new("two", 7.0)],
        &BarConfig::new().border_radius(3.0),
        80.0,
        40.0,
    );
    assert_eq!(bars.len(), 2, "one rounded fill command per bar");

    let mut rasterizer = CpuRasterizer::new(80, 40);
    let pixels = rasterizer.rasterize(&line, 80, 40, 1.0);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
}

#[test]
fn contour_scene_matches_legacy_jump_and_fill_rules() {
    let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
    let y_scale = LinearScale::new().domain(0.0, 1.0).range(100.0, 0.0);
    let contour = Contour {
        value: 0.5,
        coordinates: vec![ContourRing::new(vec![
            d3rs::shape::Point::new(0.0, 0.5),
            d3rs::shape::Point::new(0.05, 0.5),
            d3rs::shape::Point::new(0.10, 0.5),
            d3rs::shape::Point::new(0.95, 0.5),
            d3rs::shape::Point::new(1.0, 0.5),
            d3rs::shape::Point::new(0.0, 0.5),
        ])],
    };
    let config = ContourConfig::new()
        .fill(true)
        .smooth_strokes(true)
        .smoothing_iterations(1)
        .smoothing_max_deviation_px(100.0);

    let scene = contour_chart_scene(&[contour], &x_scale, &y_scale, &config, 100.0, 100.0);
    assert_eq!(scene.len(), 1, "jumped closed ring must not be filled");
    assert!(matches!(scene.commands()[0], ChartCmd::Stroke { .. }));
}

#[test]
fn heatmap_scene_ignores_nan_cells_and_rasterizes_valid_cells() {
    let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 32.0);
    let y_scale = LinearScale::new().domain(0.0, 1.0).range(32.0, 0.0);
    let data = HeatmapData::new(
        vec![0.0, 1.0],
        vec![0.0, 1.0],
        vec![0.0, f64::NAN, 1.0, 0.5],
    );
    let scene = heatmap_chart_scene(&data, &x_scale, &y_scale, &ContourConfig::new(), 32.0, 32.0);
    assert_eq!(scene.len(), 3);
    let mut rasterizer = CpuRasterizer::new(32, 32);
    assert!(
        rasterizer
            .rasterize(&scene, 32, 32, 1.0)
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0)
    );
}
