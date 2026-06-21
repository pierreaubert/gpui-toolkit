use dwrote::Font as DWriteFont;
use dwrote::FontFace as DWriteFontFace;
use pathfinder_geometry :: vector :: { Vector2F } ;
use crate :: outline :: { OutlineBuilder } ;

/// DirectWrite's representation of a font.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct NativeFont {
    /// The native DirectWrite font object.
    pub dwrite_font: DWriteFont,
    /// The native DirectWrite font face object.
    pub dwrite_font_face: DWriteFontFace,
}

pub(super) struct OutlineCanonicalizerInfo {
    pub(super) builder: OutlineBuilder,
    pub(super) last_position: Vector2F,
}

