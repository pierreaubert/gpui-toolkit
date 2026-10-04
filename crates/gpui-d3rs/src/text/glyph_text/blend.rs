use gpui::Rgba;

pub(super) fn blend_pixel(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    color: Rgba,
    alpha: u8,
) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }

    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    let src_a = (f32::from(alpha) / 255.0) * color.a.clamp(0.0, 1.0);
    let dst_a = f32::from(pixels[idx + 3]) / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        return;
    }

    let src = [color.r, color.g, color.b];
    for channel in 0..3 {
        let dst = f32::from(pixels[idx + channel]) / 255.0;
        let out = (src[channel] * src_a + dst * dst_a * (1.0 - src_a)) / out_a;
        pixels[idx + channel] = (out.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    pixels[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
}

pub(super) fn blend_raw_pixel(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    src: &[u8],
) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }

    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    let src_a = f32::from(src[3]) / 255.0;
    let dst_a = f32::from(pixels[idx + 3]) / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        return;
    }

    for channel in 0..3 {
        let src_c = f32::from(src[channel]) / 255.0;
        let dst_c = f32::from(pixels[idx + channel]) / 255.0;
        let out = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        pixels[idx + channel] = (out.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    pixels[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
}
