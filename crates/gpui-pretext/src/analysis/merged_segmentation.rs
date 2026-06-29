use super::is::is_cjk;
use super::split::split_trailing_forward_sticky_cluster;
use super::types::SegmentBreakKind;

#[derive(Debug, Clone)]
pub struct MergedSegmentation {
    pub texts: Vec<String>,
    pub is_word_like: Vec<bool>,
    pub kinds: Vec<SegmentBreakKind>,
    pub starts: Vec<usize>,
}

impl MergedSegmentation {
    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }
}

pub(super) fn carry_trailing_forward_sticky_across_cjk_boundary(
    seg: MergedSegmentation,
) -> MergedSegmentation {
    let mut texts = seg.texts;
    let is_word_like = seg.is_word_like;
    let kinds = seg.kinds;
    let mut starts = seg.starts;
    let len = texts.len();

    for i in 0..len.saturating_sub(1) {
        if kinds[i] != SegmentBreakKind::Text || kinds[i + 1] != SegmentBreakKind::Text {
            continue;
        }
        if !is_cjk(&texts[i]) || !is_cjk(&texts[i + 1]) {
            continue;
        }
        let (head_len, tail) = match split_trailing_forward_sticky_cluster(&texts[i]) {
            Some((head, tail)) => (head.len(), tail.to_string()),
            None => continue,
        };
        starts[i + 1] = starts[i] + head_len;
        texts[i + 1].insert_str(0, &tail);
        texts[i].truncate(head_len);
    }

    MergedSegmentation {
        texts,
        is_word_like,
        kinds,
        starts,
    }
}

pub(super) fn compact(
    texts: Vec<String>,
    is_word_like: Vec<bool>,
    kinds: Vec<SegmentBreakKind>,
    starts: Vec<usize>,
) -> MergedSegmentation {
    let len = texts.len();
    let mut out_texts = Vec::with_capacity(len);
    let mut out_wl = Vec::with_capacity(len);
    let mut out_kinds = Vec::with_capacity(len);
    let mut out_starts = Vec::with_capacity(len);

    for (((text, wl), kind), start) in texts.into_iter().zip(is_word_like).zip(kinds).zip(starts) {
        if text.is_empty() {
            continue;
        }
        out_texts.push(text);
        out_wl.push(wl);
        out_kinds.push(kind);
        out_starts.push(start);
    }

    MergedSegmentation {
        texts: out_texts,
        is_word_like: out_wl,
        kinds: out_kinds,
        starts: out_starts,
    }
}
