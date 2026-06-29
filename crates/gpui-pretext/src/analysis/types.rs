use super::merged_segmentation::MergedSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteSpaceMode {
    Normal,
    PreWrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentBreakKind {
    Text,
    Space,
    PreservedSpace,
    Tab,
    Glue,
    ZeroWidthBreak,
    SoftHyphen,
    HardBreak,
}

#[derive(Debug, Clone)]
pub struct AnalysisChunk {
    pub start_segment_index: usize,
    pub end_segment_index: usize,
    pub consumed_end_segment_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisProfile {
    pub carry_cjk_after_closing_quote: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WhiteSpaceProfile {
    pub(super) mode: WhiteSpaceMode,
    pub(super) preserve_ordinary_spaces: bool,
    pub(super) preserve_hard_breaks: bool,
}

pub(super) fn get_white_space_profile(ws: WhiteSpaceMode) -> WhiteSpaceProfile {
    match ws {
        WhiteSpaceMode::PreWrap => WhiteSpaceProfile {
            mode: ws,
            preserve_ordinary_spaces: true,
            preserve_hard_breaks: true,
        },
        WhiteSpaceMode::Normal => WhiteSpaceProfile {
            mode: ws,
            preserve_ordinary_spaces: false,
            preserve_hard_breaks: false,
        },
    }
}

pub(super) struct SegmentationPiece<'a> {
    pub(super) text: &'a str,
    pub(super) is_word_like: bool,
    pub(super) kind: SegmentBreakKind,
    pub(super) start: usize,
}

pub(super) fn classify_segment_break_char(
    ch: char,
    profile: &WhiteSpaceProfile,
) -> SegmentBreakKind {
    if profile.preserve_ordinary_spaces || profile.preserve_hard_breaks {
        if ch == ' ' {
            return SegmentBreakKind::PreservedSpace;
        }
        if ch == '\t' {
            return SegmentBreakKind::Tab;
        }
        if profile.preserve_hard_breaks && ch == '\n' {
            return SegmentBreakKind::HardBreak;
        }
    }
    if ch == ' ' {
        return SegmentBreakKind::Space;
    }
    if ch == '\u{00A0}' || ch == '\u{202F}' || ch == '\u{2060}' || ch == '\u{FEFF}' {
        return SegmentBreakKind::Glue;
    }
    if ch == '\u{200B}' {
        return SegmentBreakKind::ZeroWidthBreak;
    }
    if ch == '\u{00AD}' {
        return SegmentBreakKind::SoftHyphen;
    }
    SegmentBreakKind::Text
}

pub(super) fn compile_analysis_chunks(
    seg: &MergedSegmentation,
    ws_profile: &WhiteSpaceProfile,
) -> Vec<AnalysisChunk> {
    if seg.is_empty() {
        return Vec::new();
    }
    if !ws_profile.preserve_hard_breaks {
        return vec![AnalysisChunk {
            start_segment_index: 0,
            end_segment_index: seg.len(),
            consumed_end_segment_index: seg.len(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    for i in 0..seg.len() {
        if seg.kinds[i] != SegmentBreakKind::HardBreak {
            continue;
        }
        chunks.push(AnalysisChunk {
            start_segment_index: start,
            end_segment_index: i,
            consumed_end_segment_index: i + 1,
        });
        start = i + 1;
    }

    if start < seg.len() {
        chunks.push(AnalysisChunk {
            start_segment_index: start,
            end_segment_index: seg.len(),
            consumed_end_segment_index: seg.len(),
        });
    }

    chunks
}
