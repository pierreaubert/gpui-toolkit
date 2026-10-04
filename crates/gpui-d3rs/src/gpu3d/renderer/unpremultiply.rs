pub(super) fn unpremultiply_rgba(pixels: &mut [u8]) {
    for rgba in pixels.as_chunks_mut::<4>().0 {
        let alpha = rgba[3];
        if alpha == 0 || alpha == u8::MAX {
            continue;
        }

        let scale = f32::from(u8::MAX) / f32::from(alpha);
        for channel in &mut rgba[..3] {
            *channel = ((f32::from(*channel) * scale)
                .round()
                .clamp(0.0, f32::from(u8::MAX))) as u8;
        }
    }
}

#[doc(hidden)]
pub fn unpremultiply_rgba_for_testing(pixels: &mut [u8]) {
    unpremultiply_rgba(pixels);
}
