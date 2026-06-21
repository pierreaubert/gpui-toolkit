use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::file_type::FileType;
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use font_kit::outline::{Contour, Outline, OutlineBuilder, PointFlags};
use font_kit :: properties :: { Properties } ;
use pathfinder_geometry :: rect :: { RectI } ;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
#[cfg(feature = "source")]
use font_kit::source::SystemSource;
# [cfg (all (feature = "source" , target_family = "windows"))]
use super::check::rasterize_glyph;

static TEST_FONT_FILE_PATH: &str = "resources/tests/eb-garamond/EBGaramond12-Regular.otf";

static TEST_FONT_POSTSCRIPT_NAME: &str = "EBGaramond12-Regular";

static TEST_FONT_COLLECTION_FILE_PATH: &str = "resources/tests/eb-garamond/EBGaramond12.otc";

static TEST_FONT_COLLECTION_POSTSCRIPT_NAME: [&str; 2] =
    ["EBGaramond12-Regular", "EBGaramond12-Italic"];

static FILE_PATH_EB_GARAMOND_TTF: &str = "resources/tests/eb-garamond/EBGaramond12-Regular.ttf";

static FILE_PATH_INCONSOLATA_TTF: &str = "resources/tests/inconsolata/Inconsolata-Regular.ttf";

#[cfg(not(target_os = "linux"))]
static KNOWN_SYSTEM_FONT_NAME: &'static str = "Arial";

#[cfg(target_os = "linux")]
static KNOWN_SYSTEM_FONT_NAME: &str = "DejaVu Sans";

static SFNT_VERSIONS: [[u8; 4]; 4] = [
    [0x00, 0x01, 0x00, 0x00],
    [b'O', b'T', b'T', b'O'],
    [b't', b'r', b'u', b'e'],
    [b't', b'y', b'p', b'1'],
];

const OPENTYPE_TABLE_TAG_HEAD: u32 = 0x68656164;

#[cfg(feature = "source")]
#[test]
pub fn get_font_full_name() {
    let font = SystemSource::new()
        .select_best_match(
            &[FamilyName::Title(KNOWN_SYSTEM_FONT_NAME.to_string())],
            &Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(font.full_name(), KNOWN_SYSTEM_FONT_NAME);
}

#[cfg(feature = "source")]
#[test]
pub fn get_font_full_name_from_lowercase_family_name() {
    let font = SystemSource::new()
        .select_best_match(
            &[FamilyName::Title(
                KNOWN_SYSTEM_FONT_NAME.to_ascii_lowercase(),
            )],
            &Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(font.full_name(), KNOWN_SYSTEM_FONT_NAME);
}

#[test]
pub fn load_font_from_file() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn load_font_from_memory() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let mut font_data = vec![];
    file.read_to_end(&mut font_data).unwrap();
    let font = Font::from_bytes(Arc::new(font_data), 0).unwrap();
    assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn analyze_file() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    assert_eq!(Font::analyze_file(&mut file).unwrap(), FileType::Single);
}

#[test]
pub fn analyze_bytes() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let mut font_data = vec![];
    file.read_to_end(&mut font_data).unwrap();
    assert_eq!(
        Font::analyze_bytes(Arc::new(font_data)).unwrap(),
        FileType::Single
    );
}

#[cfg(all(
    feature = "source",
    not(feature = "loader-freetype-default"),
    not(any(target_os = "macos", target_os = "ios", target_family = "windows"))
))]
#[test]
pub fn get_fully_hinted_glyph_outline() {
    let mut file = File::open(FILE_PATH_INCONSOLATA_TTF).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
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
                        Vector2F::new(100.0, 100.0),
                        Vector2F::new(200.0, 100.0),
                        Vector2F::new(200.0, 400.0),
                        Vector2F::new(100.0, 400.0),
                        Vector2F::new(100.0, 500.0),
                        Vector2F::new(300.0, 500.0),
                        Vector2F::new(300.0, 100.0),
                        Vector2F::new(400.0, 100.0),
                        Vector2F::new(400.0, 0.0),
                        Vector2F::new(100.0, 0.0),
                    ],
                    flags: vec![PointFlags::empty(); 10],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(200.0, 600.0),
                        Vector2F::new(200.0, 600.0),
                        Vector2F::new(200.0, 600.0),
                        Vector2F::new(200.0, 600.0),
                        Vector2F::new(200.0, 600.0),
                        Vector2F::new(200.0, 700.0),
                        Vector2F::new(200.0, 700.0),
                        Vector2F::new(200.0, 700.0),
                        Vector2F::new(200.0, 700.0),
                        Vector2F::new(300.0, 700.0),
                        Vector2F::new(300.0, 700.0),
                        Vector2F::new(300.0, 700.0),
                        Vector2F::new(300.0, 600.0),
                        Vector2F::new(300.0, 600.0),
                        Vector2F::new(300.0, 600.0),
                        Vector2F::new(300.0, 600.0),
                        Vector2F::new(200.0, 600.0),
                    ],
                    flags: vec![
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                    ],
                },
            ],
        }
    );
}

