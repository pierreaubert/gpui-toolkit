use super::*;

#[test]
fn compute_bidi_levels_reuses_scratch() {
    let text = "Hello مرحبا world";

    // First call primes the thread-local scratch buffers.
    let _ = compute_bidi_levels(text);
    let cap_after_first = bidi_chars_scratch_capacity();
    assert!(cap_after_first >= text.chars().count());

    // Second call with the same text must reuse the allocated scratch rather
    // than grow or reallocate a fresh Vec<char>.
    let _ = compute_bidi_levels(text);
    let cap_after_second = bidi_chars_scratch_capacity();
    assert_eq!(
        cap_after_first, cap_after_second,
        "bidi char scratch buffer should be reused, not reallocated"
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
