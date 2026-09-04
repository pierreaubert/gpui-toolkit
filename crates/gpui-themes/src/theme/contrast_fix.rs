use super::editor_theme::EditorTheme;
use super::misc::{Color, contrast_ratio};

/// Minimum contrast ratio for WCAG AA normal text.
pub const WCAG_AA_MIN_RATIO: f32 = 4.5;

/// One failing foreground/background pairing with a suggested fix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContrastIssue {
    /// Pair identifier, e.g. `"text_primary/background"`.
    pub pair: &'static str,
    /// Measured contrast ratio.
    pub ratio: f32,
    /// Nearest foreground color that passes WCAG AA against the background.
    pub suggested: Color,
}

/// Search lightness around `foreground` for the nearest color reaching
/// `WCAG_AA_MIN_RATIO` against `background`.
///
/// Scans both directions in HSL space and keeps the smallest lightness
/// delta that passes, so the suggestion stays visually close to the input.
/// Preserves the input alpha channel. Returns `None` when no lightness
/// adjustment (in 1% steps) reaches the target, e.g. mid-grey on mid-grey
/// can still fail at the extremes — although black/white always pass.
pub fn nearest_passing_color(foreground: Color, background: Color) -> Option<Color> {
    if contrast_ratio(foreground, background) >= WCAG_AA_MIN_RATIO {
        return Some(foreground);
    }
    let (h, s, l) = foreground.to_hsl();
    let mut best: Option<(f32, Color)> = None;
    let mut step = 1;
    while step <= 100 {
        let delta = step as f32 / 100.0;
        for candidate_l in [l + delta, l - delta] {
            if !(0.0..=1.0).contains(&candidate_l) {
                continue;
            }
            let mut candidate = Color::from_hsl(h, s, candidate_l);
            candidate.a = foreground.a;
            if contrast_ratio(candidate, background) >= WCAG_AA_MIN_RATIO
                && best.is_none_or(|(best_delta, _)| delta < best_delta)
            {
                best = Some((delta, candidate));
            }
        }
        if best.is_some() {
            break;
        }
        step += 1;
    }
    best.map(|(_, color)| color)
}

impl EditorTheme {
    /// All failing WCAG AA pairings checked by [`EditorTheme::validate_accessibility`],
    /// each with the nearest passing foreground suggestion.
    ///
    /// Returns an empty vec when the theme passes validation.
    pub fn accessibility_issues(&self) -> Vec<ContrastIssue> {
        let pairs: [(&'static str, Color, Color); 3] = [
            (
                "text_primary/background",
                self.text_primary,
                self.background,
            ),
            ("text_primary/surface", self.text_primary, self.surface),
            ("text_on_accent/accent", self.text_on_accent, self.accent),
        ];
        pairs
            .into_iter()
            .filter_map(|(pair, foreground, background)| {
                let ratio = contrast_ratio(foreground, background);
                if ratio >= WCAG_AA_MIN_RATIO {
                    return None;
                }
                nearest_passing_color(foreground, background).map(|suggested| ContrastIssue {
                    pair,
                    ratio,
                    suggested,
                })
            })
            .collect()
    }

    /// Suggested foreground fix for one `pair` id from [`Self::accessibility_issues`].
    /// Returns `None` for passing or unknown pairs.
    pub fn suggested_contrast_fix(&self, pair: &str) -> Option<Color> {
        self.accessibility_issues()
            .into_iter()
            .find(|issue| issue.pair == pair)
            .map(|issue| issue.suggested)
    }

    /// Return a copy with each failing foreground replaced by its nearest
    /// passing color. Pairs with no reachable fix are left unchanged.
    pub fn auto_fix_contrast(mut self) -> Self {
        for issue in self.accessibility_issues() {
            match issue.pair {
                "text_primary/background" | "text_primary/surface" => {
                    self.text_primary = issue.suggested;
                }
                "text_on_accent/accent" => {
                    self.text_on_accent = issue.suggested;
                }
                _ => {}
            }
        }
        self
    }

    /// One-line editor badge: `"WCAG AA: pass"` or `"WCAG AA: N issue(s)"`.
    pub fn accessibility_badge(&self) -> String {
        let count = self.accessibility_issues().len();
        if count == 0 {
            "WCAG AA: pass".to_string()
        } else {
            format!("WCAG AA: {count} issue(s)")
        }
    }
}