#[test]
pub fn get_empty_glyph_outline() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char(' ').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::None, &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(outline, Outline::new());
}

#[test]
pub fn get_glyph_raster_bounds() {
    let mut file = File::open(FILE_PATH_INCONSOLATA_TTF).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char('J').expect("No glyph for char!");
    let transform = Transform2F::default();
    let size = 32.0;
    let hinting_options = HintingOptions::None;
    let rasterization_options = RasterizationOptions::GrayscaleAa;
    #[cfg(not(target_family = "windows"))]
    let expected_rect = RectI::new(Vector2I::new(1, -20), Vector2I::new(14, 21));
    #[cfg(target_family = "windows")]
    let expected_rect = RectI::new(Vector2I::new(1, -20), Vector2I::new(14, 20));
    assert_eq!(
        font.raster_bounds(
            glyph,
            size,
            transform,
            hinting_options,
            rasterization_options
        ),
        Ok(expected_rect)
    );
}

#[cfg(feature = "source")]
#[test]
pub fn load_font_table() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let head_table = font
        .load_font_table(OPENTYPE_TABLE_TAG_HEAD)
        .expect("Where's the `head` table?");
    assert_eq!(&head_table[12..16], &[0x5f, 0x0f, 0x3c, 0xf5]);
}

#[cfg(feature = "source")]
#[test]
pub fn rasterize_empty_glyph() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char(' ').expect("No glyph for char!");
    let mut canvas = Canvas::new(Vector2I::splat(16), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph,
        16.0,
        Transform2F::default(),
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
}

