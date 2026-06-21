
#[cfg(target_family = "windows")]
pub(super) fn stride_pixel_start(pixels: &[u8]) -> Option<u32> {
    let mut index = 0;
    for x in pixels {
        if *x != 0 {
            return Some(index);
        }
        index += 1;
    }
    None
}

pub(super) fn stripe_width(pixels: &[u8]) -> Option<u32> {
    let mut x = 0;
    // Find the initial empty part.
    while x < pixels.len() && pixels[x] == 0 {
        x += 1
    }
    if x == pixels.len() {
        return None;
    }
    // Find the stripe width.
    let mut stripe_width = 0;
    while x < pixels.len() && pixels[x] != 0 {
        x += 1;
        stripe_width += 1;
    }
    // Find the last empty part.
    while x < pixels.len() && pixels[x] == 0 {
        x += 1;
    }
    assert_eq!(x, pixels.len());
    Some(stripe_width)
}

