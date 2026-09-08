pub(super) fn unpremultiply_rgba(pixels: &mut [u8]) {
    for rgba in pixels.as_chunks_mut::<4>().0 {
        let alpha = rgba[3];
        if alpha == 0 || alpha == u8::MAX {
            continue;
        }

        let scale = u8::MAX as f32 / alpha as f32;
        for channel in &mut rgba[..3] {
            *channel = ((*channel as f32 * scale).round().clamp(0.0, u8::MAX as f32)) as u8;
        }
    }
}

#[doc(hidden)]
pub fn unpremultiply_rgba_for_testing(pixels: &mut [u8]) {
    unpremultiply_rgba(pixels);
}
