use super::misc::QUIET_ZONE;
use gpui::{RenderImage, Rgba};
use image::{Frame, RgbaImage};
use qrcode::types::Color as QrColor;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Bound for the shared QR raster cache: one entry per distinct
/// (matrix content, colors) combination, evicted wholesale when full,
/// mirroring the workflow canvas connection-path cache.
const QR_RASTER_CACHE_CAPACITY: usize = 32;

/// Cache key for a rasterized QR bitmap. Matrix content is hashed so any
/// change produces a different key and invalidates the old entry. The
/// display size is intentionally absent: the raster is kept at native
/// module resolution and scaled on the GPU at paint time.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct QrRasterCacheKey {
    colors_hash: u64,
    modules: usize,
    fg: [u32; 4],
    bg: [u32; 4],
}

fn qr_cache_key(
    colors: &[QrColor],
    modules: usize,
    fg_color: Rgba,
    bg_color: Rgba,
) -> QrRasterCacheKey {
    let mut colors_hash = 0xcbf2_9ce4_8422_2325_u64;
    for color in colors {
        colors_hash ^= u64::from(*color == QrColor::Dark);
        colors_hash = colors_hash.wrapping_mul(0x100_0000_01b3);
    }
    let channel_bits = |color: Rgba| {
        [
            color.r.to_bits(),
            color.g.to_bits(),
            color.b.to_bits(),
            color.a.to_bits(),
        ]
    };
    QrRasterCacheKey {
        colors_hash,
        modules,
        fg: channel_bits(fg_color),
        bg: channel_bits(bg_color),
    }
}

thread_local! {
    static QR_RASTER_CACHE: RefCell<HashMap<QrRasterCacheKey, Arc<RenderImage>>> =
        RefCell::new(HashMap::new());
}

/// Rasterize the QR matrix through a bounded shared cache.
///
/// Repeated renders of an unchanged code (including every frame of the
/// stateless [`crate::QrCode`]) return the identical bitmap instead of
/// re-encoding pixels, keeping QR painting at one image primitive per frame.
pub(super) fn cached_rasterize_qr_image(
    colors: &[QrColor],
    modules: usize,
    fg_color: Rgba,
    bg_color: Rgba,
) -> Option<Arc<RenderImage>> {
    let key = qr_cache_key(colors, modules, fg_color, bg_color);
    QR_RASTER_CACHE.with(|cache| {
        if let Some(image) = cache.borrow().get(&key) {
            return Some(Arc::clone(image));
        }
        let image = rasterize_qr_image(colors, modules, fg_color, bg_color)?;
        let mut cache = cache.borrow_mut();
        if cache.len() >= QR_RASTER_CACHE_CAPACITY {
            cache.clear();
        }
        cache.insert(key, Arc::clone(&image));
        Some(image)
    })
}

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
    use super::{cached_rasterize_qr_image, rasterize_qr_image};
    use gpui::rgba;
    use qrcode::types::Color as QrColor;
    use std::sync::Arc;

    #[test]
    fn cached_raster_shares_identical_matrices() {
        let colors = vec![QrColor::Dark, QrColor::Light, QrColor::Light, QrColor::Dark];
        let first = cached_rasterize_qr_image(&colors, 2, rgba(0x112233ff), rgba(0xaabbccff))
            .expect("2x2 QR should rasterize");
        let second = cached_rasterize_qr_image(&colors, 2, rgba(0x112233ff), rgba(0xaabbccff))
            .expect("2x2 QR should rasterize");
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn cached_raster_invalidates_on_content_change() {
        let before = vec![QrColor::Dark, QrColor::Light, QrColor::Light, QrColor::Dark];
        let mut after = before.clone();
        after[0] = QrColor::Light;
        let a = cached_rasterize_qr_image(&before, 2, rgba(0x112233ff), rgba(0xaabbccff))
            .expect("2x2 QR should rasterize");
        let b = cached_rasterize_qr_image(&after, 2, rgba(0x112233ff), rgba(0xaabbccff))
            .expect("2x2 QR should rasterize");
        assert!(!Arc::ptr_eq(&a, &b));
        // The flipped module now shows the background color. Total side is
        // 2 modules + quiet zone of 4 on each side.
        let bytes = b.as_bytes(0).expect("single image frame");
        let module_index = (4 * 10 + 4) * 4;
        assert_eq!(
            &bytes[module_index..module_index + 4],
            &[0xcc, 0xbb, 0xaa, 0xff]
        );
    }

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
