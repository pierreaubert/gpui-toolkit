//! Single-channel SDF atlas for world-space 3D labels.
//!
//! Glyph runs are screen-space and cannot live in perspective scenes, so 3D
//! labels sample pre-baked signed distance fields from an atlas texture
//! instead: one instanced quad per glyph stays crisp under zoom and rotation
//! with a single draw call. Baking jump-floods `fontdue` coverage rasters of
//! the bundled DejaVu Sans at 64px — no new dependency, fully deterministic,
//! backend-neutral (`Vec<u8>` + UV table; each renderer uploads natively).
//!
//! String layout uses `fontdue` advances with no kerning: tick and item
//! labels are numbers, symbols, and names, where shaping would change
//! nothing. Complex-script shaping stays in the 2D engine (`FontEngine`).

use std::collections::HashMap;

use super::SANS_TTF;

/// Bake resolution in px. 64px keeps 10px ticks crisp through the SDF
/// down to strong minification; larger only grows the atlas.
pub const SDF_BAKE_PX: f32 = 64.0;
/// Distance range encoded around the outline, in bake px.
pub const SDF_SPREAD_PX: f32 = 8.0;
/// Atlas cell padding in px per side (stops linear-filter bleeding).
pub const SDF_CELL_PAD: usize = 8;
/// Coverage threshold for inside/outside classification.
const COVERAGE_THRESHOLD: u8 = 127;

/// Printable ASCII plus `°`: digits, signs, dots, month and item names —
/// the tick-format audit for surface axes and gallery labels.
pub const ATLAS_CHARSET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~°";

/// Cell footprint of one baked glyph: bitmap plus padding per side.
pub const SDF_CELL_PX: usize = SDF_BAKE_PX as usize + 2 * SDF_CELL_PAD;

/// One atlas cell: UV rect plus bake-px placement metrics.
#[derive(Clone, Debug)]
pub struct SdfGlyph {
    /// UV rect corners; row 0 is the visual top (`v = 0` samples the top).
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Pen x to cell-left edge, in bake px (includes padding + bearing).
    pub cell_offset_x: f32,
    /// Baseline-down y to cell-top edge, in bake px.
    pub cell_offset_y: f32,
    /// Cell edge in bake px (always [`SDF_CELL_PX`]).
    pub cell_px: f32,
    /// Pen advance in bake px.
    pub advance: f32,
}

/// Baked atlas: one R8 texture plus per-char cells and line metrics.
#[derive(Clone, Debug)]
pub struct SdfAtlas {
    /// Square texture edge in px.
    pub size_px: u32,
    /// Row-major R8 SDF (`0` = far outside, `255` = far inside); row 0 is
    /// the visual top of the first cell row.
    pub data: Vec<u8>,
    /// Cell per covered char.
    pub glyphs: HashMap<char, SdfGlyph>,
    /// Bake-px ascent (positive, above baseline).
    pub ascent: f32,
    /// Bake-px descent (positive magnitude, below baseline).
    pub descent: f32,
}

impl SdfAtlas {
    /// Cell for `ch`, if covered.
    #[must_use]
    pub fn glyph(&self, ch: char) -> Option<&SdfGlyph> {
        self.glyphs.get(&ch)
    }

    /// Bake-px y (down-positive from the baseline) of the line middle —
    /// anchor labels here to center them on a world point.
    #[must_use]
    pub fn midline_offset(&self) -> f32 {
        (self.descent - self.ascent) * 0.5
    }
}

/// One laid-out glyph quad, backend-neutral.
#[derive(Clone, Debug)]
pub struct PlacedGlyph {
    /// Atlas UV rect.
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Cell top-left in string px, relative to the string origin placed at
    /// the baseline start.
    pub offset_px: [f32; 2],
    /// Cell size in string px (uniform scale of [`SDF_CELL_PX`]).
    pub size_px: [f32; 2],
}

/// A laid-out string at a concrete px size.
#[derive(Clone, Debug, Default)]
pub struct LaidOutString {
    /// Glyph quads in visual order.
    pub glyphs: Vec<PlacedGlyph>,
    /// Total pen advance in string px.
    pub advance_px: f32,
}

/// Bake the atlas over [`ATLAS_CHARSET`] from the bundled Sans Regular.
#[must_use]
pub fn bake_atlas() -> SdfAtlas {
    bake_charset(ATLAS_CHARSET.chars())
}

