use std::collections::{BTreeSet, HashSet};

pub const AUDIO_VISUAL_REGRESSION_SCHEMA_VERSION: u32 = 1;
pub const AUDIO_VISUAL_REGRESSION_REPORT_TYPE: &str = "gpui-audio-kit-visual-regression-manifest";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AudioVisualColorScheme {
    Light,
    Dark,
    HighContrast,
}

impl AudioVisualColorScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "high_contrast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioVisualViewport {
    pub id: &'static str,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioVisualStory {
    pub id: &'static str,
    pub label: &'static str,
    pub component: &'static str,
    /// Renderer shown by the ordinary showcase story.
    pub renderer: &'static str,
    /// Stable query overrides used by CPU/Legacy visual QA captures.
    pub renderer_qa_queries: &'static [&'static str],
    pub scenario: &'static str,
    pub release_focus: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioVisualCapture {
    pub id: String,
    pub story_id: &'static str,
    pub story_label: &'static str,
    pub component: &'static str,
    pub renderer: &'static str,
    pub renderer_qa_queries: &'static [&'static str],
    pub scenario: &'static str,
    pub viewport_id: &'static str,
    pub viewport_label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
    pub color_scheme: AudioVisualColorScheme,
    pub release_focus: &'static str,
    pub baseline_path: String,
    pub actual_path: String,
    pub diff_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioVisualRegressionManifest {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub stories: &'static [AudioVisualStory],
    pub viewports: &'static [AudioVisualViewport],
    pub color_schemes: &'static [AudioVisualColorScheme],
    pub captures: Vec<AudioVisualCapture>,
}

impl AudioVisualRegressionManifest {
    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    pub fn expected_capture_count(&self) -> usize {
        self.stories.len() * self.viewports.len() * self.color_schemes.len()
    }

    pub fn validate_unique_capture_ids(&self) -> bool {
        let mut ids = HashSet::new();
        self.captures
            .iter()
            .all(|capture| ids.insert(capture.id.as_str()))
    }

    pub fn components(&self) -> BTreeSet<&'static str> {
        self.stories.iter().map(|story| story.component).collect()
    }

    pub fn captures_for_component(&self, component: &str) -> Vec<&AudioVisualCapture> {
        self.captures
            .iter()
            .filter(|capture| capture.component == component)
            .collect()
    }

    pub fn to_markdown_table(&self) -> String {
        let mut output = String::from("# gpui-audio-kit Visual Regression Manifest\n\n");
        output.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n- stories: {}\n- viewports: {}\n- color_schemes: {}\n- captures: {}\n\n",
            self.schema_version,
            self.report_type,
            self.crate_name,
            self.crate_version,
            self.stories.len(),
            self.viewports.len(),
            self.color_schemes.len(),
            self.capture_count(),
        ));
        output.push_str("| capture | component | renderer | renderer QA | scenario | viewport | scheme | baseline | actual | diff | focus |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
        for capture in &self.captures {
            output.push_str(&format!(
                "| `{}` | {} | {} | `{}` | {} | {} {}x{}@{}x | {} | `{}` | `{}` | `{}` | {} |\n",
                capture.id,
                capture.component,
                capture.renderer,
                capture.renderer_qa_queries.join(","),
                capture.scenario,
                capture.viewport_label,
                capture.width,
                capture.height,
                capture.scale_factor,
                capture.color_scheme.as_str(),
                capture.baseline_path,
                capture.actual_path,
                capture.diff_path,
                capture.release_focus,
            ));
        }
        output
    }
}

