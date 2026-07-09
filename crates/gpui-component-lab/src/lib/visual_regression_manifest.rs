use super::responsive_preview_matrix::ResponsivePreviewMatrix;
use super::story_registry::StoryRegistry;
use super::story_renderer_kind::StoryRendererKind;
use super::story_renderer_registry::StoryRendererRegistry;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const COMPONENT_LAB_VISUAL_DIFF_SCHEMA_VERSION: u32 = 1;
pub const COMPONENT_LAB_VISUAL_DIFF_REPORT_TYPE: &str = "gpui-component-lab-visual-diff";

/// One deterministic screenshot capture expected by CI visual regression jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabVisualCase {
    pub capture_id: String,
    pub story_id: String,
    pub renderer_kind: StoryRendererKind,
    pub viewport_id: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub theme_id: String,
    pub design: String,
    pub reduced_motion: bool,
    pub interactive: bool,
    pub baseline_path: String,
    pub actual_path: String,
    pub diff_path: String,
}

/// CI-facing manifest for screenshot capture and baseline diff tooling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabVisualManifest {
    pub schema_version: u32,
    pub case_count: usize,
    pub cases: Vec<ComponentLabVisualCase>,
}

/// Pixel-diff status for one captured visual regression case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentLabVisualDiffStatus {
    Passed,
    Different,
    MissingBaseline,
    MissingActual,
    SizeMismatch,
    DecodeFailed,
    WriteFailed,
}

impl ComponentLabVisualDiffStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Different => "different",
            Self::MissingBaseline => "missing-baseline",
            Self::MissingActual => "missing-actual",
            Self::SizeMismatch => "size-mismatch",
            Self::DecodeFailed => "decode-failed",
            Self::WriteFailed => "write-failed",
        }
    }

    pub const fn is_passed(self) -> bool {
        matches!(self, Self::Passed)
    }
}

/// Pixel-diff result for one visual manifest case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabVisualDiffCase {
    pub capture_id: String,
    pub story_id: String,
    pub baseline_path: String,
    pub actual_path: String,
    pub diff_path: String,
    pub status: ComponentLabVisualDiffStatus,
    pub width: u32,
    pub height: u32,
    pub changed_pixels: u64,
    pub total_pixels: u64,
    pub max_channel_delta: u8,
    pub message: String,
}

/// CI-facing pixel-diff report for a visual manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabVisualDiffReport {
    pub schema_version: u32,
    pub report_type: String,
    pub passed: bool,
    pub case_count: usize,
    pub compared_count: usize,
    pub failed_count: usize,
    pub max_changed_pixels: u64,
    pub cases: Vec<ComponentLabVisualDiffCase>,
}

impl ComponentLabVisualDiffReport {
    pub fn to_markdown_table(&self) -> String {
        let mut out = format!(
            "# GPUI Component Lab Visual Diff\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - passed: {}\n\
             - max_changed_pixels: {}\n\n\
             | capture | status | changed | total | max delta | diff |\n\
             | --- | --- | ---: | ---: | ---: | --- |\n",
            self.schema_version, self.report_type, self.passed, self.max_changed_pixels
        );
        for case in &self.cases {
            out.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | `{}` |\n",
                case.capture_id,
                case.status.as_str(),
                case.changed_pixels,
                case.total_pixels,
                case.max_channel_delta,
                case.diff_path
            ));
        }
        out
    }
}