/// Bake exactly `chars` (deduplicated, in order) into an atlas.
#[must_use]
pub fn bake_charset(chars: impl Iterator<Item = char>) -> SdfAtlas {
    let font = fontdue::Font::from_bytes(SANS_TTF, fontdue::FontSettings::default())
        .expect("bundled DejaVu Sans parses");
    let line = font
        .horizontal_line_metrics(SDF_BAKE_PX)
        .expect("bundled Sans has horizontal metrics");
    let mut unique: Vec<char> = Vec::new();
    for ch in chars {
        if !unique.contains(&ch) {
            unique.push(ch);
        }
    }
    let cols = unique.len().isqrt().max(1);
    let rows = unique.len().div_ceil(cols);
    let size_px = (cols.max(rows) * SDF_CELL_PX) as u32;
    let mut data = vec![0u8; (size_px * size_px) as usize];
    let mut glyphs = HashMap::new();
    for (index, ch) in unique.iter().enumerate() {
        let (metrics, coverage) = font.rasterize(*ch, SDF_BAKE_PX);
        let sdf = sdf_from_coverage(&coverage, metrics.width, metrics.height);
        let cell_x = (index % cols) * SDF_CELL_PX;
        let cell_y = (index / cols) * SDF_CELL_PX;
        // Bitmap sits PAD inside the cell; the SDF keeps a PAD margin so
        // edge texels never sample a neighbor.
        let dst_x = cell_x + SDF_CELL_PAD;
        let dst_y = cell_y + SDF_CELL_PAD;
        // Empty bitmaps (spaces) keep a blank cell but still record metrics.
        if metrics.width > 0 && metrics.height > 0 {
            for (row, chunk) in sdf.chunks_exact(metrics.width).enumerate().take(metrics.height) {
                let start = (dst_y + row) * size_px as usize + dst_x;
                data[start..start + metrics.width].copy_from_slice(chunk);
            }
        }
        let left = cell_x as f32 / size_px as f32;
        let top = cell_y as f32 / size_px as f32;
        let cell = SDF_CELL_PX as f32 / size_px as f32;
        glyphs.insert(
            *ch,
            SdfGlyph {
                uv_min: [left, top],
                uv_max: [left + cell, top + cell],
                // Pen to bitmap-left is xmin; the cell starts PAD further left.
                cell_offset_x: metrics.xmin as f32 - SDF_CELL_PAD as f32,
                // Bitmap top sits ymin + height above the baseline; the cell
                // starts PAD above that (y-down, hence negated).
                cell_offset_y: -((metrics.ymin + metrics.height as i32) as f32)
                    - SDF_CELL_PAD as f32,
                cell_px: SDF_CELL_PX as f32,
                advance: metrics.advance_width,
            },
        );
    }
    SdfAtlas {
        size_px,
        data,
        glyphs,
        ascent: line.ascent,
        descent: -line.descent,
    }
}

/// Lay out `text` left to right at `px_size` using atlas advances.
/// Uncovered chars are skipped (the atlas audit keeps tick and label
/// strings total — shaping the 2D engine stays the fallback for prose).
#[must_use]
pub fn layout_string(atlas: &SdfAtlas, text: &str, px_size: f32) -> LaidOutString {
    let unit = px_size / SDF_BAKE_PX;
    let mut laid = LaidOutString::default();
    let mut pen = 0.0f32;
    for ch in text.chars() {
        let Some(cell) = atlas.glyph(ch) else {
            continue;
        };
        laid.glyphs.push(PlacedGlyph {
            uv_min: cell.uv_min,
            uv_max: cell.uv_max,
            offset_px: [
                pen + cell.cell_offset_x * unit,
                cell.cell_offset_y * unit,
            ],
            size_px: [cell.cell_px * unit, cell.cell_px * unit],
        });
        pen += cell.advance * unit;
    }
    laid.advance_px = pen;
    laid
}

/// Jump-flooded signed distance field of a coverage bitmap: `0` far
/// outside, `255` far inside, `~128` on the outline. Empty input yields an
/// all-outside field (spaces advance without ink).
#[must_use]
pub fn sdf_from_coverage(coverage: &[u8], width: usize, height: usize) -> Vec<u8> {
    if width == 0 || height == 0 || coverage.len() != width * height {
        return vec![0u8; width * height];
    }
    let inside = |x: usize, y: usize| coverage[y * width + x] > COVERAGE_THRESHOLD;
    // Distance of every inside pixel to outside, and vice versa.
    let to_outside = jump_flood(width, height, |x, y| !inside(x, y));
    let to_inside = jump_flood(width, height, inside);
    (0..coverage.len())
        .map(|index| {
            let (x, y) = (index % width, index / width);
            let signed = if inside(x, y) {
                to_outside[y * width + x]
            } else {
                -to_inside[y * width + x]
            };
            let normalized = 0.5 + signed / SDF_SPREAD_PX;
            (normalized.clamp(0.0, 1.0) * 255.0).round() as u8
        })
        .collect()
}

