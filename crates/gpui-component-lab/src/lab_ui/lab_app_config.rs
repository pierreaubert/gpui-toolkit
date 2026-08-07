use std::path::PathBuf;

/// Configuration for the interactive component lab app.
#[derive(Debug, Clone)]
pub struct LabAppConfig {
    pub stories_dir: PathBuf,
    pub token_paths: Vec<PathBuf>,
    pub watch: bool,
    pub(crate) visual_capture: Option<LabVisualCaptureConfig>,
}

#[derive(Debug, Clone)]
pub(crate) struct LabVisualCaptureConfig {
    pub story_id: String,
    pub viewport_id: String,
    pub theme_id: String,
    pub reduced_motion: bool,
}

impl LabAppConfig {
    pub fn new(stories_dir: PathBuf, token_paths: Vec<PathBuf>) -> Self {
        Self {
            stories_dir,
            token_paths,
            watch: false,
            visual_capture: None,
        }
    }

    pub fn with_watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }

    #[cfg(feature = "visual-capture")]
    pub(crate) fn for_visual_capture(
        mut self,
        story_id: impl Into<String>,
        viewport_id: impl Into<String>,
        theme_id: impl Into<String>,
        reduced_motion: bool,
    ) -> Self {
        self.watch = false;
        self.visual_capture = Some(LabVisualCaptureConfig {
            story_id: story_id.into(),
            viewport_id: viewport_id.into(),
            theme_id: theme_id.into(),
            reduced_motion,
        });
        self
    }
}

impl Default for LabAppConfig {
    fn default() -> Self {
        Self {
            stories_dir: PathBuf::from("crates/gpui-toolkit/stories"),
            token_paths: Vec::new(),
            watch: false,
            visual_capture: None,
        }
    }
}