#[cfg(feature = "source")]
#[test]
pub fn rasterize_empty_glyph_on_empty_canvas() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char(' ').expect("No glyph for char!");
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph,
            size,
            Transform2F::default(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )
        .unwrap();
    let mut canvas = Canvas::new(raster_rect.size(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph,
        size,
        Transform2F::from_translation(-raster_rect.origin().to_f32()),
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
}

#[test]
fn load_fonts_from_opentype_collection() {
    let mut file = File::open(TEST_FONT_COLLECTION_FILE_PATH).unwrap();
    {
        let font = Font::from_file(&mut file, 0).unwrap();
        assert_eq!(
            font.postscript_name().unwrap(),
            TEST_FONT_COLLECTION_POSTSCRIPT_NAME[0]
        );
    }
    let font = Font::from_file(&mut file, 1).unwrap();
    assert_eq!(
        font.postscript_name().unwrap(),
        TEST_FONT_COLLECTION_POSTSCRIPT_NAME[1]
    );
}

#[test]
fn get_glyph_count() {
    let font = Font::from_path(TEST_FONT_FILE_PATH, 0).unwrap();
    assert_eq!(font.glyph_count(), 3084);
}

#[test]
fn get_glyph_outline_eb_garamond_exclam() {
    let mut file = File::open(FILE_PATH_EB_GARAMOND_TTF).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char('!').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::None, &mut outline_builder)
        .unwrap();

    // The TrueType spec doesn't specify the rounding method for midpoints, as far as I can tell.
    // So we are lenient and accept either values rounded down (what Core Text provides if the
    // first point is off-curve, it seems) or precise floating-point values (what our FreeType
    // loader provides).
    let mut outline = outline_builder.into_outline();
    for contour in &mut outline.contours {
        for position in &mut contour.positions {
            *position = position.floor();
        }
    }

    println!("{:#?}", outline);
    assert_eq!(
        outline,
        Outline {
            contours: vec![
                Contour {
                    positions: vec![
                        Vector2F::new(114.0, 598.0),
                        Vector2F::new(114.0, 619.0),
                        Vector2F::new(127.0, 634.0),
                        Vector2F::new(141.0, 649.0),
                        Vector2F::new(161.0, 649.0),
                        Vector2F::new(181.0, 649.0),
                        Vector2F::new(193.0, 634.0),
                        Vector2F::new(206.0, 619.0),
                        Vector2F::new(206.0, 598.0),
                        Vector2F::new(206.0, 526.0),
                        Vector2F::new(176.0, 244.0),
                        Vector2F::new(172.0, 205.0),
                        Vector2F::new(158.0, 205.0),
                        Vector2F::new(144.0, 205.0),
                        Vector2F::new(140.0, 244.0),
                        Vector2F::new(114.0, 491.0),
                        Vector2F::new(114.0, 598.0),
                    ],
                    flags: vec![
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                    ],
                },
                Contour {
                    positions: vec![
                        Vector2F::new(117.0, 88.0),
                        Vector2F::new(135.0, 106.0),
                        Vector2F::new(160.0, 106.0),
                        Vector2F::new(185.0, 106.0),
                        Vector2F::new(202.0, 88.0),
                        Vector2F::new(220.0, 71.0),
                        Vector2F::new(220.0, 46.0),
                        Vector2F::new(220.0, 21.0),
                        Vector2F::new(202.0, 3.0),
                        Vector2F::new(185.0, -14.0),
                        Vector2F::new(160.0, -14.0),
                        Vector2F::new(135.0, -14.0),
                        Vector2F::new(117.0, 3.0),
                        Vector2F::new(100.0, 21.0),
                        Vector2F::new(100.0, 46.0),
                        Vector2F::new(100.0, 71.0),
                        Vector2F::new(117.0, 88.0),
                    ],
                    flags: vec![
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                        PointFlags::CONTROL_POINT_0,
                        PointFlags::empty(),
                    ],
                },
            ],
        }
    );
}

#[allow(non_snake_case)]
#[test]
fn get_glyph_outline_inconsolata_J() {
    let mut file = File::open(FILE_PATH_INCONSOLATA_TTF).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char('J').expect("No glyph for char!");
    let mut outline_builder = OutlineBuilder::new();
    font.outline(glyph, HintingOptions::None, &mut outline_builder)
        .unwrap();

    let outline = outline_builder.into_outline();
    assert_eq!(
        outline,
        Outline {
            contours: vec![Contour {
                positions: vec![
                    Vector2F::new(198.0, -11.0),
                    Vector2F::new(106.0, -11.0),
                    Vector2F::new(49.0, 58.0),
                    Vector2F::new(89.0, 108.0),
                    Vector2F::new(96.0, 116.0),
                    Vector2F::new(101.0, 112.0),
                    Vector2F::new(102.0, 102.0),
                    Vector2F::new(106.0, 95.0),
                    Vector2F::new(110.0, 88.0),
                    Vector2F::new(122.0, 78.0),
                    Vector2F::new(157.0, 51.0),
                    Vector2F::new(196.0, 51.0),
                    Vector2F::new(247.0, 51.0),
                    Vector2F::new(269.5, 86.5),
                    Vector2F::new(292.0, 122.0),
                    Vector2F::new(292.0, 208.0),
                    Vector2F::new(292.0, 564.0),
                    Vector2F::new(172.0, 564.0),
                    Vector2F::new(172.0, 623.0),
                    Vector2F::new(457.0, 623.0),
                    Vector2F::new(457.0, 564.0),
                    Vector2F::new(361.0, 564.0),
                    Vector2F::new(361.0, 209.0),
                    Vector2F::new(363.0, 133.0),
                    Vector2F::new(341.0, 84.0),
                    Vector2F::new(319.0, 35.0),
                    Vector2F::new(281.5, 12.0),
                    Vector2F::new(244.0, -11.0),
                    Vector2F::new(198.0, -11.0),
                ],
                flags: vec![
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                    PointFlags::CONTROL_POINT_0,
                    PointFlags::empty(),
                ],
            }],
        }
    );
}

