use image::{Frame, RgbaImage};

pub(super) fn surface_image_frame(width: u32, height: u32, mut pixels: Vec<u8>) -> Option<Frame> {
    // WGPU readback is RGBA; GPUI RenderImage frames use BGRA.
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    RgbaImage::from_raw(width, height, pixels).map(Frame::new)
}

#[cfg(test)]
mod image_tests {
    use super::surface_image_frame;

    #[test]
    fn surface_image_preserves_red_blue_and_alpha() {
        let frame = surface_image_frame(2, 1, vec![253, 14, 13, 255, 0, 0, 168, 128])
            .expect("two complete pixels");
        assert_eq!(frame.buffer().as_raw(), &[13, 14, 253, 255, 168, 0, 0, 128]);
    }
}
