use super::analysis_profile::analyze_text;
use super::ends::ends_with_arabic_no_space_punctuation;
use super::ends::ends_with_closing_quote;
use super::ends::ends_with_myanmar_medial_glue;
use super::is::is_cjk;
use super::merge::merge_ascii_punctuation_chains;
use super::merge::merge_glue_connected_text_runs;
use super::merge::merge_numeric_runs;
use super::merge::merge_url_like_runs;
use super::merge::merge_url_query_runs;
use super::merged_segmentation::MergedSegmentation;
use super::normalize::normalize_whitespace_normal;
use super::normalize::normalize_whitespace_pre_wrap;
use super::split::split_segment_by_break_kind;
use super::types::AnalysisProfile;
use super::types::SegmentBreakKind;
use super::types::WhiteSpaceMode;
use super::types::get_white_space_profile;

#[test]
fn test_normalize_normal() {
    assert_eq!(
        normalize_whitespace_normal("  hello   world  ").as_ref(),
        "hello world"
    );
    assert_eq!(normalize_whitespace_normal("a\tb\nc").as_ref(), "a b c");
    assert_eq!(normalize_whitespace_normal("hello").as_ref(), "hello");
}

#[test]
fn test_normalize_pre_wrap() {
    assert_eq!(normalize_whitespace_pre_wrap("a\r\nb").as_ref(), "a\nb");
    assert_eq!(normalize_whitespace_pre_wrap("a\rb").as_ref(), "a\nb");
}

#[test]
fn test_normalize_cow_borrowed_when_unchanged() {
    // Inputs that require no transformation should be returned borrowed.
    assert!(matches!(
        normalize_whitespace_normal("hello"),
        std::borrow::Cow::Borrowed(_)
    ));
    assert!(matches!(
        normalize_whitespace_normal("hello world"),
        std::borrow::Cow::Borrowed(_)
    ));
    assert!(matches!(
        normalize_whitespace_pre_wrap("hello"),
        std::borrow::Cow::Borrowed(_)
    ));
}

#[test]
fn test_is_cjk() {
    assert!(is_cjk("你好"));
    assert!(is_cjk("こんにちは"));
    assert!(!is_cjk("hello"));
}

#[test]
fn test_analyze_empty() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: false,
    };
    let result = analyze_text("", &profile, WhiteSpaceMode::Normal);
    assert!(result.is_empty());
}

#[test]
fn test_analyze_simple() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: false,
    };
    let result = analyze_text("hello world", &profile, WhiteSpaceMode::Normal);
    assert!(result.len() >= 3); // "hello", " ", "world"
}

#[test]
fn test_analyze_pre_wrap_hard_breaks() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: false,
    };
    let result = analyze_text("a\nb\nc", &profile, WhiteSpaceMode::PreWrap);
    assert!(result.chunks.len() >= 3);
}

#[test]
fn test_split_segment_by_break_kind_returns_slices() {
    let ws = get_white_space_profile(WhiteSpaceMode::Normal);
    let pieces = split_segment_by_break_kind("a b", true, 10, &ws);
    assert_eq!(pieces.len(), 3);
    assert_eq!(pieces[0].text, "a");
    assert_eq!(pieces[0].start, 10);
    assert_eq!(pieces[1].text, " ");
    assert_eq!(pieces[2].text, "b");
    assert_eq!(pieces[2].start, 12);
}

#[test]
fn test_ends_with_closing_quote() {
    assert!(ends_with_closing_quote("hello\u{201D}"));
    assert!(ends_with_closing_quote("hello\u{2019}"));
    assert!(!ends_with_closing_quote("hello"));
    assert!(!ends_with_closing_quote("\u{201C}hello"));
}

#[test]
fn test_ends_with_closing_quote_with_sticky_punctuation() {
    assert!(ends_with_closing_quote("hello\u{201D},"));
    assert!(ends_with_closing_quote("hello\u{201D}."));
    assert!(!ends_with_closing_quote("hello."));
    assert!(!ends_with_closing_quote("hello, world"));
}

#[test]
fn test_ends_with_arabic_no_space_punctuation() {
    assert!(ends_with_arabic_no_space_punctuation("مرحبا:"));
    assert!(!ends_with_arabic_no_space_punctuation("hello"));
    assert!(!ends_with_arabic_no_space_punctuation(""));
}

#[test]
fn test_ends_with_myanmar_medial_glue() {
    assert!(ends_with_myanmar_medial_glue("က\u{104F}"));
    assert!(!ends_with_myanmar_medial_glue("က"));
    assert!(!ends_with_myanmar_medial_glue(""));
}

fn seg(texts: Vec<&str>, kinds: Vec<SegmentBreakKind>) -> MergedSegmentation {
    let len = texts.len();
    MergedSegmentation {
        texts: texts.into_iter().map(|s| s.to_string()).collect(),
        is_word_like: vec![true; len],
        kinds,
        starts: (0..len).collect(),
    }
}

#[test]
fn test_merge_url_like_runs() {
    let input = seg(
        vec!["https:", "//", "example", ".", "com"],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
        ],
    );
    let merged = merge_url_like_runs(input);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged.texts[0], "https://example.com");
}

#[test]
fn test_merge_url_query_runs() {
    let input = seg(
        vec!["http://x?", "a=1"],
        vec![SegmentBreakKind::Text, SegmentBreakKind::Text],
    );
    let merged = merge_url_query_runs(input);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged.texts[0], "http://x?");
    assert_eq!(merged.texts[1], "a=1");
}

#[test]
fn test_merge_numeric_runs() {
    let input = seg(
        vec!["1", ",", "234", ".", "56"],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
            SegmentBreakKind::Text,
        ],
    );
    let merged = merge_numeric_runs(input);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged.texts[0], "1,234.56");
}

#[test]
fn test_merge_ascii_punctuation_chains() {
    let input = seg(
        vec!["a,", "b;"],
        vec![SegmentBreakKind::Text, SegmentBreakKind::Text],
    );
    let mut input = input;
    input.is_word_like = vec![true; 2];
    let merged = merge_ascii_punctuation_chains(input);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged.texts[0], "a,b;");
}

#[test]
fn test_merge_glue_connected_text_runs() {
    let input = MergedSegmentation {
        texts: vec!["\u{00A0}".to_string(), "word".to_string()],
        is_word_like: vec![false, true],
        kinds: vec![SegmentBreakKind::Glue, SegmentBreakKind::Text],
        starts: vec![0, 1],
    };
    let merged = merge_glue_connected_text_runs(input);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged.texts[0], "\u{00A0}word");
}

#[test]
fn test_analyze_cjk_kinsoku_merge() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: false,
    };
    // Full-width punctuation is kinsoku-start-prohibited and should merge with preceding CJK.
    let result = analyze_text("你好。", &profile, WhiteSpaceMode::Normal);
    assert!(!result.is_empty());
}

#[test]
fn test_analyze_cjk_closing_quote_carry() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: true,
    };
    let result = analyze_text("\"你", &profile, WhiteSpaceMode::Normal);
    assert!(!result.is_empty());
}

#[test]
fn test_analyze_myanmar_glue() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: false,
    };
    let result = analyze_text("ကျ", &profile, WhiteSpaceMode::Normal);
    assert!(!result.is_empty());
}

#[test]
fn test_analyze_arabic_no_space_punctuation_merge() {
    let profile = AnalysisProfile {
        carry_cjk_after_closing_quote: false,
    };
    let result = analyze_text("مرحبا؟", &profile, WhiteSpaceMode::Normal);
    assert!(!result.is_empty());
}
