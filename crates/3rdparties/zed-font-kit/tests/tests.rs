use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::file_type::FileType;
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use font_kit::outline::{Contour, Outline, OutlineBuilder, PointFlags};
use font_kit::properties::{Properties, Stretch, Weight};
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
#[cfg(feature = "source")]
use font_kit::source::SystemSource;

#[path = "tests/check.rs"]
mod check;
#[path = "tests/consts.rs"]
mod consts;
#[path = "tests/misc.rs"]
mod misc;

pub use check::*;
pub use consts::*;

#[cfg(feature = "source")]
#[test]
pub fn get_glyph_for_char() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(glyph, 68);
}

#[cfg(all(
    feature = "source",
    any(target_family = "windows", target_os = "macos")
))]
#[test]
pub fn get_glyph_outline() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::None, &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(
        outline,
        Outline {
            contours: vec![
                Contour {
                    positions: vec![
                        Vector2F::new(136.0, 1259.0),
                        Vector2F::new(136.0, 1466.0),
                        Vector2F::new(316.0, 1466.0),
                        Vector2F::new(316.0, 1259.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(136.0, 0.0),
                        Vector2F::new(136.0, 1062.0),
                        Vector2F::new(316.0, 1062.0),
                        Vector2F::new(316.0, 0.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
            ],
        }
    );
}

#[cfg(all(
    feature = "source",
    not(any(target_family = "windows", target_os = "macos", target_os = "ios"))
))]
#[test]
pub fn get_glyph_outline() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::None, &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(
        outline,
        Outline {
            contours: vec![
                Contour {
                    positions: vec![
                        Vector2F::new(193.0, 1120.0),
                        Vector2F::new(377.0, 1120.0),
                        Vector2F::new(377.0, 0.0),
                        Vector2F::new(193.0, 0.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(193.0, 1556.0),
                        Vector2F::new(377.0, 1556.0),
                        Vector2F::new(377.0, 1323.0),
                        Vector2F::new(193.0, 1323.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
            ],
        }
    );
}

#[cfg(all(
    not(any(target_os = "macos", target_os = "ios", target_family = "windows")),
    feature = "loader-freetype-default",
    feature = "source"
))]
#[test]
pub fn get_vertically_hinted_glyph_outline() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::Vertical(16.0), &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(
        outline,
        Outline {
            contours: vec![
                Contour {
                    positions: vec![
                        Vector2F::new(136.0, 1316.0),
                        Vector2F::new(136.0, 1536.0),
                        Vector2F::new(316.0, 1536.0),
                        Vector2F::new(316.0, 1316.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(136.0, 0.0),
                        Vector2F::new(136.0, 1152.0),
                        Vector2F::new(316.0, 1152.0),
                        Vector2F::new(316.0, 0.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
            ],
        }
    );
}

#[cfg(all(
    feature = "source",
    not(feature = "loader-freetype-default"),
    not(any(target_os = "macos", target_os = "ios", target_family = "windows"))
))]
#[test]
pub fn get_vertically_hinted_glyph_outline() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::Vertical(16.0), &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(
        outline,
        Outline {
            contours: vec![
                Contour {
                    positions: vec![
                        Vector2F::new(256.0, 1152.0),
                        Vector2F::new(384.0, 1152.0),
                        Vector2F::new(384.0, 0.0),
                        Vector2F::new(256.0, 0.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(256.0, 1536.0),
                        Vector2F::new(384.0, 1536.0),
                        Vector2F::new(384.0, 1280.0),
                        Vector2F::new(256.0, 1280.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
            ],
        }
    );
}

#[cfg(all(
    not(any(target_os = "macos", target_os = "ios", target_family = "windows")),
    feature = "loader-freetype-default",
    feature = "source"
))]
#[test]
pub fn get_fully_hinted_glyph_outline() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::Full(10.0), &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(
        outline,
        Outline {
            contours: vec![
                Contour {
                    positions: vec![
                        Vector2F::new(137.6, 1228.8),
                        Vector2F::new(137.6, 1433.6),
                        Vector2F::new(316.80002, 1433.6),
                        Vector2F::new(316.80002, 1228.8),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(137.6, 0.0),
                        Vector2F::new(137.6, 1024.0),
                        Vector2F::new(316.80002, 1024.0),
                        Vector2F::new(316.80002, 0.0),
                    ],
                    flags: vec![PointFlags::empty(); 4],
                },
            ],
        }
    );
}

