use std::collections::HashSet;

pub const AUDIO_AUTOMATION_PATTERN_SCHEMA_VERSION: u32 = 1;
pub const AUDIO_AUTOMATION_PATTERN_REPORT_TYPE: &str = "gpui-audio-kit-automation-patterns";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioAutomationPatternStatus {
    Implemented,
}

impl AudioAutomationPatternStatus {
    pub fn label(self) -> &'static str {
        match self {
            AudioAutomationPatternStatus::Implemented => "implemented",
        }
    }

    pub fn is_release_blocking(self) -> bool {
        !matches!(self, AudioAutomationPatternStatus::Implemented)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioAutomationPattern {
    pub id: &'static str,
    pub parameter_family: &'static str,
    pub recommended_control: &'static str,
    pub scale: &'static str,
    pub automation_sources: &'static [&'static str],
    pub expected_interactions: &'static [&'static str],
    pub accessibility_summary_contract: &'static str,
    pub release_evidence: &'static str,
    pub status: AudioAutomationPatternStatus,
}

impl AudioAutomationPattern {
    pub fn is_release_blocking(&self) -> bool {
        self.status.is_release_blocking()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAutomationPatternReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub patterns: &'static [AudioAutomationPattern],
}

impl AudioAutomationPatternReport {
    pub fn blocking_entries(&self) -> Vec<&'static AudioAutomationPattern> {
        self.patterns
            .iter()
            .filter(|pattern| pattern.is_release_blocking())
            .collect()
    }

