use super::heatmap_data::HeatmapData;
use super::heatmap_element::HeatmapElement;
use crate::scale::LinearScale;
use gpui::prelude::*;
use gpui::*;

#[test]
fn paint_batches_cells_by_color() {
    // 4x3 grid: each row has a single value, so each row becomes one batched quad.
    let values = vec![
        1.0, 1.0, 1.0, 1.0, // row 0
        2.0, 2.0, 2.0, 2.0, // row 1
        3.0, 3.0, 3.0, 3.0, // row 2
    ];
    let data = HeatmapData::new(vec![0.0, 1.0, 2.0, 3.0], vec![0.0, 1.0, 2.0], values);
    let x_scale = LinearScale::new().domain(0.0, 3.0).range(0.0, 400.0);
    let y_scale = LinearScale::new().domain(0.0, 2.0).range(400.0, 0.0);

    let mut element = HeatmapElement::new(data, x_scale, y_scale);
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(400.0), px(400.0)));
    element.prepare_quads(bounds);

    let cell_count = 4 * 3;
    let quad_count = element.cached_quads().len();
    assert!(
        quad_count < cell_count,
        "expected fewer quads ({}) than cells ({})",
        quad_count,
        cell_count
    );
    assert_eq!(quad_count, 3, "each row should be merged into one quad");
}

#[test]
fn paint_caches_quads_for_unchanged_bounds() {
    let values = vec![1.0, 1.0, 2.0, 2.0];
    let data = HeatmapData::new(vec![0.0, 1.0], vec![0.0, 1.0], values);
    let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 200.0);
    let y_scale = LinearScale::new().domain(0.0, 1.0).range(200.0, 0.0);

    let mut element = HeatmapElement::new(data, x_scale, y_scale);
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(200.0), px(200.0)));

    element.prepare_quads(bounds);
    let quads1 = element.cached_quads().to_vec();
    let gen1 = element.cache_generation;

    element.prepare_quads(bounds);
    let quads2 = element.cached_quads().to_vec();
    let gen2 = element.cache_generation;

    assert_eq!(gen1, gen2);
    assert_eq!(quads1, quads2);
}
