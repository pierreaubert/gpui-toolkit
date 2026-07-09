//! Language and script support contract for release notes and QA tooling.

use crate::WhiteSpaceMode;

/// Schema version for [`LanguageSupportReport`].
pub const LANGUAGE_SUPPORT_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`LanguageSupportReport`].
pub const LANGUAGE_SUPPORT_REPORT_TYPE: &str = "gpui-pretext-language-support";

/// Schema version for [`LocaleGoldenReport`].
pub const LOCALE_GOLDEN_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`LocaleGoldenReport`].
pub const LOCALE_GOLDEN_REPORT_TYPE: &str = "gpui-pretext-locale-golden-cases";

/// Schema version for [`BenchmarkBaselineReport`].
pub const BENCHMARK_BASELINE_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`BenchmarkBaselineReport`].
pub const BENCHMARK_BASELINE_REPORT_TYPE: &str = "gpui-pretext-benchmark-baselines";

/// Current support status for a language or script capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSupportLevel {
    /// Implemented directly by gpui-pretext.
    Supported,
    /// Correctness depends on the caller's [`TextMeasure`](crate::TextMeasure)
    /// and rendering backend.
    BackendDependent,
    /// gpui-pretext has partial logic, but callers should document and test the
    /// limitation before claiming full language support.
    Limited,
}

impl LanguageSupportLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::BackendDependent => "backend-dependent",
            Self::Limited => "limited",
        }
    }
}

/// One language/script support note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageSupportNote {
    pub category: &'static str,
    pub level: LanguageSupportLevel,
    pub summary: &'static str,
    pub recommendation: &'static str,
}

/// Versioned language/script support report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageSupportReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub notes: &'static [LanguageSupportNote],
}

/// A deterministic text-layout golden case for locale/script release QA.
///
/// These cases pin gpui-pretext segmentation and line construction behavior.
/// They do not claim platform glyph shaping correctness; final shaping remains
/// the caller's [`TextMeasure`](crate::TextMeasure) responsibility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocaleGoldenCase {
    pub id: &'static str,
    pub locale: &'static str,
    pub category: &'static str,
    pub text: &'static str,
    pub white_space: WhiteSpaceMode,
    pub max_width: f64,
    pub line_height: f64,
    pub expected_lines: &'static [&'static str],
    pub note: &'static str,
}

impl LocaleGoldenCase {
    pub const fn expected_line_count(self) -> usize {
        self.expected_lines.len()
    }
}

/// Versioned locale/script golden-case report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocaleGoldenReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub cases: &'static [LocaleGoldenCase],
}

/// A Criterion benchmark or locale sample that must have release baseline data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkBaselineCase {
    pub id: &'static str,
    pub benchmark_id: &'static str,
    pub focus: &'static str,
    pub baseline_artifact: &'static str,
    pub comparator_artifact: &'static str,
    pub release_requirement: &'static str,
}

/// A platform text stack that release QA should compare against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformTextComparator {
    pub id: &'static str,
    pub platform: &'static str,
    pub backend: &'static str,
    pub artifact: &'static str,
    pub requirement: &'static str,
}

/// Versioned benchmark-baseline inventory for release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkBaselineReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub criterion_command: &'static str,
    pub baseline_policy: &'static str,
    pub cases: &'static [BenchmarkBaselineCase],
    pub comparators: &'static [PlatformTextComparator],
    pub locale_case_ids: &'static [&'static str],
}

impl LocaleGoldenReport {
    /// Render the golden-case inventory as Markdown for release notes or QA docs.
    pub fn to_markdown(self) -> String {
        let mut markdown = format!(
            "# gpui-pretext Locale Golden Cases\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\n\
             | Case | Locale | Category | Width | Lines | Note |\n\
             | --- | --- | --- | ---: | ---: | --- |\n",
            self.schema_version, self.report_type
        );

        for case in self.cases {
            markdown.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} |\n",
                case.id,
                case.locale,
                case.category,
                case.max_width,
                case.expected_line_count(),
                case.note
            ));
        }

        markdown
    }
}

