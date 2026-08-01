use super::types::PrepareProfile;
use super::types::PreparedText;
use super::types::PreparedTextWithSegments;
use super::types::measure_analysis;
use crate::analysis::{AnalysisProfile, WhiteSpaceMode, analyze_text};
use crate::measurement::{EngineProfile, TextMeasure};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy)]
pub struct PrepareOptions {
    pub white_space: WhiteSpaceMode,
}

impl Default for PrepareOptions {
    fn default() -> Self {
        Self {
            white_space: WhiteSpaceMode::Normal,
        }
    }
}

/// Bounds for text preparation work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextBudget {
    pub max_input_bytes: usize,
    pub max_graphemes: usize,
    pub max_segments: usize,
}

impl TextBudget {
    pub const fn new(max_input_bytes: usize, max_graphemes: usize, max_segments: usize) -> Self {
        Self {
            max_input_bytes,
            max_graphemes,
            max_segments,
        }
    }

    pub const fn unlimited() -> Self {
        Self::new(usize::MAX, usize::MAX, usize::MAX)
    }
}

impl Default for TextBudget {
    fn default() -> Self {
        Self::new(16 * 1024 * 1024, 4_000_000, 1_000_000)
    }
}

/// Failure returned by bounded text preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPrepareError {
    InputBytesExceeded { limit: usize, actual: usize },
    GraphemesExceeded { limit: usize, actual: usize },
    SegmentsExceeded { limit: usize, actual: usize },
    Cancelled,
}

impl std::fmt::Display for TextPrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputBytesExceeded { limit, actual } => {
                write!(f, "text input exceeds byte budget: {actual} > {limit}")
            }
            Self::GraphemesExceeded { limit, actual } => {
                write!(f, "text input exceeds grapheme budget: {actual} > {limit}")
            }
            Self::SegmentsExceeded { limit, actual } => {
                write!(
                    f,
                    "text analysis exceeds segment budget: {actual} > {limit}"
                )
            }
            Self::Cancelled => write!(f, "text preparation cancelled"),
        }
    }
}

impl std::error::Error for TextPrepareError {}

fn analyze_with_budget<F: FnMut() -> bool>(
    text: &str,
    profile: &EngineProfile,
    options: &PrepareOptions,
    budget: TextBudget,
    cancelled: &mut F,
) -> Result<crate::analysis::TextAnalysis, TextPrepareError> {
    if cancelled() {
        return Err(TextPrepareError::Cancelled);
    }
    if text.len() > budget.max_input_bytes {
        return Err(TextPrepareError::InputBytesExceeded {
            limit: budget.max_input_bytes,
            actual: text.len(),
        });
    }
    let graphemes = text
        .graphemes(true)
        .take(budget.max_graphemes.saturating_add(1))
        .count();
    if graphemes > budget.max_graphemes {
        return Err(TextPrepareError::GraphemesExceeded {
            limit: budget.max_graphemes,
            actual: graphemes,
        });
    }

    let analysis_profile = AnalysisProfile {
        carry_cjk_after_closing_quote: profile.carry_cjk_after_closing_quote,
    };
    let analysis = analyze_text(text, &analysis_profile, options.white_space);
    if cancelled() {
        return Err(TextPrepareError::Cancelled);
    }
    if analysis.len() > budget.max_segments {
        return Err(TextPrepareError::SegmentsExceeded {
            limit: budget.max_segments,
            actual: analysis.len(),
        });
    }
    Ok(analysis)
}

/// Prepare text for layout. Segments and measures the text once.
///
/// The returned `PreparedText` can be used with [`crate::layout()`] for fast line counting.
pub fn prepare(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
) -> PreparedText {
    let analysis_profile = AnalysisProfile {
        carry_cjk_after_closing_quote: profile.carry_cjk_after_closing_quote,
    };
    let analysis = analyze_text(text, &analysis_profile, options.white_space);
    let (core, _) = measure_analysis(&analysis, measure, profile, false);
    PreparedText { core }
}

