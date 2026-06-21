// font-kit/src/loaders/core_text.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A loader that uses Apple's Core Text API to load and rasterize fonts.

mod cgpoint_ext;
mod consts;
mod core;
mod font;
mod font_data;
mod misc;
mod piecewise;
mod types;
mod unpack;
#[cfg(test)]
mod tests;

pub(crate) use consts::*;
pub use font::*;
pub(crate) use piecewise::*;
pub use types::*;