#[cfg(all(
    feature = "source",
    any(target_family = "windows", target_os = "macos", target_os = "ios")
))]
#[test]
pub fn get_glyph_typographic_bounds() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(
        font.typographic_bounds(glyph),
        Ok(RectF::new(
            Vector2F::new(74.0, -24.0),
            Vector2F::new(978.0, 1110.0)
        ))
    );
}

#[cfg(all(
    feature = "source",
    not(any(target_family = "windows", target_os = "macos", target_os = "ios"))
))]
#[test]
pub fn get_glyph_typographic_bounds() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(
        font.typographic_bounds(glyph),
        Ok(RectF::new(
            Vector2F::new(123.0, -29.0),
            Vector2F::new(946.0, 1176.0)
        ))
    );
}

#[cfg(all(feature = "source", target_family = "windows"))]
#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2F::new(1139.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Vector2F::new(74.0, 1898.0)));
}

#[cfg(all(feature = "source", target_os = "macos"))]
#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2F::new(1139.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Vector2F::default()));
}

#[cfg(all(
    feature = "source",
    not(any(target_family = "windows", target_os = "macos", target_os = "ios"))
))]
#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2F::new(1255.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Vector2F::default()));
}

#[cfg(all(
    feature = "source",
    any(target_family = "windows", target_os = "macos")
))]
#[test]
pub fn get_font_metrics() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let metrics = font.metrics();
    assert_eq!(metrics.units_per_em, 2048);
    assert_eq!(metrics.ascent, 1854.0);
    assert_eq!(metrics.descent, -434.0);
    assert_eq!(metrics.line_gap, 67.0);
    assert_eq!(metrics.underline_position, -217.0);
    assert_eq!(metrics.underline_thickness, 150.0);
    assert_eq!(metrics.cap_height, 1467.0);
    assert_eq!(metrics.x_height, 1062.0);

    // Different versions of the font can have different max heights, so ignore that.
    let bounding_box = metrics.bounding_box;
    assert_eq!(bounding_box.origin(), Vector2F::new(-1361.0, -665.0));
    assert_eq!(bounding_box.width(), 5457.0);
}

#[cfg(all(
    feature = "source",
    not(any(target_family = "windows", target_os = "macos", target_os = "ios"))
))]
#[test]
pub fn get_font_metrics() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let metrics = font.metrics();
    assert_eq!(metrics.units_per_em, 2048);
    assert_eq!(metrics.ascent, 1901.0);
    assert_eq!(metrics.descent, -483.0);
    assert_eq!(metrics.line_gap, 0.0); // FIXME(pcwalton): Huh?!
    assert_eq!(metrics.underline_position, -40.0);
    assert_eq!(metrics.underline_thickness, 90.0);
    assert_eq!(metrics.cap_height, 0.0); // FIXME(pcwalton): Huh?!
    assert_eq!(metrics.x_height, 0.0); // FIXME(pcwalton): Huh?!
    assert_eq!(
        metrics.bounding_box,
        RectF::new(
            Vector2F::new(-2090.0, -948.0),
            Vector2F::new(5763.0, 3472.0)
        )
    );
}

#[cfg(feature = "source")]
#[test]
pub fn get_font_properties() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let properties = font.properties();
    assert_eq!(properties.weight, Weight(400.0));
    assert_eq!(properties.stretch, Stretch(1.0));
}

#[cfg(feature = "source")]
#[test]
pub fn font_transform() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 16.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            Transform2F::from_translation(Vector2F::splat(8.0)),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let raster_rect2 = font
        .raster_bounds(
            glyph_id,
            size,
            Transform2F::row_major(3.0, 0.0, 0.0, 3.0, 8.0, 8.0),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    assert!((raster_rect2.width() - raster_rect.width() * 3).abs() <= 3);
    assert!((raster_rect2.height() - raster_rect.height() * 3).abs() <= 3);
    assert!((raster_rect2.origin_x() - ((raster_rect.origin_x() - 8) * 3 + 8)).abs() <= 3);
    assert!((raster_rect2.origin_y() - ((raster_rect.origin_y() - 8) * 3 + 8)).abs() <= 3);
}

