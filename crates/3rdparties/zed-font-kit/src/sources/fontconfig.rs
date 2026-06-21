// font-kit/src/sources/fontconfig.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A source that contains the fonts installed on the system, as reported by the Fontconfig
//! library.
//!
//! On macOS and Windows, the Cargo feature `source-fontconfig` can be used to opt into fontconfig
//! support. To prefer it over the native font source (only if you know what you're doing), use the
//! `source-fontconfig-default` feature.

use crate::error::SelectionError;
use crate::family_handle::FamilyHandle;
use crate::family_name::FamilyName;
use crate::handle::Handle;
use crate::properties::Properties;
use crate::source::Source;
use std::any::Any;

mod fontconfig_source;
mod misc;

pub use fontconfig_source::*;

