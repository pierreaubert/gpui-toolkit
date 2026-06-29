use std::borrow::Cow;

use super::fitness_class::FitnessClass;
use super::knuth_plass_params::KnuthPlassParams;
use super::misc::badness;
use super::types::{
    KPItem, LineBreakStrategy, PreparedLineBreakData, PreparedLineChunk, knuth_plass_chunk,
    knuth_plass_chunk_breaks_scratch_capacity,
};
use crate::analysis::SegmentBreakKind;

mod misc;

#[test]
fn test_kp_fitness_class_from_ratio() {
    assert_eq!(FitnessClass::from_ratio(-1.0), FitnessClass::Tight);
    assert_eq!(FitnessClass::from_ratio(-0.3), FitnessClass::Normal);
    assert_eq!(FitnessClass::from_ratio(0.0), FitnessClass::Normal);
    assert_eq!(FitnessClass::from_ratio(0.7), FitnessClass::Loose);
    assert_eq!(FitnessClass::from_ratio(1.5), FitnessClass::VeryLoose);
}

#[test]
fn test_kp_badness() {
    assert!((badness(0.0) - 0.0).abs() < 1e-6);
    assert!((badness(1.0) - 100.0).abs() < 1e-6);
    assert!((badness(-1.0) - 100.0).abs() < 1e-6);
    assert_eq!(badness(f64::INFINITY), 10000.0);
    // badness(0.5) = 100 * 0.125 = 12.5
    assert!((badness(0.5) - 12.5).abs() < 1e-6);
}

#[test]
fn test_kp_line_break_strategy_default() {
    assert_eq!(LineBreakStrategy::default(), LineBreakStrategy::Greedy);
}

#[test]
fn test_kp_params_default() {
    let params = KnuthPlassParams::default();
    assert_eq!(params.line_penalty, 0.0);
    assert_eq!(params.hyphen_penalty, 50.0);
    assert_eq!(params.flagged_demerits, 3000.0);
    assert_eq!(params.fitness_demerits, 10000.0);
    assert_eq!(params.tolerance, 2.0);
    assert!(params.looseness_recovery);
}

#[test]
fn knuth_plass_chunk_reuses_buffers() {
    let items: Vec<KPItem> = vec![
        KPItem {
            segment_index: 0,
            grapheme_index: 0,
            total_width: 0.0,
            total_stretch: 0.0,
            total_shrink: 0.0,
            penalty: f64::NEG_INFINITY,
            flagged: false,
            break_width: 0.0,
            glue_width: 0.0,
            glue_stretch: 0.0,
            glue_shrink: 0.0,
        },
        KPItem {
            segment_index: 1,
            grapheme_index: 0,
            total_width: 50.0,
            total_stretch: 25.0,
            total_shrink: 16.5,
            penalty: 0.0,
            flagged: false,
            break_width: 0.0,
            glue_width: 0.0,
            glue_stretch: 0.0,
            glue_shrink: 0.0,
        },
        KPItem {
            segment_index: 2,
            grapheme_index: 0,
            total_width: 100.0,
            total_stretch: 0.0,
            total_shrink: 0.0,
            penalty: f64::NEG_INFINITY,
            flagged: false,
            break_width: 0.0,
            glue_width: 0.0,
            glue_stretch: 0.0,
            glue_shrink: 0.0,
        },
    ];

    let params = KnuthPlassParams::default();
    let _ = knuth_plass_chunk(&items, 80.0, &params, params.tolerance);
    let cap_after_first = knuth_plass_chunk_breaks_scratch_capacity();

    let _ = knuth_plass_chunk(&items, 80.0, &params, params.tolerance);
    let cap_after_second = knuth_plass_chunk_breaks_scratch_capacity();

    assert_eq!(
        cap_after_first, cap_after_second,
        "knuth_plass_chunk breaks scratch buffer should be reused"
    );
}

#[test]
fn prepared_line_break_data_slice_avoids_owned_allocation() {
    let data = PreparedLineBreakData {
        widths: Cow::Borrowed(&[50.0, 50.0]),
        line_end_fit_advances: Cow::Borrowed(&[0.0, 0.0]),
        line_end_paint_advances: Cow::Borrowed(&[50.0, 50.0]),
        kinds: Cow::Borrowed(&[SegmentBreakKind::Text, SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Borrowed(&[None, None]),
        breakable_prefix_widths: Cow::Borrowed(&[None, None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Borrowed(&[PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 2,
            consumed_end_segment_index: 2,
        }]),
        start_segment: 0,
        end_segment: 2,
        kp_item_cache: None,
    };

    // Slicing exactly the existing chunk should keep the chunk list borrowed.
    let sliced = data.slice(0, 2, 2);
    assert!(
        matches!(sliced.chunks, Cow::Borrowed(_)),
        "slice aligned with existing chunk should return Cow::Borrowed"
    );
}
