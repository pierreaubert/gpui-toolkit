use super::ends::ends_with_arabic_no_space_punctuation;
use super::ends::ends_with_closing_quote;
use super::ends::ends_with_myanmar_medial_glue;
use super::is::is_cjk;
use super::is::is_cjk_line_start_prohibited_segment;
use super::is::is_escaped_quote_cluster_segment;
use super::is::is_forward_sticky_cluster_segment;
use super::is::is_left_sticky_punctuation_segment;
use super::is::is_repeated_single_char_run;
use super::merge::merge_ascii_punctuation_chains;
use super::merge::merge_glue_connected_text_runs;
use super::merge::merge_numeric_runs;
use super::merge::merge_url_like_runs;
use super::merge::merge_url_query_runs;
use super::merged_segmentation::MergedSegmentation;
use super::merged_segmentation::carry_trailing_forward_sticky_across_cjk_boundary;
use super::merged_segmentation::compact;
use super::misc::contains_arabic_script;
use super::normalize::normalize_whitespace_normal;
use super::normalize::normalize_whitespace_pre_wrap;
use super::segment::segment_is_word_like;
use super::split::split_hyphenated_numeric_runs;
use super::split::split_leading_space_and_marks;
use super::split::split_segment_by_break_kind;
use super::text_analysis::TextAnalysis;
use super::types::AnalysisProfile;
use super::types::SegmentBreakKind;
use super::types::WhiteSpaceMode;
use super::types::WhiteSpaceProfile;
use super::types::compile_analysis_chunks;
use super::types::get_white_space_profile;
/// Text segmentation and normalization, ported from chenglou/pretext.
///
/// Handles whitespace normalization, word boundary segmentation via
/// `unicode-segmentation`, segment classification by break kind,
/// URL/numeric/punctuation merging, and CJK/Arabic/Myanmar special rules.
use unicode_segmentation::UnicodeSegmentation;