/// Unsigned distance of every pixel to the nearest seed via jump flooding.
fn jump_flood(width: usize, height: usize, is_seed: impl Fn(usize, usize) -> bool) -> Vec<f32> {
    const INF: f32 = f32::INFINITY;
    // Nearest-seed coordinates per pixel; seeds start at zero distance.
    let mut seed_x = vec![0i32; width * height];
    let mut seed_y = vec![0i32; width * height];
    let mut dist_sq = vec![INF; width * height];
    for y in 0..height {
        for x in 0..width {
            if is_seed(x, y) {
                seed_x[y * width + x] = x as i32;
                seed_y[y * width + x] = y as i32;
                dist_sq[y * width + x] = 0.0;
            }
        }
    }
    let mut step = largest_power_of_two_below(width.max(height).max(2));
    while step >= 1 {
        let delta = step as i32;
        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;
                let mut best = dist_sq[index];
                let mut best_seed = (seed_x[index], seed_y[index]);
                for dy in [-delta, 0, delta] {
                    for dx in [-delta, 0, delta] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let neighbor = ny as usize * width + nx as usize;
                        if dist_sq[neighbor].is_infinite() {
                            continue;
                        }
                        let (sx, sy) = (seed_x[neighbor], seed_y[neighbor]);
                        let candidate = ((x as i32 - sx) * (x as i32 - sx)
                            + (y as i32 - sy) * (y as i32 - sy))
                            as f32;
                        if candidate < best {
                            best = candidate;
                            best_seed = (sx, sy);
                        }
                    }
                }
                dist_sq[index] = best;
                (seed_x[index], seed_y[index]) = best_seed;
            }
        }
        if step == 1 {
            break;
        }
        step /= 2;
    }
    dist_sq.iter().map(|d| d.sqrt()).collect()
}

fn largest_power_of_two_below(n: usize) -> usize {
    let mut step = 1usize;
    while step * 2 < n {
        step *= 2;
    }
    step
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 9x9 bitmap, left 4 columns filled: exact distances are known.
    fn half_filled() -> (Vec<u8>, usize, usize) {
        let (w, h) = (9, 9);
        let mut coverage = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..4 {
                coverage[y * w + x] = 255;
            }
        }
        (coverage, w, h)
    }

    #[test]
    fn sdf_sign_convention_matches_coverage() {
        let (coverage, w, h) = half_filled();
        let sdf = sdf_from_coverage(&coverage, w, h);
        // Deep inside the filled half reads high, far outside reads low,
        // and the outline column sits at the midpoint.
        assert!(sdf[4 * w] > 200, "deep inside");
        assert!(sdf[4 * w + 8] < 55, "far outside");
        let edge = sdf[4 * w + 3] as f32;
        let outside = sdf[4 * w + 4] as f32;
        assert!((edge - 128.0).abs() < 40.0, "outline near midpoint: {edge}");
        assert!((outside - 128.0).abs() < 40.0, "outline near midpoint: {outside}");
        assert!(edge > outside, "inside edge brighter than outside edge");
    }

    #[test]
    fn sdf_empty_input_stays_outside() {
        let sdf = sdf_from_coverage(&[0u8; 64], 8, 8);
        assert!(sdf.iter().all(|&v| v < 128));
        assert!(sdf_from_coverage(&[], 0, 0).is_empty());
    }

    #[test]
    fn sdf_is_deterministic() {
        let (coverage, w, h) = half_filled();
        assert_eq!(sdf_from_coverage(&coverage, w, h), sdf_from_coverage(&coverage, w, h));
    }

    #[test]
    fn atlas_covers_the_tick_charset() {
        let atlas = bake_atlas();
        assert_eq!(atlas.glyphs.len(), ATLAS_CHARSET.chars().count());
        for ch in ATLAS_CHARSET.chars() {
            let cell = atlas.glyph(ch).unwrap_or_else(|| panic!("{ch:?} baked"));
            assert!(cell.advance > 0.0, "{ch:?} advances");
            assert!(cell.uv_max[0] <= 1.0 && cell.uv_max[1] <= 1.0);
        }
        assert!(atlas.ascent > atlas.descent && atlas.descent > 0.0);
        assert_eq!(atlas.data.len(), (atlas.size_px * atlas.size_px) as usize);
    }

    #[test]
    fn atlas_sdf_spans_the_outline() {
        // Across the full charset the field must reach both extremes:
        // far-outside background and deep glyph interiors.
        let atlas = bake_atlas();
        assert!(atlas.glyph('H').is_some());
        let min = *atlas.data.iter().min().unwrap();
        let max = *atlas.data.iter().max().unwrap();
        assert!(min < 90, "far-outside texels exist: {min}");
        assert!(max > 165, "deep-inside texels exist: {max}");
    }

    #[test]
    fn layout_advance_sums_cells() {
        let atlas = bake_atlas();
        let laid = layout_string(&atlas, "100°F", 20.0);
        assert_eq!(laid.glyphs.len(), 5);
        let unit = 20.0 / SDF_BAKE_PX;
        let expected: f32 = ["1", "0", "0", "°", "F"]
            .iter()
            .map(|ch| atlas.glyph(ch.chars().next().unwrap()).unwrap().advance * unit)
            .sum();
        assert!((laid.advance_px - expected).abs() < 1e-4);
        // Offsets march monotonically along the baseline.
        let mut last = f32::NEG_INFINITY;
        for glyph in &laid.glyphs {
            assert!(glyph.offset_px[0] > last);
            last = glyph.offset_px[0];
        }
    }

    #[test]
    fn atlas_bake_is_deterministic() {
        let first = bake_charset(['A', '0', '°'].into_iter());
        let second = bake_charset(['A', '0', '°'].into_iter());
        assert_eq!(first.data, second.data);
    }
}
