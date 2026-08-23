use super::misc::QUIET_ZONE;
use gpui::{RenderImage, Rgba};
use image::{Frame, RgbaImage};
use qrcode::types::Color as QrColor;
use std::sync::Arc;

/// Rasterize the QR matrix once in its native module resolution.
///
/// GPUI uploads the retained image to its sprite atlas and performs scale and
/// compositing on the GPU. This keeps QR rendering at one image primitive per
/// frame instead of one quad per dark module.
pub(super) fn rasterize_qr_image(
    colors: &[QrColor],
    modules: usize,
    fg_color: Rgba,
    bg_color: Rgba,
) -> Option<Arc<RenderImage>> {
    let total_modules = modules + QUIET_ZONE * 2;
    if total_modules == 0 || colors.len() < modules.saturating_mul(modules) {
        return None;
    }

    let total = u32::try_from(total_modules).ok()?;
    let rgba = |color: Rgba| {
        [
            (color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    };
    // RenderImage's image bytes are consumed as BGRA by GPUI's atlas.
    let bg = rgba(bg_color);
    let fg = rgba(fg_color);
    let mut pixels = vec![0_u8; total_modules.checked_mul(total_modules)?.checked_mul(4)?];
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[bg[2], bg[1], bg[0], bg[3]]);
    }

    for row in 0..modules {
        for col in 0..modules {
            if colors[row * modules + col] == QrColor::Dark {
                let index = ((row + QUIET_ZONE) * total_modules + col + QUIET_ZONE) * 4;
                pixels[index..index + 4].copy_from_slice(&[fg[2], fg[1], fg[0], fg[3]]);
            }
        }
    }

    RgbaImage::from_raw(total, total, pixels)
        .map(|image| Arc::new(RenderImage::new(vec![Frame::new(image)])))
}

#[cfg(test)]
mod tests {
    use super::rasterize_qr_image;
    use gpui::rgba;
    use qrcode::types::Color as QrColor;

    #[test]
    fn raster_keeps_quiet_zone_and_bgra_module_colors() {
        let image = rasterize_qr_image(&[QrColor::Dark], 1, rgba(0x112233ff), rgba(0xaabbccff))
            .expect("one-module QR should rasterize");
        let bytes = image.as_bytes(0).expect("single image frame");
        // First pixel is quiet-zone background; module starts at row/column 4.
        assert_eq!(&bytes[..4], &[0xcc, 0xbb, 0xaa, 0xff]);
        let module_index = ((4 * 9) + 4) * 4;
        assert_eq!(
            &bytes[module_index..module_index + 4],
            &[0x33, 0x22, 0x11, 0xff]
        );
    }
}
