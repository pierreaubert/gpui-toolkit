use d3rs::gpu2d::primitives::LineBatch;

#[test]
fn zero_width_line_is_skipped() {
    let mut batch = LineBatch::new();

    batch.add_line(0.0, 0.0, 10.0, 0.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

    assert!(batch.is_empty());
}

#[test]
fn transparent_line_is_skipped() {
    let mut batch = LineBatch::new();

    batch.add_line(0.0, 0.0, 10.0, 0.0, 2.0, [1.0, 1.0, 1.0, 0.0]);

    assert!(batch.is_empty());
}

#[test]
fn positive_width_line_expands_with_aa_coverage() {
    let mut batch = LineBatch::new();

    batch.add_line(0.0, 0.0, 10.0, 0.0, 2.0, [1.0, 1.0, 1.0, 1.0]);

    assert_eq!(batch.vertices.len(), 4);
    assert_eq!(batch.indices.len(), 6);
    assert!((batch.vertices[0].half_width - 1.0).abs() < f32::EPSILON);
    assert!((batch.vertices[0].half_length - 5.0).abs() < f32::EPSILON);
    assert!((batch.vertices[0].local[0] + 6.0).abs() < f32::EPSILON);
    assert!((batch.vertices[0].local[1] - 2.0).abs() < f32::EPSILON);
    assert!((batch.vertices[0].position[0] + 1.0).abs() < f32::EPSILON);
    assert!((batch.vertices[0].position[1] - 2.0).abs() < f32::EPSILON);
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn try_global_does_not_poison_on_missing_adapter() {
    // Just exercises that try_global returns without panicking machinery;
    // on CI without GPU it must return Err, with GPU Ok.
    let _ = d3rs::gpu2d::Gpu2DContext::try_global();
}