pub fn audio_visual_regression_manifest() -> AudioVisualRegressionManifest {
    let mut captures = Vec::with_capacity(
        AUDIO_VISUAL_STORIES.len()
            * AUDIO_VISUAL_VIEWPORTS.len()
            * AUDIO_VISUAL_COLOR_SCHEMES.len(),
    );

    for story in AUDIO_VISUAL_STORIES {
        for viewport in AUDIO_VISUAL_VIEWPORTS {
            for &scheme in AUDIO_VISUAL_COLOR_SCHEMES {
                let capture_id = format!("{}__{}__{}", story.id, viewport.id, scheme.as_str());
                captures.push(AudioVisualCapture {
                    id: capture_id.clone(),
                    story_id: story.id,
                    story_label: story.label,
                    component: story.component,
                    renderer: story.renderer,
                    renderer_qa_queries: story.renderer_qa_queries,
                    scenario: story.scenario,
                    viewport_id: viewport.id,
                    viewport_label: viewport.label,
                    width: viewport.width,
                    height: viewport.height,
                    scale_factor: viewport.scale_factor,
                    color_scheme: scheme,
                    release_focus: story.release_focus,
                    baseline_path: artifact_path("baseline", story.id, viewport.id, scheme),
                    actual_path: artifact_path("actual", story.id, viewport.id, scheme),
                    diff_path: artifact_path("diff", story.id, viewport.id, scheme),
                });
            }
        }
    }

    AudioVisualRegressionManifest {
        schema_version: AUDIO_VISUAL_REGRESSION_SCHEMA_VERSION,
        report_type: AUDIO_VISUAL_REGRESSION_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        stories: AUDIO_VISUAL_STORIES,
        viewports: AUDIO_VISUAL_VIEWPORTS,
        color_schemes: AUDIO_VISUAL_COLOR_SCHEMES,
        captures,
    }
}

pub const AUDIO_VISUAL_VIEWPORTS: &[AudioVisualViewport] = &[
    AudioVisualViewport {
        id: "desktop-panel",
        label: "Desktop plugin panel",
        width: 960,
        height: 640,
        scale_factor: 2,
    },
    AudioVisualViewport {
        id: "compact-strip",
        label: "Compact channel strip",
        width: 390,
        height: 720,
        scale_factor: 2,
    },
];

pub const AUDIO_VISUAL_COLOR_SCHEMES: &[AudioVisualColorScheme] = &[
    AudioVisualColorScheme::Light,
    AudioVisualColorScheme::Dark,
    AudioVisualColorScheme::HighContrast,
];

pub const AUDIO_VISUAL_RENDERER_QA_QUERIES: &[&str] = &["auto", "cpu", "legacy"];

pub const AUDIO_VISUAL_STORIES: &[AudioVisualStory] = &[
    AudioVisualStory {
        id: "audio-kit.potentiometer",
        label: "Potentiometer",
        component: "Potentiometer",
        renderer: "Vello · Auto",
        renderer_qa_queries: AUDIO_VISUAL_RENDERER_QA_QUERIES,
        scenario: "log-frequency parameter",
        release_focus: "rotary ticks, logarithmic labels, selected state, and drag affordance",
    },
    AudioVisualStory {
        id: "audio-kit.vertical-slider",
        label: "Vertical Slider",
        component: "VerticalSlider",
        renderer: "Vello · Auto (descendant paints Legacy div geometry)",
        renderer_qa_queries: AUDIO_VISUAL_RENDERER_QA_QUERIES,
        scenario: "channel fader",
        release_focus: "track fill, thumb position, peak marker, ticks, and dense layout",
    },
    AudioVisualStory {
        id: "audio-kit.volume-knob",
        label: "Volume Knob",
        component: "VolumeKnob",
        renderer: "Vello · Auto",
        renderer_qa_queries: AUDIO_VISUAL_RENDERER_QA_QUERIES,
        scenario: "monitor volume with mute",
        release_focus: "circular fill, mute state, percentage label, and focus ring",
    },
    AudioVisualStory {
        id: "audio-kit.meter",
        label: "Level Meter",
        component: "LevelMeterElement",
        renderer: "Vello · Auto",
        renderer_qa_queries: AUDIO_VISUAL_RENDERER_QA_QUERIES,
        scenario: "stereo level and peak feedback",
        release_focus: "gradient fill, clipping threshold, peak hold, and channel labels",
    },
    AudioVisualStory {
        id: "audio-kit.horizontal-meter",
        label: "Horizontal Meter",
        component: "HorizontalMeter",
        renderer: "Vello · Auto (gradient path)",
        renderer_qa_queries: AUDIO_VISUAL_RENDERER_QA_QUERIES,
        scenario: "LUFS or stereo-width strip",
        release_focus: "tick alignment, threshold colors, value text, and compact rows",
    },
    AudioVisualStory {
        id: "audio-kit.spectrum",
        label: "Spectrum",
        component: "SpectrumElement",
        renderer: "Vello · Auto",
        renderer_qa_queries: AUDIO_VISUAL_RENDERER_QA_QUERIES,
        scenario: "spectrum analyzer bins",
        release_focus: "bar density, smoothing, color gradient, and zero-data handling",
    },
    AudioVisualStory {
        id: "audio-kit.spectrum-axis",
        label: "Spectrum Axes",
        component: "SpectrumAxis",
        renderer: "GPUI text/layout",
        renderer_qa_queries: &["legacy"],
        scenario: "frequency and dB axis labels",
        release_focus: "log-frequency spacing, dB tick labels, and small text legibility",
    },
];