/// Prepare text for layout, including segment strings for rich output.
///
/// Use with [`crate::layout_with_lines()`], [`crate::layout_next_line()`], or
/// [`crate::walk_line_ranges()`].
pub fn prepare_with_segments(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
) -> PreparedTextWithSegments {
    let analysis_profile = AnalysisProfile {
        carry_cjk_after_closing_quote: profile.carry_cjk_after_closing_quote,
    };
    let analysis = analyze_text(text, &analysis_profile, options.white_space);
    let (core, segments) = measure_analysis(&analysis, measure, profile, true);
    PreparedTextWithSegments {
        core,
        segments: segments.unwrap_or_default(),
    }
}

/// Prepare text under explicit byte, grapheme, segment, and cancellation limits.
pub fn prepare_with_budget(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
    budget: TextBudget,
) -> Result<PreparedText, TextPrepareError> {
    prepare_with_budget_and_cancel(text, measure, profile, options, budget, || false)
}

/// Bounded variant of [`prepare_with_segments`].
pub fn prepare_with_segments_with_budget(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
    budget: TextBudget,
) -> Result<PreparedTextWithSegments, TextPrepareError> {
    prepare_with_segments_with_budget_and_cancel(text, measure, profile, options, budget, || false)
}

/// Bounded preparation with a cooperative cancellation callback.
pub fn prepare_with_budget_and_cancel<F: FnMut() -> bool>(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
    budget: TextBudget,
    mut cancelled: F,
) -> Result<PreparedText, TextPrepareError> {
    let analysis = analyze_with_budget(text, profile, options, budget, &mut cancelled)?;
    let (core, _) = measure_analysis(&analysis, measure, profile, false);
    if cancelled() {
        return Err(TextPrepareError::Cancelled);
    }
    Ok(PreparedText { core })
}

/// Bounded `prepare_with_segments` with a cooperative cancellation callback.
pub fn prepare_with_segments_with_budget_and_cancel<F: FnMut() -> bool>(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
    budget: TextBudget,
    mut cancelled: F,
) -> Result<PreparedTextWithSegments, TextPrepareError> {
    let analysis = analyze_with_budget(text, profile, options, budget, &mut cancelled)?;
    let (core, segments) = measure_analysis(&analysis, measure, profile, true);
    if cancelled() {
        return Err(TextPrepareError::Cancelled);
    }
    Ok(PreparedTextWithSegments {
        core,
        segments: segments.unwrap_or_default(),
    })
}

/// Profile the prepare phase (for diagnostics).
pub fn profile_prepare(
    text: &str,
    measure: &dyn TextMeasure,
    profile: &EngineProfile,
    options: &PrepareOptions,
) -> PrepareProfile {
    let analysis_profile = AnalysisProfile {
        carry_cjk_after_closing_quote: profile.carry_cjk_after_closing_quote,
    };
    let analysis = analyze_text(text, &analysis_profile, options.white_space);
    let analysis_segments = analysis.len();
    let (core, _) = measure_analysis(&analysis, measure, profile, false);

    let breakable_segments = core.breakable_widths.iter().filter(|w| w.is_some()).count();

    PrepareProfile {
        analysis_segments,
        prepared_segments: core.widths.len(),
        breakable_segments,
    }
}

#[cfg(test)]
mod tests {
    use super::{PrepareOptions, TextBudget, TextPrepareError, prepare_with_budget};
    use crate::{EngineProfile, TextMeasure};

    struct Measure;

    impl TextMeasure for Measure {
        fn measure_width(&self, text: &str) -> f64 {
            text.len() as f64
        }
    }

    #[test]
    fn bounded_prepare_rejects_large_input() {
        let error = prepare_with_budget(
            "hello",
            &Measure,
            &EngineProfile::default(),
            &PrepareOptions::default(),
            TextBudget::new(4, 100, 100),
        )
        .unwrap_err();
        assert!(matches!(error, TextPrepareError::InputBytesExceeded { .. }));
    }

    #[test]
    fn bounded_prepare_supports_cancellation() {
        let mut checks = 0;
        let error = super::prepare_with_budget_and_cancel(
            "hello",
            &Measure,
            &EngineProfile::default(),
            &PrepareOptions::default(),
            TextBudget::unlimited(),
            || {
                checks += 1;
                checks > 1
            },
        )
        .unwrap_err();
        assert_eq!(error, TextPrepareError::Cancelled);
    }
}
