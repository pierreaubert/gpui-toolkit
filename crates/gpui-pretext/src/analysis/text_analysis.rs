use super::types::AnalysisChunk;
use super::types::SegmentBreakKind;
use std::ops::Index;
use std::ops::Range;
use std::sync::Arc;

/// Range-backed analyzed segments sharing the normalized text allocation.
#[derive(Debug, Clone, Default)]
pub struct SegmentTexts {
    storage: Arc<str>,
    ranges: Vec<Range<usize>>,
}

impl SegmentTexts {
    pub(super) fn new(storage: Arc<str>, texts: &[String], starts: &[usize]) -> Self {
        debug_assert_eq!(texts.len(), starts.len());
        let ranges = texts
            .iter()
            .zip(starts)
            .map(|(text, start)| {
                let range = *start..start.saturating_add(text.len());
                debug_assert_eq!(storage.get(range.clone()), Some(text.as_str()));
                range
            })
            .collect();
        Self { storage, ranges }
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.ranges.iter().map(|range| &self.storage[range.clone()])
    }
}

impl Index<usize> for SegmentTexts {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        &self.storage[self.ranges[index].clone()]
    }
}

#[derive(Debug, Clone)]
pub struct TextAnalysis {
    pub normalized: Arc<str>,
    pub grapheme_count: usize,
    pub chunks: Vec<AnalysisChunk>,
    pub texts: SegmentTexts,
    pub is_word_like: Vec<bool>,
    pub kinds: Vec<SegmentBreakKind>,
    pub starts: Vec<usize>,
}

impl TextAnalysis {
    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }
}
