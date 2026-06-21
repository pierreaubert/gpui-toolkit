// font-kit/src/loaders/directwrite.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A loader that uses the Windows DirectWrite API to load and rasterize fonts.

use byteorder::{BigEndian, ReadBytesExt};
use dwrote::CustomFontCollectionLoaderImpl;
use dwrote::Font as DWriteFont;
use dwrote::FontCollection as DWriteFontCollection;
use dwrote::FontFace as DWriteFontFace;
use dwrote::FontFallback as DWriteFontFallback;
use dwrote::FontFile as DWriteFontFile;
use dwrote::FontMetrics as DWriteFontMetrics;
use dwrote::FontStyle as DWriteFontStyle;
use dwrote::GlyphOffset as DWriteGlyphOffset;
use dwrote::GlyphRunAnalysis as DWriteGlyphRunAnalysis;
use dwrote::InformationalStringId as DWriteInformationalStringId;
use dwrote::OutlineBuilder as DWriteOutlineBuilder;
use dwrote::{DWRITE_TEXTURE_ALIASED_1x1, DWRITE_TEXTURE_CLEARTYPE_3x1};
use dwrote::{DWRITE_GLYPH_RUN, DWRITE_MEASURING_MODE_NATURAL};
use dwrote::{DWRITE_RENDERING_MODE_ALIASED, DWRITE_RENDERING_MODE_NATURAL};
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use winapi::shared::minwindef::{FALSE, MAX_PATH};
use winapi::um::dwrite::DWRITE_NUMBER_SUBSTITUTION_METHOD_NONE;
use winapi::um::dwrite::DWRITE_READING_DIRECTION;
use winapi::um::dwrite::DWRITE_READING_DIRECTION_LEFT_TO_RIGHT;
use winapi::um::fileapi;
use crate::canvas::{Canvas, Format, RasterizationOptions};
use crate::error::{FontLoadingError, GlyphLoadingError};
use crate::file_type::FileType;
use crate::handle::Handle;
use crate::hinting::HintingOptions;
use crate::loader::{FallbackFont, FallbackResult, Loader};
use crate::metrics::Metrics;
use crate::outline::{OutlineBuilder, OutlineSink};
use crate::properties::{Properties, Stretch, Style, Weight};

mod font;
mod misc;
mod my_text_analysis_source;
mod outline_canonicalizer;
mod types;

pub use font::*;
pub use types::*;

