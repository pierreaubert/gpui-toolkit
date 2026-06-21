// font-kit/src/loaders/freetype.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A cross-platform loader that uses the FreeType library to load and rasterize fonts.
//!
//! On macOS and Windows, the Cargo feature `loader-freetype-default` can be used to opt into this
//! loader by default.

use byteorder::{BigEndian, ReadBytesExt};
use freetype_sys::{
    ft_sfnt_os2, FT_Byte, FT_Done_Face, FT_Done_FreeType, FT_Error, FT_Face, FT_Fixed,
    FT_Get_Char_Index, FT_Get_Name_Index, FT_Get_Postscript_Name, FT_Get_Sfnt_Name,
    FT_Get_Sfnt_Name_Count, FT_Get_Sfnt_Table, FT_Init_FreeType, FT_Library,
    FT_Library_SetLcdFilter, FT_Load_Glyph, FT_Long, FT_Matrix, FT_New_Memory_Face, FT_Pos,
    FT_Reference_Face, FT_Set_Char_Size, FT_Set_Transform, FT_UInt, FT_ULong, FT_Vector,
    FT_FACE_FLAG_FIXED_WIDTH, FT_LCD_FILTER_DEFAULT, FT_LOAD_DEFAULT, FT_LOAD_MONOCHROME,
    FT_LOAD_NO_HINTING, FT_LOAD_RENDER, FT_LOAD_TARGET_LCD, FT_LOAD_TARGET_LIGHT,
    FT_LOAD_TARGET_MONO, FT_LOAD_TARGET_NORMAL, FT_PIXEL_MODE_GRAY, FT_PIXEL_MODE_LCD,
    FT_PIXEL_MODE_LCD_V, FT_PIXEL_MODE_MONO, FT_STYLE_FLAG_ITALIC, TT_OS2,
};
use log::warn;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_simd::default::F32x4;
use std::f32;
use std::ffi::{CStr, CString};
use std::fmt::{self, Debug, Formatter};
use std::io::{Seek, SeekFrom};
use std::iter;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;
use crate::canvas::{Canvas, Format, RasterizationOptions};
use crate::error::{FontLoadingError, GlyphLoadingError};
use crate::file_type::FileType;
use crate::handle::Handle;
use crate::hinting::HintingOptions;
use crate::loader::{FallbackResult, Loader};
use crate::metrics::Metrics;
use crate::outline::OutlineSink;
use crate::properties::{Properties, Stretch, Style, Weight};
use crate::utils;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

thread_local! {
    static FREETYPE_LIBRARY: FtLibrary = {
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(FT_Init_FreeType(&mut library), 0);
            FT_Library_SetLcdFilter(library, FT_LCD_FILTER_DEFAULT);
            FtLibrary(library)
        }
    };
}

mod consts;
mod f32_to_ft_fixed;
mod font;
mod ft_fixed_to_f32;
mod ft_library;
mod misc;
mod types;

pub use font::*;
pub use types::*;

use types::BDF_PropertyType;

#[repr(transparent)]
struct FtLibrary(FT_Library);

#[repr(C)]
struct BDF_PropertyRec {
    property_type: BDF_PropertyType,
    value: *const c_char,
}

