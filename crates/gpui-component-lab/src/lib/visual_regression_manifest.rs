use super::responsive_preview_matrix::ResponsivePreviewMatrix;
use super::story_registry::StoryRegistry;
use super::story_renderer_kind::StoryRendererKind;
use super::story_renderer_registry::StoryRendererRegistry;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::Path;

pub const COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION: u32 = 2;
pub const COMPONENT_LAB_VISUAL_DIFF_SCHEMA_VERSION: u32 = 2;
pub const COMPONENT_LAB_VISUAL_DIFF_REPORT_TYPE: &str = "gpui-component-lab-visual-diff";

/// One deterministic screenshot capture expected by CI visual regression jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabVisualCase {
    pub capture_id: String,
    pub renderer_id: String,
    pub pixel_scale: u32,
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
    pub renderer_id: String,
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
    UnexpectedDimensions,
    BlankBaseline,
    BlankActual,
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
            Self::UnexpectedDimensions => "unexpected-dimensions",
            Self::BlankBaseline => "blank-baseline",
            Self::BlankActual => "blank-actual",
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
        Self::from_registries_for_renderer(stories, renderers, output_root, "unspecified", 1)
    }

    /// Build a manifest whose baseline namespace is tied to one renderer and
    /// deterministic logical-to-device pixel scale.
    pub fn from_registries_for_renderer(
        stories: &StoryRegistry,
        renderers: &StoryRendererRegistry,
        output_root: impl AsRef<Path>,
        renderer_id: impl Into<String>,
        pixel_scale: u32,
    ) -> Self {
        let output_root = output_root.as_ref();
        let renderer_id = renderer_id.into();
        assert!(
            pixel_scale > 0,
            "visual capture pixel scale must be positive"
        );
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
                    renderer_id: renderer_id.clone(),
                    pixel_scale,
                    story_id: renderer.story_id.clone(),
                    renderer_kind: renderer.kind,
                    viewport_id,
                    viewport_width,
                    viewport_height,
                    theme_id,
                    design: cell.theme.design,
                    reduced_motion: cell.theme.reduced_motion,
                    interactive: renderer.interactive,
                    baseline_path: manifest_path(
                        output_root,
                        &renderer_id,
                        "baseline",
                        &capture_id,
                    ),
                    actual_path: manifest_path(output_root, &renderer_id, "actual", &capture_id),
                    diff_path: manifest_path(output_root, &renderer_id, "diff", &capture_id),
                });
            }
        }

        cases.sort_by(|a, b| a.capture_id.cmp(&b.capture_id));

        Self {
            schema_version: COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION,
            renderer_id,
            case_count: cases.len(),
            cases,
        }
    }

    /// Select a deterministic PR-sized set while retaining at least one case
    /// per story whenever the requested limit permits it. The first pass
    /// rotates through each story's matrix so viewport/theme coverage is not
    /// biased toward the lexicographically first preset.
    pub fn representative_cases(&self, limit: usize) -> Vec<ComponentLabVisualCase> {
        use std::collections::BTreeMap;

        if limit == 0 || limit >= self.cases.len() {
            return self.cases.clone();
        }

        let mut by_story: BTreeMap<&str, Vec<&ComponentLabVisualCase>> = BTreeMap::new();
        for case in &self.cases {
            by_story.entry(&case.story_id).or_default().push(case);
        }

        let mut selected = Vec::with_capacity(limit);
        let mut next_index = BTreeMap::new();
        for (story_ordinal, (story_id, cases)) in by_story.iter().enumerate() {
            if selected.len() == limit {
                break;
            }
            let index = story_ordinal % cases.len();
            selected.push((*cases[index]).clone());
            next_index.insert(*story_id, index + 1);
        }

        while selected.len() < limit {
            let mut added = false;
            for (story_id, cases) in &by_story {
                if selected.len() == limit {
                    break;
                }
                let cursor = next_index.entry(*story_id).or_insert(0);
                while *cursor < cases.len()
                    && selected
                        .iter()
                        .any(|selected| selected.capture_id == cases[*cursor].capture_id)
                {
                    *cursor += 1;
                }
                if *cursor < cases.len() {
                    selected.push((*cases[*cursor]).clone());
                    *cursor += 1;
                    added = true;
                }
            }
            if !added {
                break;
            }
        }
        selected.sort_by(|a, b| a.capture_id.cmp(&b.capture_id));
        selected
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
        self.diff_selected_captures(&self.cases, max_changed_pixels)
    }

    pub fn diff_selected_captures(
        &self,
        selected: &[ComponentLabVisualCase],
        max_changed_pixels: u64,
    ) -> ComponentLabVisualDiffReport {
        let cases = selected
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
            case_count: selected.len(),
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

    let expected_width = case.viewport_width.saturating_mul(case.pixel_scale);
    let expected_height = case.viewport_height.saturating_mul(case.pixel_scale);
    if (width, height) != (expected_width, expected_height) {
        return ComponentLabVisualDiffCase {
            capture_id: case.capture_id.clone(),
            story_id: case.story_id.clone(),
            baseline_path: case.baseline_path.clone(),
            actual_path: case.actual_path.clone(),
            diff_path: case.diff_path.clone(),
            status: ComponentLabVisualDiffStatus::UnexpectedDimensions,
            width,
            height,
            changed_pixels: 0,
            total_pixels: u64::from(width) * u64::from(height),
            max_channel_delta: 0,
            message: format!(
                "capture is {}x{}, manifest requires {}x{}",
                width, height, expected_width, expected_height
            ),
        };
    }
    if image_is_blank(&baseline) {
        return diff_case_error(
            case,
            ComponentLabVisualDiffStatus::BlankBaseline,
            "baseline image contains only one RGBA value",
        );
    }
    if image_is_blank(&actual) {
        return diff_case_error(
            case,
            ComponentLabVisualDiffStatus::BlankActual,
            "actual image contains only one RGBA value",
        );
    }

    let mut changed_pixels = 0_u64;
    let mut max_channel_delta = 0_u8;
    let mut diff = RgbaImage::new(width, height);
    let diff_pixels: &mut [u8] = diff.as_mut();

    for ((baseline_pixel, actual_pixel), diff_pixel) in baseline
        .as_raw()
        .chunks_exact(4)
        .zip(actual.as_raw().chunks_exact(4))
        .zip(diff_pixels.chunks_exact_mut(4))
    {
        let pixel_max_delta = baseline_pixel
            .iter()
            .zip(actual_pixel)
            .map(|(&baseline, &actual)| baseline.abs_diff(actual))
            .max()
            .unwrap_or_default();

        if pixel_max_delta > 0 {
            changed_pixels += 1;
            max_channel_delta = max_channel_delta.max(pixel_max_delta);
            diff_pixel.copy_from_slice(&[255, 0, 0, pixel_max_delta.max(96)]);
        } else {
            diff_pixel.fill(0);
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
    let mut encoded = String::with_capacity(value.len().saturating_mul(3));
    for byte in value.bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            encoded.push(byte as char);
        } else {
            write!(&mut encoded, "~{byte:02x}").expect("write into String cannot fail");
        }
    }
    encoded
}

#[cfg(test)]
mod path_name_tests {
    use super::{capture_id, sanitize_path_part};

    #[test]
    fn path_parts_and_capture_ids_preserve_raw_identity() {
        assert_ne!(sanitize_path_part("a.b"), sanitize_path_part("a-b"));
        assert_ne!(sanitize_path_part("A"), sanitize_path_part("a"));
        assert_ne!(
            capture_id("a.b", "small", "light"),
            capture_id("a-b", "small", "light")
        );
    }
}

fn image_is_blank(image: &RgbaImage) -> bool {
    let mut pixels = image.pixels();
    let Some(first) = pixels.next() else {
        return true;
    };
    pixels.all(|pixel| pixel == first)
}

fn manifest_path(output_root: &Path, renderer_id: &str, group: &str, capture_id: &str) -> String {
    output_root
        .join(sanitize_path_part(renderer_id))
        .join(group)
        .join(format!("{capture_id}.png"))
        .to_string_lossy()
        .replace('\\', "/")
}