impl ComponentLabVisualManifest {
    pub fn from_registries(
        stories: &StoryRegistry,
        renderers: &StoryRendererRegistry,
        output_root: impl AsRef<Path>,
    ) -> Self {
        let output_root = output_root.as_ref();
        let mut cases = Vec::new();

        for renderer in renderers.renderers() {
            let Some(story) = stories.story(&renderer.story_id) else {
                continue;
            };
            let matrix = ResponsivePreviewMatrix::for_story(story);
            let cells: Vec<_> = if renderer.matrix_preview {
                matrix.cells
            } else {
                matrix.cells.into_iter().take(1).collect()
            };

            for cell in cells {
                let viewport_id = cell.viewport.id.to_string();
                let theme_id = cell.theme.id.to_string();
                let capture_id = capture_id(&renderer.story_id, &viewport_id, &theme_id);
                let viewport_width = cell.viewport.width.round().max(0.0) as u32;
                let viewport_height = cell.viewport.height.round().max(0.0) as u32;

                cases.push(ComponentLabVisualCase {
                    capture_id: capture_id.clone(),
                    story_id: renderer.story_id.clone(),
                    renderer_kind: renderer.kind,
                    viewport_id,
                    viewport_width,
                    viewport_height,
                    theme_id,
                    design: cell.theme.design,
                    reduced_motion: cell.theme.reduced_motion,
                    interactive: renderer.interactive,
                    baseline_path: manifest_path(output_root, "baseline", &capture_id),
                    actual_path: manifest_path(output_root, "actual", &capture_id),
                    diff_path: manifest_path(output_root, "diff", &capture_id),
                });
            }
        }

        cases.sort_by(|a, b| a.capture_id.cmp(&b.capture_id));

        Self {
            schema_version: COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION,
            case_count: cases.len(),
            cases,
        }
    }

    pub fn to_markdown_table(&self) -> String {
        let mut out =
            String::from("| capture | story | viewport | theme | baseline | actual | diff |\n");
        out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
        for case in &self.cases {
            out.push_str("| `");
            out.push_str(&case.capture_id);
            out.push_str("` | `");
            out.push_str(&case.story_id);
            out.push_str("` | `");
            out.push_str(&case.viewport_id);
            out.push_str("` | `");
            out.push_str(&case.theme_id);
            out.push_str("` | `");
            out.push_str(&case.baseline_path);
            out.push_str("` | `");
            out.push_str(&case.actual_path);
            out.push_str("` | `");
            out.push_str(&case.diff_path);
            out.push_str("` |\n");
        }
        out
    }

    pub fn diff_captures(&self, max_changed_pixels: u64) -> ComponentLabVisualDiffReport {
        let cases = self
            .cases
            .iter()
            .map(|case| diff_visual_case(case, max_changed_pixels))
            .collect::<Vec<_>>();
        let compared_count = cases
            .iter()
            .filter(|case| {
                matches!(
                    case.status,
                    ComponentLabVisualDiffStatus::Passed | ComponentLabVisualDiffStatus::Different
                )
            })
            .count();
        let failed_count = cases.iter().filter(|case| !case.status.is_passed()).count();

        ComponentLabVisualDiffReport {
            schema_version: COMPONENT_LAB_VISUAL_DIFF_SCHEMA_VERSION,
            report_type: COMPONENT_LAB_VISUAL_DIFF_REPORT_TYPE.to_string(),
            passed: failed_count == 0,
            case_count: self.cases.len(),
            compared_count,
            failed_count,
            max_changed_pixels,
            cases,
        }
    }
}