fn build_merged_segmentation(
    normalized: &str,
    profile: &AnalysisProfile,
    ws_profile: &WhiteSpaceProfile,
) -> MergedSegmentation {
    let mut merged_texts: Vec<String> = Vec::new();
    let mut merged_word_like: Vec<bool> = Vec::new();
    let mut merged_kinds: Vec<SegmentBreakKind> = Vec::new();
    let mut merged_starts: Vec<usize> = Vec::new();

    // Use unicode-segmentation for word boundary detection
    for (seg_byte_start, segment) in normalized.split_word_bound_indices() {
        let is_wl = segment_is_word_like(segment);
        for piece in split_segment_by_break_kind(segment, is_wl, seg_byte_start, ws_profile) {
            let is_text = piece.kind == SegmentBreakKind::Text;
            let merged_len = merged_texts.len();

            // CJK closing quote carry (engine-specific)
            if profile.carry_cjk_after_closing_quote
                && is_text
                && merged_len > 0
                && merged_kinds[merged_len - 1] == SegmentBreakKind::Text
                && is_cjk(piece.text)
                && is_cjk(&merged_texts[merged_len - 1])
                && ends_with_closing_quote(&merged_texts[merged_len - 1])
            {
                merged_texts[merged_len - 1].push_str(piece.text);
                merged_word_like[merged_len - 1] =
                    merged_word_like[merged_len - 1] || piece.is_word_like;
                continue;
            }

            // Kinsoku start prohibition
            if is_text
                && merged_len > 0
                && merged_kinds[merged_len - 1] == SegmentBreakKind::Text
                && is_cjk_line_start_prohibited_segment(piece.text)
                && is_cjk(&merged_texts[merged_len - 1])
            {
                merged_texts[merged_len - 1].push_str(piece.text);
                merged_word_like[merged_len - 1] =
                    merged_word_like[merged_len - 1] || piece.is_word_like;
                continue;
            }

            // Myanmar medial glue
            if is_text
                && merged_len > 0
                && merged_kinds[merged_len - 1] == SegmentBreakKind::Text
                && ends_with_myanmar_medial_glue(&merged_texts[merged_len - 1])
            {
                merged_texts[merged_len - 1].push_str(piece.text);
                merged_word_like[merged_len - 1] =
                    merged_word_like[merged_len - 1] || piece.is_word_like;
                continue;
            }

            // Arabic no-space punctuation
            if is_text
                && piece.is_word_like
                && merged_len > 0
                && merged_kinds[merged_len - 1] == SegmentBreakKind::Text
                && contains_arabic_script(piece.text)
                && ends_with_arabic_no_space_punctuation(&merged_texts[merged_len - 1])
            {
                merged_texts[merged_len - 1].push_str(piece.text);
                merged_word_like[merged_len - 1] = true;
                continue;
            }

            // Repeated single char
            if is_text
                && !piece.is_word_like
                && merged_len > 0
                && merged_kinds[merged_len - 1] == SegmentBreakKind::Text
                && piece.text.chars().count() == 1
            {
                let ch = piece.text.chars().next().unwrap();
                if ch != '-'
                    && ch != '\u{2014}'
                    && is_repeated_single_char_run(&merged_texts[merged_len - 1], ch)
                {
                    merged_texts[merged_len - 1].push_str(piece.text);
                    continue;
                }
            }

            // Left-sticky punctuation or trailing hyphen
            if is_text
                && !piece.is_word_like
                && merged_len > 0
                && merged_kinds[merged_len - 1] == SegmentBreakKind::Text
                && (is_left_sticky_punctuation_segment(piece.text)
                    || (piece.text == "-" && merged_word_like[merged_len - 1]))
            {
                merged_texts[merged_len - 1].push_str(piece.text);
                continue;
            }

            // Default: new segment
            merged_texts.push(piece.text.to_string());
            merged_word_like.push(piece.is_word_like);
            merged_kinds.push(piece.kind);
            merged_starts.push(piece.start);
        }
    }

    let merged_len = merged_texts.len();

    // Escaped quote cluster merge (forward pass)
    for i in 1..merged_len {
        if merged_kinds[i] == SegmentBreakKind::Text
            && !merged_word_like[i]
            && is_escaped_quote_cluster_segment(&merged_texts[i])
            && merged_kinds[i - 1] == SegmentBreakKind::Text
        {
            let text = std::mem::take(&mut merged_texts[i]);
            merged_texts[i - 1].push_str(&text);
            merged_word_like[i - 1] = merged_word_like[i - 1] || merged_word_like[i];
        }
    }

    // Forward-sticky cluster merge (backward pass)
    for i in (0..merged_len.saturating_sub(1)).rev() {
        if merged_kinds[i] == SegmentBreakKind::Text
            && !merged_word_like[i]
            && is_forward_sticky_cluster_segment(&merged_texts[i])
        {
            let mut j = i + 1;
            while j < merged_len && merged_texts[j].is_empty() {
                j += 1;
            }
            if j < merged_len && merged_kinds[j] == SegmentBreakKind::Text {
                let prefix = std::mem::take(&mut merged_texts[i]);
                merged_texts[j].insert_str(0, &prefix);
                merged_starts[j] = merged_starts[i];
            }
        }
    }

    let compacted = compact(merged_texts, merged_word_like, merged_kinds, merged_starts);
    let with_glue = merge_glue_connected_text_runs(compacted);
    let with_merged = carry_trailing_forward_sticky_across_cjk_boundary(
        merge_ascii_punctuation_chains(split_hyphenated_numeric_runs(merge_numeric_runs(
            merge_url_query_runs(merge_url_like_runs(with_glue)),
        ))),
    );

    // Arabic combining mark split
    let mut result = with_merged;
    for i in 0..result.len().saturating_sub(1) {
        if let Some((_space, _marks)) = split_leading_space_and_marks(&result.texts[i]) {
            let kind_i = result.kinds[i];
            if (kind_i != SegmentBreakKind::Space && kind_i != SegmentBreakKind::PreservedSpace)
                || result.kinds[i + 1] != SegmentBreakKind::Text
                || !contains_arabic_script(&result.texts[i + 1])
            {
                continue;
            }
            // Split " " + marks into the space segment and a separate marks string,
            // then prepend the marks to the following text segment without cloning
            // either string.
            let marks = result.texts[i].split_off(1);
            result.is_word_like[i] = false;
            result.kinds[i] = if kind_i == SegmentBreakKind::PreservedSpace {
                SegmentBreakKind::PreservedSpace
            } else {
                SegmentBreakKind::Space
            };
            result.texts[i + 1].insert_str(0, &marks);
            result.starts[i + 1] = result.starts[i] + 1; // 1 byte for space
        }
    }

    result
}

pub fn analyze_text(
    text: &str,
    profile: &AnalysisProfile,
    white_space: WhiteSpaceMode,
) -> TextAnalysis {
    let ws_profile = get_white_space_profile(white_space);
    let normalized = match ws_profile.mode {
        WhiteSpaceMode::PreWrap => normalize_whitespace_pre_wrap(text),
        WhiteSpaceMode::Normal => normalize_whitespace_normal(text),
    };

    if normalized.is_empty() {
        return TextAnalysis {
            normalized: normalized.into_owned(),
            chunks: Vec::new(),
            texts: Vec::new(),
            is_word_like: Vec::new(),
            kinds: Vec::new(),
            starts: Vec::new(),
        };
    }

    let normalized_str = normalized.as_ref();
    let segmentation = build_merged_segmentation(normalized_str, profile, &ws_profile);
    let chunks = compile_analysis_chunks(&segmentation, &ws_profile);

    TextAnalysis {
        normalized: normalized.into_owned(),
        chunks,
        texts: segmentation.texts,
        is_word_like: segmentation.is_word_like,
        kinds: segmentation.kinds,
        starts: segmentation.starts,
    }
}