    pub fn pattern(&self, id: &str) -> Option<&'static AudioAutomationPattern> {
        self.patterns.iter().find(|pattern| pattern.id == id)
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# gpui-audio-kit Automation Patterns\n\n");
        output.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n\n",
            self.schema_version, self.report_type
        ));
        output.push_str(
            "| id | status | parameter family | control | scale | interactions | accessibility contract | evidence |\n",
        );
        output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- |\n");
        for pattern in self.patterns {
            output.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` | {} | {} | {} | {} |\n",
                pattern.id,
                pattern.status.label(),
                pattern.parameter_family,
                pattern.recommended_control,
                pattern.scale,
                pattern.expected_interactions.join(", "),
                pattern.accessibility_summary_contract,
                pattern.release_evidence,
            ));
        }
        output
    }

    pub fn validate_unique_ids(&self) -> bool {
        let mut ids = HashSet::new();
        self.patterns.iter().all(|pattern| ids.insert(pattern.id))
    }
}

pub fn audio_automation_pattern_report() -> AudioAutomationPatternReport {
    AudioAutomationPatternReport {
        schema_version: AUDIO_AUTOMATION_PATTERN_SCHEMA_VERSION,
        report_type: AUDIO_AUTOMATION_PATTERN_REPORT_TYPE,
        patterns: AUDIO_AUTOMATION_PATTERNS,
    }
}

pub const AUDIO_AUTOMATION_PATTERNS: &[AudioAutomationPattern] = &[
    AudioAutomationPattern {
        id: "continuous-gain",
        parameter_family: "continuous gain or mix amount",
        recommended_control: "Potentiometer",
        scale: "linear",
        automation_sources: &["drag", "scroll", "keyboard", "host automation"],
        expected_interactions: &[
            "fine scroll",
            "arrow/page keyboard steps",
            "double-click reset",
            "selected parameter state",
        ],
        accessibility_summary_contract: "AudioAccessibilitySummary reports slider role, value/range text, normalized value, unit, and selected/disabled state.",
        release_evidence: "Potentiometer accessibility and interaction builder tests",
        status: AudioAutomationPatternStatus::Implemented,
    },
    AudioAutomationPattern {
        id: "log-frequency",
        parameter_family: "frequency, crossover, or time-constant controls",
        recommended_control: "Potentiometer",
        scale: "logarithmic",
        automation_sources: &["drag", "scroll", "keyboard", "host automation"],
        expected_interactions: &[
            "logarithmic normalized position",
            "Hz value formatting",
            "keyboard shortcut label",
        ],
        accessibility_summary_contract: "AudioAccessibilitySummary carries Logarithmic scale, finite Hz value text, and normalized position.",
        release_evidence: "Potentiometer logarithmic normalization and accessibility tests",
        status: AudioAutomationPatternStatus::Implemented,
    },
    AudioAutomationPattern {
        id: "channel-fader",
        parameter_family: "channel fader, send, or macro strip",
        recommended_control: "VerticalSlider",
        scale: "linear or logarithmic",
        automation_sources: &["drag", "scroll", "keyboard", "host automation"],
        expected_interactions: &[
            "drag-start callback",
            "on-change callback",
            "double-click or Escape reset",
            "peak marker",
        ],
        accessibility_summary_contract: "AudioAccessibilitySummary reports value/range text, scale, selected/disabled state, and optional peak value.",
        release_evidence: "VerticalSlider accessibility, normalization, and builder tests",
        status: AudioAutomationPatternStatus::Implemented,
    },
    AudioAutomationPattern {
        id: "monitor-volume",
        parameter_family: "monitor volume with mute state",
        recommended_control: "VolumeKnob",
        scale: "linear percentage",
        automation_sources: &[
            "drag",
            "scroll",
            "keyboard",
            "mute toggle",
            "host automation",
        ],
        expected_interactions: &[
            "percentage value text",
            "mute toggling",
            "media-key intent",
            "effective muted value",
        ],
        accessibility_summary_contract: "AudioAccessibilitySummary reports slider role, percent value text, normalized value, and muted state.",
        release_evidence: "VolumeKnob accessibility and builder tests",
        status: AudioAutomationPatternStatus::Implemented,
    },
    AudioAutomationPattern {
        id: "meter-feedback",
        parameter_family: "read-only level, peak, clipping, or gain-reduction feedback",
        recommended_control: "LevelMeterElement or horizontal meter bar",
        scale: "dB or normalized ratio",
        automation_sources: &["meter stream", "analysis stream", "host telemetry"],
        expected_interactions: &[
            "read-only progress semantics",
            "peak value",
            "clipping description",
            "threshold coloring",
        ],
        accessibility_summary_contract: "AudioAccessibilitySummary reports progressbar role, current/range dB values, peak value, and clipping text.",
        release_evidence: "Level meter and horizontal meter accessibility tests",
        status: AudioAutomationPatternStatus::Implemented,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_pattern_report_has_stable_contract() {
        let report = audio_automation_pattern_report();

        assert_eq!(
            report.schema_version,
            AUDIO_AUTOMATION_PATTERN_SCHEMA_VERSION
        );
        assert_eq!(report.report_type, AUDIO_AUTOMATION_PATTERN_REPORT_TYPE);
        assert!(report.validate_unique_ids());
        assert!(report.blocking_entries().is_empty());
        assert!(report.patterns.len() >= 5);
    }

    #[test]
    fn automation_patterns_cover_core_audio_parameter_families() {
        let report = audio_automation_pattern_report();

        for id in [
            "continuous-gain",
            "log-frequency",
            "channel-fader",
            "monitor-volume",
            "meter-feedback",
        ] {
            let pattern = report.pattern(id).expect("pattern should be present");
            assert_eq!(pattern.status, AudioAutomationPatternStatus::Implemented);
            assert!(!pattern.expected_interactions.is_empty());
            assert!(
                pattern
                    .accessibility_summary_contract
                    .contains("AudioAccessibilitySummary")
            );
        }

        assert_eq!(
            report.pattern("log-frequency").unwrap().scale,
            "logarithmic"
        );
        assert!(
            report
                .pattern("monitor-volume")
                .unwrap()
                .expected_interactions
                .contains(&"mute toggling")
        );
    }

    #[test]
    fn automation_pattern_markdown_is_release_note_ready() {
        let markdown = audio_automation_pattern_report().to_markdown();

        assert!(markdown.contains(AUDIO_AUTOMATION_PATTERN_REPORT_TYPE));
        assert!(markdown.contains("continuous-gain"));
        assert!(markdown.contains("log-frequency"));
        assert!(markdown.contains("meter-feedback"));
        assert!(markdown.contains("AudioAccessibilitySummary"));
    }
}
