use core_graphics :: base :: { kCGImageAlphaPremultipliedLast } ;
use core_graphics::color_space::CGColorSpace;
use std::f32;
use crate :: canvas :: { Format } ;

pub(super) const TTC_TAG: [u8; 4] = [b't', b't', b'c', b'f'];

pub(super) const OTTO_TAG: [u8; 4] = [b'O', b'T', b'T', b'O'];

pub(super) const OTTO_HEX: u32 = 0x4f54544f; // 'OTTO'

pub(super) const TRUE_HEX: u32 = 0x74727565; // 'true'

pub(super) const TYP1_HEX: u32 = 0x74797031; // 'typ1'

pub(super) const SFNT_HEX: u32 = 0x73666e74; // 'sfnt'

#[allow(non_upper_case_globals)]
pub(super) const kCGImageAlphaOnly: u32 = 7;

pub(crate) static FONT_WEIGHT_MAPPING: [f32; 9] = [-0.7, -0.5, -0.23, 0.0, 0.2, 0.3, 0.4, 0.6, 0.8];

pub(super) fn format_to_cg_color_space_and_image_format(format: Format) -> Option<(CGColorSpace, u32)> {
    match format {
        Format::Rgb24 => {
            // Unsupported by Core Graphics.
            None
        }
        Format::Rgba32 => Some((
            CGColorSpace::create_device_rgb(),
            kCGImageAlphaPremultipliedLast,
        )),
        Format::A8 => Some((CGColorSpace::create_device_gray(), kCGImageAlphaOnly)),
    }
}