impl BenchmarkBaselineReport {
    /// Render the benchmark-baseline inventory as Markdown for release notes or QA docs.
    pub fn to_markdown(self) -> String {
        let mut markdown = format!(
            "# gpui-pretext Benchmark Baselines\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - criterion_command: `{}`\n\
             - baseline_policy: {}\n\n\
             | Case | Benchmark | Focus | Baseline artifact | Comparator artifact |\n\
             | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.criterion_command, self.baseline_policy
        );

        for case in self.cases {
            markdown.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` | `{}` |\n",
                case.id,
                case.benchmark_id,
                case.focus,
                case.baseline_artifact,
                case.comparator_artifact
            ));
        }

        markdown.push_str(
            "\n| Comparator | Platform | Backend | Artifact | Requirement |\n\
             | --- | --- | --- | --- | --- |\n",
        );
        for comparator in self.comparators {
            markdown.push_str(&format!(
                "| `{}` | {} | {} | `{}` | {} |\n",
                comparator.id,
                comparator.platform,
                comparator.backend,
                comparator.artifact,
                comparator.requirement
            ));
        }

        markdown
    }

    /// Return true when the report includes every locale golden case id.
    pub fn covers_locale_golden_cases(self) -> bool {
        locale_golden_cases()
            .iter()
            .all(|case| self.locale_case_ids.contains(&case.id))
    }
}

impl LanguageSupportReport {
    /// Render the report as Markdown for release notes or generated QA docs.
    pub fn to_markdown(self) -> String {
        let mut markdown = format!(
            "# gpui-pretext Language and Script Support\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\n\
             | Category | Level | Summary | Recommendation |\n\
             | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type
        );

        for note in self.notes {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                note.category,
                note.level.as_str(),
                note.summary,
                note.recommendation
            ));
        }

        markdown
    }
}

const LANGUAGE_SUPPORT_NOTES: [LanguageSupportNote; 7] = [
    LanguageSupportNote {
        category: "Latin and whitespace-separated text",
        level: LanguageSupportLevel::Supported,
        summary: "Greedy and optimal line breaking support common UI text, spaces, hard breaks, tabs, and soft hyphens.",
        recommendation: "Release as supported for ordinary product UI copy when measured by a production TextMeasure implementation.",
    },
    LanguageSupportNote {
        category: "CJK line breaking",
        level: LanguageSupportLevel::Limited,
        summary: "CJK detection and grapheme fallback are implemented, including engine-profile tuning for closing-quote behavior.",
        recommendation: "Ship with explicit CJK QA samples and compare output against the target platform text renderer before claiming full locale coverage.",
    },
    LanguageSupportNote {
        category: "Emoji and grapheme clusters",
        level: LanguageSupportLevel::BackendDependent,
        summary: "Segments use Unicode grapheme boundaries, but width and glyph fallback are delegated to TextMeasure.",
        recommendation: "Use the same shaping stack for TextMeasure and final rendering, then test ZWJ emoji, skin-tone modifiers, and fallback fonts together.",
    },
    LanguageSupportNote {
        category: "RTL and bidi ordering",
        level: LanguageSupportLevel::Limited,
        summary: "The bidi module computes simplified embedding metadata for prepared segments, not a complete Unicode Bidirectional Algorithm replacement.",
        recommendation: "Validate Hebrew, Arabic, mixed-number, and mixed-direction samples in the host platform before public RTL claims.",
    },
    LanguageSupportNote {
        category: "Complex shaping scripts",
        level: LanguageSupportLevel::BackendDependent,
        summary: "Indic, Arabic, Thai, and similar shaping-sensitive scripts depend on the backend's shaper, cluster mapping, and line-break behavior.",
        recommendation: "Back TextMeasure with the production shaper and keep per-script golden layout cases for any supported locale.",
    },
    LanguageSupportNote {
        category: "Rich text and variable fonts",
        level: LanguageSupportLevel::BackendDependent,
        summary: "Rich text spans, accessibility runs, and variable-font metadata are represented, while actual font selection and glyph shaping stay in the backend.",
        recommendation: "Treat gpui-pretext spans as layout metadata and test backend font fallback, axis application, and accessibility bridge output separately.",
    },
    LanguageSupportNote {
        category: "Untrusted or very large text",
        level: LanguageSupportLevel::Limited,
        summary: "The crate uses safe Rust and reusable scratch buffers, but callers still own input-size policy and cancellation around expensive text blocks.",
        recommendation: "Apply product-level byte, paragraph, or work-unit limits when measuring untrusted documents or logs.",
    },
];

const LATIN_WRAP_LINES: &[&str] = &["alpha beta ", "gamma"];
const CJK_WRAP_LINES: &[&str] = &["你好世", "界再见"];
const EMOJI_CLUSTER_LINES: &[&str] = &["send 👩‍💻 ", "now"];
const ARABIC_PUNCTUATION_LINES: &[&str] = &["مرحبا؟ ", "عالم"];
const MYANMAR_GLUE_LINES: &[&str] = &["ကျ ", "စာ"];
const NARROW_NBSP_LINES: &[&str] = &["10\u{202F}000 ", "EUR"];
const PRE_WRAP_LINES: &[&str] = &["a\tb", "c"];

const LOCALE_GOLDEN_CASES: [LocaleGoldenCase; 7] = [
    LocaleGoldenCase {
        id: "latin-normal-wrap",
        locale: "en",
        category: "Latin and whitespace-separated text",
        text: "alpha beta gamma",
        white_space: WhiteSpaceMode::Normal,
        max_width: 110.0,
        line_height: 20.0,
        expected_lines: LATIN_WRAP_LINES,
        note: "pins ordinary word wrapping and trailing collapsible-space ownership",
    },
    LocaleGoldenCase {
        id: "cjk-grapheme-wrap",
        locale: "zh-Hans",
        category: "CJK line breaking",
        text: "你好世界再见",
        white_space: WhiteSpaceMode::Normal,
        max_width: 30.0,
        line_height: 20.0,
        expected_lines: CJK_WRAP_LINES,
        note: "pins grapheme-level CJK wrapping under deterministic measurement",
    },
    LocaleGoldenCase {
        id: "emoji-zwj-cluster",
        locale: "und-Zsye",
        category: "Emoji and grapheme clusters",
        text: "send 👩‍💻 now",
        white_space: WhiteSpaceMode::Normal,
        max_width: 70.0,
        line_height: 20.0,
        expected_lines: EMOJI_CLUSTER_LINES,
        note: "pins that ZWJ emoji stay in a single user-perceived cluster",
    },
    LocaleGoldenCase {
        id: "arabic-no-space-punctuation",
        locale: "ar",
        category: "RTL and bidi ordering",
        text: "مرحبا؟ عالم",
        white_space: WhiteSpaceMode::Normal,
        max_width: 70.0,
        line_height: 20.0,
        expected_lines: ARABIC_PUNCTUATION_LINES,
        note: "pins Arabic no-space punctuation merging before backend bidi shaping",
    },
    LocaleGoldenCase {
        id: "myanmar-medial-glue",
        locale: "my",
        category: "Complex shaping scripts",
        text: "ကျ စာ",
        white_space: WhiteSpaceMode::Normal,
        max_width: 30.0,
        line_height: 20.0,
        expected_lines: MYANMAR_GLUE_LINES,
        note: "pins Myanmar medial glue segmentation before backend shaping",
    },
    LocaleGoldenCase {
        id: "french-narrow-nbsp",
        locale: "fr",
        category: "Locale punctuation and glue",
        text: "10\u{202F}000 EUR",
        white_space: WhiteSpaceMode::Normal,
        max_width: 70.0,
        line_height: 20.0,
        expected_lines: NARROW_NBSP_LINES,
        note: "pins narrow no-break-space grouping as a glued numeric run",
    },
    LocaleGoldenCase {
        id: "prewrap-tabs-hardbreaks",
        locale: "en",
        category: "Whitespace preservation",
        text: "a\tb\nc",
        white_space: WhiteSpaceMode::PreWrap,
        max_width: 200.0,
        line_height: 20.0,
        expected_lines: PRE_WRAP_LINES,
        note: "pins preserved tab advance and hard-break line splitting",
    },
];

const BENCHMARK_BASELINE_CASES: [BenchmarkBaselineCase; 5] = [
    BenchmarkBaselineCase {
        id: "grapheme-prefix-widths",
        benchmark_id: "measurement/get_grapheme_prefix_widths",
        focus: "prefix width vector generation for Unicode grapheme clusters",
        baseline_artifact: "artifacts/gpui-pretext/criterion/measurement-get_grapheme_prefix_widths.json",
        comparator_artifact: "artifacts/gpui-pretext/platform/measurement-prefix-widths.json",
        release_requirement: "Record current Criterion output and compare prefix-width behavior against platform-backed TextMeasure samples.",
    },
    BenchmarkBaselineCase {
        id: "width-cache-hit",
        benchmark_id: "measurement/get_width_cache_hit",
        focus: "hot cache lookup cost for repeated segment measurement",
        baseline_artifact: "artifacts/gpui-pretext/criterion/measurement-get_width_cache_hit.json",
        comparator_artifact: "artifacts/gpui-pretext/platform/measurement-cache-hit.json",
        release_requirement: "Keep cache-hit latency within the release baseline tolerance after platform TextMeasure integration.",
    },
    BenchmarkBaselineCase {
        id: "grapheme-widths-cache-hit",
        benchmark_id: "measurement/get_grapheme_widths_cache_hit",
        focus: "cached per-grapheme width lookup for cluster-aware layout",
        baseline_artifact: "artifacts/gpui-pretext/criterion/measurement-get_grapheme_widths_cache_hit.json",
        comparator_artifact: "artifacts/gpui-pretext/platform/measurement-grapheme-widths.json",
        release_requirement: "Record cache behavior for locale samples that rely on grapheme cluster boundaries.",
    },
    BenchmarkBaselineCase {
        id: "optimal-line-layout",
        benchmark_id: "layout/layout_optimal",
        focus: "Knuth-Plass optimal line breaking over prepared text",
        baseline_artifact: "artifacts/gpui-pretext/criterion/layout-layout_optimal.json",
        comparator_artifact: "artifacts/gpui-pretext/platform/layout-optimal.json",
        release_requirement: "Compare line counts and timing against platform text layout for the locale golden corpus.",
    },
    BenchmarkBaselineCase {
        id: "line-construction",
        benchmark_id: "layout/layout_with_lines",
        focus: "line object construction over prepared text",
        baseline_artifact: "artifacts/gpui-pretext/criterion/layout-layout_with_lines.json",
        comparator_artifact: "artifacts/gpui-pretext/platform/layout-with-lines.json",
        release_requirement: "Record line construction timing and verify line text parity for release locale samples.",
    },
];

const PLATFORM_TEXT_COMPARATORS: [PlatformTextComparator; 4] = [
    PlatformTextComparator {
        id: "core-text-macos",
        platform: "macOS",
        backend: "Core Text / GPUI production TextMeasure",
        artifact: "artifacts/gpui-pretext/platform/core-text-macos.json",
        requirement: "Run on the release macOS runner before claiming desktop text-layout parity.",
    },
    PlatformTextComparator {
        id: "core-text-ios",
        platform: "iOS",
        backend: "Core Text / UIKit text measurement",
        artifact: "artifacts/gpui-pretext/platform/core-text-ios.json",
        requirement: "Run on simulator or device for mobile text-layout release notes.",
    },
    PlatformTextComparator {
        id: "directwrite-windows",
        platform: "Windows",
        backend: "DirectWrite",
        artifact: "artifacts/gpui-pretext/platform/directwrite-windows.json",
        requirement: "Run on a native Windows runner before claiming Windows text-layout parity.",
    },
    PlatformTextComparator {
        id: "pango-linux",
        platform: "Linux",
        backend: "Pango/fontconfig or the selected GPUI Linux text backend",
        artifact: "artifacts/gpui-pretext/platform/pango-linux.json",
        requirement: "Run on the Linux release runner when Linux packages are in scope.",
    },
];

const BENCHMARK_BASELINE_LOCALE_CASE_IDS: &[&str] = &[
    "latin-normal-wrap",
    "cjk-grapheme-wrap",
    "emoji-zwj-cluster",
    "arabic-no-space-punctuation",
    "myanmar-medial-glue",
    "french-narrow-nbsp",
    "prewrap-tabs-hardbreaks",
];

/// Return the current language/script support report.
pub fn language_support_report() -> LanguageSupportReport {
    LanguageSupportReport {
        schema_version: LANGUAGE_SUPPORT_SCHEMA_VERSION,
        report_type: LANGUAGE_SUPPORT_REPORT_TYPE,
        notes: &LANGUAGE_SUPPORT_NOTES,
    }
}

/// Return the current support notes without allocating.
pub fn language_support_notes() -> &'static [LanguageSupportNote] {
    &LANGUAGE_SUPPORT_NOTES
}

/// Return the current locale/script golden-case report.
pub fn locale_golden_report() -> LocaleGoldenReport {
    LocaleGoldenReport {
        schema_version: LOCALE_GOLDEN_SCHEMA_VERSION,
        report_type: LOCALE_GOLDEN_REPORT_TYPE,
        cases: &LOCALE_GOLDEN_CASES,
    }
}

/// Return the current locale/script golden cases without allocating.
pub fn locale_golden_cases() -> &'static [LocaleGoldenCase] {
    &LOCALE_GOLDEN_CASES
}

/// Return the current benchmark baseline inventory.
pub fn benchmark_baseline_report() -> BenchmarkBaselineReport {
    BenchmarkBaselineReport {
        schema_version: BENCHMARK_BASELINE_SCHEMA_VERSION,
        report_type: BENCHMARK_BASELINE_REPORT_TYPE,
        criterion_command: "cargo bench -p gpui-pretext --bench layout_temporaries",
        baseline_policy: "Attach Criterion JSON plus platform comparator output for every release-candidate tag.",
        cases: &BENCHMARK_BASELINE_CASES,
        comparators: &PLATFORM_TEXT_COMPARATORS,
        locale_case_ids: BENCHMARK_BASELINE_LOCALE_CASE_IDS,
    }
}

/// Return the current benchmark baseline cases without allocating.
pub fn benchmark_baseline_cases() -> &'static [BenchmarkBaselineCase] {
    &BENCHMARK_BASELINE_CASES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EngineProfile, PrepareOptions, TextMeasure, layout_with_lines, prepare_with_segments,
    };
    use unicode_segmentation::UnicodeSegmentation;

    struct GoldenMeasure;

    impl TextMeasure for GoldenMeasure {
        fn measure_width(&self, text: &str) -> f64 {
            text.graphemes(true).count() as f64 * 10.0
        }
    }

    #[test]
    fn language_support_report_has_stable_contract() {
        let report = language_support_report();

        assert_eq!(report.schema_version, LANGUAGE_SUPPORT_SCHEMA_VERSION);
        assert_eq!(report.report_type, LANGUAGE_SUPPORT_REPORT_TYPE);
        assert!(!report.notes.is_empty());

        for note in report.notes {
            assert!(!note.category.is_empty());
            assert!(!note.level.as_str().is_empty());
            assert!(!note.summary.is_empty());
            assert!(!note.recommendation.is_empty());
        }
    }

    #[test]
    fn language_support_report_covers_release_limitations() {
        let categories = language_support_report()
            .notes
            .iter()
            .map(|note| note.category)
            .collect::<Vec<_>>();

        assert!(categories.iter().any(|category| category.contains("CJK")));
        assert!(categories.iter().any(|category| category.contains("Emoji")));
        assert!(categories.iter().any(|category| category.contains("RTL")));
        assert!(
            categories
                .iter()
                .any(|category| category.contains("Complex shaping"))
        );
        assert!(
            language_support_report()
                .notes
                .iter()
                .any(|note| note.level == LanguageSupportLevel::BackendDependent)
        );
    }

    #[test]
    fn language_support_markdown_names_schema_and_categories() {
        let markdown = language_support_report().to_markdown();

        assert!(markdown.contains("gpui-pretext Language and Script Support"));
        assert!(markdown.contains("schema_version: 1"));
        assert!(markdown.contains(LANGUAGE_SUPPORT_REPORT_TYPE));
        assert!(markdown.contains("CJK line breaking"));
        assert!(markdown.contains("RTL and bidi ordering"));
    }

    #[test]
    fn locale_golden_report_has_stable_contract() {
        let report = locale_golden_report();

        assert_eq!(report.schema_version, LOCALE_GOLDEN_SCHEMA_VERSION);
        assert_eq!(report.report_type, LOCALE_GOLDEN_REPORT_TYPE);
        assert!(report.cases.len() >= 7);

        for case in report.cases {
            assert!(!case.id.is_empty());
            assert!(!case.locale.is_empty());
            assert!(!case.category.is_empty());
            assert!(!case.text.is_empty());
            assert!(case.max_width.is_finite() && case.max_width > 0.0);
            assert!(case.line_height.is_finite() && case.line_height > 0.0);
            assert!(!case.expected_lines.is_empty());
            assert!(!case.note.is_empty());
        }
    }

    #[test]
    fn locale_golden_cases_match_current_layout_output() {
        let measure = GoldenMeasure;
        let profile = EngineProfile::default();

        for case in locale_golden_cases() {
            let options = PrepareOptions {
                white_space: case.white_space,
            };
            let prepared = prepare_with_segments(case.text, &measure, &profile, &options);
            let layout = layout_with_lines(&prepared, case.max_width, case.line_height, &profile);
            let actual = layout
                .lines
                .iter()
                .map(|line| line.text.as_ref())
                .collect::<Vec<_>>();

            assert_eq!(
                actual, case.expected_lines,
                "locale golden case `{}` changed",
                case.id
            );
            assert_eq!(layout.line_count, case.expected_line_count(), "{}", case.id);
            assert_eq!(
                layout.height,
                case.line_height * case.expected_line_count() as f64,
                "{}",
                case.id
            );
        }
    }

    #[test]
    fn locale_golden_markdown_names_cases_and_schema() {
        let markdown = locale_golden_report().to_markdown();

        assert!(markdown.contains("gpui-pretext Locale Golden Cases"));
        assert!(markdown.contains("schema_version: 1"));
        assert!(markdown.contains(LOCALE_GOLDEN_REPORT_TYPE));
        assert!(markdown.contains("cjk-grapheme-wrap"));
        assert!(markdown.contains("emoji-zwj-cluster"));
        assert!(markdown.contains("arabic-no-space-punctuation"));
        assert!(markdown.contains("french-narrow-nbsp"));
    }

    #[test]
    fn benchmark_baseline_report_has_stable_contract() {
        let report = benchmark_baseline_report();

        assert_eq!(report.schema_version, BENCHMARK_BASELINE_SCHEMA_VERSION);
        assert_eq!(report.report_type, BENCHMARK_BASELINE_REPORT_TYPE);
        assert_eq!(
            report.criterion_command,
            "cargo bench -p gpui-pretext --bench layout_temporaries"
        );
        assert!(report.covers_locale_golden_cases());
        assert!(report.cases.len() >= 5);
        assert!(report.comparators.len() >= 4);

        for case in report.cases {
            assert!(!case.id.is_empty());
            assert!(!case.benchmark_id.is_empty());
            assert!(!case.focus.is_empty());
            assert!(case.baseline_artifact.ends_with(".json"));
            assert!(case.comparator_artifact.ends_with(".json"));
            assert!(!case.release_requirement.is_empty());
        }

        for comparator in report.comparators {
            assert!(!comparator.id.is_empty());
            assert!(!comparator.platform.is_empty());
            assert!(!comparator.backend.is_empty());
            assert!(comparator.artifact.ends_with(".json"));
            assert!(!comparator.requirement.is_empty());
        }
    }

    #[test]
    fn benchmark_baseline_report_names_current_criterion_benches() {
        let ids = benchmark_baseline_cases()
            .iter()
            .map(|case| case.benchmark_id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"measurement/get_grapheme_prefix_widths"));
        assert!(ids.contains(&"measurement/get_width_cache_hit"));
        assert!(ids.contains(&"measurement/get_grapheme_widths_cache_hit"));
        assert!(ids.contains(&"layout/layout_optimal"));
        assert!(ids.contains(&"layout/layout_with_lines"));
    }

    #[test]
    fn benchmark_baseline_markdown_names_platform_comparators() {
        let markdown = benchmark_baseline_report().to_markdown();

        assert!(markdown.contains("gpui-pretext Benchmark Baselines"));
        assert!(markdown.contains("schema_version: 1"));
        assert!(markdown.contains(BENCHMARK_BASELINE_REPORT_TYPE));
        assert!(markdown.contains("layout/layout_with_lines"));
        assert!(markdown.contains("core-text-macos"));
        assert!(markdown.contains("directwrite-windows"));
        assert!(markdown.contains("pango-linux"));
    }
}