fn diff_visual_case(
    case: &ComponentLabVisualCase,
    max_changed_pixels: u64,
) -> ComponentLabVisualDiffCase {
    if !Path::new(&case.baseline_path).exists() {
        return diff_case_error(
            case,
            ComponentLabVisualDiffStatus::MissingBaseline,
            "baseline image does not exist",
        );
    }
    if !Path::new(&case.actual_path).exists() {
        return diff_case_error(
            case,
            ComponentLabVisualDiffStatus::MissingActual,
            "actual image does not exist",
        );
    }

    let baseline = match image::open(&case.baseline_path) {
        Ok(image) => image.to_rgba8(),
        Err(error) => {
            return diff_case_error(
                case,
                ComponentLabVisualDiffStatus::DecodeFailed,
                format!("decode baseline image: {error}"),
            );
        }
    };
    let actual = match image::open(&case.actual_path) {
        Ok(image) => image.to_rgba8(),
        Err(error) => {
            return diff_case_error(
                case,
                ComponentLabVisualDiffStatus::DecodeFailed,
                format!("decode actual image: {error}"),
            );
        }
    };

    let (width, height) = baseline.dimensions();
    if actual.dimensions() != (width, height) {
        return ComponentLabVisualDiffCase {
            capture_id: case.capture_id.clone(),
            story_id: case.story_id.clone(),
            baseline_path: case.baseline_path.clone(),
            actual_path: case.actual_path.clone(),
            diff_path: case.diff_path.clone(),
            status: ComponentLabVisualDiffStatus::SizeMismatch,
            width,
            height,
            changed_pixels: 0,
            total_pixels: u64::from(width) * u64::from(height),
            max_channel_delta: 0,
            message: format!(
                "baseline is {}x{}, actual is {}x{}",
                width,
                height,
                actual.width(),
                actual.height()
            ),
        };
    }

    let mut changed_pixels = 0_u64;
    let mut max_channel_delta = 0_u8;
    let mut diff = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let baseline_pixel = baseline.get_pixel(x, y).0;
            let actual_pixel = actual.get_pixel(x, y).0;
            let mut pixel_max_delta = 0_u8;
            for channel in 0..4 {
                pixel_max_delta =
                    pixel_max_delta.max(baseline_pixel[channel].abs_diff(actual_pixel[channel]));
            }

            if pixel_max_delta > 0 {
                changed_pixels += 1;
                max_channel_delta = max_channel_delta.max(pixel_max_delta);
                diff.put_pixel(x, y, Rgba([255, 0, 0, pixel_max_delta.max(96)]));
            } else {
                diff.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    if let Some(parent) = Path::new(&case.diff_path).parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return diff_case_error(
            case,
            ComponentLabVisualDiffStatus::WriteFailed,
            format!("create diff directory: {error}"),
        );
    }
    if let Err(error) = diff.save(&case.diff_path) {
        return diff_case_error(
            case,
            ComponentLabVisualDiffStatus::WriteFailed,
            format!("write diff image: {error}"),
        );
    }

    let status = if changed_pixels <= max_changed_pixels {
        ComponentLabVisualDiffStatus::Passed
    } else {
        ComponentLabVisualDiffStatus::Different
    };
    ComponentLabVisualDiffCase {
        capture_id: case.capture_id.clone(),
        story_id: case.story_id.clone(),
        baseline_path: case.baseline_path.clone(),
        actual_path: case.actual_path.clone(),
        diff_path: case.diff_path.clone(),
        status,
        width,
        height,
        changed_pixels,
        total_pixels: u64::from(width) * u64::from(height),
        max_channel_delta,
        message: if status.is_passed() {
            "within visual diff threshold".to_string()
        } else {
            "visual diff threshold exceeded".to_string()
        },
    }
}

fn diff_case_error(
    case: &ComponentLabVisualCase,
    status: ComponentLabVisualDiffStatus,
    message: impl Into<String>,
) -> ComponentLabVisualDiffCase {
    ComponentLabVisualDiffCase {
        capture_id: case.capture_id.clone(),
        story_id: case.story_id.clone(),
        baseline_path: case.baseline_path.clone(),
        actual_path: case.actual_path.clone(),
        diff_path: case.diff_path.clone(),
        status,
        width: 0,
        height: 0,
        changed_pixels: 0,
        total_pixels: 0,
        max_channel_delta: 0,
        message: message.into(),
    }
}

fn capture_id(story_id: &str, viewport_id: &str, theme_id: &str) -> String {
    format!(
        "{}__{}__{}",
        sanitize_path_part(story_id),
        sanitize_path_part(viewport_id),
        sanitize_path_part(theme_id)
    )
}

fn sanitize_path_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn manifest_path(output_root: &Path, group: &str, capture_id: &str) -> String {
    output_root
        .join(group)
        .join(format!("{capture_id}.png"))
        .to_string_lossy()
        .replace('\\', "/")
}
