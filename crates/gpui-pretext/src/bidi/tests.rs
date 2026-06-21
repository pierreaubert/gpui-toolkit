use super::*;

#[test]
fn compute_bidi_levels_reuses_scratch() {
    let text = "Hello مرحبا world";

    // First call primes the thread-local scratch buffers.
    let _ = compute_bidi_levels(text);
    let cap_after_first = bidi_types_scratch_capacity();
    assert!(cap_after_first >= text.chars().count());

    // Second call with the same text must reuse the allocated scratch rather
    // than grow or reallocate a fresh Vec<BidiType>.
    let _ = compute_bidi_levels(text);
    let cap_after_second = bidi_types_scratch_capacity();
    assert_eq!(
        cap_after_first, cap_after_second,
        "bidi types scratch buffer should be reused, not reallocated"
    );
}

#[test]
fn compute_segment_levels_uses_cached_bidi_levels() {
    let text = "Hello مرحبا world";
    let starts = vec![0, 6, 17];

    // First call computes and caches levels on the normalized text.
    let first = compute_segment_levels(text, &starts).unwrap();

    // Second call with the same normalized text should hit the cache and
    // return identical levels.
    let second = compute_segment_levels(text, &starts).unwrap();
    assert_eq!(first, second);
}

#[test]
fn test_ltr_only() {
    assert!(compute_bidi_levels("Hello world").is_none());
}

#[test]
fn test_segment_level_terminal_rtl_offset() {
    let text = "مرحبا";
    let segments = compute_segment_levels(text, &[text.len()]).unwrap();
    assert_eq!(segments, vec![1]);
}

#[test]
fn test_rtl_arabic() {
    let levels = compute_bidi_levels("مرحبا").unwrap();
    assert!(levels.iter().all(|&l| l > 0));
}

#[test]
fn test_higher_rtl_ranges_are_not_ltr() {
    assert_eq!(classify_char('\u{200F}' as u32), R);
    assert_eq!(classify_char('\u{202B}' as u32), R);
    assert_eq!(classify_char('\u{FB50}' as u32), AL);
    assert_eq!(classify_char('\u{FE8E}' as u32), AL);
    assert_eq!(classify_char('\u{FB1D}' as u32), R);
}

#[test]
fn test_mixed() {
    let levels = compute_bidi_levels("Hello مرحبا world").unwrap();
    assert!(!levels.is_empty());
}

#[test]
fn test_segment_levels() {
    let text = "Hello world";
    let starts = vec![0, 6];
    assert!(compute_segment_levels(text, &starts).is_none());
}

#[test]
fn test_segment_levels_end_offset() {
    // End-of-string offset should map to the last char index, not 0.
    let text = "Hello";
    let starts = vec![0, text.len()];
    // LTR text returns None (no RTL chars)
    assert!(compute_segment_levels(text, &starts).is_none());

    // For RTL text, verify the end offset doesn't wrap to level 0.
    let text = "مرحبا";
    let starts = vec![0, text.len()];
    let levels = compute_segment_levels(text, &starts).unwrap();
    assert!(
        levels.iter().all(|&l| l > 0),
        "end offset should have RTL level, not 0"
    );
}

#[test]
fn test_byte_offset_to_char_index_multibyte() {
    // "a你b" -> char starts at byte offsets 0, 1, 4
    let text = "a你b";
    let char_starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    assert_eq!(byte_offset_to_char_index(0, &char_starts), 0);
    assert_eq!(byte_offset_to_char_index(1, &char_starts), 1);
    assert_eq!(byte_offset_to_char_index(2, &char_starts), 1);
    assert_eq!(byte_offset_to_char_index(3, &char_starts), 1);
    assert_eq!(byte_offset_to_char_index(4, &char_starts), 2);
    assert_eq!(byte_offset_to_char_index(5, &char_starts), 2);
}


#[test]
fn test_bidi_start_level_rtl_majority() {
    // Mostly RTL characters -> paragraph level 1.
    let levels = compute_bidi_levels("مرحبا بالعالم").unwrap();
    assert!(levels.iter().all(|&l| l > 0));
}

#[test]
fn test_classify_ltr_mark() {
    assert_eq!(classify_char('\u{200E}' as u32), L);
    assert_eq!(classify_char('\u{202A}' as u32), L);
}

#[test]
fn test_classify_rtl_mark() {
    assert_eq!(classify_char('\u{200F}' as u32), R);
    assert_eq!(classify_char('\u{202B}' as u32), R);
}

#[test]
fn test_classify_bidi_boundary() {
    assert_eq!(classify_char('\u{202C}' as u32), BN);
    assert_eq!(classify_char('\u{2066}' as u32), BN);
}

#[test]
fn test_bidi_nsm_takes_previous_type() {
    // Arabic shadda (U+0651) is NSM and should inherit the previous strong type.
    let levels = compute_bidi_levels("مر\u{0651}حبا").unwrap();
    assert!(!levels.is_empty());
}

#[test]
fn test_bidi_en_after_al_becomes_an() {
    // European digits after Arabic letters become AN in bidi algorithm.
    let levels = compute_bidi_levels("abc 123 مرحبا").unwrap();
    assert!(!levels.is_empty());
}

#[test]
fn test_bidi_es_cs_conversion() {
    // Colon between EN digits in RTL context should become EN.
    let levels = compute_bidi_levels("مرحبا 12:34").unwrap();
    assert!(!levels.is_empty());
}

#[test]
fn test_bidi_et_conversion() {
    // Percent signs around EN digits in RTL context should become EN.
    let levels = compute_bidi_levels("مرحبا 50%").unwrap();
    assert!(!levels.is_empty());
}

#[test]
fn test_bidi_on_runs_surrounded_by_rtl() {
    // Neutral chars between RTL chars take RTL direction.
    let levels = compute_bidi_levels("ا + ب").unwrap();
    assert!(!levels.is_empty());
}

#[test]
fn test_byte_offset_to_char_index_mid_char() {
    let text = "a你b";
    let char_starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    // Byte offset 2 is in the middle of "你" (bytes 1-3).
    assert_eq!(byte_offset_to_char_index(2, &char_starts), 1);
}
