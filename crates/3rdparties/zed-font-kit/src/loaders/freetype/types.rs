use freetype_sys :: { FT_Byte , FT_Error , FT_Face , FT_Long , FT_UInt , FT_ULong } ;
use std::os::raw::{c_char, c_void};
use super::BDF_PropertyRec;

/// The handle that the FreeType API natively uses to represent a font.
pub type NativeFont = FT_Face;

#[allow(non_camel_case_types)]
pub(super) type BDF_PropertyType = i32;

extern "C" {
    pub(super) fn FT_Get_Font_Format(face: FT_Face) -> *const c_char;
    pub(super) fn FT_Get_BDF_Property(
        face: FT_Face,
        prop_name: *const c_char,
        aproperty: *mut BDF_PropertyRec,
    ) -> FT_Error;
    pub(super) fn FT_Get_PS_Font_Value(
        face: FT_Face,
        key: u32,
        idx: FT_UInt,
        value: *mut c_void,
        value_len: FT_Long,
    ) -> FT_Long;
    pub(super) fn FT_Load_Sfnt_Table(
        face: FT_Face,
        tag: FT_ULong,
        offset: FT_Long,
        buffer: *mut FT_Byte,
        length: *mut FT_ULong,
    ) -> FT_Error;
}

