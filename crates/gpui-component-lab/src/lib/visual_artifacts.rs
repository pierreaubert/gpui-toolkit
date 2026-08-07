use super::visual_regression_manifest::ComponentLabVisualCase;
use anyhow::{Context, Result, bail};
use image::{Rgba, RgbaImage, imageops};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const COMPONENT_LAB_BASELINE_INDEX_SCHEMA_VERSION: u32 = 1;
pub const COMPONENT_LAB_GALLERY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabBaselineEntry {
    pub capture_id: String,
    pub story_id: String,
    pub renderer_id: String,
    pub pixel_scale: u32,
    pub width: u32,
    pub height: u32,
    pub rgba_checksum: String,
    pub baseline_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabBaselineIndex {
    pub schema_version: u32,
    pub renderer_id: String,
    pub case_count: usize,
    pub cases: Vec<ComponentLabBaselineEntry>,
}

impl ComponentLabBaselineIndex {
    pub fn to_markdown_table(&self) -> String {
        let mut out = format!(
            "# GPUI Component Lab Baseline Index\n\n\
             - schema_version: {}\n\
             - renderer: `{}`\n\
             - cases: {}\n\n\
             | capture | dimensions | scale | checksum | baseline |\n\
             | --- | ---: | ---: | --- | --- |\n",
            self.schema_version, self.renderer_id, self.case_count
        );
        for case in &self.cases {
            out.push_str(&format!(
                "| `{}` | {}x{} | {} | `{}` | `{}` |\n",
                case.capture_id,
                case.width,
                case.height,
                case.pixel_scale,
                case.rgba_checksum,
                case.baseline_path
            ));
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabGalleryEntry {
    pub capture_id: String,
    pub story_id: String,
    pub theme_id: String,
    pub viewport_id: String,
    pub actual_path: String,
    pub sheet_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabGalleryReport {
    pub schema_version: u32,
    pub renderer_id: String,
    pub case_count: usize,
    pub sheet_count: usize,
    pub sheets: Vec<String>,
    pub cases: Vec<ComponentLabGalleryEntry>,
}

impl ComponentLabGalleryReport {
    pub fn to_markdown(&self) -> String {
        let mut out = format!(
            "# GPUI Component Lab Gallery\n\n\
             Renderer: `{}`  \n\
             Captures: {}  \n\
             Contact sheets: {}\n\n",
            self.renderer_id, self.case_count, self.sheet_count
        );
        for sheet in &self.sheets {
            let file_name = Path::new(sheet)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(sheet);
            out.push_str(&format!("![Component gallery]({file_name})\n\n"));
        }
        out.push_str(
            "| capture | story | viewport | theme | pixels |\n| --- | --- | --- | --- | --- |\n",
        );
        for case in &self.cases {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` |\n",
                case.capture_id, case.story_id, case.viewport_id, case.theme_id, case.actual_path
            ));
        }
        out
    }
}

/// Promote validated actual images into a renderer-specific baseline set.
/// Callers must expose this only behind an explicit approval/update flag.
pub fn promote_component_lab_baselines(
    renderer_id: &str,
    cases: &[ComponentLabVisualCase],
    index_path: &Path,
) -> Result<ComponentLabBaselineIndex> {
    let mut entries = Vec::with_capacity(cases.len());
    for case in cases {
        if case.renderer_id != renderer_id {
            bail!(
                "case {} belongs to renderer {}, not {}",
                case.capture_id,
                case.renderer_id,
                renderer_id
            );
        }
        let image = load_validated_actual(case)?;
        let baseline = Path::new(&case.baseline_path);
        if let Some(parent) = baseline.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::copy(&case.actual_path, baseline)
            .with_context(|| format!("promote {} to {}", case.actual_path, baseline.display()))?;
        entries.push(ComponentLabBaselineEntry {
            capture_id: case.capture_id.clone(),
            story_id: case.story_id.clone(),
            renderer_id: renderer_id.to_string(),
            pixel_scale: case.pixel_scale,
            width: image.width(),
            height: image.height(),
            rgba_checksum: rgba_checksum(&image),
            baseline_path: case.baseline_path.clone(),
        });
    }
    entries.sort_by(|a, b| a.capture_id.cmp(&b.capture_id));
    let index = ComponentLabBaselineIndex {
        schema_version: COMPONENT_LAB_BASELINE_INDEX_SCHEMA_VERSION,
        renderer_id: renderer_id.to_string(),
        case_count: entries.len(),
        cases: entries,
    };
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(index_path, serde_json::to_string_pretty(&index)?)
        .with_context(|| format!("write {}", index_path.display()))?;
    Ok(index)
}

/// Create compact PNG contact sheets and a machine-readable gallery index
/// from renderer pixels. Every selected image must exist and satisfy the
/// manifest's dimensions/non-blank contract.
pub fn generate_component_lab_gallery(
    renderer_id: &str,
    cases: &[ComponentLabVisualCase],
    gallery_root: &Path,
) -> Result<ComponentLabGalleryReport> {
    const COLUMNS: usize = 4;
    const ROWS: usize = 3;
    const TILE_WIDTH: u32 = 320;
    const TILE_HEIGHT: u32 = 200;
    const GUTTER: u32 = 16;
    const PER_SHEET: usize = COLUMNS * ROWS;

    std::fs::create_dir_all(gallery_root)
        .with_context(|| format!("create {}", gallery_root.display()))?;
    let mut sheets = Vec::new();
    let mut entries = Vec::with_capacity(cases.len());

    for (sheet_index, chunk) in cases.chunks(PER_SHEET).enumerate() {
        let width = GUTTER + COLUMNS as u32 * (TILE_WIDTH + GUTTER);
        let height = GUTTER + ROWS as u32 * (TILE_HEIGHT + GUTTER);
        let mut sheet = RgbaImage::from_pixel(width, height, Rgba([245, 245, 245, 255]));
        for (tile_index, case) in chunk.iter().enumerate() {
            if case.renderer_id != renderer_id {
                bail!(
                    "gallery renderer namespace mismatch for {}",
                    case.capture_id
                );
            }
            let image = load_validated_actual(case)?;
            let thumbnail = imageops::thumbnail(&image, TILE_WIDTH, TILE_HEIGHT);
            let column = tile_index % COLUMNS;
            let row = tile_index / COLUMNS;
            let x = i64::from(
                GUTTER
                    + column as u32 * (TILE_WIDTH + GUTTER)
                    + (TILE_WIDTH - thumbnail.width()) / 2,
            );
            let y = i64::from(
                GUTTER
                    + row as u32 * (TILE_HEIGHT + GUTTER)
                    + (TILE_HEIGHT - thumbnail.height()) / 2,
            );
            imageops::overlay(&mut sheet, &thumbnail, x, y);
            entries.push(ComponentLabGalleryEntry {
                capture_id: case.capture_id.clone(),
                story_id: case.story_id.clone(),
                theme_id: case.theme_id.clone(),
                viewport_id: case.viewport_id.clone(),
                actual_path: case.actual_path.clone(),
                sheet_index,
            });
        }
        let sheet_path = gallery_root.join(format!("contact-sheet-{:03}.png", sheet_index + 1));
        sheet
            .save(&sheet_path)
            .with_context(|| format!("write {}", sheet_path.display()))?;
        sheets.push(normalized_path(&sheet_path));
    }

    let report = ComponentLabGalleryReport {
        schema_version: COMPONENT_LAB_GALLERY_SCHEMA_VERSION,
        renderer_id: renderer_id.to_string(),
        case_count: entries.len(),
        sheet_count: sheets.len(),
        sheets,
        cases: entries,
    };
    std::fs::write(
        gallery_root.join("gallery.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    std::fs::write(gallery_root.join("README.md"), report.to_markdown())?;
    Ok(report)
}

fn load_validated_actual(case: &ComponentLabVisualCase) -> Result<RgbaImage> {
    let image = image::open(&case.actual_path)
        .with_context(|| format!("decode {}", case.actual_path))?
        .to_rgba8();
    let expected = (
        case.viewport_width.saturating_mul(case.pixel_scale),
        case.viewport_height.saturating_mul(case.pixel_scale),
    );
    if image.dimensions() != expected {
        bail!(
            "{} is {}x{}, expected {}x{}",
            case.actual_path,
            image.width(),
            image.height(),
            expected.0,
            expected.1
        );
    }
    if image_is_blank(&image) {
        bail!("{} is blank", case.actual_path);
    }
    Ok(image)
}

fn image_is_blank(image: &RgbaImage) -> bool {
    let mut pixels = image.pixels();
    let Some(first) = pixels.next() else {
        return true;
    };
    pixels.all(|pixel| pixel == first)
}

fn rgba_checksum(image: &RgbaImage) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in image.as_raw() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
