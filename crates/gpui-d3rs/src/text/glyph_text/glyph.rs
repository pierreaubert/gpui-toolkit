pub(super) fn glyph_advance(metrics: &fontdue::Metrics) -> f32 {
    metrics.advance_width.ceil()
}

pub(super) fn glyph_left(cursor_x: f32, metrics: &fontdue::Metrics) -> f32 {
    (cursor_x + metrics.bounds.xmin).floor()
}

pub(super) fn glyph_top(metrics: &fontdue::Metrics) -> f32 {
    (-metrics.bounds.height - metrics.bounds.ymin).floor()
}
