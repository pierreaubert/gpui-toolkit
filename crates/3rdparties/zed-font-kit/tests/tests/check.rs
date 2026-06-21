use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::hinting::HintingOptions;
use font_kit :: properties :: { Properties } ;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry :: vector :: { Vector2F } ;
#[cfg(feature = "source")]
use font_kit::source::SystemSource;
# [cfg (target_family = "windows")]
use super::misc::stride_pixel_start;
use super::misc::stripe_width;

#[cfg(feature = "source")]
#[test]
pub fn rasterize_glyph_with_grayscale_aa() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            Transform2F::default(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )
        .unwrap();
    let mut canvas = Canvas::new(raster_rect.size(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        Transform2F::from_translation(-raster_rect.origin().to_f32()),
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    check_L_shape(&canvas);
}

#[cfg(feature = "source")]
#[test]
pub fn rasterize_glyph_bilevel() {
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
            Transform2F::default(),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let mut canvas = Canvas::new(raster_rect.size(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        Transform2F::from_translation(-raster_rect.origin().to_f32()),
        HintingOptions::None,
        RasterizationOptions::Bilevel,
    )
    .unwrap();
    assert!(canvas
        .pixels
        .iter()
        .all(|&value| value == 0 || value == 0xff));
    check_L_shape(&canvas);
}

#[cfg(feature = "source")]
#[test]
pub fn rasterize_glyph_bilevel_offset() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            Transform2F::from_translation(Vector2F::new(30.0, 100.0)),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let mut canvas = Canvas::new(raster_rect.size(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        Transform2F::from_translation(-raster_rect.origin().to_f32() + Vector2F::new(30.0, 100.0)),
        HintingOptions::None,
        RasterizationOptions::Bilevel,
    )
    .unwrap();

    assert!(canvas
        .pixels
        .iter()
        .all(|&value| value == 0 || value == 0xff));
    check_L_shape(&canvas);
}

#[cfg(all(
    feature = "source",
    any(
        not(any(target_os = "macos", target_os = "ios", target_family = "windows")),
        feature = "loader-freetype-default"
    )
))]
#[test]
pub fn rasterize_glyph_with_full_hinting() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            Transform2F::default(),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let origin = -raster_rect.origin().to_f32();
    let mut canvas = Canvas::new(raster_rect.size(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        Transform2F::from_translation(origin),
        HintingOptions::Full(size),
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    check_L_shape(&canvas);

    // Make sure the top and bottom (non-blank) rows have some fully black pixels in them.
    let mut top_row = &canvas.pixels[0..canvas.stride];
    if top_row.iter().all(|&value| value == 0) {
        top_row = &canvas.pixels[canvas.stride..(2 * canvas.stride)];
    }

    assert!(top_row.iter().any(|&value| value == 0xff));
    for y in (0..(canvas.size.y() as usize)).rev() {
        let bottom_row = &canvas.pixels[(y * canvas.stride)..((y + 1) * canvas.stride)];
        if bottom_row.iter().all(|&value| value == 0) {
            continue;
        }
        assert!(bottom_row.iter().any(|&value| value == 0xff));
        break;
    }
}

#[cfg(all(feature = "source", target_family = "windows"))]
#[test]
pub fn rasterize_glyph() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('{').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            Transform2F::default(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )
        .unwrap();
    let mut canvas = Canvas::new(raster_rect.size(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        Transform2F::from_translation(-raster_rect.origin().to_f32()),
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    check_curly_shape(&canvas);
}

#[allow(non_snake_case)]
fn check_L_shape(canvas: &Canvas) {
    // Find any empty rows at the start.
    let mut y = 0;
    while y < canvas.size.y() {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        if canvas.pixels[row_start..row_end].iter().any(|&p| p != 0) {
            break;
        }
        y += 1;
    }
    assert!(y < canvas.size.y());

    // Find the top part of the L.
    let mut top_stripe_width = None;
    while y < canvas.size.y() {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        if let Some(stripe_width) = stripe_width(&canvas.pixels[row_start..row_end]) {
            if let Some(top_stripe_width) = top_stripe_width {
                if stripe_width > top_stripe_width {
                    break;
                }
                assert_eq!(stripe_width, top_stripe_width);
            }
            top_stripe_width = Some(stripe_width);
        }
        y += 1;
    }
    assert!(y < canvas.size.y());

    // Find the bottom part of the L.
    let mut bottom_stripe_width = None;
    while y < canvas.size.y() {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        if let Some(stripe_width) = stripe_width(&canvas.pixels[row_start..row_end]) {
            if let Some(bottom_stripe_width) = bottom_stripe_width {
                assert!(bottom_stripe_width > top_stripe_width.unwrap());
                assert_eq!(stripe_width, bottom_stripe_width);
            }
            bottom_stripe_width = Some(stripe_width);
        }
    }

    // Find any empty rows at the end.
    while y < canvas.size.y() {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        if canvas.pixels[row_start..row_end].iter().any(|&p| p != 0) {
            break;
        }
    }

    // Make sure we made it to the end.
    assert_eq!(y, canvas.size.y());
}

#[cfg(target_family = "windows")]
fn check_curly_shape(canvas: &Canvas) {
    let mut y = 0;
    let height = canvas.size.y();
    // check the upper row and the lower rows are symmetrical
    while y < height / 2 {
        let (upper_row_start, upper_row_end) =
            (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        let (lower_row_start, lower_row_end) = (
            canvas.stride * (height - y - 1) as usize,
            canvas.stride * (height - y) as usize,
        );
        let upper_row = &canvas.pixels[upper_row_start..upper_row_end];
        let lower_row = &canvas.pixels[lower_row_start..lower_row_end];
        let top_row_width = stripe_width(upper_row).unwrap();
        let lower_row_width = stripe_width(lower_row).unwrap();
        let upper_row_pixel_start = stride_pixel_start(upper_row).unwrap();
        let lower_row_pixel_start = stride_pixel_start(lower_row).unwrap();
        if top_row_width == lower_row_width {
            // check non-zero pixels start at the same index
            assert_eq!(upper_row_pixel_start, lower_row_pixel_start);
        } else {
            //if not, assert that the difference is not greater than 1
            assert_eq!((lower_row_width as i32 - top_row_width as i32).abs(), 1);
            // and assert that the non-zero pixel index difference is not greater than 1
            assert_eq!(
                (upper_row_pixel_start as i32 - lower_row_pixel_start as i32).abs(),
                1
            );
        }
        y += 1;
    }
}