fn artifact_path(
    kind: &str,
    story_id: &str,
    viewport_id: &str,
    color_scheme: AudioVisualColorScheme,
) -> String {
    format!(
        "artifacts/gpui-audio-kit/visual/{kind}/{story_id}/{viewport_id}/{}.png",
        color_scheme.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_regression_manifest_has_stable_contract() {
        let manifest = audio_visual_regression_manifest();

        assert_eq!(
            manifest.schema_version,
            AUDIO_VISUAL_REGRESSION_SCHEMA_VERSION
        );
        assert_eq!(manifest.report_type, AUDIO_VISUAL_REGRESSION_REPORT_TYPE);
        assert_eq!(manifest.capture_count(), manifest.expected_capture_count());
        assert_eq!(manifest.stories.len(), 7);
        assert_eq!(manifest.viewports.len(), 2);
        assert_eq!(manifest.color_schemes.len(), 3);
        assert!(manifest.validate_unique_capture_ids());
        assert!(
            manifest
                .stories
                .iter()
                .all(|story| !story.renderer.is_empty())
        );
        assert!(manifest.stories.iter().all(|story| {
            !story.renderer_qa_queries.is_empty()
                && (story.renderer == "GPUI text/layout"
                    || story.renderer_qa_queries == AUDIO_VISUAL_RENDERER_QA_QUERIES)
        }));
        assert!(
            manifest
                .captures
                .iter()
                .all(|capture| !capture.renderer.is_empty())
        );
        assert!(
            manifest
                .captures
                .iter()
                .all(|capture| !capture.renderer_qa_queries.is_empty())
        );
    }

    #[test]
    fn visual_regression_manifest_covers_core_audio_surfaces() {
        let manifest = audio_visual_regression_manifest();
        let components = manifest.components();

        for component in [
            "Potentiometer",
            "VerticalSlider",
            "VolumeKnob",
            "LevelMeterElement",
            "HorizontalMeter",
            "SpectrumElement",
            "SpectrumAxis",
        ] {
            assert!(components.contains(component));
            assert_eq!(
                manifest.captures_for_component(component).len(),
                manifest.viewports.len() * manifest.color_schemes.len()
            );
        }
    }

    #[test]
    fn visual_regression_manifest_uses_stable_artifact_paths() {
        let manifest = audio_visual_regression_manifest();
        let capture = manifest
            .captures
            .iter()
            .find(|capture| capture.id == "audio-kit.potentiometer__desktop-panel__dark")
            .expect("potentiometer desktop dark capture should exist");

        assert_eq!(
            capture.baseline_path,
            "artifacts/gpui-audio-kit/visual/baseline/audio-kit.potentiometer/desktop-panel/dark.png"
        );
        assert_eq!(
            capture.actual_path,
            "artifacts/gpui-audio-kit/visual/actual/audio-kit.potentiometer/desktop-panel/dark.png"
        );
        assert_eq!(
            capture.diff_path,
            "artifacts/gpui-audio-kit/visual/diff/audio-kit.potentiometer/desktop-panel/dark.png"
        );
    }

    #[test]
    fn visual_regression_manifest_markdown_is_release_attachable() {
        let markdown = audio_visual_regression_manifest().to_markdown_table();

        assert!(markdown.contains(AUDIO_VISUAL_REGRESSION_REPORT_TYPE));
        assert!(markdown.contains("renderer QA"));
        assert!(markdown.contains("audio-kit.potentiometer__desktop-panel__dark"));
        assert!(markdown.contains("LevelMeterElement"));
        assert!(markdown.contains("Vello · Auto"));
        assert!(markdown.contains("artifacts/gpui-audio-kit/visual/diff"));
        assert!(markdown.contains("high_contrast"));
    }
}
